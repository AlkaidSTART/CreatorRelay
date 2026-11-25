use chrono::{DateTime, TimeZone, Utc};
use git2::{Commit, DiffFormat, Repository, Signature, Sort, Tree};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
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
    history_root: PathBuf,
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

fn digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}


fn history_repository_path(history_root: &Path, workspace: &Path) -> Result<PathBuf, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let identity = digest(&workspace.to_string_lossy());
    Ok(history_root.join(format!("{identity}.git")))
}

fn open_history_repository(history_root: &Path, workspace: &Path) -> Result<Repository, String> {
    fs::create_dir_all(history_root).map_err(|e| e.to_string())?;
    let path = history_repository_path(history_root, workspace)?;
    if path.exists() {
        Repository::open_bare(path).map_err(|e| e.message().to_string())
    } else {
        Repository::init_bare(path).map_err(|e| e.message().to_string())
    }
}

fn managed_history_path(relative: &str) -> Result<(String, String), String> {
    let path = Path::new(relative);
    let components = path.components().collect::<Vec<_>>();
    if components.len() != 3
        || components[0].as_os_str() != "articles"
        || components[2].as_os_str() != "article.md"
        || !components
            .iter()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err("版本历史仅支持工作区 articles/<id>/article.md 主稿".to_string());
    }
    let document_id = components[1]
        .as_os_str()
        .to_str()
        .ok_or("文章目录名不是有效 UTF-8")?
        .to_string();
    Ok((document_id, relative.replace('\\', "/")))
}

fn copy_tree_entries(
    builder: &mut git2::TreeBuilder<'_>,
    tree: Option<&Tree<'_>>,
    excluded_name: &str,
) -> Result<(), String> {
    if let Some(tree) = tree {
        for entry in tree.iter() {
            let name = entry.name().ok_or("Git 树包含无效文件名")?;
            if name != excluded_name {
                builder
                    .insert(name, entry.id(), entry.filemode())
                    .map_err(|e| e.message().to_string())?;
            }
        }
    }
    Ok(())
}

fn tree_blob_id(tree: &Tree<'_>, relative: &str) -> Option<git2::Oid> {
    tree.get_path(Path::new(relative)).ok().map(|entry| entry.id())
}

fn tree_with_document(
    repo: &Repository,
    base: Option<&Tree<'_>>,
    relative: &str,
    content: &str,
) -> Result<Tree<'_>, String> {
    let (document_id, _) = managed_history_path(relative)?;
    let base_articles = base
        .and_then(|tree| tree.get_name("articles").ok())
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| object.peel_to_tree().ok());
    let base_document = base_articles
        .as_ref()
        .and_then(|tree| tree.get_name(&document_id).ok())
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| object.peel_to_tree().ok());

    let blob_id = repo
        .blob(content.as_bytes())
        .map_err(|e| e.message().to_string())?;
    let mut document_builder = repo
        .treebuilder(base_document.as_ref())
        .map_err(|e| e.message().to_string())?;
    document_builder
        .insert("article.md", blob_id, 0o100644)
        .map_err(|e| e.message().to_string())?;
    let document_tree = document_builder
        .write()
        .map_err(|e| e.message().to_string())?;

    let mut articles_builder = repo
        .treebuilder(base_articles.as_ref())
        .map_err(|e| e.message().to_string())?;
    copy_tree_entries(&mut articles_builder, base_articles.as_ref(), &document_id)?;
    articles_builder
        .insert(&document_id, document_tree, 0o040000)
        .map_err(|e| e.message().to_string())?;
    let articles_tree = articles_builder
        .write()
        .map_err(|e| e.message().to_string())?;

    let mut root_builder = repo
        .treebuilder(base)
        .map_err(|e| e.message().to_string())?;
    copy_tree_entries(&mut root_builder, base, "articles")?;
    root_builder
        .insert("articles", articles_tree, 0o040000)
        .map_err(|e| e.message().to_string())?;
    let root_tree = root_builder
        .write()
        .map_err(|e| e.message().to_string())?;
    repo.find_tree(root_tree)
        .map_err(|e| e.message().to_string())
}

