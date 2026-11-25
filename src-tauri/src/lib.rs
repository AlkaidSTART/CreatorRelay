use chrono::{DateTime, TimeZone, Utc};
use git2::{Commit, DiffFormat, Repository, Signature, Sort, Tree};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
    time::Instant,
};
use tauri::{AppHandle, Manager, State};
use tempfile::NamedTempFile;
use uuid::Uuid;
use walkdir::WalkDir;

pub mod ai;
pub mod content;
pub mod creator_centers;
pub mod database;
mod diagnostics;
pub mod ecosystem;
mod logging;
pub mod mcp;
pub mod operations;
mod platforms;
pub mod plugins;

mod commands;
mod workspace_support;
pub(crate) use workspace_support::*;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

pub(crate) struct AppState {
    db: Mutex<Connection>,
    history_root: PathBuf,
    platform_session_root: PathBuf,
    data_dir: PathBuf,
    database_path: PathBuf,
    log_path: PathBuf,
    started_at: Instant,
    plugin_runtime: plugins::PluginRuntime,
    mcp_host: mcp::McpHost,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Document {
    id: String,
    title: String,
    path: String,
    updated_at: String,
    preview: String,
    word_count: usize,
    char_count: usize,
    tags: Vec<String>,
    content_hash: String,
    assets: Vec<WorkspaceAsset>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentContent {
    content: String,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceAsset {
    path: String,
    markdown: String,
    mime_type: String,
    size: u64,
    available: bool,
    warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceAssetContent {
    bytes: Vec<u8>,
    mime_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Revision {
    id: String,
    created_at: String,
    label: String,
    preview: String,
    diff: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceInfo {
    path: String,
    name: String,
}

pub(crate) fn active_workspace(conn: &Connection) -> Result<Option<PathBuf>, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'workspace_path'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(value.map(PathBuf::from))
}

pub(crate) fn set_workspace(conn: &Connection, path: &Path) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('workspace_path', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [path.to_string_lossy().to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn parse_frontmatter(content: &str) -> (String, Vec<String>) {
    let mut title = String::new();
    let mut tags = Vec::new();
    if !content.starts_with("---\n") {
        return (title, tags);
    }
    if let Some(end) = content[4..].find("\n---") {
        let mut in_tags = false;
        for line in content[4..4 + end].lines() {
            if let Some(value) = line.strip_prefix("title:") {
                title = value.trim().trim_matches(['"', '\'']).to_string();
                in_tags = false;
            } else if line.trim() == "tags:" {
                in_tags = true;
            } else if in_tags {
                if let Some(tag) = line.trim().strip_prefix("- ") {
                    tags.push(tag.trim().trim_matches(['"', '\'']).to_string());
                } else if !line.starts_with(' ') && !line.starts_with('\t') {
                    in_tags = false;
                }
            }
        }
    }
    (title, tags)
}

pub(crate) fn title_for(content: &str, path: &Path) -> String {
    let (frontmatter_title, _) = parse_frontmatter(content);
    if !frontmatter_title.is_empty() {
        return frontmatter_title;
    }
    content
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("未命名文档")
                .replace('-', " ")
        })
}

pub(crate) fn document_matches_query(
    document: &Document,
    path: &Path,
    query: &str,
) -> Result<bool, String> {
    if query.is_empty() {
        return Ok(true);
    }
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let needle = query.to_lowercase();
    let body = body_for_word_count(&content).to_lowercase();
    Ok(document.title.to_lowercase().contains(&needle)
        || document.preview.to_lowercase().contains(&needle)
        || body.contains(&needle)
        || document
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(&needle)))
}

pub(crate) fn body_for_word_count(content: &str) -> &str {
    if let Some(stripped) = content.strip_prefix("---\n")
        && let Some(end) = stripped.find("\n---")
    {
        return &stripped[end + 5..];
    }
    content
}

pub(crate) fn document_from_file(
    id: String,
    path: &Path,
    workspace: &Path,
) -> Result<Document, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let relative = path
        .strip_prefix(workspace)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    let (frontmatter_title, tags) = parse_frontmatter(&content);
    let title = if frontmatter_title.is_empty() {
        title_for(&content, path)
    } else {
        frontmatter_title
    };
    let body = body_for_word_count(&content);
    let preview = body
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim().starts_with('#'))
        .take(2)
        .collect::<Vec<_>>()
        .join(" ");
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let updated_at = metadata
        .modified()
        .ok()
        .map(chrono::DateTime::<Utc>::from)
        .map(|time| time.to_rfc3339())
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let assets = list_document_assets(workspace, &relative, &content)?;
    Ok(Document {
        id,
        title,
        path: relative,
        updated_at,
        preview,
        word_count: body.split_whitespace().count(),
        char_count: body
            .chars()
            .filter(|character| !character.is_whitespace())
            .count(),
        tags,
        content_hash: digest(&content),
        assets,
    })
}

pub(crate) fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    atomic_write_bytes(path, content.as_bytes())
}

pub(crate) fn atomic_write_bytes(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("目标文件没有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, content).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

pub(crate) fn resolve_document(workspace: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        || relative_path
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("md")
    {
        return Err("无效的工作区文档路径".to_string());
    }
    let workspace_canonical = workspace.canonicalize().map_err(|e| e.to_string())?;
    let candidate = workspace.join(relative_path);
    let mut current = workspace_canonical.clone();
    for component in relative_path.components() {
        let Component::Normal(part) = component else {
            continue;
        };
        current.push(part);
        if fs::symlink_metadata(&current).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err("文档路径不能经过符号链接".to_string());
        }
    }
    if candidate.exists() {
        let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
        if !canonical.starts_with(&workspace_canonical) {
            return Err("文档路径超出工作区范围".to_string());
        }
    }
    Ok(candidate)
}

pub(crate) fn asset_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "pdf" => "application/pdf",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(crate) fn resolve_workspace_asset(
    workspace: &Path,
    document_path: &str,
    asset_path: &str,
) -> Result<PathBuf, String> {
    let document = resolve_document(workspace, document_path)?;
    let relative = Path::new(asset_path);
    let components = relative.components().collect::<Vec<_>>();
    if components.len() != 2
        || components[0].as_os_str() != "assets"
        || !components
            .iter()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err("素材路径必须是文章目录下的 assets/<file> 相对路径".to_string());
    }
    let article_root = document.parent().ok_or("文章目录不可用")?;
    let candidate = article_root.join(relative);
    if candidate.exists() {
        for path in [
            article_root.to_path_buf(),
            article_root.join("assets"),
            candidate.clone(),
        ] {
            if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
                return Err("素材路径不能经过符号链接".to_string());
            }
        }
        let root = workspace.canonicalize().map_err(|e| e.to_string())?;
        let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
        if !canonical.starts_with(root) || !canonical.is_file() {
            return Err("素材路径超出工作区范围或不是文件".to_string());
        }
    }
    Ok(candidate)
}

