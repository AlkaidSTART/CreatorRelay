use serde::{Deserialize, Serialize};

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

mod drafts;
mod package;
mod paths;
#[cfg(test)]
mod tests;

pub use drafts::*;
pub use package::*;