fn current_head(repo: &Repository) -> Result<Option<Commit<'_>>, String> {
    match repo.head() {
        Ok(reference) => reference
            .target()
            .map(|oid| repo.find_commit(oid).map_err(|e| e.message().to_string()))
            .transpose(),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(error.message().to_string()),
    }
}

fn commit_document_content(
    repo: &Repository,
    relative: &str,
    content: &str,
    message: &str,
    timestamp: Option<DateTime<Utc>>,
) -> Result<Option<git2::Oid>, String> {
    let head = current_head(repo)?;
    let parent_tree = head
        .as_ref()
        .map(|commit| commit.tree().map_err(|e| e.message().to_string()))
        .transpose()?;
    if parent_tree
        .as_ref()
        .and_then(|tree| tree_blob_id(tree, relative))
        .is_some_and(|oid| {
            repo.find_blob(oid)
                .is_ok_and(|blob| blob.content() == content.as_bytes())
        })
    {
        return Ok(None);
    }

    let tree = tree_with_document(repo, parent_tree.as_ref(), relative, content)?;
    let signature = if let Some(timestamp) = timestamp {
        let time = git2::Time::new(timestamp.timestamp(), 0);
        Signature::new("CreatorRelay", "local@creatorrelay.invalid", &time)
    } else {
        Signature::now("CreatorRelay", "local@creatorrelay.invalid")
    }
    .map_err(|e| e.message().to_string())?;
    let parents = head.iter().collect::<Vec<_>>();
    let parent_refs = parents.iter().collect::<Vec<_>>();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent_refs,
    )
    .map(Some)
    .map_err(|e| e.message().to_string())
}

fn ensure_document_history(
    repo: &Repository,
    conn: &Connection,
    document_id: &str,
    relative: &str,
    current_content: &str,
) -> Result<(), String> {
    let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;
    if revwalk.push_head().is_ok() {
        revwalk
            .set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)
            .map_err(|e| e.message().to_string())?;
    }
    let has_history = revwalk
        .filter_map(Result::ok)
        .filter_map(|oid| repo.find_commit(oid).ok())
        .any(|commit| {
            commit
                .tree()
                .ok()
                .and_then(|tree| tree_blob_id(&tree, relative))
                .is_some()
        });
    if has_history {
        return Ok(());
    }

    let mut statement = conn
        .prepare("SELECT content, label, created_at FROM revisions WHERE document_id=?1 ORDER BY id ASC")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([document_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let snapshots = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    drop(statement);

    for (content, label, created_at) in snapshots {
        let timestamp = DateTime::parse_from_rfc3339(&created_at)
            .ok()
            .map(|date| date.with_timezone(&Utc));
        commit_document_content(
            repo,
            relative,
            &content,
            &format!("迁移旧版本：{label}"),
            timestamp,
        )?;
    }
    commit_document_content(repo, relative, current_content, "迁移当前版本", None)?;
    Ok(())
}

fn format_revision_diff(repo: &Repository, commit: &Commit<'_>) -> Result<String, String> {
    let current_tree = commit.tree().map_err(|e| e.message().to_string())?;
    let parent_tree = commit
        .parent(0)
        .ok()
        .and_then(|parent| parent.tree().ok());
    let mut options = git2::DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&current_tree), Some(&mut options))
        .map_err(|e| e.message().to_string())?;
    let mut output = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        output.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(|e| e.message().to_string())?;
    Ok(output)
}

