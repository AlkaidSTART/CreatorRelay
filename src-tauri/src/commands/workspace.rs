use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn get_workspace(state: State<'_, AppState>) -> Result<Option<WorkspaceInfo>, String> {
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
pub(crate) fn set_workspace_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<WorkspaceInfo, String> {
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
pub(crate) fn list_documents(
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
pub(crate) fn rebuild_workspace_index(state: State<'_, AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    sync_index(&conn, &workspace, Some(&state.history_root))?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    usize::try_from(count).map_err(|_| "文档数量超出可表示范围".to_string())
}

#[tauri::command]
pub(crate) fn read_document(
    path: String,
    state: State<'_, AppState>,
) -> Result<DocumentContent, String> {
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
pub(crate) fn import_document_bundle(
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

pub(crate) fn list_document_assets_in_export(
    source: &Path,
    markdown: &str,
) -> Result<Vec<String>, String> {
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
pub(crate) fn import_markdown(
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
pub(crate) fn create_document(
    title: String,
    state: State<'_, AppState>,
) -> Result<Document, String> {
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
pub(crate) fn export_document_bundle(
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
pub(crate) fn import_workspace_asset(
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
pub(crate) fn list_document_assets_for_document(
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
pub(crate) fn read_workspace_asset(
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
pub(crate) fn reveal_workspace_asset(
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
pub(crate) fn preserve_external_document(
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
pub(crate) fn save_document(
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
pub(crate) fn list_revisions(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<Revision>, String> {
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
pub(crate) fn create_document_revision(
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
pub(crate) fn restore_revision(
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
