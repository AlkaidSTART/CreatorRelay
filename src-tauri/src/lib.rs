use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};
use tempfile::NamedTempFile;
use uuid::Uuid;
use walkdir::WalkDir;

struct AppState {
    db: Mutex<Connection>,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentContent {
    content: String,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Revision {
    id: i64,
    created_at: String,
    label: String,
    preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceInfo {
    path: String,
    name: String,
}

fn app_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("creatorrelay.sqlite3"))
}

fn setup_workspace(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path.join("articles")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("assets")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("exports")).map_err(|e| e.to_string())?;
    Ok(())
}

fn active_workspace(conn: &Connection) -> Result<Option<PathBuf>, String> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM app_settings WHERE key = 'workspace_path'",
            [],
            |r| r.get(0),
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

fn body_for_word_count(content: &str) -> String {
    if content.starts_with("---\n") {
        if let Some(end) = content[4..].find("\n---") {
            return content[4 + end + 4..].to_string();
        }
    }
    content.to_string()
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
    let word_count = body.split_whitespace().count();
    Ok(Document {
        id,
        title,
        path: relative,
        updated_at,
        preview,
        word_count,
        char_count: body.chars().filter(|c| !c.is_whitespace()).count(),
        tags,
        content_hash: format!("{:x}", Sha256::digest(content.as_bytes())),
    })
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("目标文件没有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, content.as_bytes()).map_err(|e| e.to_string())?;
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
        || relative_path.extension().and_then(|ext| ext.to_str()) != Some("md")
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

fn create_revision(
    conn: &Connection,
    document_id: &str,
    content: &str,
    label: &str,
) -> Result<(), String> {
    let digest = format!("{:x}", Sha256::digest(content.as_bytes()));
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO revisions(document_id, content, content_hash, label, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![document_id, content, digest, label, created_at],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM revisions WHERE document_id = ?1 AND id NOT IN
         (SELECT id FROM revisions WHERE document_id = ?1 ORDER BY id DESC LIMIT 30)",
        [document_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn sync_index(conn: &Connection, workspace: &Path) -> Result<(), String> {
    if !workspace.is_dir() {
        return Err("工作区路径不可用，请重新选择工作区".to_string());
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    for entry in WalkDir::new(workspace.join("articles"))
        .follow_links(false)
        .into_iter()
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        let relative = path
            .strip_prefix(workspace)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        seen.insert(relative.clone());
        let indexed: Option<(String, String)> = tx
            .query_row(
                "SELECT id, content_hash FROM documents WHERE path = ?1",
                [&relative],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let id = indexed
            .as_ref()
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let doc = document_from_file(id.clone(), path, workspace)?;
        let changed =
            indexed.as_ref().map(|(_, hash)| hash.as_str()) != Some(doc.content_hash.as_str());
        tx.execute(
            "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title, updated_at=excluded.updated_at, content_hash=excluded.content_hash",
            params![doc.id, doc.path, doc.title, doc.updated_at, doc.content_hash],
        )
        .map_err(|e| e.to_string())?;
        if changed {
            let current = fs::read_to_string(path).map_err(|e| e.to_string())?;
            let latest_hash: Option<String> = tx
                .query_row(
                    "SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1",
                    [&id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if latest_hash.as_deref() != Some(doc.content_hash.as_str()) {
                create_revision(
                    &tx,
                    &id,
                    &current,
                    if indexed.is_some() {
                        "检测到外部修改"
                    } else {
                        "导入"
                    },
                )?;
            }
        }
    }
    let indexed: Vec<(String, String)> = {
        let mut statement = tx
            .prepare("SELECT id, path FROM documents")
            .map_err(|e| e.to_string())?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    };
    for (id, path) in indexed {
        if !seen.contains(&path) {
            tx.execute("DELETE FROM documents WHERE id = ?1", [id])
                .map_err(|e| e.to_string())?;
        }
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
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !path.is_dir() {
        return Err("请选择一个文件夹作为工作区".to_string());
    }
    setup_workspace(&path)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    sync_index(&conn, &path)?;
    set_workspace(&conn, &path)?;
    Ok(WorkspaceInfo {
        name: path
            .file_name()
            .and_then(|part| part.to_str())
            .unwrap_or("CreatorRelay")
            .to_string(),
        path: path.to_string_lossy().to_string(),
    })
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
    sync_index(&conn, &workspace)?;
    let filter = query.unwrap_or_default().trim().to_lowercase();
    let mut output = Vec::new();
    for entry in WalkDir::new(workspace.join("articles"))
        .follow_links(false)
        .into_iter()
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|e| e.to_str()) != Some("md")
        {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(&workspace)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let id: String = conn
            .query_row(
                "SELECT id FROM documents WHERE path = ?1",
                [&relative],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let doc = document_from_file(id, entry.path(), &workspace)?;
        if filter.is_empty()
            || doc.title.to_lowercase().contains(&filter)
            || doc.preview.to_lowercase().contains(&filter)
            || doc
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&filter))
        {
            output.push(doc);
        }
    }
    output.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(output)
}

#[tauri::command]
fn read_document(path: String, state: State<'_, AppState>) -> Result<DocumentContent, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let path = resolve_document(&workspace, &path)?;
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    Ok(DocumentContent {
        content,
        content_hash,
    })
}

#[tauri::command]
fn create_document(title: String, state: State<'_, AppState>) -> Result<Document, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("标题不能为空".to_string());
    }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let id = Uuid::new_v4().to_string();
    let folder = id.clone();
    let relative = format!("articles/{folder}/article.md");
    let path = resolve_document(&workspace, &relative)?;
    if path.exists() {
        return Err("同名文章已存在，请使用不同标题".to_string());
    }
    let title_yaml = serde_json::to_string(title).map_err(|e| e.to_string())?;
    let created_at = Utc::now().to_rfc3339();
    let content = format!(
        "---\nid: {id}\ntitle: {title_yaml}\ncreated_at: {created_at}\nupdated_at: {created_at}\ntags: []\nstatus: draft\n---\n\n# {title}\n\n"
    );
    atomic_write(&path, &content)?;
    conn.execute("INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)", params![id, relative, title, Utc::now().to_rfc3339(), format!("{:x}", Sha256::digest(content.as_bytes()))]).map_err(|e| e.to_string())?;
    create_revision(&conn, &id, &content, "创建")?;
    document_from_file(id, &path, &workspace)
}

#[tauri::command]
fn save_document(
    path: String,
    content: String,
    base_hash: Option<String>,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn
        .query_row("SELECT id FROM documents WHERE path = ?1", [&path], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    if file.exists() {
        let existing = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let current_hash = format!("{:x}", Sha256::digest(existing.as_bytes()));
        if base_hash
            .as_deref()
            .is_some_and(|expected| expected != current_hash)
        {
            return Err("此文档已在其他位置修改。为避免覆盖，请重新载入或另存副本。".to_string());
        }
        if existing != content {
            let revisions: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM revisions WHERE document_id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            if revisions == 0 {
                create_revision(&conn, &id, &existing, "初始版本")?;
            }
        }
    }
    let now = Utc::now().to_rfc3339();
    atomic_write(&file, &content)?;
    let previous_hash: Option<String> = conn
        .query_row(
            "SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1",
            [&id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let saved_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    if previous_hash.as_deref() != Some(saved_hash.as_str()) {
        create_revision(&conn, &id, &content, "自動保存")?;
    }
    let hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    conn.execute(
        "UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4",
        params![title_for(&content, &file), now, hash, id],
    )
    .map_err(|e| e.to_string())?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM revisions WHERE document_id=?1",
            [&id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if count == 0 {
        create_revision(&conn, &id, &content, "初始版本")?;
    }
    document_from_file(id, &file, &workspace)
}

#[tauri::command]
fn list_revisions(path: String, state: State<'_, AppState>) -> Result<Vec<Revision>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn
        .query_row("SELECT id FROM documents WHERE path=?1", [&path], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    if file.exists() {
        let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM revisions WHERE document_id=?1",
                [&id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if count == 0 {
            create_revision(&conn, &id, &current, "当前版本")?;
        }
    }
    let mut stmt = conn.prepare("SELECT id, created_at, label, content FROM revisions WHERE document_id=?1 ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([id], |r| {
            let content: String = r.get(3)?;
            let preview = content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Revision {
                id: r.get(0)?,
                created_at: r.get(1)?,
                label: r.get(2)?,
                preview,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_revision(
    path: String,
    revision_id: i64,
    state: State<'_, AppState>,
) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn
        .query_row("SELECT id FROM documents WHERE path=?1", [&path], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    let restored: String = conn
        .query_row(
            "SELECT content FROM revisions WHERE id=?1 AND document_id=?2",
            params![revision_id, id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if file.exists() {
        create_revision(
            &conn,
            &id,
            &fs::read_to_string(&file).map_err(|e| e.to_string())?,
            "恢复前",
        )?;
    }
    atomic_write(&file, &restored)?;
    create_revision(&conn, &id, &restored, "恢复版本")?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4",
        params![
            title_for(&restored, &file),
            now,
            format!("{:x}", Sha256::digest(restored.as_bytes())),
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    document_from_file(id, &file, &workspace)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let path = app_db_path(app.handle()).map_err(std::io::Error::other)?;
            let conn = Connection::open(path).map_err(std::io::Error::other)?;
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
                 PRAGMA foreign_keys=ON;
                 CREATE TABLE IF NOT EXISTS schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
                 CREATE TABLE IF NOT EXISTS documents(
                    id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL,
                    updated_at TEXT NOT NULL, content_hash TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS revisions(
                    id INTEGER PRIMARY KEY AUTOINCREMENT, document_id TEXT NOT NULL,
                    content TEXT NOT NULL, content_hash TEXT NOT NULL, label TEXT NOT NULL,
                    created_at TEXT NOT NULL, FOREIGN KEY(document_id) REFERENCES documents(id) ON DELETE CASCADE
                 );
                 CREATE INDEX IF NOT EXISTS idx_revisions_document ON revisions(document_id, id DESC);"
            ).map_err(std::io::Error::other)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, ?1)",
                [Utc::now().to_rfc3339()],
            )
            .map_err(std::io::Error::other)?;
            app.manage(AppState { db: Mutex::new(conn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_workspace, set_workspace_path, list_documents, read_document, create_document, save_document, list_revisions, restore_revision])
        .run(tauri::generate_context!())
        .expect("error while running CreatorRelay");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_title_and_tags() {
        let source = "---\ntitle: \"Draft title\"\ntags:\n  - Rust\n  - Writing\n---\n\n# Body";
        let (title, tags) = parse_frontmatter(source);
        assert_eq!(title, "Draft title");
        assert_eq!(tags, vec!["Rust", "Writing"]);
    }

    #[test]
    fn extracts_body_without_frontmatter_for_metrics() {
        let source = "---\nid: doc\ntitle: test\n---\n\n# Hello\n\nworld";
        assert_eq!(body_for_word_count(source), "\n\n# Hello\n\nworld");
    }

    #[test]
    fn document_paths_cannot_escape_or_follow_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        fs::create_dir_all(workspace.join("articles")).unwrap();
        assert!(resolve_document(&workspace, "../outside.md").is_err());
        assert!(resolve_document(&workspace, "/tmp/outside.md").is_err());
        #[cfg(unix)]
        {
            let outside = temp.path().join("outside");
            fs::create_dir_all(&outside).unwrap();
            std::os::unix::fs::symlink(outside, workspace.join("articles/link")).unwrap();
            assert!(resolve_document(&workspace, "articles/link/escaped.md").is_err());
        }
    }

    #[test]
    fn markdown_is_authoritative_and_index_is_rebuildable() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let path = workspace.join("articles/first/article.md");
        atomic_write(&path, "---\ntitle: First\ntags: []\n---\n\n# First\n").unwrap();
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE documents(id TEXT PRIMARY KEY, path TEXT UNIQUE NOT NULL, title TEXT NOT NULL, updated_at TEXT NOT NULL, content_hash TEXT NOT NULL);\n             CREATE TABLE revisions(id INTEGER PRIMARY KEY, document_id TEXT NOT NULL, content TEXT NOT NULL, content_hash TEXT NOT NULL, label TEXT NOT NULL, created_at TEXT NOT NULL);"
        ).unwrap();
        sync_index(&connection, &workspace).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        fs::write(&path, "---\ntitle: Updated\ntags: []\n---\n\n# Updated\n").unwrap();
        sync_index(&connection, &workspace).unwrap();
        let title: String = connection
            .query_row("SELECT title FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(title, "Updated");
    }
}