pub(crate) fn database_document_path(workspace: &Path, relative: &str) -> Result<String, String> {
    let canonical_workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let path = canonical_workspace.join(relative);
    Ok(path.to_string_lossy().to_string())
}

pub(crate) fn normalize_legacy_paths(conn: &Connection, workspace: &Path) -> Result<(), String> {
    let legacy_workspace = active_workspace(conn)?.unwrap_or_else(|| workspace.to_path_buf());
    let mut statement = conn
        .prepare("SELECT id, path FROM documents")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let legacy = rows
        .filter_map(Result::ok)
        .filter(|(_, path)| !Path::new(path).is_absolute())
        .collect::<Vec<_>>();
    drop(statement);
    for (id, path) in legacy {
        let absolute = database_document_path(&legacy_workspace, &path)?;
        conn.execute(
            "UPDATE documents SET path=?1 WHERE id=?2",
            params![absolute, id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) fn sync_index(
    conn: &Connection,
    workspace: &Path,
    history_root: Option<&Path>,
) -> Result<(), String> {
    if !workspace.is_dir() {
        return Err("工作区路径不可用，请重新选择工作区".to_string());
    }
    normalize_legacy_paths(conn, workspace)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut seen_paths = Vec::new();
    for entry in WalkDir::new(workspace.join("articles"))
        .follow_links(false)
        .into_iter()
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !entry.file_type().is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("md")
        {
            continue;
        }
        if fs::symlink_metadata(path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            continue;
        }
        let relative = path
            .strip_prefix(workspace)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let database_path = database_document_path(workspace, &relative)?;
        seen_paths.push(database_path.clone());
        let indexed: Option<(String, String)> = tx
            .query_row(
                "SELECT id, content_hash FROM documents WHERE path = ?1",
                [&database_path],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let id = indexed
            .as_ref()
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let document = document_from_file(id.clone(), path, workspace)?;
        let previous_hash = indexed.as_ref().map(|(_, hash)| hash.as_str());
        if let Some(history_root) = history_root {
            if previous_hash.is_some_and(|hash| hash != document.content_hash) {
                let history = open_history_repository(history_root, workspace)?;
                let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
                ensure_document_history(&history, &tx, &id, &relative, &content)?;
                commit_document_content(&history, &relative, &content, "检测到外部修改", None)?;
            } else if previous_hash.is_none() {
                let history = open_history_repository(history_root, workspace)?;
                let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
                commit_document_content(&history, &relative, &content, "导入工作区文章", None)?;
            }
        }
        tx.execute(
            "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title, updated_at=excluded.updated_at, content_hash=excluded.content_hash",
            params![document.id, database_path, document.title, document.updated_at, document.content_hash],
        )
        .map_err(|e| e.to_string())?;
    }
    let articles_prefix = workspace
        .canonicalize()
        .map_err(|e| e.to_string())?
        .join("articles")
        .to_string_lossy()
        .to_string();
    let mut statement = tx
        .prepare("SELECT id, path FROM documents")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?;
    let stale = rows
        .filter_map(Result::ok)
        .filter(|(_, path)| {
            path.starts_with(&format!("{articles_prefix}/")) && !seen_paths.contains(path)
        })
        .collect::<Vec<_>>();
    drop(statement);
    for (id, _) in stale {
        tx.execute("DELETE FROM documents WHERE id=?1", [&id])
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app_data_dir(app.handle()).map_err(std::io::Error::other)?;
            fs::create_dir_all(&data_dir).map_err(std::io::Error::other)?;
            let database_path = app_db_path(app.handle()).map_err(std::io::Error::other)?;
            let log_path = logging::initialize(&data_dir).map_err(std::io::Error::other)?;
            let mut conn = Connection::open(&database_path).map_err(std::io::Error::other)?;
            let migration = match database::migrate(&mut conn) {
                Ok(report) => report,
                Err(error) => {
                    let _ = logging::append_log(
                        &log_path,
                        "error",
                        "startup.migration",
                        &format!("数据库迁移失败 error={error}"),
                    );
                    eprintln!("CreatorRelay startup migration failed: {error}");
                    return Err(Box::new(std::io::Error::other(format!(
                        "数据库迁移失败：{error}"
                    ))));
                }
            };
            let _ = logging::append_log(
                &log_path,
                "info",
                "startup.ready",
                &format!(
                    "应用初始化完成 schema_from={} schema_to={} applied={:?} data_dir={}",
                    migration.from_version,
                    migration.to_version,
                    migration.applied_versions,
                    data_dir.to_string_lossy()
                ),
            );
            let history_root = data_dir.join("article-history");
            let platform_session_root = data_dir.join("platform-sessions");
            let plugin_root = data_dir.join("plugins");
            let runner_executable = std::env::current_exe()
                .map_err(|error| std::io::Error::other(format!("无法定位应用运行时：{error}")))?;
            let plugin_runtime = plugins::PluginRuntime::new(
                plugin_root,
                data_dir.clone(),
                database_path.clone(),
                runner_executable.clone(),
            )
            .map_err(std::io::Error::other)?;
            let mcp_settings = mcp::get_settings(&conn).unwrap_or_default();
            let mcp_host = mcp::McpHost::new(runner_executable);
            if let Err(error) = mcp_host.start(Some(mcp::McpHostConfig {
                auto_restart: mcp_settings.auto_restart,
                max_restarts: mcp_settings.max_restarts,
            })) {
                let _ = logging::append_log(
                    &log_path,
                    "warn",
                    "startup.mcp",
                    &format!(
                        "本地 MCP Server 启动失败 error={}",
                        mcp::redact_text(&error)
                    ),
                );
            }
            app.manage(AppState {
                db: Mutex::new(conn),
                history_root,
                platform_session_root,
                data_dir,
                database_path,
                log_path,
                started_at: Instant::now(),
                plugin_runtime,
                mcp_host,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::content::list_content_packages,
            commands::content::create_content_package,
            commands::content::read_content_package,
            commands::content::save_content_package,
            commands::content::import_content_asset,
            commands::content::read_content_asset,
            commands::content::derive_content_package,
            commands::content::list_platform_drafts,
            commands::content::create_platform_drafts,
            commands::content::read_platform_draft,
            commands::content::save_platform_draft,
            commands::content::acknowledge_platform_draft,
            commands::content::regenerate_platform_draft,
            commands::content::read_platform_draft_asset,
            commands::ai::list_ai_providers,
            commands::ai::save_ai_provider,
            commands::ai::delete_ai_provider,
            commands::ai::set_default_ai_provider,
            commands::ai::test_ai_provider,
            commands::ai::ai_generate,
            commands::workspace::get_workspace,
            commands::workspace::set_workspace_path,
            commands::workspace::list_documents,
            commands::workspace::rebuild_workspace_index,
            commands::workspace::read_document,
            commands::workspace::import_markdown,
            commands::workspace::import_document_bundle,
            commands::workspace::create_document,
            commands::workspace::save_document,
            commands::workspace::export_document_bundle,
            commands::workspace::import_workspace_asset,
            commands::workspace::list_document_assets_for_document,
            commands::workspace::read_workspace_asset,
            commands::workspace::reveal_workspace_asset,
            commands::workspace::preserve_external_document,
            commands::workspace::list_revisions,
            commands::workspace::create_document_revision,
            commands::workspace::restore_revision,
            commands::platforms::list_platform_catalog,
            commands::platforms::list_platform_accounts,
            commands::platforms::create_platform_account,
            commands::platforms::update_platform_account,
            commands::platforms::begin_platform_authentication,
            commands::platforms::cancel_platform_authentication,
            commands::platforms::complete_platform_authentication,
            commands::platforms::check_platform_account_status,
            commands::platforms::ensure_platform_account_active,
            commands::platforms::preview_platform_account_deletion,
            commands::platforms::delete_platform_account,
            commands::platforms::list_platform_audit_logs,
            commands::plugins::validate_adapter_manifest,
            commands::plugins::list_adapter_catalog,
            commands::plugins::list_plugins,
            commands::plugins::install_official_adapter,
            commands::plugins::save_plugin_permissions,
            commands::plugins::enable_plugin,
            commands::plugins::disable_plugin,
            commands::plugins::restart_plugin,
            commands::plugins::rollback_plugin,
            commands::plugins::probe_plugin_capabilities,
            commands::plugins::simulate_plugin_fault,
            commands::plugins::uninstall_plugin,
            commands::plugins::list_plugin_events,
            commands::plugins::run_plugin_security_self_test,
            commands::plugins::run_publish_payload_self_check,
            commands::plugins::validate_plugin_publish_input,
            commands::operations::create_publish_tasks,
            commands::operations::list_publish_tasks,
            commands::operations::prepare_publish_task,
            commands::operations::confirm_publish_task,
            commands::operations::cancel_publish_task,
            commands::operations::retry_publish_task,
            commands::operations::update_publish_task_schedule,
            commands::operations::resolve_uncertain_publish_task,
            commands::operations::export_manual_publish_package,
            commands::operations::complete_manual_publish_task,
            commands::operations::resume_account_publish_tasks,
            commands::operations::run_due_publish_tasks,
            commands::operations::sync_platform_works,
            commands::operations::list_external_works,
            commands::operations::get_external_work_detail,
            commands::operations::get_analytics_trend,
            commands::operations::get_operations_dashboard,
            commands::operations::list_sync_runs,
            commands::operations::list_publish_task_events,
            commands::operations::get_runtime_diagnostics,
            commands::operations::run_diagnostic_probe,
            commands::ecosystem::get_ecosystem_overview,
            commands::mcp::get_mcp_host_status,
            commands::mcp::start_mcp_server,
            commands::mcp::stop_mcp_server,
            commands::mcp::mcp_health_check,
            commands::creator_centers::list_creator_center_links,
            commands::creator_centers::save_creator_center_link,
            commands::mcp::list_platform_mcp_servers,
            commands::mcp::mcp_list_tools,
            commands::mcp::mcp_cancel_tool,
            commands::mcp::get_mcp_settings,
            commands::mcp::save_mcp_settings,
            commands::mcp::mcp_preview_tool_call,
            commands::mcp::mcp_call_tool,
            commands::mcp::list_mcp_audit,
            commands::mcp::mcp_security_self_test,
            commands::mcp::list_assistant_threads,
            commands::mcp::create_assistant_thread,
            commands::mcp::get_assistant_thread,
            commands::mcp::assistant_plan_message,
            commands::mcp::assistant_context_preview,
            commands::mcp::assistant_generate_reply
        ])
        .run(tauri::generate_context!())
        .expect("error while running CreatorRelay");
}