fn list_document_revisions(
    repo: &Repository,
    relative: &str,
) -> Result<Vec<Revision>, String> {
    let mut revwalk = repo.revwalk().map_err(|e| e.message().to_string())?;
    let Some(head) = current_head(repo)? else {
        return Ok(Vec::new());
    };
    revwalk
        .push(head.id())
        .map_err(|e| e.message().to_string())?;
    revwalk
        .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(|e| e.message().to_string())?;

    let mut revisions = Vec::new();
    for oid in revwalk {
        let oid = oid.map_err(|e| e.message().to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.message().to_string())?;
        let tree = commit.tree().map_err(|e| e.message().to_string())?;
        let Some(blob_id) = tree_blob_id(&tree, relative) else {
            continue;
        };
        if commit
            .parent(0)
            .ok()
            .and_then(|parent| parent.tree().ok())
            .and_then(|parent_tree| tree_blob_id(&parent_tree, relative))
            == Some(blob_id)
        {
            continue;
        }
        let blob = repo
            .find_blob(blob_id)
            .map_err(|e| e.message().to_string())?;
        let content = String::from_utf8_lossy(blob.content());
        let preview = body_for_word_count(&content)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        let created_at = Utc
            .timestamp_opt(commit.time().seconds(), 0)
            .single()
            .unwrap_or_else(Utc::now)
            .to_rfc3339();
        revisions.push(Revision {
            id: oid.to_string(),
            created_at,
            label: commit.message().unwrap_or("本地版本").trim().to_string(),
            preview,
            diff: format_revision_diff(repo, &commit)?,
        });
    }
    Ok(revisions)
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

fn body_for_word_count(content: &str) -> &str {
    if content.starts_with("---\n") {
        if let Some(end) = content[4..].find("\n---") {
            return &content[4 + end + 5..];
        }
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

fn create_revision(
    conn: &Connection,
    document_id: &str,
    content: &str,
    label: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO revisions(document_id, content, content_hash, label, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            document_id,
            content,
            digest(content),
            label,
            Utc::now().to_rfc3339()
        ],
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

fn create_revision_tx(
    tx: &Transaction<'_>,
    document_id: &str,
    content: &str,
    label: &str,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO revisions(document_id, content, content_hash, label, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            document_id,
            content,
            digest(content),
            label,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "DELETE FROM revisions WHERE document_id = ?1 AND id NOT IN
         (SELECT id FROM revisions WHERE document_id = ?1 ORDER BY id DESC LIMIT 30)",
        [document_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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

fn sync_index(conn: &Connection, workspace: &Path) -> Result<(), String> {
    if !workspace.is_dir() {
        return Err("工作区路径不可用，请重新选择工作区".to_string());
    }
    normalize_legacy_paths(conn, workspace)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
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
        seen.insert(database_path.clone());
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
        let changed =
            indexed.as_ref().map(|(_, hash)| hash.as_str()) != Some(document.content_hash.as_str());
        tx.execute(
            "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(path) DO UPDATE SET title=excluded.title, updated_at=excluded.updated_at, content_hash=excluded.content_hash",
            params![document.id, database_path, document.title, document.updated_at, document.content_hash],
        )
        .map_err(|e| e.to_string())?;
        if changed {
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            let latest_hash: Option<String> = tx
                .query_row(
                    "SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1",
                    [&id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if latest_hash.as_deref() != Some(document.content_hash.as_str()) {
                create_revision_tx(
                    &tx,
                    &id,
                    &content,
                    if indexed.is_some() {
                        "检测到外部修改"
                    } else {
                        "导入"
                    },
                )?;
            }
        }
    }
    // Keep missing-file rows and revision snapshots. The file is authoritative,
    // but an external deletion should not silently destroy recoverable history.
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
        if filter.is_empty()
            || document.title.to_lowercase().contains(&filter)
            || document.preview.to_lowercase().contains(&filter)
            || document
                .tags
                .iter()
                .any(|tag| tag.to_lowercase().contains(&filter))
        {
            output.push(document);
        }
    }
    output.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(output)
}

#[tauri::command]
fn read_document(path: String, state: State<'_, AppState>) -> Result<DocumentContent, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let path = resolve_document(&workspace, &path)?;
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(DocumentContent {
        content_hash: digest(&content),
        content,
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
    let relative = format!("articles/{id}/article.md");
    let path = resolve_document(&workspace, &relative)?;
    let title_yaml = serde_json::to_string(title).map_err(|e| e.to_string())?;
    let created_at = Utc::now().to_rfc3339();
    let content = format!(
        "---\nid: {id}\ntitle: {title_yaml}\ncreated_at: {created_at}\nupdated_at: {created_at}\ntags: []\nstatus: draft\n---\n\n# {title}\n\n"
    );
    atomic_write(&path, &content)?;
    let database_path = database_document_path(&workspace, &relative)?;
    conn.execute(
        "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![id, database_path, title, created_at, digest(&content)],
    )
    .map_err(|e| e.to_string())?;
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
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let database_path = database_document_path(&workspace, &path)?;
    let id: String = conn
        .query_row(
            "SELECT id FROM documents WHERE path = ?1",
            [&database_path],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let existing = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let current_hash = digest(&existing);
    if base_hash
        .as_deref()
        .is_some_and(|expected| expected != current_hash)
    {
        return Err("此文档已在其他位置修改。为避免覆盖，请重新载入或另存副本。".to_string());
    }
    if existing == content {
        return document_from_file(id, &file, &workspace);
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let latest_hash: Option<String> = tx
        .query_row(
            "SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if latest_hash.as_deref() != Some(current_hash.as_str()) {
        create_revision_tx(&tx, &id, &existing, "编辑前版本")?;
    }
    create_revision_tx(&tx, &id, &content, "自动保存")?;
    atomic_write(&file, &content)?;
    tx.execute(
        "UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4",
        params![
            title_for(&content, &file),
            Utc::now().to_rfc3339(),
            digest(&content),
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    document_from_file(id, &file, &workspace)
}

#[tauri::command]
fn list_revisions(path: String, state: State<'_, AppState>) -> Result<Vec<Revision>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn
        .query_row("SELECT id FROM documents WHERE path=?1", [&path], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    if !file.exists() {
        return Err("文档文件已不存在".to_string());
    }
    let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let current_hash = digest(&current);
    let latest_hash: Option<String> = conn
        .query_row(
            "SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if latest_hash.as_deref() != Some(current_hash.as_str()) {
        create_revision(&conn, &id, &current, "当前版本")?;
    }
    let mut statement = conn
        .prepare("SELECT id, created_at, label, content FROM revisions WHERE document_id=?1 ORDER BY id DESC")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([id], |row| {
            let content: String = row.get(3)?;
            let preview = content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Revision {
                id: row.get(0)?,
                created_at: row.get(1)?,
                label: row.get(2)?,
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
    let mut conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let file = resolve_document(&workspace, &path)?;
    let id: String = conn
        .query_row("SELECT id FROM documents WHERE path=?1", [&path], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;
    let restored: String = conn
        .query_row(
            "SELECT content FROM revisions WHERE id=?1 AND document_id=?2",
            params![revision_id, id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let current = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let latest_hash: Option<String> = tx
        .query_row(
            "SELECT content_hash FROM revisions WHERE document_id=?1 ORDER BY id DESC LIMIT 1",
            [&id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if latest_hash.as_deref() != Some(digest(&current).as_str()) {
        create_revision_tx(&tx, &id, &current, "恢复前")?;
    }
    create_revision_tx(&tx, &id, &restored, "恢复版本")?;
    atomic_write(&file, &restored)?;
    tx.execute(
        "UPDATE documents SET title=?1, updated_at=?2, content_hash=?3 WHERE id=?4",
        params![
            title_for(&restored, &file),
            Utc::now().to_rfc3339(),
            digest(&restored),
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
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
                 CREATE INDEX IF NOT EXISTS idx_revisions_document ON revisions(document_id, id DESC);",
            )
            .map_err(std::io::Error::other)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, ?1)",
                [Utc::now().to_rfc3339()],
            )
            .map_err(std::io::Error::other)?;
            app.manage(AppState {
                db: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_workspace,
            set_workspace_path,
            list_documents,
            read_document,
            create_document,
            save_document,
            list_revisions,
            restore_revision
        ])
        .run(tauri::generate_context!())
        .expect("error while running CreatorRelay");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(
            "CREATE TABLE app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE documents(id TEXT PRIMARY KEY, path TEXT UNIQUE NOT NULL, title TEXT NOT NULL, updated_at TEXT NOT NULL, content_hash TEXT NOT NULL);
             CREATE TABLE revisions(id INTEGER PRIMARY KEY AUTOINCREMENT, document_id TEXT NOT NULL, content TEXT NOT NULL, content_hash TEXT NOT NULL, label TEXT NOT NULL, created_at TEXT NOT NULL);",
        ).unwrap();
        connection
    }

    #[test]
    fn parses_frontmatter_title_and_tags() {
        let source = "---\ntitle: \"Draft title\"\ntags:\n  - Rust\n  - Writing\nstatus: draft\n---\n\n# Body";
        let (title, tags) = parse_frontmatter(source);
        assert_eq!(title, "Draft title");
        assert_eq!(tags, vec!["Rust", "Writing"]);
    }

    #[test]
    fn extracts_body_without_frontmatter_for_metrics() {
        let source = "---\nid: doc\ntitle: test\n---\n\n# Hello\n\nworld";
        assert_eq!(body_for_word_count(source), "\n# Hello\n\nworld");
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
        let connection = test_db();
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
        let revisions: i64 = connection
            .query_row("SELECT COUNT(*) FROM revisions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(revisions, 2);
    }

    #[test]
    fn legacy_relative_paths_stay_bound_to_the_original_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("original");
        let next = temp.path().join("next");
        setup_workspace(&original).unwrap();
        setup_workspace(&next).unwrap();
        let original_file = original.join("articles/doc/article.md");
        atomic_write(&original_file, "# Original\n").unwrap();
        let connection = test_db();
        set_workspace(&connection, &original).unwrap();
        connection
            .execute(
                "INSERT INTO documents VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    "legacy",
                    "articles/doc/article.md",
                    "Original",
                    Utc::now().to_rfc3339(),
                    digest("# Original\n")
                ],
            )
            .unwrap();
        normalize_legacy_paths(&connection, &next).unwrap();
        let migrated: String = connection
            .query_row("SELECT path FROM documents WHERE id='legacy'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(
            PathBuf::from(migrated),
            original_file.canonicalize().unwrap()
        );
    }

    #[test]
    fn same_relative_path_in_distinct_workspaces_has_distinct_identity() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        setup_workspace(&first).unwrap();
        setup_workspace(&second).unwrap();
        let first_doc = first.join("articles/shared/article.md");
        let second_doc = second.join("articles/shared/article.md");
        atomic_write(&first_doc, "# First\n").unwrap();
        atomic_write(&second_doc, "# Second\n").unwrap();
        let connection = test_db();
        sync_index(&connection, &first).unwrap();
        sync_index(&connection, &second).unwrap();
        let rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        let revisions: i64 = connection
            .query_row("SELECT COUNT(*) FROM revisions", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 2);
        assert_eq!(revisions, 2);
    }

    #[test]
    fn save_conflict_is_detected_and_versions_are_bounded() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let path = workspace.join("articles/doc/article.md");
        let original = "---\ntitle: Draft\ntags: []\n---\n\n# Draft\n";
        atomic_write(&path, original).unwrap();
        let connection = test_db();
        let id = "doc-1";
        connection
            .execute(
                "INSERT INTO documents VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    id,
                    "articles/doc/article.md",
                    "Draft",
                    Utc::now().to_rfc3339(),
                    digest(original)
                ],
            )
            .unwrap();
        create_revision(&connection, id, original, "创建").unwrap();
        let external = original.replace("Draft", "External");
        fs::write(&path, &external).unwrap();
        assert_ne!(
            digest(&fs::read_to_string(&path).unwrap()),
            digest(original)
        );
        for index in 0..35 {
            create_revision(&connection, id, &format!("revision {index}"), "自动保存").unwrap();
        }
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM revisions WHERE document_id=?1",
                [id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 30);
        assert!(resolve_document(&workspace, "articles/doc/article.md").is_ok());
    }
}
