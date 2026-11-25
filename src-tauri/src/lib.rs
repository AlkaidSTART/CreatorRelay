use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};
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
        .query_row("SELECT value FROM app_settings WHERE key = 'workspace_path'", [], |r| r.get(0))
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
        for line in content[4..4 + end].lines() {
            if let Some(value) = line.strip_prefix("title:") {
                title = value.trim().trim_matches(['"', '\'']).to_string();
            }
            if line.trim() == "tags:" {
                continue;
            }
            if let Some(tag) = line.trim().strip_prefix("- ") {
                tags.push(tag.trim().trim_matches(['"', '\'']).to_string());
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
    let title = if frontmatter_title.is_empty() { title_for(&content, path) } else { frontmatter_title };
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
    })
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("目标文件没有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".creatorrelay-{}.tmp", Uuid::new_v4()));
    fs::write(&temp, content).map_err(|e| e.to_string())?;
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error.to_string());
    }
    Ok(())
}

fn resolve_document(workspace: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|part| !matches!(part, Component::Normal(_)))
        || relative_path.extension().and_then(|ext| ext.to_str()) != Some("md")
    {
        return Err("无效的工作区文档路径".to_string());
    }
    let workspace_canonical = workspace.canonicalize().map_err(|e| e.to_string())?;
    let candidate = workspace.join(relative_path);
    if candidate.exists() {
        let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
        if !canonical.starts_with(&workspace_canonical) {
            return Err("文档路径超出工作区范围".to_string());
        }
    }
    Ok(candidate)
}

