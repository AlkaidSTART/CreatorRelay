use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishTaskStatus {
    Created,
    Validating,
    Pending,
    Scheduled,
    Publishing,
    Success,
    Failed,
    Cancelled,
    NeedsManualAction,
}

impl PublishTaskStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "CREATED",
            Self::Validating => "VALIDATING",
            Self::Pending => "PENDING",
            Self::Scheduled => "SCHEDULED",
            Self::Publishing => "PUBLISHING",
            Self::Success => "SUCCESS",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
            Self::NeedsManualAction => "NEEDS_MANUAL_ACTION",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "CREATED" => Ok(Self::Created),
            "VALIDATING" => Ok(Self::Validating),
            "PENDING" => Ok(Self::Pending),
            "SCHEDULED" => Ok(Self::Scheduled),
            "PUBLISHING" => Ok(Self::Publishing),
            "SUCCESS" => Ok(Self::Success),
            "FAILED" => Ok(Self::Failed),
            "CANCELLED" => Ok(Self::Cancelled),
            "NEEDS_MANUAL_ACTION" => Ok(Self::NeedsManualAction),
            _ => Err(format!("未知发布任务状态：{value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublishExecutionMode {
    Auto,
    Manual,
}

impl PublishExecutionMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            _ => Err(format!("未知发布执行方式：{value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublishSourceType {
    PlatformDraft,
    Content,
}

impl PublishSourceType {
    fn as_str(self) -> &'static str {
        match self {
            Self::PlatformDraft => "platformDraft",
            Self::Content => "content",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "platformDraft" => Ok(Self::PlatformDraft),
            "content" => Ok(Self::Content),
            _ => Err(format!("未知发布来源类型：{value}")),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishTargetInput {
    pub platform_id: String,
    pub platform_name: String,
    pub account_id: String,
    pub account_alias: String,
    #[serde(default)]
    pub adapter_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePublishTasksInput {
    pub source_type: PublishSourceType,
    pub source_id: String,
    pub targets: Vec<PublishTargetInput>,
    #[serde(default)]
    pub scheduled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishTask {
    pub id: String,
    pub source_type: PublishSourceType,
    pub source_id: String,
    pub content_id: String,
    pub content_version: String,
    pub content_kind: ContentKind,
    pub platform_id: String,
    pub platform_name: String,
    pub account_id: String,
    pub account_alias: String,
    pub adapter_id: Option<String>,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub cover: Option<ContentAsset>,
    pub media: Vec<ContentAsset>,
    pub execution_mode: PublishExecutionMode,
    pub status: PublishTaskStatus,
    pub paused: bool,
    pub pause_reason: Option<String>,
    pub scheduled_at: Option<String>,
    pub external_id: Option<String>,
    pub external_url: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub next_action: Option<String>,
    pub attempt_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePublishTasksResult {
    pub tasks: Vec<PublishTask>,
    pub deduplicated_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewAction {
    pub id: &'static str,
    pub label: &'static str,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishPreview {
    pub task: PublishTask,
    pub task_id: String,
    pub confirmation_token: Option<String>,
    pub confirmation_expires_at: Option<String>,
    pub execution_mode: PublishExecutionMode,
    pub capability: CapabilityLevel,
    pub capability_reason: Option<String>,
    pub validation: PublishValidationResult,
    pub actions: Vec<PreviewAction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualCompletionInput {
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub external_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveUncertainInput {
    pub outcome: UncertainOutcome,
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub external_url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UncertainOutcome {
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualPackageAsset {
    pub asset_id: String,
    pub kind: String,
    pub role: String,
    pub order: u32,
    pub file: String,
    pub original_name: String,
    pub mime_type: String,
    pub byte_size: u64,
    pub caption: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualPublishPackage {
    pub task: PublishTask,
    pub directory: String,
    pub manifest_path: String,
    pub body_path: String,
    pub assets: Vec<ManualPackageAsset>,
    pub generated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishTaskEvent {
    pub id: i64,
    pub task_id: String,
    pub action: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
struct SourceSnapshot {
    source_type: PublishSourceType,
    source_id: String,
    content_id: String,
    content_version: String,
    content_kind: ContentKind,
    title: String,
    body: String,
    tags: Vec<String>,
    cover: Option<ContentAsset>,
    media: Vec<ContentAsset>,
}

#[derive(Debug, Clone)]
struct TaskRow {
    id: String,
    source_type: PublishSourceType,
    source_id: String,
    content_id: String,
    content_version: String,
    content_kind: ContentKind,
    platform_id: String,
    platform_name: String,
    account_id: String,
    account_alias: String,
    adapter_id: Option<String>,
    title: String,
    body: String,
    tags_json: String,
    cover_json: Option<String>,
    media_json: String,
    execution_mode: PublishExecutionMode,
    status: PublishTaskStatus,
    paused: bool,
    pause_reason: Option<String>,
    scheduled_at: Option<String>,
    confirmation_token: Option<String>,
    confirmation_hash: Option<String>,
    confirmation_expires_at: Option<String>,
    external_id: Option<String>,
    external_url: Option<String>,
    error_code: Option<String>,
    error_message: Option<String>,
    next_action: Option<String>,
    attempt_count: u32,
    idempotency_key: String,
    created_at: String,
    updated_at: String,
}

impl TaskRow {
    fn cover(&self) -> Result<Option<ContentAsset>, String> {
        self.cover_json
            .as_deref()
            .map(|value| {
                serde_json::from_str::<ContentAsset>(value)
                    .map_err(|error| format!("封面快照无法解析：{error}"))
            })
            .transpose()
    }

    fn media(&self) -> Result<Vec<ContentAsset>, String> {
        serde_json::from_str::<Vec<ContentAsset>>(&self.media_json)
            .map_err(|error| format!("媒体快照无法解析：{error}"))
    }

    fn tags(&self) -> Result<Vec<String>, String> {
        serde_json::from_str::<Vec<String>>(&self.tags_json)
            .map_err(|error| format!("标签快照无法解析：{error}"))
    }

    fn to_view(&self) -> Result<PublishTask, String> {
        Ok(PublishTask {
            id: self.id.clone(),
            source_type: self.source_type,
            source_id: self.source_id.clone(),
            content_id: self.content_id.clone(),
            content_version: self.content_version.clone(),
            content_kind: self.content_kind,
            platform_id: self.platform_id.clone(),
            platform_name: self.platform_name.clone(),
            account_id: self.account_id.clone(),
            account_alias: self.account_alias.clone(),
            adapter_id: self.adapter_id.clone(),
            title: self.title.clone(),
            body: self.body.clone(),
            tags: self.tags()?,
            cover: self.cover()?,
            media: self.media()?,
            execution_mode: self.execution_mode,
            status: self.status,
            paused: self.paused,
            pause_reason: self.pause_reason.clone(),
            scheduled_at: self.scheduled_at.clone(),
            external_id: self.external_id.clone(),
            external_url: self.external_url.clone(),
            error_code: self.error_code.clone(),
            error_message: self.error_message.clone(),
            next_action: self.next_action.clone(),
            attempt_count: self.attempt_count,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        })
    }
}

