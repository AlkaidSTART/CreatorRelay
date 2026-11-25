use chrono::{DateTime, TimeZone, Utc};
use git2::{Commit, DiffFormat, Repository, Signature, Sort, Tree};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
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

fn digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn ensure_base_hash(content: &str, base_hash: Option<&str>) -> Result<(), String> {
    if base_hash.is_some_and(|expected| expected != digest(content)) {
        return Err("此文档已在其他位置修改。为避免覆盖，请重新载入后再操作。".to_string());
    }
    Ok(())
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
    let normalized = relative.replace('\\', "/");
    Ok((document_id, normalized))
}

fn copy_tree_entries(
    builder: &mut git2::TreeBuilder<'_>,
    tree: Option<&Tree<'_>>,
    excluded_name: &str,
) -> Result<(), String> {
    if let Some(tree) = tree {
        for entry in tree.iter() {
            let name = entry.name().map_err(|e| e.message().to_string())?;
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
    tree.get_path(Path::new(relative))
        .ok()
        .map(|entry| entry.id())
}

fn tree_with_document<'repo>(
    repo: &'repo Repository,
    base: Option<&Tree<'repo>>,
    relative: &str,
    content: &str,
) -> Result<Tree<'repo>, String> {
    let (document_id, _) = managed_history_path(relative)?;
    let base_articles = base
        .and_then(|tree| tree.get_name("articles"))
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| object.peel_to_tree().ok());
    let base_document = base_articles
        .as_ref()
        .and_then(|tree| tree.get_name(&document_id))
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
    let root_tree = root_builder.write().map_err(|e| e.message().to_string())?;
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
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
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

    let legacy_table_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='revisions')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let snapshots = if legacy_table_exists {
        let mut statement = conn
            .prepare(
                "SELECT content, label, created_at FROM revisions WHERE document_id=?1 ORDER BY id ASC",
            )
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
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

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

fn format_revision_diff(
    repo: &Repository,
    commit: &Commit<'_>,
    relative: &str,
) -> Result<String, String> {
    let current_tree = commit.tree().map_err(|e| e.message().to_string())?;
    let parent_tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());
    let mut options = git2::DiffOptions::new();
    options.pathspec(relative);
    let diff = repo
        .diff_tree_to_tree(
            parent_tree.as_ref(),
            Some(&current_tree),
            Some(&mut options),
        )
        .map_err(|e| e.message().to_string())?;
    let mut output = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        output.push_str(&String::from_utf8_lossy(line.content()));
        true
    })
    .map_err(|e| e.message().to_string())?;
    Ok(output)
}

fn list_document_revisions(repo: &Repository, relative: &str) -> Result<Vec<Revision>, String> {
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
            diff: format_revision_diff(repo, &commit, relative)?,
        });
    }
    Ok(revisions)
}

fn app_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("creatorrelay.sqlite3"))
}

