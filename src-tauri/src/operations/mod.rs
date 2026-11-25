use chrono::{DateTime, Datelike, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

use crate::{
    content::{self, ContentAsset, ContentKind},
    ecosystem, platforms,
    plugins::{
        self, CapabilityLevel, PluginRuntime, PublishInput, PublishMediaRef,
        PublishValidationResult, validate_publish_payload,
    },
};

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

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn parse_content_kind(value: &str) -> Result<ContentKind, String> {
    match value {
        "video" => Ok(ContentKind::Video),
        "imageText" => Ok(ContentKind::ImageText),
        "imageArticle" => Ok(ContentKind::ImageArticle),
        "article" => Ok(ContentKind::Article),
        _ => Err(format!("未知内容形态：{value}")),
    }
}

fn plugin_content_kind(kind: ContentKind) -> plugins::ContentKind {
    match kind {
        ContentKind::Video => plugins::ContentKind::Video,
        ContentKind::ImageText => plugins::ContentKind::ImageText,
        ContentKind::ImageArticle => plugins::ContentKind::ImageArticle,
        ContentKind::Article => plugins::ContentKind::Article,
    }
}

fn task_select() -> &'static str {
    "SELECT id, source_type, source_id, content_id, content_version, content_kind,
            platform_id, platform_name, account_id, account_alias, adapter_id,
            title, body, tags_json, cover_json, media_json, execution_mode, status,
            is_paused, pause_reason, scheduled_at, confirmation_token, confirmation_hash,
            confirmation_expires_at, external_id, external_url, error_code, error_message,
            next_action, attempt_count, idempotency_key, created_at, updated_at
       FROM publish_tasks"
}

fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskRow> {
    let source_type: String = row.get(1)?;
    let content_kind: String = row.get(5)?;
    let execution_mode: String = row.get(16)?;
    let status: String = row.get(17)?;
    let parse_error = |index: usize, error: String| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
        )
    };
    Ok(TaskRow {
        id: row.get(0)?,
        source_type: PublishSourceType::parse(&source_type)
            .map_err(|error| parse_error(1, error))?,
        source_id: row.get(2)?,
        content_id: row.get(3)?,
        content_version: row.get(4)?,
        content_kind: parse_content_kind(&content_kind).map_err(|error| parse_error(5, error))?,
        platform_id: row.get(6)?,
        platform_name: row.get(7)?,
        account_id: row.get(8)?,
        account_alias: row.get(9)?,
        adapter_id: row.get(10)?,
        title: row.get(11)?,
        body: row.get(12)?,
        tags_json: row.get(13)?,
        cover_json: row.get(14)?,
        media_json: row.get(15)?,
        execution_mode: PublishExecutionMode::parse(&execution_mode)
            .map_err(|error| parse_error(16, error))?,
        status: PublishTaskStatus::parse(&status).map_err(|error| parse_error(17, error))?,
        paused: row.get::<_, i64>(18)? != 0,
        pause_reason: row.get(19)?,
        scheduled_at: row.get(20)?,
        confirmation_token: row.get(21)?,
        confirmation_hash: row.get(22)?,
        confirmation_expires_at: row.get(23)?,
        external_id: row.get(24)?,
        external_url: row.get(25)?,
        error_code: row.get(26)?,
        error_message: row.get(27)?,
        next_action: row.get(28)?,
        attempt_count: u32::try_from(row.get::<_, i64>(29)?).unwrap_or(0),
        idempotency_key: row.get(30)?,
        created_at: row.get(31)?,
        updated_at: row.get(32)?,
    })
}

fn task_record(conn: &Connection, task_id: &str) -> Result<TaskRow, String> {
    conn.query_row(
        &format!("{} WHERE id=?1", task_select()),
        [task_id],
        task_from_row,
    )
    .optional()
    .map_err(|error| format!("发布任务读取失败：{error}"))?
    .ok_or_else(|| "未找到发布任务".to_string())
}

fn source_snapshot(
    workspace: &Path,
    source_type: PublishSourceType,
    source_id: &str,
) -> Result<SourceSnapshot, String> {
    match source_type {
        PublishSourceType::PlatformDraft => {
            let draft = content::read_platform_draft(workspace, source_id)?;
            let package = content::read_content_package(workspace, &draft.content_id)?;
            Ok(SourceSnapshot {
                source_type,
                source_id: draft.id,
                content_id: draft.content_id,
                content_version: draft.hash,
                content_kind: package.manifest.kind,
                title: draft.title,
                body: draft.body,
                tags: draft.tags,
                cover: draft.cover,
                media: draft.media,
            })
        }
        PublishSourceType::Content => {
            let package = content::read_content_package(workspace, source_id)?;
            Ok(SourceSnapshot {
                source_type,
                source_id: package.manifest.id.clone(),
                content_id: package.manifest.id.clone(),
                content_version: package.content_version,
                content_kind: package.manifest.kind,
                title: package.manifest.title,
                body: package.body,
                tags: package.manifest.tags,
                cover: package.manifest.cover,
                media: package.manifest.media,
            })
        }
    }
}

fn validate_source_unchanged(workspace: &Path, task: &TaskRow) -> Result<SourceSnapshot, String> {
    let snapshot = source_snapshot(workspace, task.source_type, &task.source_id)?;
    if snapshot.content_version != task.content_version {
        return Err("源内容版本已经变化，请重新创建发布任务以核对最新内容".to_string());
    }
    if snapshot.content_kind != task.content_kind
        || snapshot.title != task.title
        || snapshot.body != task.body
        || snapshot.media != task.media()?
        || snapshot.cover != task.cover()?
    {
        return Err("源内容与已确认快照不一致，请重新创建发布任务".to_string());
    }
    Ok(snapshot)
}

fn publish_input_from_task(task: &TaskRow) -> Result<PublishInput, String> {
    let media = task
        .media()?
        .into_iter()
        .map(|asset| PublishMediaRef {
            asset_id: asset.id,
            kind: match asset.kind {
                content::AssetKind::Video => plugins::MediaKind::Video,
                content::AssetKind::Image => plugins::MediaKind::Image,
            },
            mime_type: asset.mime_type,
            byte_size: asset.byte_size,
            order: asset.order,
            role: Some("media".to_string()),
        })
        .collect::<Vec<_>>();
    let cover = task.cover()?.map(|asset| PublishMediaRef {
        asset_id: asset.id,
        kind: plugins::MediaKind::Image,
        mime_type: asset.mime_type,
        byte_size: asset.byte_size,
        order: asset.order,
        role: Some("cover".to_string()),
    });
    Ok(PublishInput {
        content_kind: plugin_content_kind(task.content_kind),
        title: task.title.clone(),
        body: if task.body.trim().is_empty() {
            None
        } else {
            Some(task.body.clone())
        },
        media,
        cover,
    })
}

fn insert_task_event(
    conn: &Connection,
    task_id: &str,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO publish_task_events(task_id, action, detail, created_at)
         VALUES(?1, ?2, ?3, ?4)",
        params![task_id, action, detail, Utc::now().to_rfc3339()],
    )
    .map_err(|error| format!("发布任务事件写入失败：{error}"))?;
    Ok(())
}

