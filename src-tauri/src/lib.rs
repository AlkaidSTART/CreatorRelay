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

mod workspace_support;
pub(crate) use workspace_support::*;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;

struct AppState {
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
struct Document {
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
struct DocumentContent {
    content: String,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceAsset {
    path: String,
    markdown: String,
    mime_type: String,
    size: u64,
    available: bool,
    warning: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceAssetContent {
    bytes: Vec<u8>,
    mime_type: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Revision {
    id: String,
    created_at: String,
    label: String,
    preview: String,
    diff: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    path: String,
    name: String,
}

#[tauri::command]
fn list_content_packages(
    state: State<'_, AppState>,
) -> Result<Vec<content::ContentPackageSummary>, String> {
    content::list_content_packages(&require_active_workspace(&state)?)
}

#[tauri::command]
fn create_content_package(
    kind: content::ContentKind,
    title: String,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::create_content_package(&require_active_workspace(&state)?, kind, &title)
}

#[tauri::command]
fn read_content_package(
    package_id: String,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::read_content_package(&require_active_workspace(&state)?, &package_id)
}

#[tauri::command]
fn save_content_package(
    package_id: String,
    input: content::ContentSaveInput,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::save_content_package(&require_active_workspace(&state)?, &package_id, input)
}

#[tauri::command]
fn import_content_asset(
    package_id: String,
    role: String,
    source_name: String,
    mime_type: String,
    bytes: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<content::ContentAsset, String> {
    content::import_content_asset(
        &require_active_workspace(&state)?,
        &package_id,
        &role,
        &source_name,
        &mime_type,
        &bytes,
    )
}

#[tauri::command]
fn read_content_asset(
    package_id: String,
    asset_id: String,
    state: State<'_, AppState>,
) -> Result<content::ContentAssetContent, String> {
    content::read_content_asset(&require_active_workspace(&state)?, &package_id, &asset_id)
}

#[tauri::command]
fn derive_content_package(
    source_id: String,
    target_kind: content::ContentKind,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::derive_content_package(
        &require_active_workspace(&state)?,
        &source_id,
        target_kind,
        title,
    )
}

#[tauri::command]
fn list_platform_drafts(
    content_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<content::PlatformDraft>, String> {
    content::list_platform_drafts(&require_active_workspace(&state)?, content_id.as_deref())
}

#[tauri::command]
fn create_platform_drafts(
    content_id: String,
    targets: Vec<content::DraftTarget>,
    state: State<'_, AppState>,
) -> Result<Vec<content::PlatformDraft>, String> {
    if targets.len() > 12 {
        return Err("一次最多生成 12 个平台稿".to_string());
    }
    content::create_platform_drafts(&require_active_workspace(&state)?, &content_id, &targets)
}

#[tauri::command]
fn read_platform_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::read_platform_draft(&require_active_workspace(&state)?, &draft_id)
}

#[tauri::command]
fn save_platform_draft(
    draft_id: String,
    input: content::DraftSaveInput,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::save_platform_draft(&require_active_workspace(&state)?, &draft_id, input)
}

#[tauri::command]
fn acknowledge_platform_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::acknowledge_platform_draft_source(&require_active_workspace(&state)?, &draft_id)
}

#[tauri::command]
fn regenerate_platform_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::regenerate_platform_draft(&require_active_workspace(&state)?, &draft_id)
}

#[tauri::command]
fn read_platform_draft_asset(
    draft_id: String,
    asset_id: String,
    state: State<'_, AppState>,
) -> Result<content::ContentAssetContent, String> {
    content::read_platform_draft_asset(&require_active_workspace(&state)?, &draft_id, &asset_id)
}

#[tauri::command]
fn list_ai_providers(state: State<'_, AppState>) -> Result<Vec<ai::AiProviderSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::list_providers(&conn)
}

#[tauri::command]
fn save_ai_provider(
    input: ai::AiProviderInput,
    state: State<'_, AppState>,
) -> Result<ai::AiProviderSummary, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::save_provider_with_vault(&conn, &input, &ai::keyring_vault())
}

#[tauri::command]
fn delete_ai_provider(provider_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::delete_provider_with_vault(&conn, &provider_id, &ai::keyring_vault())
}

#[tauri::command]
fn set_default_ai_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ai::AiProviderSummary, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::set_default_provider(&conn, &provider_id)
}

#[tauri::command]
fn test_ai_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ai::AiTestResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::test_provider_with_vault(&conn, &provider_id, &ai::keyring_vault())
}

#[tauri::command]
fn ai_generate(
    request: ai::AiGenerateRequest,
    state: State<'_, AppState>,
) -> Result<ai::AiGenerateResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::generate_with_vault(&conn, &request, &ai::keyring_vault())
}

#[tauri::command]
fn get_mcp_host_status(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    state.mcp_host.status()
}

#[tauri::command]
fn start_mcp_server(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let settings = mcp::get_settings(&conn)?;
    state.mcp_host.start(Some(mcp::McpHostConfig {
        auto_restart: settings.auto_restart,
        max_restarts: settings.max_restarts,
    }))
}

#[tauri::command]
fn stop_mcp_server(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    state.mcp_host.stop()
}

#[tauri::command]
fn mcp_health_check(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    state.mcp_host.health_check()
}

#[tauri::command]
fn list_creator_center_links(
    platform_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<creator_centers::CreatorCenterLink>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    creator_centers::list(&conn, platform_id.as_deref())
}

