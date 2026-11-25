use super::*;
use serde::{Deserialize, Serialize};

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
