use super::content_tools::field_error;
use super::outcome::{CoreToolExecution, success_result};
use super::*;

pub(super) fn validate_publish_targets(
    conn: &Connection,
    arguments: &Value,
) -> Result<Vec<PublishTargetInput>, McpCallError> {
    let targets = arguments
        .get("targets")
        .and_then(Value::as_array)
        .ok_or_else(|| McpCallError {
            code: "INVALID_ARGUMENT".to_string(),
            message: "缺少发布目标".to_string(),
            field_errors: vec![field_error("targets", "必须是数组")],
        })?;
    let accounts = platforms::list_accounts(conn).map_err(|error| McpCallError {
        code: "ACCOUNT_SERVICE_ERROR".to_string(),
        message: error,
        field_errors: Vec::new(),
    })?;
    let by_id = accounts
        .into_iter()
        .map(|account| (account.id.clone(), account))
        .collect::<HashMap<_, _>>();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        let platform_id = target
            .get("platformId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let platform_name = target
            .get("platformName")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let account_id = target
            .get("accountId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let account_alias = target
            .get("accountAlias")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let adapter_id = target
            .get("adapterId")
            .and_then(Value::as_str)
            .map(str::to_string);
        match by_id.get(account_id) {
            Some(account) if account.platform_id == platform_id => {}
            Some(_) => errors.push(field_error(
                &format!("targets[{index}].platformId"),
                "账号与平台不匹配",
            )),
            None => errors.push(field_error(
                &format!("targets[{index}].accountId"),
                "Core 中不存在该账号",
            )),
        }
        output.push(PublishTargetInput {
            platform_id: platform_id.to_string(),
            platform_name: platform_name.to_string(),
            account_id: account_id.to_string(),
            account_alias: account_alias.to_string(),
            adapter_id,
        });
    }
    if errors.is_empty() {
        Ok(output)
    } else {
        Err(McpCallError {
            code: "TARGET_VALIDATION_FAILED".to_string(),
            message: "发布目标校验失败".to_string(),
            field_errors: errors,
        })
    }
}

pub(super) fn publish_preview_details(
    conn: &Connection,
    arguments: &Value,
) -> Result<Value, McpCallError> {
    let task_ids = arguments
        .get("taskIds")
        .and_then(Value::as_array)
        .ok_or_else(|| McpCallError {
            code: "INVALID_ARGUMENT".to_string(),
            message: "缺少发布任务".to_string(),
            field_errors: vec![field_error("taskIds", "必须是数组")],
        })?;
    let mut targets = Vec::new();
    let mut errors = Vec::new();
    for (index, value) in task_ids.iter().enumerate() {
        let Some(task_id) = value.as_str() else {
            errors.push(field_error(
                &format!("taskIds[{index}]"),
                "任务 ID 必须是字符串",
            ));
            continue;
        };
        match operations::publish_task(conn, task_id) {
            Ok(task) => targets.push(json!({
                "taskId": task.id,
                "contentId": task.content_id,
                "contentVersion": task.content_version,
                "contentKind": task.content_kind.as_str(),
                "title": task.title,
                "body": task.body,
                "bodyPreview": task.body.chars().take(600).collect::<String>(),
                "bodyTruncated": task.body.chars().count() > 600,
                "platformId": task.platform_id,
                "platformName": task.platform_name,
                "accountId": task.account_id,
                "accountAlias": task.account_alias,
                "adapterId": task.adapter_id,
                "status": format!("{:?}", task.status).to_ascii_uppercase(),
                "scheduledAt": task.scheduled_at,
                "media": task.media.iter().map(|asset| json!({
                    "assetId": asset.id,
                    "kind": match asset.kind {
                        content::AssetKind::Video => "video",
                        content::AssetKind::Image => "image",
                    },
                    "originalName": asset.original_name,
                    "mimeType": asset.mime_type,
                    "byteSize": asset.byte_size,
                    "order": asset.order,
                    "caption": asset.caption,
                })).collect::<Vec<_>>(),
                "cover": task.cover.as_ref().map(|asset| json!({
                    "assetId": asset.id,
                    "originalName": asset.original_name,
                    "mimeType": asset.mime_type,
                    "byteSize": asset.byte_size,
                })),
            })),
            Err(error) => errors.push(field_error(&format!("taskIds[{index}]"), error)),
        }
    }
    if !errors.is_empty() {
        return Err(McpCallError {
            code: "PUBLISH_PREVIEW_FAILED".to_string(),
            message: "最终发布内容读取失败".to_string(),
            field_errors: errors,
        });
    }
    Ok(json!({ "targets": targets }))
}

pub(super) fn execute_publishing_tool(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
    cancel: &AtomicBool,
) -> Result<CoreToolExecution, McpCallError> {
    let result = match tool_name {
        "publishing_create_task" => {
            let targets = validate_publish_targets(conn, arguments)?;
            let source_type = match arguments
                .get("sourceType")
                .and_then(Value::as_str)
                .unwrap_or("content")
            {
                "content" => PublishSourceType::Content,
                "platformDraft" => PublishSourceType::PlatformDraft,
                other => {
                    return Err(McpCallError {
                        code: "INVALID_ARGUMENT".to_string(),
                        message: format!("未知发布来源类型：{other}"),
                        field_errors: vec![field_error("sourceType", "来源类型无效")],
                    });
                }
            };
            let result = operations::create_publish_tasks(
                conn,
                workspace,
                &CreatePublishTasksInput {
                    source_type,
                    source_id: arguments
                        .get("sourceId")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    targets,
                    scheduled_at: arguments
                        .get("scheduledAt")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                },
            )
            .map_err(|error| McpCallError {
                code: "PUBLISH_TASK_CREATE_FAILED".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
            success_result(json!({
                "tasks": result.tasks,
                "deduplicatedCount": result.deduplicated_count,
            }))
        }
        "publishing_publish_task" => {
            let task_ids = arguments
                .get("taskIds")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut targets = Vec::new();
            let mut success_count = 0;
            let mut failure_count = 0;
            for task_id in task_ids.iter().filter_map(Value::as_str) {
                if cancel.load(Ordering::SeqCst) {
                    targets.push(json!({
                        "taskId": task_id,
                        "status": "cancelled",
                        "message": "用户取消；未提交该目标"
                    }));
                    failure_count += 1;
                    continue;
                }
                let outcome = (|| {
                    let preview =
                        operations::prepare_publish_task(conn, runtime, workspace, task_id)
                            .map_err(|error| error.to_string())?;
                    if !preview.validation.ok {
                        return Err(format!(
                            "发布前校验失败：{}",
                            preview
                                .validation
                                .issues
                                .first()
                                .map(|issue| issue.message.clone())
                                .unwrap_or_else(|| "未知校验错误".to_string())
                        ));
                    }
                    let token = preview
                        .confirmation_token
                        .as_deref()
                        .ok_or_else(|| "缺少发布确认令牌".to_string())?;
                    let task =
                        operations::confirm_publish_task(conn, runtime, workspace, task_id, token)?;
                    let published = matches!(
                        task.status,
                        operations::PublishTaskStatus::Success
                            | operations::PublishTaskStatus::Scheduled
                    );
                    Ok::<_, String>(json!({
                        "taskId": task.id,
                        "status": format!("{:?}", task.status).to_ascii_uppercase(),
                        "published": published,
                        "externalId": task.external_id,
                        "externalUrl": task.external_url,
                        "errorCode": task.error_code,
                        "errorMessage": task.error_message,
                    }))
                })();
                match outcome {
                    Ok(value) => {
                        if value.get("published").and_then(Value::as_bool) == Some(true) {
                            success_count += 1;
                        } else {
                            failure_count += 1;
                        }
                        targets.push(value);
                    }
                    Err(message) => {
                        failure_count += 1;
                        targets.push(json!({
                            "taskId": task_id,
                            "status": "failed",
                            "errorCode": "PUBLISH_FAILED",
                            "errorMessage": redact_text(&message),
                        }));
                    }
                }
            }
            let status = if failure_count == 0 {
                "success"
            } else if success_count == 0 {
                "failed"
            } else {
                "partial"
            };
            CoreToolExecution {
                status: status.to_string(),
                result: json!({
                    "overallStatus": status,
                    "targets": targets,
                    "successCount": success_count,
                    "failureCount": failure_count,
                }),
            }
        }
        _ => {
            return Err(McpCallError {
                code: "TOOL_NOT_FOUND".to_string(),
                message: format!("未注册发布工具：{tool_name}"),
                field_errors: Vec::new(),
            });
        }
    };
    Ok(result)
}

pub(super) fn preview_publishing_tool(
    conn: &Connection,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, Vec<McpFieldError>> {
    match tool_name {
        "publishing_create_task" => {
            let targets =
                validate_publish_targets(conn, arguments).map_err(|error| error.field_errors)?;
            let source_id = arguments
                .get("sourceId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let source_type = arguments
                .get("sourceType")
                .and_then(Value::as_str)
                .unwrap_or("content");
            let source = if source_type == "platformDraft" {
                content::read_platform_draft(workspace, source_id)
                    .map(|draft| {
                        json!({
                            "sourceType": "platformDraft",
                            "sourceId": draft.id,
                            "contentId": draft.content_id,
                            "title": draft.title,
                            "media": draft.media,
                            "cover": draft.cover,
                        })
                    })
                    .map_err(|error| vec![field_error("sourceId", error)])?
            } else {
                content::read_content_package(workspace, source_id)
                    .map(|package| {
                        json!({
                            "sourceType": "content",
                            "sourceId": package.manifest.id,
                            "contentId": package.manifest.id,
                            "contentKind": package.manifest.kind.as_str(),
                            "title": package.manifest.title,
                            "body": package.body,
                            "bodyPreview": package.body.chars().take(600).collect::<String>(),
                            "media": package.manifest.media,
                            "cover": package.manifest.cover,
                        })
                    })
                    .map_err(|error| vec![field_error("sourceId", error)])?
            };
            let targets = targets
                .into_iter()
                .map(|target| {
                    json!({
                        "platformId": target.platform_id,
                        "platformName": target.platform_name,
                        "accountId": target.account_id,
                        "accountAlias": target.account_alias,
                        "adapterId": target.adapter_id,
                    })
                })
                .collect::<Vec<_>>();
            Ok(json!({ "source": source, "targets": targets }))
        }
        "publishing_publish_task" => {
            publish_preview_details(conn, arguments).map_err(|error| error.field_errors)
        }
        _ => Ok(arguments.clone()),
    }
}
