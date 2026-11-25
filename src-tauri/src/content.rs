use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};
use tempfile::NamedTempFile;
use uuid::Uuid;

pub const CONTENT_SCHEMA_VERSION: u32 = 1;
const MANIFEST_NAME: &str = "content.json";
const BODY_NAME: &str = "body.md";
const MAX_CONTENT_TITLE_CHARS: usize = 200;
const MAX_TAGS: usize = 20;
const MAX_MEDIA_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Video,
    ImageText,
    ImageArticle,
    Article,
}

impl ContentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::ImageText => "imageText",
            Self::ImageArticle => "imageArticle",
            Self::Article => "article",
        }
    }

    pub fn requires_markdown(self) -> bool {
        matches!(self, Self::ImageArticle | Self::Article)
    }

    pub fn accepts_video(self) -> bool {
        matches!(self, Self::Video | Self::ImageArticle | Self::Article)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Video,
    Image,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAsset {
    pub id: String,
    pub kind: AssetKind,
    pub file: String,
    pub original_name: String,
    pub mime_type: String,
    pub byte_size: u64,
    #[serde(default)]
    pub caption: String,
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSourceRef {
    pub content_id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentManifest {
    pub schema_version: u32,
    pub id: String,
    pub kind: ContentKind,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub cover: Option<ContentAsset>,
    #[serde(default)]
    pub media: Vec<ContentAsset>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub source: Option<ContentSourceRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPackage {
    #[serde(flatten)]
    pub manifest: ContentManifest,
    pub body: String,
    pub path: String,
    pub content_hash: String,
    pub content_version: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentPackageSummary {
    pub id: String,
    pub kind: ContentKind,
    pub title: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub updated_at: String,
    pub has_cover: bool,
    pub media_count: usize,
    pub path: String,
    pub content_version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSaveInput {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub cover: Option<ContentAsset>,
    #[serde(default)]
    pub media: Vec<ContentAsset>,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub base_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentAssetContent {
    pub bytes: Vec<u8>,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftTarget {
    pub platform_id: String,
    pub platform_name: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub account_alias: String,
    #[serde(default)]
    pub adapter_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDraft {
    pub id: String,
    pub content_id: String,
    pub platform_id: String,
    pub platform_name: String,
    pub account_id: String,
    pub account_alias: String,
    pub adapter_id: Option<String>,
    pub title: String,
    pub summary: String,
    pub body: String,
    pub tags: Vec<String>,
    pub cover: Option<ContentAsset>,
    pub media: Vec<ContentAsset>,
    pub source_version: String,
    pub generated_at: String,
    pub updated_at: String,
    pub manually_edited: bool,
    pub hash: String,
    #[serde(default)]
    pub source_changed: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSaveInput {
    pub title: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub cover: Option<ContentAsset>,
    #[serde(default)]
    pub media: Vec<ContentAsset>,
    #[serde(default)]
    pub body: String,
    pub base_hash: String,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn content_root(workspace: &Path) -> PathBuf {
    workspace.join("contents")
}

fn draft_root(workspace: &Path) -> PathBuf {
    workspace.join("platform-drafts")
}

fn validate_component(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(format!("{label}格式无效"));
    }
    Ok(())
}

fn package_directory(workspace: &Path, package_id: &str) -> Result<PathBuf, String> {
    validate_component(package_id, "内容 ID")?;
    let root = content_root(workspace);
    let candidate = root.join(package_id);
    if root.exists() {
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
        if candidate.exists() {
            let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
            if !canonical.starts_with(&canonical_root) {
                return Err("内容路径超出工作区范围".to_string());
            }
        }
    }
    Ok(candidate)
}

fn draft_directory(workspace: &Path, draft_id: &str) -> Result<PathBuf, String> {
    validate_component(draft_id, "平台稿 ID")?;
    let root = draft_root(workspace);
    let candidate = root.join(draft_id);
    if root.exists() {
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
        if candidate.exists() {
            let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
            if !canonical.starts_with(&canonical_root) {
                return Err("平台稿路径超出工作区范围".to_string());
            }
        }
    }
    Ok(candidate)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("目标文件没有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

fn normalized_title(value: &str) -> Result<String, String> {
    let title = value.trim();
    if title.is_empty() {
        return Err("标题不能为空".to_string());
    }
    if title.chars().count() > MAX_CONTENT_TITLE_CHARS {
        return Err(format!("标题不能超过 {MAX_CONTENT_TITLE_CHARS} 个字符"));
    }
    if title.chars().any(char::is_control) {
        return Err("标题包含非法控制字符".to_string());
    }
    Ok(title.to_string())
}

fn normalized_tags(tags: Vec<String>) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 40 {
            return Err("单个标签不能超过 40 个字符".to_string());
        }
        if !output.iter().any(|existing: &String| existing == tag) {
            output.push(tag.to_string());
        }
    }
    if output.len() > MAX_TAGS {
        return Err(format!("标签不能超过 {MAX_TAGS} 个"));
    }
    Ok(output)
}

fn validate_asset_path(package_dir: &Path, file: &str) -> Result<PathBuf, String> {
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

fn validate_assets(
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

fn validate_media_order(media: &mut [ContentAsset]) -> Result<(), String> {
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

fn manifest_bytes(manifest: &ContentManifest) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(manifest).map_err(|e| e.to_string())
}

fn content_version(manifest: &ContentManifest, body: &str) -> Result<String, String> {
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

fn read_manifest(package_dir: &Path) -> Result<(ContentManifest, Vec<u8>), String> {
    let path = package_dir.join(MANIFEST_NAME);
    let bytes = fs::read(&path).map_err(|e| format!("无法读取内容清单：{e}"))?;
    let manifest: ContentManifest =
        serde_json::from_slice(&bytes).map_err(|e| format!("内容清单格式无效：{e}"))?;
    if manifest.schema_version != CONTENT_SCHEMA_VERSION {
        return Err(format!("不支持的内容清单版本：{}", manifest.schema_version));
    }
    Ok((manifest, bytes))
}

fn read_body(package_dir: &Path, kind: ContentKind) -> Result<String, String> {
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

fn package_from_directory(workspace: &Path, package_dir: &Path) -> Result<ContentPackage, String> {
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

fn copy_asset(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn workspace() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("contents")).unwrap();
        fs::create_dir_all(temp.path().join("platform-drafts")).unwrap();
        temp
    }

    fn image_bytes() -> &'static [u8] {
        b"\x89PNG\r\n\x1a\nfixture-image"
    }

    fn video_bytes() -> &'static [u8] {
        b"\x00\x00\x00\x18ftypmp42fixture-video"
    }

    #[test]
    fn creates_all_four_kinds_and_survives_restart() {
        let temp = workspace();
        for kind in [
            ContentKind::Video,
            ContentKind::ImageText,
            ContentKind::ImageArticle,
            ContentKind::Article,
        ] {
            let package = create_content_package(temp.path(), kind, kind.as_str()).unwrap();
            assert_eq!(package.manifest.kind, kind);
            if kind.requires_markdown() {
                assert!(!package.body.is_empty());
                assert_eq!(package.path, format!("contents/{}", package.manifest.id));
            } else {
                assert!(package.body.is_empty());
            }
        }
        let listed = list_content_packages(temp.path()).unwrap();
        assert_eq!(listed.len(), 4);
        let reopened = read_content_package(temp.path(), &listed[0].id).unwrap();
        assert_eq!(reopened.manifest.kind, listed[0].kind);
    }

    #[test]
    fn video_assets_and_metadata_persist_without_markdown_body() {
        let temp = workspace();
        let package = create_content_package(temp.path(), ContentKind::Video, "视频草稿").unwrap();
        let video = import_content_asset(
            temp.path(),
            &package.manifest.id,
            "media",
            "clip.mp4",
            "video/mp4",
            video_bytes(),
        )
        .unwrap();
        let cover = import_content_asset(
            temp.path(),
            &package.manifest.id,
            "cover",
            "cover.png",
            "image/png",
            image_bytes(),
        )
        .unwrap();
        let saved = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "视频新标题".to_string(),
                summary: "视频简介".to_string(),
                tags: vec!["视频".to_string()],
                cover: Some(cover),
                media: vec![video],
                body: String::new(),
                base_hash: Some(package.content_hash),
            },
        )
        .unwrap();
        assert!(saved.body.is_empty());
        assert!(!temp.path().join(&saved.path).join("body.md").exists());
        let reopened = read_content_package(temp.path(), &package.manifest.id).unwrap();
        assert_eq!(reopened.manifest.summary, "视频简介");
        assert_eq!(reopened.manifest.media.len(), 1);
        assert!(reopened.manifest.cover.is_some());
    }

    #[test]
    fn image_order_cover_and_captions_persist() {
        let temp = workspace();
        let package = create_content_package(temp.path(), ContentKind::ImageText, "图集").unwrap();
        let mut media = Vec::new();
        for index in 0..3 {
            media.push(
                import_content_asset(
                    temp.path(),
                    &package.manifest.id,
                    "media",
                    &format!("image-{index}.png"),
                    "image/png",
                    image_bytes(),
                )
                .unwrap(),
            );
        }
        media.reverse();
        media[0].caption = "封面说明".to_string();
        media[1].caption = "第二张".to_string();
        let cover = media[0].clone();
        let saved = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "图集".to_string(),
                summary: "短文案".to_string(),
                tags: vec![],
                cover: Some(cover.clone()),
                media,
                body: String::new(),
                base_hash: Some(package.content_hash),
            },
        )
        .unwrap();
        assert_eq!(saved.manifest.cover.as_ref().unwrap().id, cover.id);
        assert_eq!(
            saved
                .manifest
                .media
                .iter()
                .map(|asset| asset.order)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let reopened = read_content_package(temp.path(), &package.manifest.id).unwrap();
        assert_eq!(reopened.manifest.media[0].caption, "封面说明");
        assert_eq!(reopened.manifest.media[1].caption, "第二张");
    }

    #[test]
    fn article_body_markdown_is_authoritative_and_base_hash_blocks_overwrite() {
        let temp = workspace();
        let package = create_content_package(temp.path(), ContentKind::Article, "文章").unwrap();
        let saved = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "文章".to_string(),
                summary: "摘要".to_string(),
                tags: vec!["Rust".to_string()],
                cover: None,
                media: vec![],
                body: "# 标题\n\n正文".to_string(),
                base_hash: Some(package.content_hash.clone()),
            },
        )
        .unwrap();
        assert_eq!(saved.body, "# 标题\n\n正文");
        let reopened = read_content_package(temp.path(), &package.manifest.id).unwrap();
        assert_eq!(reopened.body, "# 标题\n\n正文");
        let error = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "覆盖".to_string(),
                summary: String::new(),
                tags: vec![],
                cover: None,
                media: vec![],
                body: "external".to_string(),
                base_hash: Some(package.content_hash),
            },
        )
        .unwrap_err();
        assert!(error.contains("其他位置修改"));
        assert_eq!(
            read_content_package(temp.path(), &package.manifest.id)
                .unwrap()
                .body,
            "# 标题\n\n正文"
        );
    }

    #[test]
    fn disk_write_failure_returns_error_without_replacing_content() {
        let temp = workspace();
        let package = create_content_package(temp.path(), ContentKind::Article, "只读").unwrap();
        let package_dir = content_root(temp.path()).join(&package.manifest.id);
        let mut permissions = fs::metadata(&package_dir).unwrap().permissions();
        permissions.set_mode(0o500);
        fs::set_permissions(&package_dir, permissions).unwrap();
        let result = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "无法保存".to_string(),
                summary: String::new(),
                tags: vec![],
                cover: None,
                media: vec![],
                body: "# 新内容".to_string(),
                base_hash: Some(package.content_hash),
            },
        );
        let mut permissions = fs::metadata(&package_dir).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&package_dir, permissions).unwrap();
        assert!(result.is_err());
        assert_eq!(
            read_content_package(temp.path(), &package.manifest.id)
                .unwrap()
                .body,
            "# 只读\n\n"
        );
    }

    #[test]
    fn drafts_are_independent_and_track_source_changes() {
        let temp = workspace();
        let package = create_content_package(temp.path(), ContentKind::Article, "主稿").unwrap();
        let saved = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "主稿".to_string(),
                summary: "摘要".to_string(),
                tags: vec![],
                cover: None,
                media: vec![],
                body: "# 主稿\n\n第一版".to_string(),
                base_hash: Some(package.content_hash),
            },
        )
        .unwrap();
        let drafts = create_platform_drafts(
            temp.path(),
            &package.manifest.id,
            &[
                DraftTarget {
                    platform_id: "juejin".to_string(),
                    platform_name: "掘金".to_string(),
                    account_id: "a1".to_string(),
                    account_alias: "主号".to_string(),
                    adapter_id: None,
                },
                DraftTarget {
                    platform_id: "csdn".to_string(),
                    platform_name: "CSDN".to_string(),
                    account_id: "a2".to_string(),
                    account_alias: "副号".to_string(),
                    adapter_id: None,
                },
            ],
        )
        .unwrap();
        assert_eq!(drafts.len(), 2);
        let first = drafts[0].clone();
        let second = drafts[1].clone();
        let edited = save_platform_draft(
            temp.path(),
            &first.id,
            DraftSaveInput {
                title: "掘金手动标题".to_string(),
                summary: first.summary.clone(),
                tags: first.tags.clone(),
                cover: first.cover.clone(),
                media: first.media.clone(),
                body: "掘金手动正文".to_string(),
                base_hash: first.hash.clone(),
            },
        )
        .unwrap();
        assert!(edited.manually_edited);
        assert_eq!(
            read_platform_draft(temp.path(), &second.id).unwrap().body,
            second.body
        );
        assert_eq!(
            read_content_package(temp.path(), &package.manifest.id)
                .unwrap()
                .body,
            "# 主稿\n\n第一版"
        );
        let _ = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "主稿".to_string(),
                summary: "摘要".to_string(),
                tags: vec![],
                cover: None,
                media: vec![],
                body: "# 主稿\n\n第二版".to_string(),
                base_hash: Some(saved.content_hash),
            },
        )
        .unwrap();
        let changed = read_platform_draft(temp.path(), &first.id).unwrap();
        assert!(changed.source_changed);
        assert_eq!(changed.body, "掘金手动正文");
        let acknowledged = acknowledge_platform_draft_source(temp.path(), &first.id).unwrap();
        assert!(!acknowledged.source_changed);
        let regenerated = regenerate_platform_draft(temp.path(), &first.id).unwrap();
        assert!(!regenerated.source_changed);
        assert!(!regenerated.manually_edited);
        assert!(regenerated.body.contains("第二版"));
    }

    #[test]
    fn video_drafts_copy_video_and_cover_metadata() {
        let temp = workspace();
        let package = create_content_package(temp.path(), ContentKind::Video, "视频主稿").unwrap();
        let video = import_content_asset(
            temp.path(),
            &package.manifest.id,
            "media",
            "clip.mp4",
            "video/mp4",
            video_bytes(),
        )
        .unwrap();
        let cover = import_content_asset(
            temp.path(),
            &package.manifest.id,
            "cover",
            "cover.png",
            "image/png",
            image_bytes(),
        )
        .unwrap();
        let saved = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "视频主稿".to_string(),
                summary: "简介".to_string(),
                tags: vec![],
                cover: Some(cover),
                media: vec![video],
                body: String::new(),
                base_hash: Some(package.content_hash),
            },
        )
        .unwrap();
        let drafts = create_platform_drafts(
            temp.path(),
            &saved.manifest.id,
            &[DraftTarget {
                platform_id: "bilibili".to_string(),
                platform_name: "Bilibili".to_string(),
                account_id: "bili-1".to_string(),
                account_alias: "视频号".to_string(),
                adapter_id: None,
            }],
        )
        .unwrap();
        let draft = &drafts[0];
        assert_eq!(draft.media.len(), 1);
        assert_eq!(draft.media[0].kind, AssetKind::Video);
        assert_eq!(draft.media[0].mime_type, "video/mp4");
        assert_eq!(draft.media[0].byte_size, video_bytes().len() as u64);
        assert_eq!(draft.cover.as_ref().unwrap().mime_type, "image/png");
        assert!(!draft.body.is_empty());
        assert!(
            temp.path()
                .join("platform-drafts")
                .join(&draft.id)
                .join(&draft.media[0].file)
                .is_file()
        );
    }

    #[test]
    fn image_text_drafts_keep_media_order_and_cover_independently() {
        let temp = workspace();
        let package =
            create_content_package(temp.path(), ContentKind::ImageText, "图集主稿").unwrap();
        let mut media = Vec::new();
        for index in 0..3 {
            media.push(
                import_content_asset(
                    temp.path(),
                    &package.manifest.id,
                    "media",
                    &format!("image-{index}.png"),
                    "image/png",
                    image_bytes(),
                )
                .unwrap(),
            );
        }
        let cover = media[1].clone();
        let saved = save_content_package(
            temp.path(),
            &package.manifest.id,
            ContentSaveInput {
                title: "图集主稿".to_string(),
                summary: "短文案".to_string(),
                tags: vec![],
                cover: Some(cover.clone()),
                media,
                body: String::new(),
                base_hash: Some(package.content_hash),
            },
        )
        .unwrap();
        let drafts = create_platform_drafts(
            temp.path(),
            &saved.manifest.id,
            &[
                DraftTarget {
                    platform_id: "douyin".to_string(),
                    platform_name: "抖音".to_string(),
                    account_id: "douyin-1".to_string(),
                    account_alias: "视频号".to_string(),
                    adapter_id: None,
                },
                DraftTarget {
                    platform_id: "xiaohongshu".to_string(),
                    platform_name: "小红书".to_string(),
                    account_id: "xhs-1".to_string(),
                    account_alias: "图集号".to_string(),
                    adapter_id: None,
                },
            ],
        )
        .unwrap();
        for draft in &drafts {
            assert_eq!(
                draft
                    .media
                    .iter()
                    .map(|asset| asset.order)
                    .collect::<Vec<_>>(),
                vec![1, 2, 3]
            );
            assert_eq!(draft.cover.as_ref().unwrap().order, cover.order);
            assert_eq!(draft.media.len(), 3);
        }
        let edited = save_platform_draft(
            temp.path(),
            &drafts[0].id,
            DraftSaveInput {
                title: "抖音独立标题".to_string(),
                summary: drafts[0].summary.clone(),
                tags: drafts[0].tags.clone(),
                cover: drafts[0].cover.clone(),
                media: drafts[0].media.clone(),
                body: "抖音独立正文".to_string(),
                base_hash: drafts[0].hash.clone(),
            },
        )
        .unwrap();
        assert_eq!(edited.media.len(), 3);
        assert_eq!(
            read_platform_draft(temp.path(), &drafts[1].id)
                .unwrap()
                .title,
            "图集主稿"
        );
        assert_eq!(
            read_content_package(temp.path(), &saved.manifest.id)
                .unwrap()
                .manifest
                .title,
            "图集主稿"
        );
    }

    #[test]
    fn derived_content_is_a_new_package_with_source_reference() {
        let temp = workspace();
        let source = create_content_package(temp.path(), ContentKind::Article, "原始文章").unwrap();
        let source = save_content_package(
            temp.path(),
            &source.manifest.id,
            ContentSaveInput {
                title: "原始文章".to_string(),
                summary: "摘要".to_string(),
                tags: vec!["标签".to_string()],
                cover: None,
                media: vec![],
                body: "# 原始文章\n\n正文".to_string(),
                base_hash: Some(source.content_hash),
            },
        )
        .unwrap();
        let derived = derive_content_package(
            temp.path(),
            &source.manifest.id,
            ContentKind::ImageText,
            Some("派生图集".to_string()),
        )
        .unwrap();
        assert_ne!(derived.manifest.id, source.manifest.id);
        assert_eq!(derived.manifest.kind, ContentKind::ImageText);
        assert_eq!(
            derived.manifest.source.as_ref().unwrap().content_id,
            source.manifest.id
        );
        assert_eq!(
            read_content_package(temp.path(), &source.manifest.id)
                .unwrap()
                .body,
            "# 原始文章\n\n正文"
        );
    }
}