pub fn create_publish_tasks(
    conn: &mut Connection,
    workspace: &Path,
    input: &CreatePublishTasksInput,
) -> Result<CreatePublishTasksResult, String> {
    if input.targets.is_empty() {
        return Err("请至少选择一个发布目标".to_string());
    }
    if let Some(scheduled_at) = input.scheduled_at.as_deref() {
        DateTime::parse_from_rfc3339(scheduled_at)
            .map_err(|_| "定时发布时间必须是 RFC3339 时间".to_string())?;
    }
    let snapshot = source_snapshot(workspace, input.source_type, &input.source_id)?;
    let payload = PublishInput {
        content_kind: plugin_content_kind(snapshot.content_kind),
        title: snapshot.title.clone(),
        body: if snapshot.body.trim().is_empty() {
            None
        } else {
            Some(snapshot.body.clone())
        },
        media: snapshot
            .media
            .iter()
            .map(|asset| PublishMediaRef {
                asset_id: asset.id.clone(),
                kind: match asset.kind {
                    content::AssetKind::Video => plugins::MediaKind::Video,
                    content::AssetKind::Image => plugins::MediaKind::Image,
                },
                mime_type: asset.mime_type.clone(),
                byte_size: asset.byte_size,
                order: asset.order,
                role: Some("media".to_string()),
            })
            .collect(),
        cover: snapshot.cover.as_ref().map(|asset| PublishMediaRef {
            asset_id: asset.id.clone(),
            kind: plugins::MediaKind::Image,
            mime_type: asset.mime_type.clone(),
            byte_size: asset.byte_size,
            order: asset.order,
            role: Some("cover".to_string()),
        }),
    };
    let validation = validate_publish_payload(&payload);
    let mut seen_targets = HashSet::new();
    let mut task_ids = Vec::new();
    let mut deduplicated_count = 0;
    let tx = conn
        .transaction()
        .map_err(|error| format!("无法开始发布任务事务：{error}"))?;
    for target in &input.targets {
        if target.platform_id.trim().is_empty()
            || target.platform_name.trim().is_empty()
            || target.account_id.trim().is_empty()
            || target.account_alias.trim().is_empty()
        {
            return Err("发布目标必须包含平台、账号与显示名称".to_string());
        }
        if !seen_targets.insert(target.account_id.clone()) {
            continue;
        }
        let idempotency_key = digest(&format!(
            "{}:{}:{}:{}:{}",
            snapshot.source_type.as_str(),
            snapshot.source_id,
            snapshot.content_version,
            target.account_id,
            snapshot.content_kind.as_str()
        ));
        if let Some(existing_id) = tx
            .query_row(
                "SELECT id FROM publish_tasks WHERE idempotency_key=?1",
                [&idempotency_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("发布任务去重查询失败：{error}"))?
        {
            task_ids.push(existing_id);
            deduplicated_count += 1;
            continue;
        }
        let now = Utc::now().to_rfc3339();
        let id = Uuid::new_v4().to_string();
        let initial_status = PublishTaskStatus::Created;
        let error_code = validation.issues.first().map(|issue| issue.code.clone());
        let error_message = validation.issues.first().map(|issue| issue.message.clone());
        tx.execute(
            "INSERT INTO publish_tasks(
                id, source_type, source_id, content_id, content_version, content_kind,
                platform_id, platform_name, account_id, account_alias, adapter_id,
                title, body, tags_json, cover_json, media_json, execution_mode, status,
                scheduled_at, error_code, error_message, next_action, idempotency_key,
                created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                      ?15, ?16, 'manual', ?17, ?18, ?19, ?20, 'review_and_confirm', ?21, ?22, ?22)",
            params![
                id,
                snapshot.source_type.as_str(),
                snapshot.source_id,
                snapshot.content_id,
                snapshot.content_version,
                snapshot.content_kind.as_str(),
                target.platform_id,
                target.platform_name,
                target.account_id,
                target.account_alias,
                target.adapter_id,
                snapshot.title,
                snapshot.body,
                serde_json::to_string(&snapshot.tags).map_err(|error| error.to_string())?,
                snapshot
                    .cover
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|error| error.to_string())?,
                serde_json::to_string(&snapshot.media).map_err(|error| error.to_string())?,
                initial_status.as_str(),
                input.scheduled_at,
                error_code,
                error_message,
                idempotency_key,
                now,
            ],
        )
        .map_err(|error| format!("发布任务创建失败：{error}"))?;
        insert_task_event(
            &tx,
            &id,
            "CREATED",
            &format!(
                "来源={} 形态={} 平台={} 账号={}",
                snapshot.source_id,
                snapshot.content_kind.as_str(),
                target.platform_name,
                target.account_alias
            ),
        )?;
        task_ids.push(id);
    }
    tx.commit()
        .map_err(|error| format!("发布任务事务提交失败：{error}"))?;
    let mut tasks = Vec::new();
    for task_id in task_ids {
        tasks.push(task_record(conn, &task_id)?.to_view()?);
    }
    Ok(CreatePublishTasksResult {
        tasks,
        deduplicated_count,
    })
}

