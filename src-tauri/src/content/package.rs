use super::paths::{
    atomic_write, content_root, digest, normalized_tags, normalized_title, package_directory,
    validate_component,
};
use super::*;
use chrono::Utc;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

pub(super) fn validate_asset_path(package_dir: &Path, file: &str) -> Result<PathBuf, String> {
    let path = Path::new(file);
    let components = path.components().collect::<Vec<_>>();
    if components.len() != 2
        || components[0].as_os_str() != "assets"
        || !components
            .iter()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err("素材路径必须是 assets/<file>".to_string());
    }
    let candidate = package_dir.join(path);
    let assets_dir = package_dir.join("assets");
    if fs::symlink_metadata(&assets_dir).is_ok_and(|metadata| metadata.file_type().is_symlink())
        || fs::symlink_metadata(&candidate).is_ok_and(|metadata| metadata.file_type().is_symlink())
    {
        return Err("素材路径不能经过符号链接".to_string());
    }
    if !candidate.is_file() {
        return Err(format!("素材文件不存在：{file}"));
    }
    let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
    let root = package_dir.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(root) {
        return Err("素材路径超出内容包范围".to_string());
    }
    Ok(candidate)
}

pub(super) fn validate_assets(
    package_dir: &Path,
    cover: Option<&ContentAsset>,
    media: &[ContentAsset],
) -> Result<(), String> {
    let mut ids = std::collections::HashSet::new();
    for asset in media {
        validate_component(&asset.id, "素材 ID")?;
        if !ids.insert(asset.id.clone()) {
            return Err("素材 ID 重复".to_string());
        }
        validate_asset_path(package_dir, &asset.file)?;
        if asset.original_name.chars().count() > 255 {
            return Err("素材原始文件名过长".to_string());
        }
        if asset.mime_type.trim().is_empty() {
            return Err("素材 MIME 类型不能为空".to_string());
        }
    }
    if let Some(asset) = cover {
        validate_component(&asset.id, "封面素材 ID")?;
        validate_asset_path(package_dir, &asset.file)?;
        if asset.original_name.chars().count() > 255 {
            return Err("封面原始文件名过长".to_string());
        }
        if asset.mime_type.trim().is_empty() {
            return Err("封面 MIME 类型不能为空".to_string());
        }
    }
    if let Some(cover) = cover
        && cover.kind != AssetKind::Image
    {
        return Err("封面必须是图片素材".to_string());
    }
    Ok(())
}

pub(super) fn validate_media_order(media: &mut [ContentAsset]) -> Result<(), String> {
    if media.is_empty() {
        return Ok(());
    }
    media.sort_by_key(|asset| asset.order);
    for (index, asset) in media.iter_mut().enumerate() {
        asset.order = (index + 1) as u32;
    }
    let mut ids = std::collections::HashSet::new();
    if !media.iter().all(|asset| ids.insert(asset.id.clone())) {
        return Err("素材 ID 重复".to_string());
    }
    Ok(())
}

pub(super) fn manifest_bytes(manifest: &ContentManifest) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(manifest).map_err(|e| e.to_string())
}

pub(super) fn content_version(manifest: &ContentManifest, body: &str) -> Result<String, String> {
    let mut media = manifest.media.clone();
    media.sort_by_key(|asset| asset.order);
    let view = serde_json::json!({
        "kind": manifest.kind,
        "title": manifest.title,
        "summary": manifest.summary,
        "tags": manifest.tags,
        "cover": manifest.cover,
        "media": media,
        "body": body,
    });
    let bytes = serde_json::to_vec(&view).map_err(|e| e.to_string())?;
    Ok(digest(&bytes))
}

pub(super) fn read_manifest(package_dir: &Path) -> Result<(ContentManifest, Vec<u8>), String> {
    let path = package_dir.join(MANIFEST_NAME);
    let bytes = fs::read(&path).map_err(|e| format!("无法读取内容清单：{e}"))?;
    let manifest: ContentManifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("内容清单格式无效：{e}"))?;
    if manifest.schema_version != CONTENT_SCHEMA_VERSION {
        return Err(format!("不支持的内容清单版本：{}", manifest.schema_version));
    }
    Ok((manifest, bytes))
}

pub(super) fn read_body(package_dir: &Path, kind: ContentKind) -> Result<String, String> {
    if !kind.requires_markdown() {
        return Ok(String::new());
    }
    let path = package_dir.join(BODY_NAME);
    match fs::read_to_string(path) {
        Ok(body) => Ok(body),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("无法读取 Markdown 正文：{error}")),
    }
}

pub(super) fn package_from_directory(
    workspace: &Path,
    package_dir: &Path,
) -> Result<ContentPackage, String> {
    let (mut manifest, manifest_raw) = read_manifest(package_dir)?;
    validate_media_order(&mut manifest.media)?;
    validate_assets(package_dir, manifest.cover.as_ref(), &manifest.media)?;
    let body = read_body(package_dir, manifest.kind)?;
    let relative = package_dir
        .strip_prefix(workspace)
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .replace('\\', "/");
    let version = content_version(&manifest, &body)?;
    Ok(ContentPackage {
        manifest,
        body,
        path: relative,
        content_hash: digest(&manifest_raw),
        content_version: version,
    })
}