fn create_revision(conn: &Connection, document_id: &str, content: &str, label: &str) -> Result<(), String> {
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
    setup_workspace(workspace)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for entry in WalkDir::new(workspace.join("articles")).follow_links(false).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() { continue; }
        let relative = path.strip_prefix(workspace).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        let id: Option<String> = tx.query_row("SELECT id FROM documents WHERE path = ?1", [&relative], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        let id = id.unwrap_or_else(|| Uuid::new_v4().to_string());
        let doc = document_from_file(id.clone(), path, workspace)?;
        tx.execute(
            "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title, updated_at=excluded.updated_at, content_hash=excluded.content_hash",
            params![doc.id, doc.path, doc.title, doc.updated_at, format!("{:x}", Sha256::digest(fs::read(path).map_err(|e| e.to_string())?))],
        ).map_err(|e| e.to_string())?;
    }
    tx.execute("DELETE FROM documents WHERE path NOT IN (SELECT replace(path, char(92), '/') FROM documents)", []).ok();
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    Ok(active_workspace(&conn)?.map(|path| WorkspaceInfo {
        name: path.file_name().and_then(|part| part.to_str()).unwrap_or("CreatorRelay").to_string(),
        path: path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
fn set_workspace_path(path: String, state: State<'_, AppState>) -> Result<WorkspaceInfo, String> {
    let path = PathBuf::from(path).canonicalize().map_err(|e| e.to_string())?;
    if !path.is_dir() { return Err("请选择一个文件夹作为工作区".to_string()); }
    setup_workspace(&path)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    set_workspace(&conn, &path)?;
    sync_index(&conn, &path)?;
    Ok(WorkspaceInfo { name: path.file_name().and_then(|part| part.to_str()).unwrap_or("CreatorRelay").to_string(), path: path.to_string_lossy().to_string() })
}

#[tauri::command]
fn list_documents(query: Option<String>, state: State<'_, AppState>) -> Result<Vec<Document>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let Some(workspace) = active_workspace(&conn)? else { return Ok(Vec::new()); };
    sync_index(&conn, &workspace)?;
    let filter = query.unwrap_or_default().trim().to_lowercase();
    let mut output = Vec::new();
    for entry in WalkDir::new(workspace.join("articles")).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() || entry.path().extension().and_then(|e| e.to_str()) != Some("md") { continue; }
        let relative = entry.path().strip_prefix(&workspace).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
        let id: String = conn.query_row("SELECT id FROM documents WHERE path = ?1", [&relative], |r| r.get(0)).map_err(|e| e.to_string())?;
        let doc = document_from_file(id, entry.path(), &workspace)?;
        if filter.is_empty() || doc.title.to_lowercase().contains(&filter) || doc.preview.to_lowercase().contains(&filter) || doc.tags.iter().any(|tag| tag.to_lowercase().contains(&filter)) {
            output.push(doc);
        }
    }
    output.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(output)
}

#[tauri::command]
fn read_document(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let path = resolve_document(&workspace, &path)?;
    fs::read_to_string(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_document(title: String, state: State<'_, AppState>) -> Result<Document, String> {
    let title = title.trim();
    if title.is_empty() { return Err("标题不能为空".to_string()); }
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let id = Uuid::new_v4().to_string();
    let safe_name: String = title.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else if c.is_whitespace() { '-' } else { c }).collect();
    let safe_name = safe_name.trim_matches('-');
    let folder = if safe_name.is_empty() { format!("article-{}", &id[..8]) } else { safe_name.to_string() };
    let relative = format!("articles/{folder}/article.md");
    let path = resolve_document(&workspace, &relative)?;
    if path.exists() { return Err("同名文章已存在，请使用不同标题".to_string()); }
    let content = format!("---\nid: {id}\ntitle: \"{}\"\ncreated_at: {}\nupdated_at: {}\ntags: []\nstatus: draft\n---\n\n# {}\n\n", title.replace('"', "\\\""), Utc::now().to_rfc3339(), Utc::now().to_rfc3339(), title);
    atomic_write(&path, &content)?;
    conn.execute("INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)", params![id, relative, title, Utc::now().to_rfc3339(), format!("{:x}", Sha256::digest(content.as_bytes()))]).map_err(|e| e.to_string())?;
    create_revision(&conn, &id, &content, "创建")?;
    document_from_file(id, &path, &workspace)
}

#[tauri::command]
fn save_document(path: String, content: String, base_hash: Option<String>, state: State<'_, AppState>) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn.query_row("SELECT id FROM documents WHERE path = ?1", [&path], |r| r.get(0)).map_err(|e| e.to_string())?;
    if file.exists() {
        let existing = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let current_hash = format!("{:x}", Sha256::digest(existing.as_bytes()));
        if base_hash.as_deref().is_some_and(|expected| expected != current_hash) {
            return Err("此文档已在其他位置修改。为避免覆盖，请重新载入或另存副本。".to_string());
        }
        if existing != content { create_revision(&conn, &id, &existing, "自动保存前").ok(); }
    }
    let now = Utc::now().to_rfc3339();
    let updated = if content.starts_with("---\n") {
        if let Some(end) = content[4..].find("\n---") {
            let fm = &content[4..4 + end];
            let lines: Vec<String> = fm.lines().filter(|line| !line.starts_with("updated_at:")).map(str::to_string).collect();
            format!("---\n{}\nupdated_at: {}\n---{}", lines.join("\n"), now, &content[4 + end + 4..])
        } else { content }
    } else { content };
    atomic_write(&file, &updated)?;
    let hash = format!("{:x}", Sha256::digest(updated.as_bytes()));
    conn.execute("UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4", params![title_for(&updated, &file), now, hash, id]).map_err(|e| e.to_string())?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM revisions WHERE document_id=?1", [&id], |r| r.get(0)).map_err(|e| e.to_string())?;
    if count == 0 { create_revision(&conn, &id, &updated, "初始版本")?; }
    document_from_file(id, &file, &workspace)
}

#[tauri::command]
fn list_revisions(path: String, state: State<'_, AppState>) -> Result<Vec<Revision>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn.query_row("SELECT id FROM documents WHERE path=?1", [&path], |r| r.get(0)).map_err(|e| e.to_string())?;
    if file.exists() {
        let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let digest = format!("{:x}", Sha256::digest(current.as_bytes()));
        let latest: Option<String> = conn.query_row("SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1", [&id], |r| r.get(0)).optional().map_err(|e| e.to_string())?;
        if latest.as_deref() != Some(&digest) { create_revision(&conn, &id, &current, "当前版本")?; }
    }
    let mut stmt = conn.prepare("SELECT id, created_at, label, content FROM revisions WHERE document_id=?1 ORDER BY id DESC").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([id], |r| {
        let content: String = r.get(3)?;
        let preview = content.lines().filter(|line| !line.trim().is_empty()).take(2).collect::<Vec<_>>().join(" ");
        Ok(Revision { id: r.get(0)?, created_at: r.get(1)?, label: r.get(2)?, preview })
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_revision(path: String, revision_id: i64, state: State<'_, AppState>) -> Result<Document, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn.query_row("SELECT id FROM documents WHERE path=?1", [&path], |r| r.get(0)).map_err(|e| e.to_string())?;
    let restored: String = conn.query_row("SELECT content FROM revisions WHERE id=?1 AND document_id=?2", params![revision_id, id], |r| r.get(0)).map_err(|e| e.to_string())?;
    if file.exists() { create_revision(&conn, &id, &fs::read_to_string(&file).map_err(|e| e.to_string())?, "恢复前")?; }
    atomic_write(&file, &restored)?;
    create_revision(&conn, &id, &restored, "恢复版本")?;
    let now = Utc::now().to_rfc3339();
    conn.execute("UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4", params![title_for(&restored, &file), now, format!("{:x}", Sha256::digest(restored.as_bytes())), id]).map_err(|e| e.to_string())?;
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
            app.manage(AppState { db: Mutex::new(conn) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_workspace, set_workspace_path, list_documents, read_document, create_document, save_document, list_revisions, restore_revision])
        .run(tauri::generate_context!())
        .expect("error while running CreatorRelay");
}