pub fn list_publish_tasks(conn: &Connection) -> Result<Vec<PublishTask>, String> {
    let mut statement = conn
        .prepare(&format!(
            "{} ORDER BY created_at DESC, id DESC",
            task_select()
        ))
        .map_err(|error| format!("发布任务列表准备失败：{error}"))?;
    let rows = statement
        .query_map([], task_from_row)
        .map_err(|error| format!("发布任务列表读取失败：{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("发布任务列表解析失败：{error}"))?;
    rows.into_iter().map(|row| row.to_view()).collect()
}

pub fn publish_task(conn: &Connection, task_id: &str) -> Result<PublishTask, String> {
    task_record(conn, task_id)?.to_view()
}

pub fn list_publish_task_events(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<PublishTaskEvent>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, task_id, action, detail, created_at
             FROM publish_task_events WHERE task_id=?1 ORDER BY id ASC",
        )
        .map_err(|error| format!("发布事件准备失败：{error}"))?;
    let rows = statement
        .query_map([task_id], |row| {
            Ok(PublishTaskEvent {
                id: row.get(0)?,
                task_id: row.get(1)?,
                action: row.get(2)?,
                detail: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|error| format!("发布事件读取失败：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("发布事件解析失败：{error}"))
}

fn capability_for_task(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    task: &TaskRow,
) -> Result<(CapabilityLevel, Option<String>), String> {
    let Some(adapter_id) = task.adapter_id.as_deref() else {
        return Ok((
            CapabilityLevel::Unavailable,
            Some("目标未绑定已验证的发布适配器，只能走人工发布".to_string()),
        ));
    };
    let status = plugins::plugin_publish_capability(
        conn,
        runtime,
        adapter_id,
        &task.account_id,
        plugin_content_kind(task.content_kind),
    )?;
    if Uuid::parse_str(&task.account_id).is_ok() {
        let policy = ecosystem::publish_policy(
            conn,
            &task.account_id,
            &task.platform_id,
            plugin_content_kind(task.content_kind),
        )?;
        if !policy.auto_publish_allowed {
            let reason = status
                .reason
                .map(|adapter_reason| format!("{}；{}", policy.reason, adapter_reason))
                .unwrap_or(policy.reason);
            return Ok((CapabilityLevel::Restricted, Some(reason)));
        }
    }
    Ok((status.level, status.reason))
}

pub fn prepare_publish_task(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
    task_id: &str,
) -> Result<PublishPreview, String> {
    let task = task_record(conn, task_id)?;
    if matches!(
        task.status,
        PublishTaskStatus::Success | PublishTaskStatus::Publishing | PublishTaskStatus::Cancelled
    ) {
        return Err("当前任务状态不能重新准备发布".to_string());
    }
    if task.paused {
        return Err(task
            .pause_reason
            .clone()
            .unwrap_or_else(|| "任务已暂停，请先恢复账号".to_string()));
    }
    let payload = publish_input_from_task(&task)?;
    let mut validation = validate_publish_payload(&payload);
    let mut source_issue = None;
    if let Err(error) = validate_source_unchanged(workspace, &task) {
        source_issue = Some(error.clone());
        validation.ok = false;
        validation.issues.push(plugins::PublishIssue {
            field: "source".to_string(),
            code: "source_changed".to_string(),
            message: error,
        });
    }
    let (capability, capability_reason) = capability_for_task(conn, runtime, &task)?;
    let execution_mode = if capability == CapabilityLevel::Supported && validation.ok {
        PublishExecutionMode::Auto
    } else {
        PublishExecutionMode::Manual
    };
    let confirmation_hash = digest(
        &serde_json::to_string(&serde_json::json!({
            "taskId": task.id,
            "title": task.title,
            "body": task.body,
            "tags": task.tags_json,
            "cover": task.cover_json,
            "media": task.media_json,
            "contentKind": task.content_kind.as_str(),
            "contentVersion": task.content_version,
            "platformId": task.platform_id,
            "accountId": task.account_id,
            "adapterId": task.adapter_id,
            "executionMode": execution_mode.as_str(),
            "scheduledAt": task.scheduled_at,
        }))
        .map_err(|error| error.to_string())?,
    );
    let token = if validation.ok {
        Some(Uuid::new_v4().to_string())
    } else {
        None
    };
    let expires_at = token
        .as_ref()
        .map(|_| (Utc::now() + Duration::minutes(10)).to_rfc3339());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE publish_tasks
         SET status=?1, execution_mode=?2, confirmation_token=?3, confirmation_hash=?4,
             confirmation_expires_at=?5, error_code=?6, error_message=?7,
             next_action=?8, updated_at=?9
         WHERE id=?10",
        params![
            PublishTaskStatus::Created.as_str(),
            execution_mode.as_str(),
            token,
            token.as_ref().map(|_| confirmation_hash),
            expires_at,
            validation.issues.first().map(|issue| issue.code.clone()),
            source_issue.or_else(|| validation.issues.first().map(|issue| issue.message.clone())),
            if execution_mode == PublishExecutionMode::Auto {
                "confirm_and_publish"
            } else {
                "confirm_manual_review"
            },
            now,
            task.id,
        ],
    )
    .map_err(|error| format!("发布预览保存失败：{error}"))?;
    insert_task_event(
        conn,
        &task.id,
        "PREVIEWED",
        &format!(
            "形态={} 执行方式={} 能力={} 校验={} idempotency={}",
            task.content_kind.as_str(),
            execution_mode.as_str(),
            capability.as_str(),
            if validation.ok { "pass" } else { "blocked" },
            task.idempotency_key
        ),
    )?;
    let actions = vec![
        PreviewAction {
            id: "auto_publish",
            label: "自动发布",
            available: execution_mode == PublishExecutionMode::Auto,
            reason: if execution_mode == PublishExecutionMode::Auto {
                None
            } else {
                Some(
                    capability_reason
                        .clone()
                        .unwrap_or_else(|| "未验证的自动发布能力不会显示为可用".to_string()),
                )
            },
        },
        PreviewAction {
            id: "manual_package",
            label: "导出人工发布包",
            available: true,
            reason: None,
        },
        PreviewAction {
            id: "cancel",
            label: "取消任务",
            available: true,
            reason: None,
        },
    ];
    Ok(PublishPreview {
        task: task_record(conn, &task.id)?.to_view()?,
        task_id: task.id,
        confirmation_token: token,
        confirmation_expires_at: expires_at,
        execution_mode,
        capability,
        capability_reason,
        validation,
        actions,
    })
}

fn task_is_real_account(conn: &Connection, account_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM platform_accounts WHERE id=?1)",
        [account_id],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value != 0)
    .map_err(|error| format!("账号存在性检查失败：{error}"))
}

fn ensure_account_ready(conn: &Connection, task: &TaskRow) -> Result<(), String> {
    if task_is_real_account(conn, &task.account_id)? {
        platforms::require_active_account(conn, &task.account_id)?;
    }
    Ok(())
}

fn update_task_failure(
    conn: &Connection,
    task_id: &str,
    status: PublishTaskStatus,
    code: &str,
    message: &str,
    next_action: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE publish_tasks
         SET status=?1, error_code=?2, error_message=?3, next_action=?4,
             confirmation_token=NULL, confirmation_hash=NULL,
             confirmation_expires_at=NULL, updated_at=?5
         WHERE id=?6",
        params![
            status.as_str(),
            code,
            message,
            next_action,
            Utc::now().to_rfc3339(),
            task_id
        ],
    )
    .map_err(|error| format!("发布任务失败状态保存失败：{error}"))?;
    insert_task_event(
        conn,
        task_id,
        status.as_str(),
        &format!("{code}: {message}"),
    )?;
    Ok(())
}

fn mark_account_invalid(conn: &mut Connection, account_id: &str) -> Result<(), String> {
    if !task_is_real_account(conn, account_id)? {
        return Ok(());
    }
    let now = Utc::now().to_rfc3339();
    let tx = conn
        .transaction()
        .map_err(|error| format!("无法开始账号失效事务：{error}"))?;
    tx.execute(
        "UPDATE platform_accounts
         SET status='NEEDS_REAUTH', last_error='平台会话已过期，请重新授权', updated_at=?1
         WHERE id=?2",
        params![now, account_id],
    )
    .map_err(|error| format!("账号失效状态保存失败：{error}"))?;
    tx.execute(
        "UPDATE platform_sessions
         SET state='NEEDS_REAUTH', last_error='平台会话已过期，请重新授权', updated_at=?1
         WHERE account_id=?2",
        params![now, account_id],
    )
    .map_err(|error| format!("账号会话失效状态保存失败：{error}"))?;
    tx.execute(
        "UPDATE publish_tasks
         SET is_paused=1, pause_reason='账号已失效，请重新授权后显式恢复任务', updated_at=?1
         WHERE account_id=?2 AND status NOT IN ('SUCCESS','FAILED','CANCELLED')",
        params![now, account_id],
    )
    .map_err(|error| format!("依赖账号的发布任务暂停失败：{error}"))?;
    tx.commit()
        .map_err(|error| format!("账号失效事务提交失败：{error}"))?;
    Ok(())
}

fn insert_or_update_task_work(
    conn: &Connection,
    task: &TaskRow,
    external_id: &str,
    external_url: Option<&str>,
    source: &str,
    now: &str,
) -> Result<String, String> {
    let source_key = format!("publish:{}", task.id);
    if let Some(existing) = conn
        .query_row(
            "SELECT id FROM external_works WHERE source_key=?1
             OR (platform_id=?2 AND account_id=?3 AND external_id=?4)",
            params![source_key, task.platform_id, task.account_id, external_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("发布作品去重查询失败：{error}"))?
    {
        conn.execute(
            "UPDATE external_works
             SET title=?1, content_kind=?2, original_content_type=?3, url=?4,
                 source=?5, synced_at=?6, updated_at=?6
             WHERE id=?7",
            params![
                task.title,
                task.content_kind.as_str(),
                source,
                external_url,
                source,
                now,
                existing
            ],
        )
        .map_err(|error| format!("发布作品更新失败：{error}"))?;
        return Ok(existing);
    }
    let work_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO external_works(
            id, source_key, account_id, platform_id, platform_name, account_alias,
            external_id, title, content_kind, original_content_type, url, published_at,
            source, synced_at, created_at, updated_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?10, ?12, ?12, ?12)",
        params![
            work_id,
            source_key,
            task.account_id,
            task.platform_id,
            task.platform_name,
            task.account_alias,
            external_id,
            task.title,
            task.content_kind.as_str(),
            source,
            external_url,
            now
        ],
    )
    .map_err(|error| format!("发布作品保存失败：{error}"))?;
    Ok(work_id)
}

pub fn confirm_publish_task(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
    task_id: &str,
    confirmation_token: &str,
) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if task.status != PublishTaskStatus::Created {
        return Err("只有已准备且未确认的发布任务可以执行".to_string());
    }
    if task.confirmation_token.as_deref() != Some(confirmation_token) {
        return Err("发布确认令牌无效或已被使用".to_string());
    }
    if task
        .confirmation_hash
        .as_deref()
        .is_none_or(|value| value.len() != 64)
    {
        return Err("发布确认摘要缺失，请重新预览最终内容包".to_string());
    }
    let expires_at = task
        .confirmation_expires_at
        .as_deref()
        .ok_or_else(|| "发布确认已失效，请重新预览".to_string())?;
    let expiry = DateTime::parse_from_rfc3339(expires_at)
        .map_err(|_| "发布确认过期时间无效，请重新预览".to_string())?;
    if expiry <= Utc::now() {
        return Err("发布确认已过期，请重新预览最终内容包".to_string());
    }
    validate_source_unchanged(workspace, &task)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE publish_tasks
         SET confirmed_at=?1, confirmation_token=NULL, confirmation_hash=NULL,
             confirmation_expires_at=NULL, updated_at=?1
         WHERE id=?2",
        params![now, task.id],
    )
    .map_err(|error| format!("发布确认保存失败：{error}"))?;
    insert_task_event(
        conn,
        &task.id,
        "CONFIRMED",
        "用户已确认最终内容包与目标账号",
    )?;
    if task.execution_mode == PublishExecutionMode::Manual {
        conn.execute(
            "UPDATE publish_tasks
             SET status=?1, next_action='export_manual_package_or_open_platform', updated_at=?2
             WHERE id=?3",
            params![
                PublishTaskStatus::NeedsManualAction.as_str(),
                Utc::now().to_rfc3339(),
                task.id
            ],
        )
        .map_err(|error| format!("人工发布任务状态保存失败：{error}"))?;
        return publish_task(conn, &task.id);
    }
    if let Some(scheduled_at) = task.scheduled_at.as_deref() {
        let scheduled = DateTime::parse_from_rfc3339(scheduled_at)
            .map_err(|_| "定时发布时间无效".to_string())?;
        if scheduled > Utc::now() {
            conn.execute(
                "UPDATE publish_tasks SET status=?1, next_action='wait_until_scheduled', updated_at=?2
                 WHERE id=?3",
                params![
                    PublishTaskStatus::Scheduled.as_str(),
                    Utc::now().to_rfc3339(),
                    task.id
                ],
            )
            .map_err(|error| format!("定时发布任务状态保存失败：{error}"))?;
            return publish_task(conn, &task.id);
        }
    }
    execute_auto_publish(conn, runtime, workspace, &task.id)
}