pub fn list_content_packages(workspace: &Path) -> Result<Vec<ContentPackageSummary>, String> {
    let root = content_root(workspace);
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut packages = Vec::new();
    for entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        if fs::symlink_metadata(entry.path())
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            continue;
        }
        let package = match package_from_directory(workspace, &entry.path()) {
            Ok(package) => package,
            Err(_) => continue,
        };
        packages.push(ContentPackageSummary {
            id: package.manifest.id.clone(),
            kind: package.manifest.kind,
            title: package.manifest.title.clone(),
            summary: package.manifest.summary.clone(),
            tags: package.manifest.tags.clone(),
            updated_at: package.manifest.updated_at.clone(),
            has_cover: package.manifest.cover.is_some(),
            media_count: package.manifest.media.len(),
            path: package.path,
            content_version: package.content_version,
        });
    }
    packages.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(packages)
}

pub fn create_content_package(
    workspace: &Path,
    kind: ContentKind,
    title: &str,
) -> Result<ContentPackage, String> {
    let title = normalized_title(title)?;
    let id = Uuid::new_v4().to_string();
    let package_dir = package_directory(workspace, &id)?;
    fs::create_dir_all(package_dir.join("assets")).map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    let manifest = ContentManifest {
        schema_version: CONTENT_SCHEMA_VERSION,
        id: id.clone(),
        kind,
        title: title.clone(),
        summary: String::new(),
        tags: Vec::new(),
        cover: None,
        media: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        source: None,
    };
    let body = if kind.requires_markdown() {
        format!("# {title}\n\n")
    } else {
        String::new()
    };
    if kind.requires_markdown() {
        atomic_write(&package_dir.join(BODY_NAME), body.as_bytes())?;
    }
    let raw = manifest_bytes(&manifest)?;
    atomic_write(&package_dir.join(MANIFEST_NAME), &raw)?;
    package_from_directory(workspace, &package_dir)
}

pub fn read_content_package(workspace: &Path, package_id: &str) -> Result<ContentPackage, String> {
    let package_dir = package_directory(workspace, package_id)?;
    if !package_dir.is_dir() {
        return Err("未找到内容包".to_string());
    }
    package_from_directory(workspace, &package_dir)
}

pub fn save_content_package(
    workspace: &Path,
    package_id: &str,
    input: ContentSaveInput,
) -> Result<ContentPackage, String> {
    let package_dir = package_directory(workspace, package_id)?;
    if !package_dir.is_dir() {
        return Err("未找到内容包".to_string());
    }
    let (mut manifest, raw) = read_manifest(&package_dir)?;
    if let Some(expected) = input.base_hash.as_deref()
        && expected != digest(&raw)
    {
        return Err("内容清单已在其他位置修改；请重新载入后重试".to_string());
    }
    manifest.title = normalized_title(&input.title)?;
    manifest.summary = input.summary.trim().to_string();
    manifest.tags = normalized_tags(input.tags)?;
    manifest.cover = input.cover;
    manifest.media = input.media;
    validate_media_order(&mut manifest.media)?;
    validate_assets(&package_dir, manifest.cover.as_ref(), &manifest.media)?;
    manifest.updated_at = Utc::now().to_rfc3339();

    if manifest.kind.requires_markdown() {
        atomic_write(&package_dir.join(BODY_NAME), input.body.as_bytes())?;
    } else if package_dir.join(BODY_NAME).exists() {
        fs::remove_file(package_dir.join(BODY_NAME)).map_err(|e| e.to_string())?;
    }
    let raw = manifest_bytes(&manifest)?;
    atomic_write(&package_dir.join(MANIFEST_NAME), &raw)?;
    package_from_directory(workspace, &package_dir)
}

pub fn delete_content_package(workspace: &Path, package_id: &str) -> Result<(), String> {
    let package_dir = package_directory(workspace, package_id)?;
    let metadata = fs::symlink_metadata(&package_dir).map_err(|_| "未找到内容包".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("内容包路径不是安全的普通目录".to_string());
    }
    fs::remove_dir_all(package_dir).map_err(|error| format!("内容包删除失败：{error}"))
}