fn markdown_url_is_external(url: &str) -> bool {
    ["http://", "https://", "mailto:", "tel:", "data:", "#", "?"]
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

fn markdown_url_is_unsafe_local(url: &str) -> bool {
    url.starts_with('/')
        || url.starts_with("file:")
        || url.split('/').any(|component| component == "..")
}

fn list_document_assets(
    workspace: &Path,
    document_path: &str,
    markdown: &str,
) -> Result<Vec<WorkspaceAsset>, String> {
    let mut output = Vec::new();
    let patterns = [
        (
            regex::Regex::new(r"!\[[^\]]*\]\(([^)]+)\)").map_err(|e| e.to_string())?,
            true,
        ),
        (
            regex::Regex::new(r"\[[^\]]+\]\(([^)]+)\)").map_err(|e| e.to_string())?,
            false,
        ),
    ];
    for (captures, is_image) in patterns {
        for capture in captures.captures_iter(markdown) {
            let raw_url = capture
                .get(1)
                .ok_or("invalid Markdown link")?
                .as_str()
                .trim();
            let url = raw_url
                .trim_matches(['<', '>'])
                .split_whitespace()
                .next()
                .unwrap_or("");
            if url.is_empty() {
                continue;
            }
            if markdown_url_is_unsafe_local(url) {
                return Err("Markdown 本地引用不能使用绝对路径或目录穿越".to_string());
            }
            if markdown_url_is_external(url) || !url.starts_with("assets/") {
                if is_image && !markdown_url_is_external(url) {
                    return Err("文章图片必须使用文章目录下的 assets/ 相对路径".to_string());
                }
                continue;
            }
            let relative = url;
            if output
                .iter()
                .any(|asset: &WorkspaceAsset| asset.path == relative)
            {
                continue;
            }
            let path = match resolve_workspace_asset(workspace, document_path, relative) {
                Ok(path) => path,
                Err(error) => {
                    output.push(WorkspaceAsset {
                        path: relative.to_string(),
                        markdown: String::new(),
                        mime_type: "application/octet-stream".to_string(),
                        size: 0,
                        available: false,
                        warning: Some(error),
                    });
                    continue;
                }
            };
            let metadata = match fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() => metadata,
                Ok(_) => {
                    output.push(WorkspaceAsset {
                        path: relative.to_string(),
                        markdown: String::new(),
                        mime_type: "application/octet-stream".to_string(),
                        size: 0,
                        available: false,
                        warning: Some("素材不是普通文件".to_string()),
                    });
                    continue;
                }
                Err(error) => {
                    output.push(WorkspaceAsset {
                        path: relative.to_string(),
                        markdown: String::new(),
                        mime_type: "application/octet-stream".to_string(),
                        size: 0,
                        available: false,
                        warning: Some(format!("素材无法定位：{error}")),
                    });
                    continue;
                }
            };
            output.push(WorkspaceAsset {
                path: relative.to_string(),
                markdown: String::new(),
                mime_type: asset_mime_type(&path).to_string(),
                size: metadata.len(),
                available: true,
                warning: None,
            });
        }
    }
    Ok(output)
}