fn execute_auto_publish(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    _workspace: &Path,
    task_id: &str,
) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if task.status != PublishTaskStatus::Created && task.status != PublishTaskStatus::Scheduled {
        return Err("当前任务状态不能执行自动发布".to_string());
    }
    if task.paused {
        return Err("任务已因账号失效而暂停".to_string());
    }
    if let Err(error) = ensure_account_ready(conn, &task) {
        update_task_failure(
            conn,
            &task.id,
            PublishTaskStatus::NeedsManualAction,
            "ACCOUNT_NOT_READY",
            &error,
            "reauthorize_and_resume",
        )?;
        return publish_task(conn, &task.id);
    }
    let (capability, capability_reason) = capability_for_task(conn, runtime, &task)?;
    if capability != CapabilityLevel::Supported {
        update_task_failure(
            conn,
            &task.id,
            PublishTaskStatus::NeedsManualAction,
            "AUTO_PUBLISH_UNAVAILABLE",
            capability_reason
                .as_deref()
                .unwrap_or("该平台/账号未验证自动发布能力"),
            "export_manual_package",
        )?;
        return publish_task(conn, &task.id);
    }
    let input = publish_input_from_task(&task)?;
    let validation = validate_publish_payload(&input);
    if !validation.ok {
        update_task_failure(
            conn,
            &task.id,
            PublishTaskStatus::Failed,
            validation
                .issues
                .first()
                .map(|issue| issue.code.as_str())
                .unwrap_or("INVALID_PAYLOAD"),
            &validation
                .issues
                .first()
                .map(|issue| issue.message.clone())
                .unwrap_or_else(|| "发布载荷校验失败".to_string()),
            "fix_and_retry",
        )?;
        return publish_task(conn, &task.id);
    }
    let adapter_id = task
        .adapter_id
        .clone()
        .ok_or_else(|| "自动发布缺少适配器".to_string())?;
    conn.execute(
        "UPDATE publish_tasks
         SET status=?1, attempt_count=attempt_count+1, error_code=NULL, error_message=NULL,
             next_action='await_adapter_result', updated_at=?2
         WHERE id=?3",
        params![
            PublishTaskStatus::Publishing.as_str(),
            Utc::now().to_rfc3339(),
            task.id
        ],
    )
    .map_err(|error| format!("发布中状态保存失败：{error}"))?;
    insert_task_event(
        conn,
        &task.id,
        "SUBMITTED",
        &format!(
            "adapter={} contentKind={} mediaOrder={:?} cover={}",
            adapter_id,
            task.content_kind.as_str(),
            input
                .media
                .iter()
                .map(|media| media.order)
                .collect::<Vec<_>>(),
            input
                .cover
                .as_ref()
                .map(|cover| cover.asset_id.as_str())
                .unwrap_or("none")
        ),
    )?;
    match plugins::execute_plugin_publish(conn, runtime, &adapter_id, &task.account_id, &input) {
        Ok(output) => {
            let expected_order = input
                .media
                .iter()
                .map(|media| media.order)
                .collect::<Vec<_>>();
            let expected_cover = input.cover.as_ref().map(|cover| cover.asset_id.as_str());
            if output.media_order != expected_order
                || output.cover_asset_id.as_deref() != expected_cover
            {
                update_task_failure(
                    conn,
                    &task.id,
                    PublishTaskStatus::NeedsManualAction,
                    "RESULT_MISMATCH",
                    "适配器返回的媒体顺序或封面与提交快照不一致，请到平台核对实际结果",
                    "verify_platform_result",
                )?;
                return publish_task(conn, &task.id);
            }
            let now = Utc::now().to_rfc3339();
            insert_or_update_task_work(
                conn,
                &task,
                &output.external_id,
                output.external_url.as_deref(),
                "publish",
                &now,
            )?;
            if task_is_real_account(conn, &task.account_id)? {
                let legacy_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO platform_works(id, account_id, title, external_id, created_at)
                     VALUES(?1, ?2, ?3, ?4, ?5)",
                    params![
                        legacy_id,
                        task.account_id,
                        task.title,
                        output.external_id,
                        now
                    ],
                )
                .map_err(|error| format!("平台作品历史保存失败：{error}"))?;
            }
            conn.execute(
                "UPDATE publish_tasks
                 SET status=?1, external_id=?2, external_url=?3, error_code=NULL,
                     error_message=NULL, next_action=NULL, updated_at=?4
                 WHERE id=?5",
                params![
                    PublishTaskStatus::Success.as_str(),
                    output.external_id,
                    output.external_url,
                    now,
                    task.id
                ],
            )
            .map_err(|error| format!("发布成功结果保存失败：{error}"))?;
            insert_task_event(
                conn,
                &task.id,
                "SUCCESS",
                &format!(
                    "externalId={} mediaOrder={:?} cover={}",
                    output.external_id,
                    output.media_order,
                    output.cover_asset_id.as_deref().unwrap_or("none")
                ),
            )?;
        }
        Err(error) => {
            if error.uncertain {
                update_task_failure(
                    conn,
                    &task.id,
                    PublishTaskStatus::NeedsManualAction,
                    "RESULT_UNCERTAIN",
                    &format!("发布请求超时，平台结果不确定：{}", error.message),
                    "query_platform_or_mark_result",
                )?;
            } else {
                if error.code == "AUTH_EXPIRED" || error.message.contains("会话") {
                    let _ = mark_account_invalid(conn, &task.account_id);
                }
                update_task_failure(
                    conn,
                    &task.id,
                    PublishTaskStatus::Failed,
                    &error.code,
                    &error.message,
                    "review_error_and_retry",
                )?;
            }
        }
    }
    publish_task(conn, &task.id)
}

pub fn run_due_publish_tasks(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
) -> Result<Vec<PublishTask>, String> {
    let now = Utc::now();
    let mut statement = conn
        .prepare(
            "SELECT id, scheduled_at FROM publish_tasks
             WHERE status='SCHEDULED' AND is_paused=0 AND scheduled_at IS NOT NULL
             ORDER BY scheduled_at ASC",
        )
        .map_err(|error| format!("待执行定时任务准备失败：{error}"))?;
    let candidates = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("待执行定时任务读取失败：{error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("待执行定时任务解析失败：{error}"))?;
    drop(statement);
    let mut executed = Vec::new();
    for (task_id, scheduled_at) in candidates {
        let Some(scheduled) = parse_datetime(&scheduled_at) else {
            update_task_failure(
                conn,
                &task_id,
                PublishTaskStatus::Failed,
                "INVALID_SCHEDULE",
                "定时发布时间无效，无法执行",
                "fix_and_retry",
            )?;
            continue;
        };
        if scheduled > now {
            continue;
        }
        executed.push(execute_auto_publish(conn, runtime, workspace, &task_id)?);
    }
    Ok(executed)
}

pub fn cancel_publish_task(
    conn: &Connection,
    task_id: &str,
    reason: Option<&str>,
) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if matches!(
        task.status,
        PublishTaskStatus::Success | PublishTaskStatus::Publishing
    ) {
        return Err("已成功或正在提交的任务不能取消".to_string());
    }
    conn.execute(
        "UPDATE publish_tasks
         SET status=?1, error_code='CANCELLED', error_message=?2, next_action=NULL,
             confirmation_token=NULL, confirmation_hash=NULL, confirmation_expires_at=NULL,
             updated_at=?3
         WHERE id=?4",
        params![
            PublishTaskStatus::Cancelled.as_str(),
            reason.unwrap_or("用户取消发布任务"),
            Utc::now().to_rfc3339(),
            task_id
        ],
    )
    .map_err(|error| format!("取消发布任务失败：{error}"))?;
    insert_task_event(
        conn,
        task_id,
        "CANCELLED",
        reason.unwrap_or("用户取消发布任务"),
    )?;
    publish_task(conn, task_id)
}

pub fn retry_publish_task(conn: &Connection, task_id: &str) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if task.status != PublishTaskStatus::Failed {
        return Err("仅明确失败的任务可重新准备；超时不确定任务必须先核对结果".to_string());
    }
    conn.execute(
        "UPDATE publish_tasks
         SET status=?1, error_code=NULL, error_message=NULL, next_action='review_and_confirm',
             confirmation_token=NULL, confirmation_hash=NULL, confirmation_expires_at=NULL,
             updated_at=?2
         WHERE id=?3",
        params![
            PublishTaskStatus::Created.as_str(),
            Utc::now().to_rfc3339(),
            task_id
        ],
    )
    .map_err(|error| format!("发布任务重试准备失败：{error}"))?;
    insert_task_event(conn, task_id, "RETRY_PREPARED", "用户显式发起重新准备")?;
    publish_task(conn, task_id)
}

pub fn update_publish_task_schedule(
    conn: &Connection,
    task_id: &str,
    scheduled_at: Option<&str>,
) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if task.status != PublishTaskStatus::Created {
        return Err("只有尚未确认的发布任务可以调整计划时间".to_string());
    }
    if let Some(value) = scheduled_at {
        let scheduled = DateTime::parse_from_rfc3339(value)
            .map_err(|_| "计划发布时间必须是 RFC3339 时间".to_string())?;
        if scheduled <= Utc::now() {
            return Err("计划发布时间必须晚于当前时间".to_string());
        }
    }
    conn.execute(
        "UPDATE publish_tasks
         SET scheduled_at=?1, confirmation_token=NULL, confirmation_hash=NULL,
             confirmation_expires_at=NULL, next_action='review_and_confirm', updated_at=?2
         WHERE id=?3",
        params![scheduled_at, Utc::now().to_rfc3339(), task_id],
    )
    .map_err(|error| format!("发布计划时间保存失败：{error}"))?;
    insert_task_event(
        conn,
        task_id,
        "SCHEDULE_UPDATED",
        scheduled_at.unwrap_or("立即发布"),
    )?;
    publish_task(conn, task_id)
}

