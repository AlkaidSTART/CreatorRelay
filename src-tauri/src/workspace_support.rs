use super::*;

pub(crate) fn digest(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

pub(crate) fn ensure_base_hash(content: &str, base_hash: Option<&str>) -> Result<(), String> {
    if base_hash.is_some_and(|expected| expected != digest(content)) {
        return Err("此文档已在其他位置修改。为避免覆盖，请重新载入后再操作。".to_string());
    }
    Ok(())
}

pub(crate) fn history_repository_path(
    history_root: &Path,
    workspace: &Path,
) -> Result<PathBuf, String> {
    let workspace = workspace.canonicalize().map_err(|e| e.to_string())?;
    let identity = digest(&workspace.to_string_lossy());
    Ok(history_root.join(format!("{identity}.git")))
}

pub(crate) fn open_history_repository(
    history_root: &Path,
    workspace: &Path,
) -> Result<Repository, String> {
    fs::create_dir_all(history_root).map_err(|e| e.to_string())?;
    let path = history_repository_path(history_root, workspace)?;
    if path.exists() {
        Repository::open_bare(path).map_err(|e| e.message().to_string())
    } else {
        Repository::init_bare(path).map_err(|e| e.message().to_string())
    }
}

pub(crate) fn managed_history_path(relative: &str) -> Result<(String, String), String> {
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

pub(crate) fn copy_tree_entries(
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

pub(crate) fn tree_blob_id(tree: &Tree<'_>, relative: &str) -> Option<git2::Oid> {
    tree.get_path(Path::new(relative))
        .ok()
        .map(|entry| entry.id())
}

pub(crate) fn tree_with_document<'repo>(
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

pub(crate) fn current_head(repo: &Repository) -> Result<Option<Commit<'_>>, String> {
    match repo.head() {
        Ok(reference) => reference
            .target()
            .map(|oid| repo.find_commit(oid).map_err(|e| e.message().to_string()))
            .transpose(),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(error.message().to_string()),
    }
}

pub(crate) fn commit_document_content(
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

pub(crate) fn ensure_document_history(
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

pub(crate) fn format_revision_diff(
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

pub(crate) fn list_document_revisions(
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
            diff: format_revision_diff(repo, &commit, relative)?,
        });
    }
    Ok(revisions)
}

pub(crate) fn app_db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("creatorrelay.sqlite3"))
}

pub(crate) fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(debug_assertions)]
    if let Ok(path) = std::env::var("CREATORRELAY_DATA_DIR")
        && !path.trim().is_empty()
    {
        return Ok(PathBuf::from(path));
    }
    app.path().app_data_dir().map_err(|e| e.to_string())
}

pub(crate) fn log_command_outcome<T>(
    state: &AppState,
    stage: &str,
    context: &str,
    result: &Result<T, String>,
) {
    let message = match result {
        Ok(_) => format!("成功 context={context}"),
        Err(error) => format!("失败 context={context} error={error}"),
    };
    let level = if result.is_ok() { "info" } else { "error" };
    let _ = logging::append_log(&state.log_path, level, stage, &message);
}

pub(crate) fn markdown_url_is_external(url: &str) -> bool {
    ["http://", "https://", "mailto:", "tel:", "data:", "#", "?"]
        .iter()
        .any(|prefix| url.starts_with(prefix))
}

pub(crate) fn markdown_url_is_unsafe_local(url: &str) -> bool {
    url.starts_with('/')
        || url.starts_with("file:")
        || url.split('/').any(|component| component == "..")
}

pub(crate) fn list_document_assets(
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

pub(crate) fn validate_asset_references(
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

pub(crate) fn setup_workspace(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path.join("articles")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("contents")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("platform-drafts")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("assets")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("exports")).map_err(|e| e.to_string())?;
    Ok(())
}

pub(crate) fn require_active_workspace(state: &State<'_, AppState>) -> Result<PathBuf, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())
}
