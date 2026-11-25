use super::*;

fn validate_publish_targets(
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

fn publish_preview_details(conn: &Connection, arguments: &Value) -> Result<Value, McpCallError> {
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