pub fn resolve_uncertain_publish_task(
    conn: &mut Connection,
    task_id: &str,
    input: &ResolveUncertainInput,
) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if task.error_code.as_deref() != Some("RESULT_UNCERTAIN")
        && task.error_code.as_deref() != Some("RESULT_MISMATCH")
    {
        return Err("只有结果不确定或待核对的发布任务可以记录人工核对结果".to_string());
    }
    let now = Utc::now().to_rfc3339();
    match input.outcome {
        UncertainOutcome::Success => {
            let external_id = input
                .external_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "标记发布成功时必须填写平台外部作品 ID".to_string())?;
            insert_or_update_task_work(
                conn,
                &task,
                external_id,
                input.external_url.as_deref(),
                "manual_check",
                &now,
            )?;
            conn.execute(
                "UPDATE publish_tasks
                 SET status=?1, external_id=?2, external_url=?3, error_code=NULL,
                     error_message=NULL, next_action=NULL, is_paused=0, pause_reason=NULL,
                     updated_at=?4
                 WHERE id=?5",
                params![
                    PublishTaskStatus::Success.as_str(),
                    external_id,
                    input.external_url,
                    now,
                    task.id
                ],
            )
            .map_err(|error| format!("人工核对成功结果保存失败：{error}"))?;
        }
        UncertainOutcome::Failed => {
            conn.execute(
                "UPDATE publish_tasks
                 SET status=?1, error_code='VERIFIED_FAILED', error_message=?2,
                     next_action='review_error_and_retry', is_paused=0, pause_reason=NULL,
                     updated_at=?3
                 WHERE id=?4",
                params![
                    PublishTaskStatus::Failed.as_str(),
                    input.note.as_deref().unwrap_or("人工核对确认平台未发布"),
                    now,
                    task.id
                ],
            )
            .map_err(|error| format!("人工核对失败结果保存失败：{error}"))?;
        }
        UncertainOutcome::Cancelled => {
            conn.execute(
                "UPDATE publish_tasks
                 SET status=?1, error_code='CANCELLED_AFTER_CHECK', error_message=?2,
                     next_action=NULL, is_paused=0, pause_reason=NULL, updated_at=?3
                 WHERE id=?4",
                params![
                    PublishTaskStatus::Cancelled.as_str(),
                    input.note.as_deref().unwrap_or("人工核对后取消任务"),
                    now,
                    task.id
                ],
            )
            .map_err(|error| format!("人工核对取消结果保存失败：{error}"))?;
        }
    }
    insert_task_event(
        conn,
        task_id,
        "MANUALLY_VERIFIED",
        input.note.as_deref().unwrap_or("已记录人工核对结果"),
    )?;
    publish_task(conn, task_id)
}

pub fn complete_manual_publish_task(
    conn: &mut Connection,
    task_id: &str,
    input: &ManualCompletionInput,
) -> Result<PublishTask, String> {
    let task = task_record(conn, task_id)?;
    if task.status != PublishTaskStatus::NeedsManualAction {
        return Err("只有待人工发布的务可以标记完成".to_string());
    }
    let external_id = input
        .external_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("manual-{}", task.id));
    let now = Utc::now().to_rfc3339();
    insert_or_update_task_work(
        conn,
        &task,
        &external_id,
        input.external_url.as_deref(),
        "manual",
        &now,
    )?;
    conn.execute(
        "UPDATE publish_tasks
         SET status=?1, external_id=?2, external_url=?3, error_code=NULL, error_message=NULL,
             next_action=NULL, is_paused=0, pause_reason=NULL, updated_at=?4
         WHERE id=?5",
        params![
            PublishTaskStatus::Success.as_str(),
            external_id,
            input.external_url,
            now,
            task.id
        ],
    )
    .map_err(|error| format!("人工发布完成状态保存失败：{error}"))?;
    insert_task_event(
        conn,
        task_id,
        "MANUAL_COMPLETED",
        &format!("externalId={external_id}"),
    )?;
    publish_task(conn, task_id)
}