#[tauri::command]
fn save_creator_center_link(
    platform_id: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<creator_centers::CreatorCenterLink, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    creator_centers::save_url(&conn, &platform_id, &url)
}

#[tauri::command]
fn list_platform_mcp_servers() -> Vec<mcp::PlatformMcpProfile> {
    mcp::platform_mcp_catalog()
}

#[tauri::command]
fn mcp_list_tools(state: State<'_, AppState>) -> Result<Vec<mcp::McpToolDefinition>, String> {
    state.mcp_host.list_tools()
}

#[tauri::command]
fn mcp_cancel_tool(call_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.mcp_host.cancel(&call_id)
}

#[tauri::command]
fn get_mcp_settings(state: State<'_, AppState>) -> Result<mcp::McpSettings, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::get_settings(&conn)
}

#[tauri::command]
fn save_mcp_settings(
    input: mcp::McpSettingsInput,
    state: State<'_, AppState>,
) -> Result<mcp::McpSettings, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let settings = mcp::save_settings(&conn, input)?;
    state.mcp_host.configure(mcp::McpHostConfig {
        auto_restart: settings.auto_restart,
        max_restarts: settings.max_restarts,
    })?;
    Ok(settings)
}

#[tauri::command]
fn mcp_preview_tool_call(
    request: mcp::McpPreviewRequest,
    state: State<'_, AppState>,
) -> Result<mcp::McpAuthorizationPreview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    mcp::preview_tool_call(&conn, &workspace, request)
}

#[tauri::command]
fn mcp_call_tool(
    request: mcp::McpCallRequest,
    state: State<'_, AppState>,
) -> Result<mcp::McpCallResult, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    mcp::call_tool(
        &mut conn,
        &state.plugin_runtime,
        &workspace,
        &state.mcp_host,
        request,
    )
}

#[tauri::command]
fn list_mcp_audit(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<mcp::McpAuditRecord>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::list_audit(&conn, limit.unwrap_or(50))
}

#[tauri::command]
fn mcp_security_self_test() -> mcp::McpSecurityEvidence {
    mcp::security_evidence()
}

#[tauri::command]
fn list_assistant_threads(state: State<'_, AppState>) -> Result<Vec<mcp::AssistantThread>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::list_assistant_threads(&conn)
}

#[tauri::command]
fn create_assistant_thread(
    title: Option<String>,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantThread, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::create_assistant_thread(&conn, title, provider_id)
}

#[tauri::command]
fn get_assistant_thread(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantThreadDetail, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::get_assistant_thread(&conn, &thread_id)
}

#[tauri::command]
fn assistant_plan_message(
    prompt: String,
    thread_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantPlan, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::plan_assistant_message(&conn, &prompt, thread_id.as_deref())
}

#[tauri::command]
fn assistant_context_preview(
    provider_id: Option<String>,
    content_id: Option<String>,
    scope: Option<mcp::ModelContextScope>,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantContextPreview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    let scope = match scope {
        Some(scope) => scope,
        None => mcp::default_context_scope(&conn)?,
    };
    mcp::preview_assistant_context(
        &conn,
        &workspace,
        provider_id.as_deref(),
        content_id.as_deref(),
        &scope,
    )
}

#[tauri::command]
fn assistant_generate_reply(
    request: mcp::AssistantGenerateRequest,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantGenerateResult, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    mcp::generate_assistant_reply(&conn, &workspace, request, &ai::keyring_vault())
}

#[tauri::command]
fn get_ecosystem_overview(
    state: State<'_, AppState>,
) -> Result<ecosystem::EcosystemOverview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    ecosystem::ecosystem_overview(&conn)
}

