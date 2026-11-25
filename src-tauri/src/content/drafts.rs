use super::*;
use super::package::{
    copy_asset, package_directory, read_content_package, validate_asset_path, validate_assets,
    validate_media_order,
};
use super::paths::{
    atomic_write, digest, draft_directory, normalized_tags, normalized_title, validate_component,
};

fn parse_draft(directory: &Path) -> Result<PlatformDraft, String> {
    let raw = fs::read(directory.join("draft.json")).map_err(|e| e.to_string())?;
    let mut draft: PlatformDraft =
        serde_json::from_slice(&raw).map_err(|e| format!("平台稿格式无效：{e}"))?;
    let current_hash = digest(
        &serde_json::to_vec(&serde_json::json!({
            "title": draft.title,
            "summary": draft.summary,
            "tags": draft.tags,
            "cover": draft.cover,
            "media": draft.media,
            "body": draft.body,
        }))
        .map_err(|e| e.to_string())?,
    );
    draft.hash = current_hash;
    Ok(draft)
}

fn write_draft(directory: &Path, draft: &PlatformDraft) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(draft).map_err(|e| e.to_string())?;
    atomic_write(&directory.join("draft.json"), &bytes)
}

fn draft_hash(draft: &PlatformDraft) -> Result<String, String> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "title": draft.title,
        "summary": draft.summary,
        "tags": draft.tags,
        "cover": draft.cover,
        "media": draft.media,
        "body": draft.body,
    }))
    .map_err(|e| e.to_string())?;
    Ok(digest(&bytes))
}

fn draft_with_source_state(workspace: &Path, mut draft: PlatformDraft) -> PlatformDraft {
    draft.hash = draft_hash(&draft).unwrap_or_default();
    draft.source_changed = read_content_package(workspace, &draft.content_id)
        .map(|content| content.content_version != draft.source_version)
        .unwrap_or(true);
    draft
}

fn adapt_text(content: &ContentPackage, target: &DraftTarget) -> String {
    match content.manifest.kind {
        ContentKind::Article => content.body.clone(),
        ContentKind::ImageArticle => content.body.clone(),
        ContentKind::ImageText => {
            let captions = content
                .manifest
                .media
                .iter()
                .filter(|asset| !asset.caption.trim().is_empty())
                .map(|asset| format!("{}. {}", asset.order, asset.caption.trim()))
                .collect::<Vec<_>>();
            let base = if content.manifest.summary.trim().is_empty() {
                content.manifest.title.clone()
            } else {
                content.manifest.summary.clone()
            };
            if captions.is_empty() {
                base
            } else {
                format!("{base}\n\n{}", captions.join("\n"))
            }
        }
        ContentKind::Video => {
            if content.manifest.summary.trim().is_empty() {
                format!(
                    "{}\n\n（来自 {} 的视频主稿）",
                    content.manifest.title, target.platform_name
                )
            } else {
                content.manifest.summary.clone()
            }
        }
    }
}

pub fn create_platform_drafts(
    workspace: &Path,
    content_id: &str,
    targets: &[DraftTarget],
) -> Result<Vec<PlatformDraft>, String> {
    if targets.is_empty() {
        return Err("请至少选择一个适配目标".to_string());
    }
    let content = read_content_package(workspace, content_id)?;
    let source_dir = package_directory(workspace, content_id)?;
    let mut output = Vec::new();
    for target in targets {
        let pipeline_id = format!("platform-{}", Uuid::new_v4());
        let directory = draft_directory(workspace, &pipeline_id)?;
        fs::create_dir_all(directory.join("assets")).map_err(|e| e.to_string())?;
        let cover = content
            .manifest
            .cover
            .as_ref()
            .map(|asset| copy_asset(&source_dir, &directory, asset))
            .transpose()?;
        let mut media = content
            .manifest
            .media
            .iter()
            .map(|asset| copy_asset(&source_dir, &directory, asset))
            .collect::<Result<Vec<_>, _>>()?;
        validate_media_order(&mut media)?;
        let now = Utc::now().to_rfc3339();
        let mut draft = PlatformDraft {
            id: pipeline_id.clone(),
            content_id: content.manifest.id.clone(),
            platform_id: target.platform_id.clone(),
            platform_name: target.platform_name.clone(),
            account_id: target.account_id.clone(),
            account_alias: target.account_alias.clone(),
            adapter_id: target.adapter_id.clone(),
            title: content.manifest.title.clone(),
            summary: content.manifest.summary.clone(),
            body: adapt_text(&content, target),
            tags: content.manifest.tags.clone(),
            cover,
            media,
            source_version: content.content_version.clone(),
            generated_at: now.clone(),
            updated_at: now,
            manually_edited: false,
            hash: String::new(),
            source_changed: false,
        };
        draft.hash = draft_hash(&draft)?;
        write_draft(&directory, &draft)?;
        output.push(draft_with_source_state(workspace, draft));
    }
    Ok(output)
}