fn validate_asset_references(
    workspace: &Path,
    relative: &str,
    content: &str,
) -> Result<(), String> {
    let assets = list_document_assets(workspace, relative, content)?;
    if let Some(asset) = assets.iter().find(|asset| !asset.available) {
        return Err(asset
            .warning
            .clone()
            .unwrap_or_else(|| format!("素材无法定位：{}", asset.path)));
    }
    Ok(())
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
        if history_root.is_some() && previous_hash.is_some_and(|hash| hash != document.content_hash)
        {
            let history = open_history_repository(history_root.unwrap(), workspace)?;
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            ensure_document_history(&history, &tx, &id, &relative, &content)?;
            commit_document_content(&history, &relative, &content, "检测到外部修改", None)?;
        } else if history_root.is_some() && previous_hash.is_none() {
            let history = open_history_repository(history_root.unwrap(), workspace)?;
            let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
            commit_document_content(&history, &relative, &content, "导入工作区文章", None)?;
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
    let path = PathBuf::from(path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
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
    let history = open_history_repository(&state.history_root, &workspace)?;
    commit_document_content(&history, &relative, &content, "创建文章", None)?;
    let database_path = database_document_path(&workspace, &relative)?;
    conn.execute(
        "INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES(?1, ?2, ?3, ?4, ?5)",
        params![id, database_path, title, created_at, digest(&content)],
    )
    .map_err(|e| e.to_string())?;
    document_from_file(id, &path, &workspace)
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
        .map_err(|e| e.to_string())?;
    let existing = fs::read_to_string(&file).map_err(|e| e.to_string())?;
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
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
                 );",
            )
            .map_err(std::io::Error::other)?;
            conn.execute(
                "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, ?1)",
                [Utc::now().to_rfc3339()],
            )
            .map_err(std::io::Error::other)?;
            let history_root = app
                .path()
                .app_data_dir()
                .map_err(std::io::Error::other)?
                .join("article-history");
            app.manage(AppState {
                db: Mutex::new(conn),
                history_root,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
    fn search_matches_title_body_and_tags_case_insensitively() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("article.md");
        let content =
            "---\ntitle: Rust Notes\ntags:\n  - CreatorRelay\n---\n\nA private body phrase.\n";
        fs::write(&file, content).unwrap();
        let document = document_from_file("doc".to_string(), &file, temp.path()).unwrap();
        assert!(document_matches_query(&document, &file, "rust").unwrap());
        assert!(document_matches_query(&document, &file, "PRIVATE BODY").unwrap());
        assert!(document_matches_query(&document, &file, "creatorrelay").unwrap());
        assert!(!document_matches_query(&document, &file, "not present").unwrap());
        assert!(document_matches_query(&document, &file, "").unwrap());
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
        sync_index(&connection, &workspace, None).unwrap();
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        fs::write(&path, "---\ntitle: Updated\ntags: []\n---\n\n# Updated\n").unwrap();
        sync_index(&connection, &workspace, None).unwrap();
        let title: String = connection
            .query_row("SELECT title FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(title, "Updated");
    }

    #[test]
    fn sync_index_records_external_article_changes_in_git_history() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let history_root = temp.path().join("app-data/article-history");
        let path = workspace.join("articles/doc/article.md");
        let original = "# Original\n";
        atomic_write(&path, original).unwrap();
        let connection = test_db();
        set_workspace(&connection, &workspace).unwrap();
        sync_index(&connection, &workspace, Some(&history_root)).unwrap();
        let external = "# External\n";
        fs::write(&path, external).unwrap();
        sync_index(&connection, &workspace, Some(&history_root)).unwrap();
        let relative = "articles/doc/article.md";
        let repository = open_history_repository(&history_root, &workspace).unwrap();
        let revisions = list_document_revisions(&repository, relative).unwrap();
        assert_eq!(revisions.len(), 2);
        assert!(
            revisions
                .iter()
                .any(|revision| revision.preview == "# Original")
        );
        assert!(
            revisions
                .iter()
                .any(|revision| revision.preview == "# External")
        );
        assert_eq!(fs::read_to_string(path).unwrap(), external);
    }

    #[test]
    fn workspace_assets_are_confined_to_article_assets_and_reject_symlinks() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let article_dir = workspace.join("articles/doc");
        fs::create_dir_all(article_dir.join("assets")).unwrap();
        fs::write(article_dir.join("article.md"), "# Article").unwrap();
        fs::write(article_dir.join("assets/image.png"), b"image").unwrap();
        let normal =
            resolve_workspace_asset(&workspace, "articles/doc/article.md", "assets/image.png")
                .unwrap();
        assert_eq!(normal, article_dir.join("assets/image.png"));
        assert!(
            resolve_workspace_asset(&workspace, "articles/doc/article.md", "../../outside.png",)
                .is_err()
        );
        #[cfg(unix)]
        {
            let outside = temp.path().join("outside.png");
            fs::write(&outside, b"outside").unwrap();
            std::os::unix::fs::symlink(outside, article_dir.join("assets/linked.png")).unwrap();
            assert!(resolve_workspace_asset(
                &workspace,
                "articles/doc/article.md",
                "assets/linked.png",
            )
            .is_err());
        }
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
        sync_index(&connection, &first, None).unwrap();
        sync_index(&connection, &second, None).unwrap();
        let rows: i64 = connection
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(rows, 2);
    }

    #[test]
    fn git_history_tracks_manual_article_versions_and_path_scoped_diffs() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let history_root = temp.path().join("app-data/article-history");
        let article_path = "articles/doc-a/article.md";
        let first = "---\ntitle: Draft\ncustom: preserved\n---\n\n# Draft\n";
        let second = first.replace("# Draft", "# Updated");
        let other_path = "articles/doc-b/article.md";
        let other = "# Other article\n";
        let repo = open_history_repository(&history_root, &workspace).unwrap();

        commit_document_content(&repo, article_path, first, "创建文章", None).unwrap();
        commit_document_content(&repo, other_path, other, "创建另一篇", None).unwrap();
        commit_document_content(&repo, article_path, &second, "手动保存版本", None).unwrap();
        commit_document_content(&repo, article_path, &second, "重复快照", None).unwrap();

        let revisions = list_document_revisions(&repo, article_path).unwrap();
        assert_eq!(revisions.len(), 2);
        assert!(revisions[0].diff.contains("# Updated"));
        assert!(!revisions[0].diff.contains("Other article"));
        assert!(revisions[1].diff.contains("custom: preserved"));
    }

    #[test]
    fn git_history_migrates_legacy_snapshots_without_changing_markdown_files() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let file = workspace.join("articles/doc/article.md");
        let current = "---\ntitle: Current\ncustom: keep\n---\n\n# Current\n";
        atomic_write(&file, current).unwrap();
        let before = digest(&fs::read_to_string(&file).unwrap());
        let connection = test_db();
        let created_at = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO documents VALUES(?1, ?2, ?3, ?4, ?5)",
                params![
                    "doc",
                    file.to_string_lossy(),
                    "Current",
                    created_at,
                    digest(current)
                ],
            )
            .unwrap();
        let old = "---\ntitle: Old\ncustom: keep\n---\n\n# Old\n";
        connection.execute(
            "INSERT INTO revisions(document_id, content, content_hash, label, created_at) VALUES(?1, ?2, ?3, ?4, ?5)",
            params!["doc", old, digest(old), "旧版本", created_at],
        ).unwrap();
        let history_root = temp.path().join("history");
        let repo = open_history_repository(&history_root, &workspace).unwrap();
        ensure_document_history(
            &repo,
            &connection,
            "doc",
            "articles/doc/article.md",
            current,
        )
        .unwrap();

        let revisions = list_document_revisions(&repo, "articles/doc/article.md").unwrap();
        assert_eq!(revisions.len(), 2);
        assert!(
            revisions
                .iter()
                .any(|revision| revision.preview.contains("# Old"))
        );
        assert_eq!(digest(&fs::read_to_string(file).unwrap()), before);
    }

    #[test]
    fn restoring_a_git_version_creates_a_new_recoverable_tip() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let root = temp.path().join("history");
        let path = "articles/doc/article.md";
        let first = "# First\n";
        let second = "# Second\n";
        let repo = open_history_repository(&root, &workspace).unwrap();
        commit_document_content(&repo, path, first, "创建", None).unwrap();
        let first_id = list_document_revisions(&repo, path).unwrap()[0].id.clone();
        commit_document_content(&repo, path, second, "更新", None).unwrap();
        let selected = repo
            .find_commit(git2::Oid::from_str(&first_id).unwrap())
            .unwrap();
        let selected_tree = selected.tree().unwrap();
        let selected_blob = repo
            .find_blob(tree_blob_id(&selected_tree, path).unwrap())
            .unwrap();
        let restored = std::str::from_utf8(selected_blob.content()).unwrap();
        commit_document_content(&repo, path, second, "恢复前保留当前版本", None).unwrap();
        commit_document_content(&repo, path, restored, "恢复选定版本", None).unwrap();

        let revisions = list_document_revisions(&repo, path).unwrap();
        assert_eq!(revisions.len(), 3);
        assert_eq!(revisions[0].preview, "# First");
        assert!(
            revisions
                .iter()
                .any(|revision| revision.preview == "# Second")
        );
    }

    #[test]
    fn git_tree_preserves_workspace_files_but_commits_only_article_markdown() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        fs::write(workspace.join("assets/image.png"), b"asset bytes").unwrap();
        fs::write(workspace.join("notes.txt"), "outside article scope").unwrap();
        let repo = open_history_repository(&temp.path().join("history"), &workspace).unwrap();
        commit_document_content(
            &repo,
            "articles/doc/article.md",
            "# Versioned article\n",
            "create",
            None,
        )
        .unwrap();
        let head = current_head(&repo).unwrap().unwrap();
        let tree = head.tree().unwrap();
        assert_eq!(tree.len(), 1);
        assert!(tree.get_name("articles").is_some());
        assert!(tree.get_name("notes.txt").is_none());
        assert!(workspace.join("assets/image.png").exists());
        assert!(!workspace.join(".git").exists());
    }

    #[test]
    fn index_rebuild_only_reindexes_markdown_and_does_not_change_files() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let article = workspace.join("articles/doc/article.md");
        let content = "---\ntitle: Rebuild\ncustom: value\n---\n\n# Keep me\n";
        atomic_write(&article, content).unwrap();
        let before = digest(&fs::read_to_string(&article).unwrap());
        let db = test_db();
        sync_index(&db, &workspace, None).unwrap();
        db.execute("DELETE FROM documents", []).unwrap();
        sync_index(&db, &workspace, None).unwrap();
        let title: String = db
            .query_row("SELECT title FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(title, "Rebuild");
        assert_eq!(digest(&fs::read_to_string(&article).unwrap()), before);
        fs::remove_file(&article).unwrap();
        sync_index(&db, &workspace, None).unwrap();
        let remaining: i64 = db
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn referenced_assets_are_listed_and_missing_assets_are_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let article_dir = workspace.join("articles/doc");
        fs::create_dir_all(article_dir.join("assets")).unwrap();
        fs::write(article_dir.join("article.md"), "# Article").unwrap();
        fs::write(article_dir.join("assets/image.png"), b"image").unwrap();
        let markdown = "# Article\n\n![alt](assets/image.png)\n";
        let assets = list_document_assets(&workspace, "articles/doc/article.md", markdown).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].mime_type, "image/png");
        let missing = list_document_assets(
            &workspace,
            "articles/doc/article.md",
            "![missing](assets/missing.png)",
        )
        .unwrap();
        assert_eq!(missing.len(), 1);
        assert!(!missing[0].available);
        assert!(missing[0].warning.is_some());
    }

    #[test]
    fn article_asset_references_are_checked_before_saving() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        setup_workspace(&workspace).unwrap();
        let document_dir = workspace.join("articles/doc");
        fs::create_dir_all(document_dir.join("assets")).unwrap();
        fs::write(document_dir.join("assets/image.png"), b"image").unwrap();
        assert!(
            validate_asset_references(
                &workspace,
                "articles/doc/article.md",
                "![image](assets/image.png)",
            )
            .is_ok()
        );
        assert!(
            validate_asset_references(
                &workspace,
                "articles/doc/article.md",
                "![missing](assets/missing.png)",
            )
            .is_err()
        );
        assert!(
            validate_asset_references(
                &workspace,
                "articles/doc/article.md",
                "![escape](../../outside.png)",
            )
            .is_err()
        );
        assert!(
            validate_asset_references(
                &workspace,
                "articles/doc/article.md",
                "[external](https://example.com/article)",
            )
            .is_ok()
        );
        assert!(
            validate_asset_references(
                &workspace,
                "articles/doc/article.md",
                "![remote](https://example.com/cover.png)",
            )
            .is_ok()
        );
        assert!(
            validate_asset_references(
                &workspace,
                "articles/doc/article.md",
                "![other](other/image.png)",
            )
            .is_err()
        );
    }

    #[test]
    fn save_and_restore_base_hash_reject_external_changes() {
        let original = "# Original\n";
        let external = "# External\n";
        assert!(ensure_base_hash(original, Some(&digest(original))).is_ok());
        let error = ensure_base_hash(external, Some(&digest(original))).unwrap_err();
        assert!(error.contains("其他位置修改"));
        assert!(ensure_base_hash(external, None).is_ok());
    }
}