fn active_workspace(conn: &Connection) -> Result<Option<PathBuf>, String> {
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

fn set_workspace(conn: &Connection, path: &Path) -> Result<(), String> {
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('workspace_path', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [path.to_string_lossy().to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_frontmatter(content: &str) -> (String, Vec<String>) {
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

fn title_for(content: &str, path: &Path) -> String {
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

fn document_matches_query(document: &Document, path: &Path, query: &str) -> Result<bool, String> {
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

fn body_for_word_count(content: &str) -> &str {
    if let Some(stripped) = content.strip_prefix("---\n")
        && let Some(end) = stripped.find("\n---")
    {
        return &stripped[end + 5..];
    }
    content
}

fn document_from_file(id: String, path: &Path, workspace: &Path) -> Result<Document, String> {
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

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    atomic_write_bytes(path, content.as_bytes())
}

fn atomic_write_bytes(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("目标文件没有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, content).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

fn resolve_document(workspace: &Path, relative: &str) -> Result<PathBuf, String> {
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

fn asset_mime_type(path: &Path) -> &'static str {
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

fn resolve_workspace_asset(
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

fn database_document_path(workspace: &Path, relative: &str) -> Result<String, String> {
    let canonical_workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let path = canonical_workspace.join(relative);
    Ok(path.to_string_lossy().to_string())
}

fn normalize_legacy_paths(conn: &Connection, workspace: &Path) -> Result<(), String> {
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

fn sync_index(
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

#[tauri::command]
fn get_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(active_workspace(&conn)?.map(|path| WorkspaceInfo {
        name: path
            .file_name()
            .and_then(|part| part.to_str())
            .unwrap_or("CreatorRelay")
            .to_string(),
        path: path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
fn set_workspace_path(path: String, state: State<'_, AppState>) -> Result<WorkspaceInfo, String> {
    let requested_path = path.clone();
    let result = (|| {
        let path = PathBuf::from(path)
            .canonicalize()
            .map_err(|_| "工作区路径不存在或不可访问".to_string())?;
        if !path.is_dir() {
            return Err("请选择一个文件夹作为工作区".to_string());
        }
        setup_workspace(&path)?;
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        sync_index(&conn, &path, Some(&state.history_root))?;
        set_workspace(&conn, &path)?;
        Ok(WorkspaceInfo {
            name: path
                .file_name()
                .and_then(|part| part.to_str())
                .unwrap_or("CreatorRelay")
                .to_string(),
            path: path.to_string_lossy().to_string(),
        })
    })();
    log_command_outcome(
        &state,
        "ipc.workspace.set",
        &format!("requested_path={requested_path}"),
        &result,
    );
    result
}

#[tauri::command]
fn list_documents(
    query: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Document>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let Some(workspace) = active_workspace(&conn)? else {
        return Ok(Vec::new());
    };
    sync_index(&conn, &workspace, Some(&state.history_root))?;
    let filter = query.unwrap_or_default().trim().to_lowercase();
    let mut output = Vec::new();
    for entry in WalkDir::new(workspace.join("articles"))
        .follow_links(false)
        .into_iter()
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file()
            || entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                != Some("md")
        {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&workspace)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let database_path = database_document_path(&workspace, &relative)?;
        let id: String = conn
            .query_row(
                "SELECT id FROM documents WHERE path = ?1",
                [&database_path],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let document = document_from_file(id, entry.path(), &workspace)?;
        if document_matches_query(&document, entry.path(), &filter)? {
            output.push(document);
        }
    }
    output.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(output)
}

#[tauri::command]
fn rebuild_workspace_index(state: State<'_, AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    sync_index(&conn, &workspace, Some(&state.history_root))?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    usize::try_from(count).map_err(|_| "文档数量超出可表示范围".to_string())
}

#[tauri::command]
fn read_document(path: String, state: State<'_, AppState>) -> Result<DocumentContent, String> {
    let requested_path = path.clone();
    let result = (|| {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
        let path = resolve_document(&workspace, &path)?;
        let content = fs::read_to_string(path).map_err(|_| "文档读取失败".to_string())?;
        Ok(DocumentContent {
            content_hash: digest(&content),
            content,
        })
    })();
    if result.is_err() {
        log_command_outcome(
            &state,
            "ipc.document.read",
            &format!("path={requested_path}"),
            &result,
        );
    }
    result
}

#[tauri::command]
fn import_document_bundle(
    source_path: String,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let source = PathBuf::from(source_path);
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    if !source.is_dir() {
        return Err("请选择已导出的内容包目录".to_string());
    }
    let body_path = source.join("contents/article/body.md");
    let body_metadata = fs::symlink_metadata(&body_path).map_err(|e| e.to_string())?;
    if body_metadata.file_type().is_symlink() || !body_metadata.is_file() {
        return Err("内容包缺少有效的文章正文".to_string());
    }
    if body_metadata.len() > 16 * 1024 * 1024 {
        return Err("文章正文超过 16 MiB 限制".to_string());
    }
    let content = fs::read_to_string(&body_path).map_err(|e| e.to_string())?;
    let mut asset_paths = Vec::new();
    for asset in list_document_assets_in_export(&source, &content)? {
        asset_paths.push(asset);
    }
    let id = Uuid::new_v4().to_string();
    let relative = format!("articles/{id}/article.md");
    let target = resolve_document(&workspace, &relative)?;
    atomic_write(&target, &content)?;
    let target_root = target.parent().ok_or("文章目录无效")?;
    for asset_path in asset_paths {
        let relative_asset = Path::new(&asset_path);
        let components = relative_asset.components().collect::<Vec<_>>();
        if components.len() != 2
            || components[0].as_os_str() != "assets"
            || !components
                .iter()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err("内容包包含无效素材路径".to_string());
        }
        let source_asset = source.join("contents/article").join(relative_asset);
        let metadata = fs::symlink_metadata(&source_asset).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > 256 * 1024 * 1024
        {
            return Err("内容包素材必须是小于 256 MiB 的普通文件".to_string());
        }
        let destination = target_root.join(relative_asset);
        atomic_write_bytes(
            &destination,
            &fs::read(source_asset).map_err(|e| e.to_string())?,
        )?;
    }
    let database_path = database_document_path(&workspace, &relative)?;
    let title = title.unwrap_or_else(|| title_for(&content, &target));
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![id, database_path, title, now, digest(&content)],
    )
    .map_err(|e| e.to_string())?;
    let history = open_history_repository(&state.history_root, &workspace)?;
    commit_document_content(&history, &relative, &content, "导入文章内容包", None)?;
    document_from_file(id, &target, &workspace)
}

fn list_document_assets_in_export(source: &Path, markdown: &str) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    let pattern = regex::Regex::new(r"(?:!\[[^\]]*\]|\[[^\]]+\])\((assets/[^)]+)\)")
        .map_err(|e| e.to_string())?;
    for captures in pattern.captures_iter(markdown) {
        let relative = captures.get(1).ok_or("invalid asset ref")?.as_str();
        if output.iter().any(|path: &String| path == relative) {
            continue;
        }
        let path = Path::new(relative);
        if path.components().count() != 2
            || !path
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
        {
            return Err("内容包素材路径穿越被拒绝".to_string());
        }
        let file = source.join("contents/article").join(path);
        let metadata = fs::symlink_metadata(&file).map_err(|e| format!("内容包素材缺失：{e}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("内容包素材不是普通文件".to_string());
        }
        output.push(relative.to_string());
    }
    Ok(output)
}

#[tauri::command]
fn import_markdown(
    source_name: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let extension = Path::new(&source_name)
        .extension()
        .and_then(|value| value.to_str());
    if !matches!(extension, Some("md" | "markdown")) || content.len() > 16 * 1024 * 1024 {
        return Err("请选择不超过 16 MiB 的 Markdown 文件".to_string());
    }
    let id = Uuid::new_v4().to_string();
    let relative = format!("articles/{id}/article.md");
    let target = resolve_document(&workspace, &relative)?;
    atomic_write(&target, &content)?;
    let history = open_history_repository(&state.history_root, &workspace)?;
    commit_document_content(&history, &relative, &content, "导入 Markdown", None)?;
    let title = title_for(&content, &target);
    let database_path = database_document_path(&workspace, &relative)?;
    conn.execute(
        "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![id, database_path, title, Utc::now().to_rfc3339(), digest(&content)],
    )
    .map_err(|e| e.to_string())?;
    document_from_file(id, &target, &workspace)
}

#[tauri::command]
fn create_document(title: String, state: State<'_, AppState>) -> Result<Document, String> {
    let requested_title = title.trim().to_string();
    let result = (|| {
        if requested_title.is_empty() {
            return Err("标题不能为空".to_string());
        }
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
        let id = Uuid::new_v4().to_string();
        let relative = format!("articles/{id}/article.md");
        let path = resolve_document(&workspace, &relative)?;
        let title_yaml = serde_json::to_string(&requested_title).map_err(|e| e.to_string())?;
        let created_at = Utc::now().to_rfc3339();
        let content = format!(
            "---\nid: {id}\ntitle: {title_yaml}\ncreated_at: {created_at}\nupdated_at: {created_at}\ntags: []\nstatus: draft\n---\n\n# {requested_title}\n\n"
        );
        atomic_write(&path, &content)?;
        let history = open_history_repository(&state.history_root, &workspace)?;
        commit_document_content(&history, &relative, &content, "创建文章", None)?;
        let database_path = database_document_path(&workspace, &relative)?;
        conn.execute(
            "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![id, database_path, requested_title, created_at, digest(&content)],
        )
        .map_err(|e| e.to_string())?;
        document_from_file(id, &path, &workspace)
    })();
    log_command_outcome(
        &state,
        "ipc.document.create",
        &format!("title_length={}", requested_title.chars().count()),
        &result,
    );
    result
}

#[tauri::command]
fn export_document_bundle(
    document_path: String,
    destination_path: String,
    state: State<'_, AppState>,
) -> Result<PathBuf, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let source = resolve_document(&workspace, &document_path)?;
    let source_relative = Path::new(&document_path);
    if source_relative.extension().and_then(|value| value.to_str()) != Some("md") {
        return Err("仅支持导出 Markdown 内容包".to_string());
    }
    let content = fs::read_to_string(&source).map_err(|e| e.to_string())?;
    let entries = list_document_assets(&workspace, &document_path, &content)?;
    if entries.iter().any(|asset| !asset.available) {
        return Err("无法导出：文章包含缺失或不安全的素材引用".to_string());
    }
    let output = PathBuf::from(destination_path);
    if output.exists() {
        return Err("导出目标已存在，请选择新的目标路径".to_string());
    }
    let parent = output.parent().ok_or("导出目标无效")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging = tempfile::Builder::new()
        .prefix("creatorrelay-export-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let content_dir = staging.path().join("contents/article");
    fs::create_dir_all(&content_dir).map_err(|e| e.to_string())?;
    let body_path = content_dir.join("body.md");
    atomic_write(&body_path, &content)?;
    for asset in entries {
        let media_path = resolve_workspace_asset(&workspace, &document_path, &asset.path)?;
        let bytes = fs::read(&media_path).map_err(|e| e.to_string())?;
        let target = content_dir.join(&asset.path);
        atomic_write_bytes(&target, &bytes)?;
    }
    fs::rename(staging.path(), &output).map_err(|e| e.to_string())?;
    Ok(output)
}

#[tauri::command]
fn import_workspace_asset(
    document_path: String,
    source_name: String,
    bytes: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<WorkspaceAsset, String> {
    const MAX_ASSET_BYTES: usize = 256 * 1024 * 1024;
    if bytes.is_empty() || bytes.len() > MAX_ASSET_BYTES {
        return Err("素材必须大于 0 且不超过 256 MiB".to_string());
    }
    let extension = Path::new(&source_name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension.is_empty()
        || extension.len() > 12
        || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err("无法识别素材文件扩展名".to_string());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let document = resolve_document(&workspace, &document_path)?;
    if !document.is_file() {
        return Err("请先保存文章后再导入素材".to_string());
    }
    let database_path = database_document_path(&workspace, &document_path)?;
    let _: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path=?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let name = format!("{}.{}", Uuid::new_v4(), extension);
    let relative = format!("assets/{name}");
    let target = resolve_workspace_asset(&workspace, &document_path, &relative)?;
    atomic_write_bytes(&target, &bytes)?;
    let markdown = if asset_mime_type(&target).starts_with("image/") {
        format!("![素材]({relative})")
    } else {
        format!(
            "[{}]({relative})",
            source_name.replace(['[', ']', '\n', '\r'], "_")
        )
    };
    Ok(WorkspaceAsset {
        path: relative,
        markdown,
        mime_type: asset_mime_type(&target).to_string(),
        size: bytes.len() as u64,
        available: true,
        warning: None,
    })
}

#[tauri::command]
fn list_document_assets_for_document(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceAsset>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let document_path = resolve_document(&workspace, &path)?;
    let database_path = database_document_path(&workspace, &path)?;
    let _: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path=?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let content = fs::read_to_string(document_path).map_err(|e| e.to_string())?;
    list_document_assets(&workspace, &path, &content)
}

#[tauri::command]
fn read_workspace_asset(
    document_path: String,
    asset_path: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceAssetContent, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let path = resolve_workspace_asset(&workspace, &document_path, &asset_path)?;
    let metadata = fs::metadata(&path).map_err(|e| e.to_string())?;
    if metadata.len() > 32 * 1024 * 1024 {
        return Err("预览素材超过 32 MiB 限制".to_string());
    }
    Ok(WorkspaceAssetContent {
        bytes: fs::read(&path).map_err(|e| e.to_string())?,
        mime_type: asset_mime_type(&path).to_string(),
    })
}

#[tauri::command]
fn reveal_workspace_asset(
    document_path: String,
    asset_path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let path = resolve_workspace_asset(&workspace, &document_path, &asset_path)?;
    let canonical = path.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.is_file() {
        return Err("素材不是普通文件".to_string());
    }
    Ok(canonical.to_string_lossy().to_string())
}

#[tauri::command]
fn preserve_external_document(
    path: String,
    state: State<'_, AppState>,
) -> Result<DocumentContent, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let relative = managed_history_path(&path)?.1;
    let database_path = database_document_path(&workspace, &path)?;
    let id: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path=?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let repository = open_history_repository(&state.history_root, &workspace)?;
    ensure_document_history(&repository, &conn, &id, &relative, &current)?;
    commit_document_content(&repository, &relative, &current, "检测到外部版本", None)?;
    Ok(DocumentContent {
        content_hash: digest(&current),
        content: current,
    })
}

#[tauri::command]
fn save_document(
    path: String,
    content: String,
    base_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let requested_path = path.clone();
    let result = (|| {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
        let file = resolve_document(&workspace, &path)?;
        let database_path = database_document_path(&workspace, &path)?;
        let id: String = conn
            .query_row(
                "SELECT id FROM documents WHERE path = ?1",
                [&database_path],
                |row| row.get(0),
            )
            .map_err(|_| "文档索引记录不存在".to_string())?;
        let existing = fs::read_to_string(&file).map_err(|_| "文档读取失败".to_string())?;
        ensure_base_hash(&existing, base_hash.as_deref())?;
        if existing == content {
            return document_from_file(id, &file, &workspace);
        }
        validate_asset_references(&workspace, &path, &content)?;
        atomic_write(&file, &content)?;
        conn.execute(
            "UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4",
            params![
                title_for(&content, &file),
                Utc::now().to_rfc3339(),
                digest(&content),
                id
            ],
        )
        .map_err(|e| e.to_string())?;
        document_from_file(id, &file, &workspace)
    })();
    log_command_outcome(
        &state,
        "ipc.document.save",
        &format!("path={requested_path}"),
        &result,
    );
    result
}

#[tauri::command]
fn list_revisions(path: String, state: State<'_, AppState>) -> Result<Vec<Revision>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    if !file.exists() {
        return Err("文档文件已不存在".to_string());
    }
    let (_, relative) = managed_history_path(&path)?;
    let database_path = database_document_path(&workspace, &path)?;
    let indexed_id: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path=?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let repository = open_history_repository(&state.history_root, &workspace)?;
    ensure_document_history(&repository, &conn, &indexed_id, &relative, &current)?;
    list_document_revisions(&repository, &relative)
}

#[tauri::command]
fn create_document_revision(
    path: String,
    label: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<Revision>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    if !file.exists() {
        return Err("文档文件已不存在".to_string());
    }
    let relative = managed_history_path(&path)?.1;
    let database_path = database_document_path(&workspace, &path)?;
    let id: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path=?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let repository = open_history_repository(&state.history_root, &workspace)?;
    ensure_document_history(&repository, &conn, &id, &relative, &current)?;
    let label = label.unwrap_or_else(|| "手动保存版本".to_string());
    if label.trim().is_empty() || label.len() > 120 {
        return Err("版本说明需为 1 至 120 个字符".to_string());
    }
    commit_document_content(&repository, &relative, &current, label.trim(), None)?;
    list_document_revisions(&repository, &relative)
}

#[tauri::command]
fn restore_revision(
    path: String,
    revision_id: String,
    base_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    if !file.exists() {
        return Err("文档文件已不存在".to_string());
    }
    let relative = managed_history_path(&path)?.1;
    let database_path = database_document_path(&workspace, &path)?;
    let id: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path=?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    ensure_base_hash(&current, base_hash.as_deref())?;
    let repository = open_history_repository(&state.history_root, &workspace)?;
    ensure_document_history(&repository, &conn, &id, &relative, &current)?;
    commit_document_content(&repository, &relative, &current, "恢复前保留当前版本", None)?;
    let oid = git2::Oid::from_str(&revision_id).map_err(|e| e.message().to_string())?;
    let revision = repository
        .find_commit(oid)
        .map_err(|_| "找不到所选版本".to_string())?;
    let tree = revision.tree().map_err(|e| e.message().to_string())?;
    let blob_id = tree_blob_id(&tree, &relative).ok_or("所选版本不包含此文章")?;
    let restored_blob = repository
        .find_blob(blob_id)
        .map_err(|e| e.message().to_string())?;
    let restored = std::str::from_utf8(restored_blob.content())
        .map_err(|_| "所选版本不是有效 UTF-8 Markdown")?;
    atomic_write(&file, restored)?;
    let now = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    commit_document_content(&repository, &relative, &now, "恢复选定版本", None)?;
    conn.execute(
        "UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4",
        params![
            title_for(&now, &file),
            Utc::now().to_rfc3339(),
            digest(&now),
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    document_from_file(id, &file, &workspace)
}

#[tauri::command]
fn list_platform_catalog() -> Vec<platforms::PlatformCatalogEntry> {
    platforms::platform_catalog()
}

#[tauri::command]
fn list_platform_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<platforms::PlatformAccount>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::list_accounts(&conn)
}

#[tauri::command]
fn create_platform_account(
    platform_id: String,
    alias: String,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::create_account(
        &mut conn,
        &state.platform_session_root,
        &platform_id,
        &alias,
    )
}

#[tauri::command]
fn update_platform_account(
    account_id: String,
    alias: Option<String>,
    enabled: Option<bool>,
    is_default: Option<bool>,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::update_account(&mut conn, &account_id, alias, enabled, is_default)
}

#[tauri::command]
fn begin_platform_authentication(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::AuthChallenge, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::begin_authentication(&mut conn, &account_id)
}

#[tauri::command]
fn cancel_platform_authentication(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::cancel_authentication(&mut conn, &account_id)
}

#[tauri::command]
fn complete_platform_authentication(
    account_id: String,
    challenge_id: String,
    credential: String,
    state: State<'_, AppState>,
) -> Result<platforms::AccountCheckResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::complete_authentication(
        &mut conn,
        &platforms::KeyringVault,
        &platforms::PlatformHttpVerifier::default(),
        &account_id,
        &challenge_id,
        &credential,
    )
}

#[tauri::command]
fn check_platform_account_status(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::AccountCheckResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::check_account_status(
        &mut conn,
        &platforms::KeyringVault,
        &platforms::PlatformHttpVerifier::default(),
        &account_id,
    )
}

#[tauri::command]
fn ensure_platform_account_active(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::require_active_account(&conn, &account_id)
}

#[tauri::command]
fn preview_platform_account_deletion(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::DeletionPreview, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::deletion_preview(&conn, &state.platform_session_root, &account_id)
}

#[tauri::command]
fn delete_platform_account(
    account_id: String,
    confirmation: platforms::DeletionConfirmation,
    state: State<'_, AppState>,
) -> Result<platforms::DeletionResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::delete_account(
        &mut conn,
        &platforms::KeyringVault,
        &state.platform_session_root,
        &account_id,
        &confirmation,
    )
}

#[tauri::command]
fn list_platform_audit_logs(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<platforms::AccountAuditLog>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::list_audit_logs(&conn, limit.unwrap_or(50))
}

#[tauri::command]
fn validate_adapter_manifest(manifest_json: String) -> plugins::ManifestValidation {
    plugins::validate_manifest(&manifest_json)
}

#[tauri::command]
fn list_adapter_catalog(
    state: State<'_, AppState>,
) -> Result<Vec<plugins::AdapterCatalogEntry>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    plugins::list_catalog(&conn, &state.plugin_runtime)
}

#[tauri::command]
fn list_plugins(state: State<'_, AppState>) -> Result<Vec<plugins::PluginSummary>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    plugins::list_plugins(&conn, &state.plugin_runtime)
}

#[tauri::command]
fn install_official_adapter(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::install_official_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.install",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn save_plugin_permissions(
    plugin_id: String,
    decisions: Vec<plugins::PluginPermissionDecision>,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result =
        plugins::set_plugin_permissions(&mut conn, &state.plugin_runtime, &plugin_id, &decisions);
    log_command_outcome(
        &state,
        "plugin.permissions",
        &format!("plugin={plugin_id} decisions={}", decisions.len()),
        &result,
    );
    result
}

#[tauri::command]
fn enable_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::enable_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.enable",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn disable_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::disable_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.disable",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn restart_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::restart_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.restart",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn rollback_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::rollback_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.rollback",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn probe_plugin_capabilities(
    plugin_id: String,
    account_ref: Option<String>,
    state: State<'_, AppState>,
) -> Result<plugins::PluginProbeResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::probe_plugin_capabilities(
        &mut conn,
        &state.plugin_runtime,
        &plugin_id,
        account_ref,
    );
    if let Ok(probe) = &result
        && !probe.ok
    {
        let _ = logging::append_log(
            &state.log_path,
            "error",
            "plugin.probe_failed",
            &format!(
                "plugin={} account={} error={}",
                plugin_id,
                probe.account_ref,
                probe.error.as_deref().unwrap_or("unknown")
            ),
        );
    } else {
        log_command_outcome(
            &state,
            "plugin.probe",
            &format!("plugin={plugin_id}"),
            &result,
        );
    }
    result
}

#[tauri::command]
fn simulate_plugin_fault(
    plugin_id: String,
    fault: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginFaultResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result =
        plugins::simulate_plugin_fault(&mut conn, &state.plugin_runtime, &plugin_id, &fault);
    match &result {
        Ok(fault_result) => {
            let _ = logging::append_log(
                &state.log_path,
                "error",
                "plugin.fault_detected",
                &format!(
                    "plugin={} fault={} error={}",
                    plugin_id, fault, fault_result.error
                ),
            );
        }
        Err(error) => {
            let _ = logging::append_log(
                &state.log_path,
                "error",
                "plugin.fault",
                &format!("plugin={plugin_id} fault={fault} error={error}"),
            );
        }
    }
    result
}

#[tauri::command]
fn uninstall_plugin(
    plugin_id: String,
    delete_plugin_data: bool,
    state: State<'_, AppState>,
) -> Result<plugins::PluginUninstallResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::uninstall_plugin(
        &mut conn,
        &state.plugin_runtime,
        &plugin_id,
        delete_plugin_data,
    );
    log_command_outcome(
        &state,
        "plugin.uninstall",
        &format!("plugin={plugin_id} deleteData={delete_plugin_data}"),
        &result,
    );
    result
}

#[tauri::command]
fn list_plugin_events(
    plugin_id: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<plugins::PluginEvent>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    plugins::list_plugin_events(&conn, plugin_id.as_deref(), limit)
}

#[tauri::command]
fn run_plugin_security_self_test(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::SecuritySelfTestResult, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区后再运行安全边界自检")?;
    let canary = workspace.join(format!(".creatorrelay-sandbox-canary-{}", Uuid::new_v4()));
    fs::write(&canary, b"test-only-workspace-canary")
        .map_err(|error| format!("无法创建工作区测试哨兵：{error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&canary, fs::Permissions::from_mode(0o600));
    }
    let result = plugins::security_self_test(
        &conn,
        &state.plugin_runtime,
        &plugin_id,
        Some(canary.clone()),
    );
    let _ = fs::remove_file(&canary);
    log_command_outcome(
        &state,
        "plugin.security_self_test",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn run_publish_payload_self_check() -> Vec<plugins::PublishPayloadCheckResult> {
    plugins::payload_self_check()
}

#[tauri::command]
fn validate_plugin_publish_input(
    plugin_id: String,
    account_ref: Option<String>,
    input: plugins::PublishInput,
    state: State<'_, AppState>,
) -> Result<plugins::PluginPublishValidationResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::validate_plugin_publish_input(
        &mut conn,
        &state.plugin_runtime,
        &plugin_id,
        account_ref,
        &input,
    );
    log_command_outcome(
        &state,
        "plugin.validate_publish",
        &format!("plugin={plugin_id} kind={}", input.content_kind.as_str()),
        &result,
    );
    result
}

#[tauri::command]
fn create_publish_tasks(
    input: operations::CreatePublishTasksInput,
    state: State<'_, AppState>,
) -> Result<operations::CreatePublishTasksResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区后再创建发布任务")?;
    let result = operations::create_publish_tasks(&mut conn, &workspace, &input);
    log_command_outcome(
        &state,
        "publishing.create_tasks",
        &format!(
            "source={}:{} targets={}",
            input.source_type as u8,
            input.source_id,
            input.targets.len()
        ),
        &result,
    );
    result
}

#[tauri::command]
fn list_publish_tasks(state: State<'_, AppState>) -> Result<Vec<operations::PublishTask>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_publish_tasks(&conn)
}

#[tauri::command]
fn prepare_publish_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<operations::PublishPreview, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let result =
        operations::prepare_publish_task(&mut conn, &state.plugin_runtime, &workspace, &task_id);
    log_command_outcome(
        &state,
        "publishing.prepare",
        &format!("task={task_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn confirm_publish_task(
    task_id: String,
    confirmation_token: String,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let result = operations::confirm_publish_task(
        &mut conn,
        &state.plugin_runtime,
        &workspace,
        &task_id,
        &confirmation_token,
    );
    log_command_outcome(
        &state,
        "publishing.confirm",
        &format!("task={task_id}"),
        &result,
    );
    result
}

#[tauri::command]
fn cancel_publish_task(
    task_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::cancel_publish_task(&conn, &task_id, reason.as_deref())
}

#[tauri::command]
fn retry_publish_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::retry_publish_task(&conn, &task_id)
}

#[tauri::command]
fn update_publish_task_schedule(
    task_id: String,
    scheduled_at: Option<String>,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::update_publish_task_schedule(&conn, &task_id, scheduled_at.as_deref())
}

#[tauri::command]
fn resolve_uncertain_publish_task(
    task_id: String,
    input: operations::ResolveUncertainInput,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::resolve_uncertain_publish_task(&mut conn, &task_id, &input)
}

#[tauri::command]
fn export_manual_publish_package(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<operations::ManualPublishPackage, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    operations::export_manual_publish_package(&conn, &workspace, &task_id)
}

#[tauri::command]
fn complete_manual_publish_task(
    task_id: String,
    input: operations::ManualCompletionInput,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::complete_manual_publish_task(&mut conn, &task_id, &input)
}

#[tauri::command]
fn resume_account_publish_tasks(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTask>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::resume_account_tasks(&conn, &account_id)
}

#[tauri::command]
fn run_due_publish_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTask>, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    operations::run_due_publish_tasks(&mut conn, &state.plugin_runtime, &workspace)
}

#[tauri::command]
fn sync_platform_works(
    input: operations::SyncPlatformWorksInput,
    state: State<'_, AppState>,
) -> Result<operations::SyncRunResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = operations::sync_platform_works(&mut conn, &state.plugin_runtime, &input);
    log_command_outcome(
        &state,
        "operations.sync",
        &format!("account={} adapter={}", input.account_id, input.adapter_id),
        &result,
    );
    result
}

#[tauri::command]
fn list_external_works(
    filter: Option<operations::ExternalWorkFilter>,
    state: State<'_, AppState>,
) -> Result<Vec<operations::ExternalWork>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_external_works(&conn, &filter.unwrap_or_default())
}

#[tauri::command]
fn get_external_work_detail(
    work_id: String,
    state: State<'_, AppState>,
) -> Result<operations::ExternalWork, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::external_work(&conn, &work_id)
}

#[tauri::command]
fn get_analytics_trend(
    query: operations::AnalyticsQuery,
    state: State<'_, AppState>,
) -> Result<operations::AnalyticsResult, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::analytics_trend(&conn, &query)
}

#[tauri::command]
fn get_operations_dashboard(
    filter: Option<operations::DashboardFilter>,
    state: State<'_, AppState>,
) -> Result<operations::OperationsDashboard, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::operations_dashboard(&conn, &filter.unwrap_or_default())
}

#[tauri::command]
fn list_sync_runs(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<operations::SyncRunResult>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_sync_runs(&conn, limit.unwrap_or(20).min(200))
}

#[tauri::command]
fn list_publish_task_events(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTaskEvent>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_publish_task_events(&conn, &task_id)
}

#[tauri::command]
fn get_runtime_diagnostics(
    state: State<'_, AppState>,
) -> Result<diagnostics::RuntimeDiagnostics, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?;
    diagnostics::collect(
        &conn,
        &state.data_dir,
        &state.database_path,
        &state.log_path,
        workspace,
        state.started_at.elapsed(),
    )
}

#[tauri::command]
fn run_diagnostic_probe(
    probe: String,
    request_context: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    diagnostics::run_probe(&conn, &state.log_path, &probe, request_context.as_deref())
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
            list_content_packages,
            create_content_package,
            read_content_package,
            save_content_package,
            import_content_asset,
            read_content_asset,
            derive_content_package,
            list_platform_drafts,
            create_platform_drafts,
            read_platform_draft,
            save_platform_draft,
            acknowledge_platform_draft,
            regenerate_platform_draft,
            read_platform_draft_asset,
            list_ai_providers,
            save_ai_provider,
            delete_ai_provider,
            set_default_ai_provider,
            test_ai_provider,
            ai_generate,
            get_workspace,
            set_workspace_path,
            list_documents,
            rebuild_workspace_index,
            read_document,
            import_markdown,
            import_document_bundle,
            create_document,
            save_document,
            export_document_bundle,
            import_workspace_asset,
            list_document_assets_for_document,
            read_workspace_asset,
            reveal_workspace_asset,
            preserve_external_document,
            list_revisions,
            create_document_revision,
            restore_revision,
            list_platform_catalog,
            list_platform_accounts,
            create_platform_account,
            update_platform_account,
            begin_platform_authentication,
            cancel_platform_authentication,
            complete_platform_authentication,
            check_platform_account_status,
            ensure_platform_account_active,
            preview_platform_account_deletion,
            delete_platform_account,
            list_platform_audit_logs,
            validate_adapter_manifest,
            list_adapter_catalog,
            list_plugins,
            install_official_adapter,
            save_plugin_permissions,
            enable_plugin,
            disable_plugin,
            restart_plugin,
            rollback_plugin,
            probe_plugin_capabilities,
            simulate_plugin_fault,
            uninstall_plugin,
            list_plugin_events,
            run_plugin_security_self_test,
            run_publish_payload_self_check,
            validate_plugin_publish_input,
            create_publish_tasks,
            list_publish_tasks,
            prepare_publish_task,
            confirm_publish_task,
            cancel_publish_task,
            retry_publish_task,
            update_publish_task_schedule,
            resolve_uncertain_publish_task,
            export_manual_publish_package,
            complete_manual_publish_task,
            resume_account_publish_tasks,
            run_due_publish_tasks,
            sync_platform_works,
            list_external_works,
            get_external_work_detail,
            get_analytics_trend,
            get_operations_dashboard,
            list_sync_runs,
            list_publish_task_events,
            get_runtime_diagnostics,
            run_diagnostic_probe,
            get_ecosystem_overview,
            get_mcp_host_status,
            start_mcp_server,
            stop_mcp_server,
            mcp_health_check,
            list_creator_center_links,
            save_creator_center_link,
            list_platform_mcp_servers,
            mcp_list_tools,
            mcp_cancel_tool,
            get_mcp_settings,
            save_mcp_settings,
            mcp_preview_tool_call,
            mcp_call_tool,
            list_mcp_audit,
            mcp_security_self_test,
            list_assistant_threads,
            create_assistant_thread,
            get_assistant_thread,
            assistant_plan_message,
            assistant_context_preview,
            assistant_generate_reply
        ])
        .run(tauri::generate_context!())
        .expect("error while running CreatorRelay");
}