pub fn import_content_asset(
    workspace: &Path,
    package_id: &str,
    role: &str,
    source_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<ContentAsset, String> {
    if bytes.is_empty() {
        return Err("素材文件为空".to_string());
    }
    if bytes.len() > MAX_MEDIA_BYTES {
        return Err("素材不能超过 512 MiB".to_string());
    }
    let package_dir = package_directory(workspace, package_id)?;
    let (manifest, _) = read_manifest(&package_dir)?;
    let is_video = mime_type.starts_with("video/");
    let is_image = mime_type.starts_with("image/");
    let kind = if is_video {
        AssetKind::Video
    } else if is_image {
        AssetKind::Image
    } else {
        return Err("仅支持图片或视频素材".to_string());
    };
    if role == "cover" && kind != AssetKind::Image {
        return Err("封面必须是图片".to_string());
    }
    if role == "media" && kind == AssetKind::Video && !manifest.kind.accepts_video() {
        return Err("当前内容形态不接受视频素材".to_string());
    }
    if role == "media" && kind == AssetKind::Image && manifest.kind == ContentKind::Video {
        return Err("视频内容可以将图片设为封面，但不能加入图集媒体".to_string());
    }
    if role != "media" && role != "cover" {
        return Err("未知素材用途".to_string());
    }
    let safe_name = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("asset")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let safe_name = safe_name.trim_matches('-');
    let safe_name = if safe_name.is_empty() {
        "asset"
    } else {
        safe_name
    };
    let id = Uuid::new_v4().to_string();
    let file_name = format!("{id}-{safe_name}");
    let relative = format!("assets/{file_name}");
    let target = package_dir.join(&relative);
    atomic_write(&target, bytes)?;
    let order = manifest
        .media
        .iter()
        .map(|asset| asset.order)
        .max()
        .unwrap_or(0)
        + 1;
    Ok(ContentAsset {
        id,
        kind,
        file: relative,
        original_name: source_name.to_string(),
        mime_type: mime_type.to_string(),
        byte_size: bytes.len() as u64,
        caption: String::new(),
        order,
    })
}

pub fn read_content_asset(
    workspace: &Path,
    package_id: &str,
    asset_id: &str,
) -> Result<ContentAssetContent, String> {
    let package_dir = package_directory(workspace, package_id)?;
    let (manifest, _) = read_manifest(&package_dir)?;
    let asset = manifest
        .media
        .iter()
        .chain(manifest.cover.iter())
        .find(|asset| asset.id == asset_id)
        .ok_or("内容包中不存在该素材")?;
    let path = validate_asset_path(&package_dir, &asset.file)?;
    Ok(ContentAssetContent {
        bytes: fs::read(path).map_err(|e| e.to_string())?,
        mime_type: asset.mime_type.clone(),
    })
}

pub(super) fn copy_asset(
    source_package_dir: &Path,
    target_package_dir: &Path,
    asset: &ContentAsset,
) -> Result<ContentAsset, String> {
    let source = validate_asset_path(source_package_dir, &asset.file)?;
    let id = Uuid::new_v4().to_string();
    let extension = Path::new(&asset.file)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("bin");
    let relative = format!("assets/{id}.{extension}");
    let bytes = fs::read(source).map_err(|e| e.to_string())?;
    atomic_write(&target_package_dir.join(&relative), &bytes)?;
    let mut copy = asset.clone();
    copy.id = id;
    copy.file = relative;
    Ok(copy)
}

pub fn derive_content_package(
    workspace: &Path,
    source_id: &str,
    target_kind: ContentKind,
    title: Option<String>,
) -> Result<ContentPackage, String> {
    let source = read_content_package(workspace, source_id)?;
    let title = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}（派生稿）", source.manifest.title));
    let id = Uuid::new_v4().to_string();
    let package_dir = package_directory(workspace, &id)?;
    let source_dir = package_directory(workspace, source_id)?;
    fs::create_dir_all(package_dir.join("assets")).map_err(|e| e.to_string())?;
    let cover = source
        .manifest
        .cover
        .as_ref()
        .map(|asset| copy_asset(&source_dir, &package_dir, asset))
        .transpose()?;
    let media = source
        .manifest
        .media
        .iter()
        .map(|asset| copy_asset(&source_dir, &package_dir, asset))
        .collect::<Result<Vec<_>, _>>()?;
    let now = Utc::now().to_rfc3339();
    let body = if target_kind.requires_markdown() {
        let base = if source.body.trim().is_empty() {
            format!("# {}\n\n{}", source.manifest.title, source.manifest.summary)
        } else {
            source.body.clone()
        };
        let mut references = Vec::new();
        for asset in &media {
            let marker = format!("asset:{}", asset.id);
            if !base.contains(&marker) {
                let prefix = if asset.kind == AssetKind::Image {
                    "!"
                } else {
                    ""
                };
                references.push(format!("{prefix}[{}]({marker})", asset.caption));
            }
        }
        if references.is_empty() {
            base
        } else {
            format!("{base}\n\n{}\n", references.join("\n\n"))
        }
    } else {
        String::new()
    };
    let manifest = ContentManifest {
        schema_version: CONTENT_SCHEMA_VERSION,
        id: id.clone(),
        kind: target_kind,
        title: normalized_title(&title)?,
        summary: source.manifest.summary.clone(),
        tags: source.manifest.tags.clone(),
        cover,
        media,
        created_at: now.clone(),
        updated_at: now,
        source: Some(ContentSourceRef {
            content_id: source.manifest.id,
            version: source.content_version,
        }),
    };
    if target_kind.requires_markdown() {
        atomic_write(&package_dir.join(BODY_NAME), body.as_bytes())?;
    }
    let raw = manifest_bytes(&manifest)?;
    atomic_write(&package_dir.join(MANIFEST_NAME), &raw)?;
    package_from_directory(workspace, &package_dir)
}