pub fn resume_account_tasks(
    conn: &Connection,
    account_id: &str,
) -> Result<Vec<PublishTask>, String> {
    if task_is_real_account(conn, account_id)? {
        platforms::require_active_account(conn, account_id)?;
    }
    conn.execute(
        "UPDATE publish_tasks
         SET is_paused=0, pause_reason=NULL, status=?1, next_action='review_and_confirm',
             confirmation_token=NULL, confirmation_hash=NULL, confirmation_expires_at=NULL,
             updated_at=?2
         WHERE account_id=?3 AND is_paused=1
           AND status NOT IN ('SUCCESS','FAILED','CANCELLED')",
        params![
            PublishTaskStatus::Created.as_str(),
            Utc::now().to_rfc3339(),
            account_id
        ],
    )
    .map_err(|error| format!("账号任务恢复失败：{error}"))?;
    let mut statement = conn
        .prepare(&format!(
            "{} WHERE account_id=?1 ORDER BY created_at DESC",
            task_select()
        ))
        .map_err(|error| format!("恢复任务列表准备失败：{error}"))?;
    let rows = statement
        .query_map([account_id], task_from_row)
        .map_err(|error| format!("恢复任务列表读取失败：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("恢复任务列表解析失败：{error}"))?
        .into_iter()
        .map(|row| row.to_view())
        .collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricSnapshot {
    pub id: String,
    pub work_id: String,
    pub metric_name: String,
    pub metric_definition: String,
    pub unit: String,
    pub value: f64,
    pub captured_at: String,
    pub snapshot_day: String,
    pub source: String,
    pub canonical_key: Option<String>,
    pub comparable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalWork {
    pub id: String,
    pub source_key: String,
    pub account_id: Option<String>,
    pub platform_id: String,
    pub platform_name: String,
    pub account_alias: String,
    pub external_id: String,
    pub title: String,
    pub content_kind: String,
    pub original_content_type: String,
    pub raw_fields: Value,
    pub url: Option<String>,
    pub published_at: String,
    pub source: String,
    pub synced_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metrics: Vec<MetricSnapshot>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalWorkFilter {
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
    #[serde(default)]
    pub published_from: Option<String>,
    #[serde(default)]
    pub published_to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncPlatformWorksInput {
    pub account_id: String,
    pub platform_id: String,
    pub platform_name: String,
    pub account_alias: String,
    pub adapter_id: String,
    #[serde(default)]
    pub as_of: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunResult {
    pub id: String,
    pub account_id: String,
    pub platform_id: String,
    pub adapter_id: String,
    pub status: String,
    pub message: String,
    pub imported_count: u32,
    pub updated_count: u32,
    pub skipped_count: u32,
    pub error_count: u32,
    pub account_invalid: bool,
    pub errors: Vec<String>,
    pub started_at: String,
    pub completed_at: String,
}

fn metric_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetricSnapshot> {
    Ok(MetricSnapshot {
        id: row.get(0)?,
        work_id: row.get(1)?,
        metric_name: row.get(2)?,
        metric_definition: row.get(3)?,
        unit: row.get(4)?,
        value: row.get(5)?,
        captured_at: row.get(6)?,
        snapshot_day: row.get(7)?,
        source: row.get(8)?,
        canonical_key: row.get(9)?,
        comparable: row.get::<_, i64>(10)? != 0,
    })
}

fn metrics_for_work(conn: &Connection, work_id: &str) -> Result<Vec<MetricSnapshot>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, work_id, metric_name, metric_definition, unit, value, captured_at,
                    snapshot_day, source, canonical_key, comparable
             FROM metric_snapshots WHERE work_id=?1
             ORDER BY captured_at ASC, metric_name ASC",
        )
        .map_err(|error| format!("指标快照准备失败：{error}"))?;
    let rows = statement
        .query_map([work_id], metric_from_row)
        .map_err(|error| format!("指标快照读取失败：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("指标快照解析失败：{error}"))
}

type ExternalWorkRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
);

fn external_work_from_values(
    conn: &Connection,
    row: ExternalWorkRow,
) -> Result<ExternalWork, String> {
    let (
        id,
        source_key,
        account_id,
        platform_id,
        platform_name,
        account_alias,
        external_id,
        title,
        content_kind,
        original_content_type,
        raw_fields_json,
        url,
        published_at,
        source,
        synced_at,
        created_at,
        updated_at,
    ) = row;
    Ok(ExternalWork {
        metrics: metrics_for_work(conn, &id)?,
        id,
        source_key,
        account_id,
        platform_id,
        platform_name,
        account_alias,
        external_id,
        title,
        content_kind,
        original_content_type,
        raw_fields: serde_json::from_str(&raw_fields_json)
            .map_err(|error| format!("外部作品原始字段解析失败：{error}"))?,
        url,
        published_at,
        source,
        synced_at,
        created_at,
        updated_at,
    })
}

pub fn list_external_works(
    conn: &Connection,
    filter: &ExternalWorkFilter,
) -> Result<Vec<ExternalWork>, String> {
    let keyword = filter
        .keyword
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let from = filter
        .published_from
        .as_deref()
        .and_then(parse_datetime)
        .map(|value| value.date_naive());
    let to = filter
        .published_to
        .as_deref()
        .and_then(parse_datetime)
        .map(|value| value.date_naive());
    let mut statement = conn
        .prepare(
            "SELECT id, source_key, account_id, platform_id, platform_name, account_alias,
                    external_id, title, content_kind, original_content_type, raw_fields_json,
                    url, published_at, source, synced_at, created_at, updated_at
             FROM external_works ORDER BY published_at DESC, updated_at DESC",
        )
        .map_err(|error| format!("作品列表准备失败：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, String>(16)?,
            ))
        })
        .map_err(|error| format!("作品列表读取失败：{error}"))?;
    let mut filtered = Vec::new();
    for row in rows {
        let row = row.map_err(|error| format!("作品列表解析失败：{error}"))?;
        let account_id = &row.2;
        let platform_id = &row.3;
        let external_id = &row.6;
        let title = &row.7;
        let content_kind = &row.8;
        let content_type = &row.9;
        let published_at = &row.12;
        if filter
            .platform_id
            .as_deref()
            .is_some_and(|value| value != platform_id)
        {
            continue;
        }
        if filter
            .account_id
            .as_deref()
            .is_some_and(|value| account_id.as_deref() != Some(value))
        {
            continue;
        }
        if filter
            .content_kind
            .as_deref()
            .is_some_and(|value| value != content_kind)
        {
            continue;
        }
        let published_date = parse_datetime(published_at).map(|value| value.date_naive());
        if from.is_some_and(|from| published_date.is_none_or(|date| date < from)) {
            continue;
        }
        if to.is_some_and(|to| published_date.is_none_or(|date| date > to)) {
            continue;
        }
        if !keyword.is_empty()
            && !format!("{title} {external_id} {platform_id} {content_type}")
                .to_lowercase()
                .contains(&keyword)
        {
            continue;
        }
        filtered.push(row);
    }
    let mut works = Vec::new();
    for row in filtered {
        works.push(external_work_from_values(conn, row)?);
    }
    Ok(works)
}

pub fn external_work(conn: &Connection, work_id: &str) -> Result<ExternalWork, String> {
    let row = conn
        .query_row(
            "SELECT id, source_key, account_id, platform_id, platform_name, account_alias,
                    external_id, title, content_kind, original_content_type, raw_fields_json,
                    url, published_at, source, synced_at, created_at, updated_at
             FROM external_works WHERE id=?1",
            [work_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("作品详情读取失败：{error}"))?
        .ok_or_else(|| "未找到外部作品".to_string())?;
    external_work_from_values(conn, row)
}

fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn day_bucket(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn upsert_external_work(
    conn: &Connection,
    input: &SyncPlatformWorksInput,
    work: &plugins::AdapterSyncWork,
    synced_at: &str,
) -> Result<(String, bool), String> {
    let source_key = format!(
        "sync:{}:{}:{}",
        input.platform_id, input.account_id, work.external_id
    );
    if let Some(existing) = conn
        .query_row(
            "SELECT id FROM external_works WHERE source_key=?1",
            [&source_key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("同步作品去重查询失败：{error}"))?
    {
        conn.execute(
            "UPDATE external_works
             SET title=?1, content_kind=?2, original_content_type=?3, raw_fields_json=?4,
                 url=?5, published_at=?6, source='sync', synced_at=?7, updated_at=?7
             WHERE id=?8",
            params![
                work.title,
                work.content_kind
                    .map(|kind| kind.as_str())
                    .unwrap_or("unknown"),
                work.content_type,
                serde_json::to_string(&work.raw_fields)
                    .map_err(|error| format!("平台原始字段编码失败：{error}"))?,
                work.url,
                work.published_at,
                synced_at,
                existing
            ],
        )
        .map_err(|error| format!("同步作品更新失败：{error}"))?;
        return Ok((existing, false));
    }
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO external_works(
            id, source_key, account_id, platform_id, platform_name, account_alias,
            external_id, title, content_kind, original_content_type, raw_fields_json,
            url, published_at, source, synced_at, created_at, updated_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'sync', ?14, ?14, ?14)",
        params![
            id,
            source_key,
            input.account_id,
            input.platform_id,
            input.platform_name,
            input.account_alias,
            work.external_id,
            work.title,
            work.content_kind
                .map(|kind| kind.as_str())
                .unwrap_or("unknown"),
            work.content_type,
            serde_json::to_string(&work.raw_fields)
                .map_err(|error| format!("平台原始字段编码失败：{error}"))?,
            work.url,
            work.published_at,
            synced_at
        ],
    )
    .map_err(|error| format!("同步作品保存失败：{error}"))?;
    Ok((id, true))
}

fn record_metric_snapshots(
    conn: &Connection,
    work_id: &str,
    metrics: &[plugins::AdapterSyncMetric],
    captured_at: DateTime<Utc>,
    source: &str,
) -> Result<u32, String> {
    let mut recorded = 0;
    let day = day_bucket(captured_at);
    for metric in metrics {
        conn.execute(
            "INSERT INTO metric_snapshots(
                id, work_id, metric_name, metric_definition, unit, value, captured_at,
                snapshot_day, source, canonical_key, comparable
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, 0)
             ON CONFLICT(work_id, metric_name, snapshot_day) DO UPDATE SET
                metric_definition=excluded.metric_definition,
                unit=excluded.unit,
                value=excluded.value,
                captured_at=excluded.captured_at,
                source=excluded.source",
            params![
                Uuid::new_v4().to_string(),
                work_id,
                metric.name,
                metric.definition,
                metric.unit,
                metric.value,
                captured_at.to_rfc3339(),
                day,
                source
            ],
        )
        .map_err(|error| format!("指标快照保存失败：{error}"))?;
        recorded += 1;
    }
    Ok(recorded)
}

pub fn sync_platform_works(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    input: &SyncPlatformWorksInput,
) -> Result<SyncRunResult, String> {
    let started_at = Utc::now();
    let as_of = input
        .as_of
        .as_deref()
        .and_then(parse_datetime)
        .unwrap_or(started_at);
    let adapter_result = plugins::execute_plugin_sync(
        conn,
        runtime,
        &input.adapter_id,
        &input.account_id,
        Some(&as_of.to_rfc3339()),
    );
    let run_id = Uuid::new_v4().to_string();
    match adapter_result {
        Ok(output) => {
            let tx = conn
                .transaction()
                .map_err(|error| format!("无法开始同步事务：{error}"))?;
            let mut imported_count = 0;
            let mut updated_count = 0;
            let mut skipped_count = 0;
            let mut errors = output.errors;
            for work in &output.works {
                let (work_id, inserted) =
                    upsert_external_work(&tx, input, work, &started_at.to_rfc3339())?;
                if inserted {
                    imported_count += 1;
                } else {
                    updated_count += 1;
                }
                let captured_at = work
                    .captured_at
                    .as_deref()
                    .and_then(parse_datetime)
                    .unwrap_or(as_of);
                let recorded = record_metric_snapshots(
                    &tx,
                    &work_id,
                    &work.metrics,
                    captured_at,
                    &format!("adapter:{}", input.adapter_id),
                )?;
                if recorded == 0 {
                    skipped_count += 1;
                }
            }
            let error_count = errors.len() as u32;
            let status = if error_count > 0 {
                "PARTIAL"
            } else {
                "SUCCESS"
            };
            let message = if error_count > 0 {
                format!(
                    "同步完成但有 {error_count} 项失败；已导入 {imported_count} 个、更新 {updated_count} 个作品"
                )
            } else {
                format!(
                    "同步完成：导入 {imported_count} 个，更新 {updated_count} 个作品；指标按 UTC 日采样"
                )
            };
            let completed_at = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO sync_runs(
                    id, account_id, platform_id, adapter_id, status, message,
                    imported_count, updated_count, skipped_count, error_count,
                    started_at, completed_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    run_id,
                    input.account_id,
                    input.platform_id,
                    input.adapter_id,
                    status,
                    message,
                    imported_count,
                    updated_count,
                    skipped_count,
                    error_count,
                    started_at.to_rfc3339(),
                    completed_at
                ],
            )
            .map_err(|error| format!("同步运行记录保存失败：{error}"))?;
            tx.commit()
                .map_err(|error| format!("同步事务提交失败：{error}"))?;
            Ok(SyncRunResult {
                id: run_id,
                account_id: input.account_id.clone(),
                platform_id: input.platform_id.clone(),
                adapter_id: input.adapter_id.clone(),
                status: status.to_string(),
                message,
                imported_count,
                updated_count,
                skipped_count,
                error_count,
                account_invalid: false,
                errors: std::mem::take(&mut errors),
                started_at: started_at.to_rfc3339(),
                completed_at: Utc::now().to_rfc3339(),
            })
        }
        Err(error) => {
            let account_invalid =
                error.code == "AUTH_EXPIRED" || error.message.contains("AUTH_EXPIRED");
            if account_invalid {
                mark_account_invalid(conn, &input.account_id)?;
            }
            let completed_at = Utc::now().to_rfc3339();
            let message = if account_invalid {
                "账号已失效，依赖该账号的发布任务已暂停；请重新授权后显式恢复".to_string()
            } else {
                format!("同步失败：{}", error.message)
            };
            conn.execute(
                "INSERT INTO sync_runs(
                    id, account_id, platform_id, adapter_id, status, message,
                    imported_count, updated_count, skipped_count, error_count,
                    started_at, completed_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0, 1, ?7, ?8)",
                params![
                    run_id,
                    input.account_id,
                    input.platform_id,
                    input.adapter_id,
                    if account_invalid {
                        "ACCOUNT_INVALID"
                    } else {
                        "FAILED"
                    },
                    message,
                    started_at.to_rfc3339(),
                    completed_at
                ],
            )
            .map_err(|db_error| format!("同步失败记录保存失败：{db_error}"))?;
            Ok(SyncRunResult {
                id: run_id,
                account_id: input.account_id.clone(),
                platform_id: input.platform_id.clone(),
                adapter_id: input.adapter_id.clone(),
                status: if account_invalid {
                    "ACCOUNT_INVALID".to_string()
                } else {
                    "FAILED".to_string()
                },
                message,
                imported_count: 0,
                updated_count: 0,
                skipped_count: 0,
                error_count: 1,
                account_invalid,
                errors: vec![error.message],
                started_at: started_at.to_rfc3339(),
                completed_at,
            })
        }
    }
}

pub fn list_sync_runs(conn: &Connection, limit: usize) -> Result<Vec<SyncRunResult>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, account_id, platform_id, adapter_id, status, message,
                    imported_count, updated_count, skipped_count, error_count,
                    started_at, completed_at
             FROM sync_runs ORDER BY completed_at DESC LIMIT ?1",
        )
        .map_err(|error| format!("同步记录准备失败：{error}"))?;
    let rows = statement
        .query_map([limit as i64], |row| {
            Ok(SyncRunResult {
                id: row.get(0)?,
                account_id: row.get(1)?,
                platform_id: row.get(2)?,
                adapter_id: row.get(3)?,
                status: row.get(4)?,
                message: row.get(5)?,
                imported_count: u32::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
                updated_count: u32::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
                skipped_count: u32::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
                error_count: u32::try_from(row.get::<_, i64>(9)?).unwrap_or(0),
                account_invalid: row.get::<_, String>(4)? == "ACCOUNT_INVALID",
                errors: Vec::new(),
                started_at: row.get(10)?,
                completed_at: row.get(11)?,
            })
        })
        .map_err(|error| format!("同步记录读取失败：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("同步记录解析失败：{error}"))
}

pub fn export_manual_publish_package(
    conn: &Connection,
    workspace: &Path,
    task_id: &str,
) -> Result<ManualPublishPackage, String> {
    let task = task_record(conn, task_id)?;
    let view = task.to_view()?;
    if !matches!(
        task.status,
        PublishTaskStatus::Created
            | PublishTaskStatus::NeedsManualAction
            | PublishTaskStatus::Failed
    ) {
        return Err("当前任务状态不能导出人工发布包".to_string());
    }
    validate_source_unchanged(workspace, &task)?;
    let safe_id = task
        .id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>();
    let directory = workspace.join("exports").join(format!(
        "manual-{safe_id}-{}",
        Utc::now().format("%Y%m%d%H%M%S")
    ));
    fs::create_dir_all(directory.join("assets"))
        .map_err(|error| format!("无法创建人工发布包目录：{error}"))?;
    let mut assets = Vec::new();
    let cover_id = task.cover()?.map(|cover| cover.id);
    for (index, asset) in task.media()?.into_iter().enumerate() {
        let role = if cover_id.as_deref() == Some(asset.id.as_str()) {
            "cover-and-media"
        } else {
            "media"
        };
        assets.push(copy_manual_asset(
            &directory,
            &task,
            asset,
            role,
            index + 1,
        )?);
    }
    if let Some(cover) = task.cover()?
        && !task.media()?.iter().any(|asset| asset.id == cover.id)
    {
        assets.push(copy_manual_asset(&directory, &task, cover, "cover", 0)?);
    }
    let body_path = directory.join("body.md");
    fs::write(&body_path, task.body.as_bytes())
        .map_err(|error| format!("无法写入人工发布正文：{error}"))?;
    let generated_at = Utc::now().to_rfc3339();
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "generatedAt": generated_at,
        "taskId": task.id,
        "contentId": task.content_id,
        "contentVersion": task.content_version,
        "contentKind": task.content_kind.as_str(),
        "platformId": task.platform_id,
        "platformName": task.platform_name,
        "accountId": task.account_id,
        "accountAlias": task.account_alias,
        "title": task.title,
        "bodyFile": "body.md",
        "tags": task.tags()?,
        "mediaOrder": assets.iter().filter(|asset| asset.role != "cover").map(|asset| asset.order).collect::<Vec<_>>(),
        "coverAssetId": cover_id,
        "assets": assets,
        "instructions": [
            "打开目标平台并选择对应账号",
            "按 assets 中 order 顺序上传媒体；role 为 cover 的素材设为封面",
            "将 body.md 内容粘贴或导入正文编辑器",
            "发布完成后在 CreatorRelay 中填写外部作品 ID/URL 并标记完成",
            "若结果不确定，请先到平台作品列表核对，避免重复发布"
        ]
    });
    let manifest_path = directory.join("publish-package.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("无法写入人工发布清单：{error}"))?;
    insert_task_event(
        conn,
        &task.id,
        "MANUAL_PACKAGE_EXPORTED",
        &directory.to_string_lossy(),
    )?;
    Ok(ManualPublishPackage {
        task: view,
        directory: directory.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        body_path: body_path.to_string_lossy().to_string(),
        assets,
        generated_at,
    })
}

fn copy_manual_asset(
    directory: &Path,
    task: &TaskRow,
    asset: ContentAsset,
    role: &str,
    index: usize,
) -> Result<ManualPackageAsset, String> {
    let bytes = match task.source_type {
        PublishSourceType::PlatformDraft => content::read_platform_draft_asset(
            &task_source_workspace(directory)?,
            &task.source_id,
            &asset.id,
        ),
        PublishSourceType::Content => content::read_content_asset(
            &task_source_workspace(directory)?,
            &task.content_id,
            &asset.id,
        ),
    };
    let content = bytes?;
    let extension = Path::new(&asset.file)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let prefix = if index == 0 {
        "cover".to_string()
    } else {
        format!("{index:02}")
    };
    let file_name = format!(
        "{prefix}-{role}-{}.{}",
        safe_component(&asset.original_name),
        extension
    );
    let relative = format!("assets/{file_name}");
    fs::write(directory.join(&relative), content.bytes)
        .map_err(|error| format!("无法复制人工发布素材：{error}"))?;
    Ok(ManualPackageAsset {
        asset_id: asset.id,
        kind: match asset.kind {
            content::AssetKind::Video => "video".to_string(),
            content::AssetKind::Image => "image".to_string(),
        },
        role: role.to_string(),
        order: asset.order,
        file: relative,
        original_name: asset.original_name,
        mime_type: content.mime_type,
        byte_size: asset.byte_size,
        caption: asset.caption,
    })
}

fn task_source_workspace(directory: &Path) -> Result<PathBuf, String> {
    directory
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "人工发布包无法定位工作区".to_string())
}

fn safe_component(value: &str) -> String {
    let output = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = output.trim_matches('-');
    if trimmed.is_empty() {
        "asset".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalyticsGranularity {
    Day,
    Week,
    Month,
}

impl AnalyticsGranularity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyticsQuery {
    pub range: String,
    pub granularity: AnalyticsGranularity,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
    #[serde(default)]
    pub work_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsPoint {
    pub bucket: String,
    pub captured_at: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSeries {
    pub key: String,
    pub work_id: String,
    pub title: String,
    pub platform_id: String,
    pub platform_name: String,
    pub account_alias: String,
    pub content_kind: String,
    pub metric_name: String,
    pub metric_definition: String,
    pub unit: String,
    pub points: Vec<AnalyticsPoint>,
    pub comparable: bool,
    pub comparability_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsResult {
    pub range: String,
    pub from: String,
    pub to: String,
    pub granularity: String,
    pub series: Vec<AnalyticsSeries>,
    pub warnings: Vec<String>,
}

fn range_days(range: &str) -> Result<i64, String> {
    match range {
        "7d" => Ok(7),
        "30d" => Ok(30),
        "90d" => Ok(90),
        _ => Err("趋势范围必须是 7d、30d 或 90d".to_string()),
    }
}

fn analytics_bucket(value: DateTime<Utc>, granularity: AnalyticsGranularity) -> String {
    match granularity {
        AnalyticsGranularity::Day => value.format("%Y-%m-%d").to_string(),
        AnalyticsGranularity::Week => {
            let iso = value.iso_week();
            format!("{}-W{:02}", iso.year(), iso.week())
        }
        AnalyticsGranularity::Month => value.format("%Y-%m").to_string(),
    }
}

pub fn analytics_trend(
    conn: &Connection,
    query: &AnalyticsQuery,
) -> Result<AnalyticsResult, String> {
    let days = range_days(&query.range)?;
    let to = Utc::now();
    let from = to - Duration::days(days - 1);
    let work_filter = ExternalWorkFilter {
        platform_id: query.platform_id.clone(),
        account_id: query.account_id.clone(),
        content_kind: query.content_kind.clone(),
        ..ExternalWorkFilter::default()
    };
    let works = list_external_works(conn, &work_filter)?;
    let mut selected = Vec::new();
    for work in works {
        if query
            .work_id
            .as_deref()
            .is_some_and(|work_id| work.id != work_id)
        {
            continue;
        }
        selected.push(work);
    }
    let mut grouped: HashMap<String, BTreeMap<String, MetricSnapshot>> = HashMap::new();
    let mut metadata: HashMap<String, (ExternalWork, String)> = HashMap::new();
    for work in &selected {
        for metric in &work.metrics {
            let captured = match parse_datetime(&metric.captured_at) {
                Some(value) if value >= from && value <= to => value,
                _ => continue,
            };
            let key = format!("{}::{}", work.id, metric.metric_name);
            let bucket = analytics_bucket(captured, query.granularity);
            let bucket_map = grouped.entry(key.clone()).or_default();
            let replace = bucket_map
                .get(&bucket)
                .and_then(|existing| parse_datetime(&existing.captured_at))
                .is_none_or(|existing| captured > existing);
            if replace {
                bucket_map.insert(bucket, metric.clone());
            }
            metadata
                .entry(key)
                .or_insert_with(|| (work.clone(), metric.metric_name.clone()));
        }
    }
    let mut series = Vec::new();
    let mut warnings = Vec::new();
    for (key, buckets) in grouped {
        let Some((work, metric_name)) = metadata.get(&key) else {
            continue;
        };
        let metric = buckets.values().next().cloned();
        let Some(metric) = metric else {
            continue;
        };
        let points = buckets
            .into_iter()
            .map(|(bucket, snapshot)| AnalyticsPoint {
                bucket,
                captured_at: snapshot.captured_at,
                value: snapshot.value,
            })
            .collect::<Vec<_>>();
        if points.len() < 2 {
            warnings.push(format!(
                "《{}》的「{}」仅有 {} 个采样点，暂不绘制趋势，也不会补零",
                work.title,
                metric_name,
                points.len()
            ));
        }
        series.push(AnalyticsSeries {
            key,
            work_id: work.id.clone(),
            title: work.title.clone(),
            platform_id: work.platform_id.clone(),
            platform_name: work.platform_name.clone(),
            account_alias: work.account_alias.clone(),
            content_kind: work.content_kind.clone(),
            metric_name: metric_name.clone(),
            metric_definition: metric.metric_definition.clone(),
            unit: metric.unit.clone(),
            points,
            comparable: metric.comparable,
            comparability_note: if metric.comparable {
                "指标有明确统一字典定义，可在相同口径范围内比较".to_string()
            } else {
                "仅展示同一作品、同一平台的原始指标，不与其他平台或形态聚合".to_string()
            },
        });
    }
    series.sort_by(|left, right| {
        left.platform_name
            .cmp(&right.platform_name)
            .then(left.title.cmp(&right.title))
            .then(left.metric_name.cmp(&right.metric_name))
    });
    if series.is_empty() {
        warnings.push("所选范围内没有可追溯的指标采样；不会用 0 代替缺失数据".to_string());
    }
    Ok(AnalyticsResult {
        range: query.range.clone(),
        from: from.to_rfc3339(),
        to: to.to_rfc3339(),
        granularity: query.granularity.as_str().to_string(),
        series,
        warnings,
    })
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DashboardFilter {
    #[serde(default)]
    pub range: Option<String>,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationsDashboard {
    pub platform_count: usize,
    pub account_count: usize,
    pub recent_publish_count: usize,
    pub pending_task_count: usize,
    pub sync_error_count: usize,
    pub recent_works: Vec<ExternalWork>,
    pub pending_tasks: Vec<PublishTask>,
    pub recent_publishes: Vec<PublishTask>,
    pub sync_runs: Vec<SyncRunResult>,
    pub filter: DashboardFilterApplied,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardFilterApplied {
    pub range: String,
    pub platform_id: Option<String>,
    pub account_id: Option<String>,
    pub content_kind: Option<String>,
    pub from: String,
    pub to: String,
}

pub fn operations_dashboard(
    conn: &Connection,
    filter: &DashboardFilter,
) -> Result<OperationsDashboard, String> {
    let range = filter.range.as_deref().unwrap_or("7d");
    let days = range_days(range)?;
    let to = Utc::now();
    let from = to - Duration::days(days - 1);
    let accounts = platforms::list_accounts(conn)?;
    let visible_accounts = accounts
        .into_iter()
        .filter(|account| {
            filter
                .platform_id
                .as_deref()
                .is_none_or(|value| account.platform_id == value)
                && filter
                    .account_id
                    .as_deref()
                    .is_none_or(|value| account.id == value)
        })
        .collect::<Vec<_>>();
    let platform_count = visible_accounts
        .iter()
        .map(|account| account.platform_id.as_str())
        .collect::<HashSet<_>>()
        .len();
    let account_count = visible_accounts.len();
    let mut tasks = list_publish_tasks(conn)?;
    tasks.retain(|task| {
        let created = parse_datetime(&task.created_at).unwrap_or(to);
        created >= from
            && filter
                .platform_id
                .as_deref()
                .is_none_or(|value| task.platform_id == value)
            && filter
                .account_id
                .as_deref()
                .is_none_or(|value| task.account_id == value)
            && filter
                .content_kind
                .as_deref()
                .is_none_or(|value| task.content_kind.as_str() == value)
    });
    let recent_publishes = tasks
        .iter()
        .filter(|task| task.status == PublishTaskStatus::Success)
        .take(10)
        .cloned()
        .collect::<Vec<_>>();
    let pending_tasks = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.status,
                PublishTaskStatus::Created
                    | PublishTaskStatus::Validating
                    | PublishTaskStatus::Pending
                    | PublishTaskStatus::Scheduled
                    | PublishTaskStatus::NeedsManualAction
            )
        })
        .take(10)
        .cloned()
        .collect::<Vec<_>>();
    let work_filter = ExternalWorkFilter {
        platform_id: filter.platform_id.clone(),
        account_id: filter.account_id.clone(),
        content_kind: filter.content_kind.clone(),
        published_from: Some(from.to_rfc3339()),
        published_to: Some(to.to_rfc3339()),
        ..ExternalWorkFilter::default()
    };
    let recent_works = list_external_works(conn, &work_filter)?
        .into_iter()
        .take(10)
        .collect::<Vec<_>>();
    let sync_runs = list_sync_runs(conn, 50)?
        .into_iter()
        .filter(|run| {
            parse_datetime(&run.completed_at).is_some_and(|value| value >= from)
                && filter
                    .platform_id
                    .as_deref()
                    .is_none_or(|value| run.platform_id == value)
                && filter
                    .account_id
                    .as_deref()
                    .is_none_or(|value| run.account_id == value)
        })
        .collect::<Vec<_>>();
    let sync_error_count = sync_runs
        .iter()
        .filter(|run| run.status != "SUCCESS")
        .count();
    Ok(OperationsDashboard {
        platform_count,
        account_count,
        recent_publish_count: recent_publishes.len(),
        pending_task_count: pending_tasks.len(),
        sync_error_count,
        recent_works: recent_works.clone(),
        pending_tasks,
        recent_publishes,
        sync_runs,
        filter: DashboardFilterApplied {
            range: range.to_string(),
            platform_id: filter.platform_id.clone(),
            account_id: filter.account_id.clone(),
            content_kind: filter.content_kind.clone(),
            from: from.to_rfc3339(),
            to: to.to_rfc3339(),
        },
    })
}