pub fn list_platform_drafts(
    workspace: &Path,
    content_id: Option<&str>,
) -> Result<Vec<PlatformDraft>, String> {
    let root = draft_root(workspace);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut output = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let draft = match parse_draft(&entry.path()) {
            Ok(draft) => draft_with_source_state(workspace, draft),
            Err(_) => continue,
        };
        if content_id.is_some_and(|id| id != draft.content_id) {
            continue;
        }
        output.push(draft);
    }
    output.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(output)
}

pub fn read_platform_draft(workspace: &Path, draft_id: &str) -> Result<PlatformDraft, String> {
    let directory = draft_directory(workspace, draft_id)?;
    if !directory.is_dir() {
        return Err("未找到平台稿".to_string());
    }
    parse_draft(&directory).map(|draft| draft_with_source_state(workspace, draft))
}

fn find_draft_by_content<'a>(
    drafts: &'a [PlatformDraft],
    content_id: &str,
) -> Option<&'a PlatformDraft> {
    drafts.iter().find(|draft| draft.content_id == content_id)
}

pub fn save_platform_draft(
    workspace: &Path,
    draft_id: &str,
    input: DraftSaveInput,
) -> Result<PlatformDraft, String> {
    let directory = draft_directory(workspace, draft_id)?;
    let mut draft = parse_draft(&directory)?;
    if input.base_hash != draft_hash(&draft)? {
        return Err("平台稿已在其他位置修改；请重新载入后重试".to_string());
    }
    draft.title = normalized_title(&input.title)?;
    draft.summary = input.summary.trim().to_string();
    draft.tags = normalized_tags(input.tags)?;
    draft.cover = input.cover;
    draft.media = input.media;
    validate_media_order(&mut draft.media)?;
    validate_assets(&directory, draft.cover.as_ref(), &draft.media)?;
    draft.body = input.body;
    draft.updated_at = Utc::now().to_rfc3339();
    draft.manually_edited = true;
    draft.hash = draft_hash(&draft)?;
    write_draft(&directory, &draft)?;
    Ok(draft_with_source_state(workspace, draft))
}

pub fn acknowledge_platform_draft_source(
    workspace: &Path,
    draft_id: &str,
) -> Result<PlatformDraft, String> {
    let directory = draft_directory(workspace, draft_id)?;
    let mut draft = parse_draft(&directory)?;
    let content = read_content_package(workspace, &draft.content_id)?;
    draft.source_version = content.content_version;
    draft.updated_at = Utc::now().to_rfc3339();
    draft.hash = draft_hash(&draft)?;
    write_draft(&directory, &draft)?;
    Ok(draft_with_source_state(workspace, draft))
}

pub fn regenerate_platform_draft(
    workspace: &Path,
    draft_id: &str,
) -> Result<PlatformDraft, String> {
    let directory = draft_directory(workspace, draft_id)?;
    let previous = parse_draft(&directory)?;
    let content = read_content_package(workspace, &previous.content_id)?;
    let source_dir = package_directory(workspace, &previous.content_id)?;
    let assets_dir = directory.join("assets");
    if assets_dir.exists() {
        fs::remove_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;
    let target = DraftTarget {
        platform_id: previous.platform_id.clone(),
        platform_name: previous.platform_name.clone(),
        account_id: previous.account_id.clone(),
        account_alias: previous.account_alias.clone(),
        adapter_id: previous.adapter_id.clone(),
    };
    let cover = content
        .manifest
        .cover
        .as_ref()
        .map(|asset| copy_asset(&source_dir, &directory, asset))
        .transpose()?;
    let mut media = content
        .manifest
        .media
        .iter()
        .map(|asset| copy_asset(&source_dir, &directory, asset))
        .collect::<Result<Vec<_>, _>>()?;
    validate_media_order(&mut media)?;
    let mut draft = PlatformDraft {
        title: content.manifest.title.clone(),
        summary: content.manifest.summary.clone(),
        body: adapt_text(&content, &target),
        tags: content.manifest.tags.clone(),
        cover,
        media,
        source_version: content.content_version,
        updated_at: Utc::now().to_rfc3339(),
        manually_edited: false,
        ..previous
    };
    draft.hash = draft_hash(&draft)?;
    write_draft(&directory, &draft)?;
    Ok(draft_with_source_state(workspace, draft))
}

pub fn read_platform_draft_asset(
    workspace: &Path,
    draft_id: &str,
    asset_id: &str,
) -> Result<ContentAssetContent, String> {
    let directory = draft_directory(workspace, draft_id)?;
    let draft = parse_draft(&directory)?;
    let asset = draft
        .media
        .iter()
        .chain(draft.cover.iter())
        .find(|asset| asset.id == asset_id)
        .ok_or("平台稿中不存在该素材")?;
    let path = validate_asset_path(&directory, &asset.file)?;
    Ok(ContentAssetContent {
        bytes: fs::read(path).map_err(|e| e.to_string())?,
        mime_type: asset.mime_type.clone(),
    })
}

#[allow(dead_code)]
pub fn first_draft_for_content(
    workspace: &Path,
    content_id: &str,
) -> Result<Option<PlatformDraft>, String> {
    Ok(find_draft_by_content(
        &list_platform_drafts(workspace, Some(content_id))?,
        content_id,
    )
    .cloned())
}

