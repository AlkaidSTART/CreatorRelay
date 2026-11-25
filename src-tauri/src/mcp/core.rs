use super::*;

struct CoreToolExecution {
    status: String,
    result: Value,
}

fn success_result(result: Value) -> CoreToolExecution {
    CoreToolExecution {
        status: "success".to_string(),
        result,
    }
}


fn execute_core_tool(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
    scope: &ModelContextScope,
    cancel: &AtomicBool,
) -> Result<CoreToolExecution, McpCallError> {
    if cancel.load(Ordering::SeqCst) {
        return Err(McpCallError {
            code: "TOOL_CANCELLED".to_string(),
            message: "工具调用已取消，未执行剩余副作用".to_string(),
            field_errors: Vec::new(),
        });
    }
    let result = match tool_name {
        "account_get_info" => {
            let accounts = platforms::list_accounts(conn).map_err(|error| McpCallError {
                code: "ACCOUNT_SERVICE_ERROR".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
            let account_id = arguments.get("accountId").and_then(Value::as_str);
            let platform_id = arguments.get("platformId").and_then(Value::as_str);
            let views = accounts
                .iter()
                .filter(|account| {
                    account_id.is_none_or(|value| account.id == value)
                        && platform_id.is_none_or(|value| account.platform_id == value)
                })
                .map(account_view)
                .collect::<Vec<_>>();
            success_result(json!({ "accounts": views, "count": views.len() }))
        }
        "platform_capabilities_get" => {
            let platform_id = arguments.get("platformId").and_then(Value::as_str);
            let account_id = arguments.get("accountId").and_then(Value::as_str);
            let admissions = ecosystem::platform_admissions()
                .map_err(|error| McpCallError {
                    code: "ECOSYSTEM_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?
                .into_iter()
                .filter(|admission| platform_id.is_none_or(|value| admission.platform_id == value))
                .collect::<Vec<_>>();
            let accounts = ecosystem::list_account_capability_snapshots(conn)
                .map_err(|error| McpCallError {
                    code: "ECOSYSTEM_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?
                .into_iter()
                .filter(|account| {
                    platform_id.is_none_or(|value| account.platform_id == value)
                        && account_id.is_none_or(|value| account.account_id == value)
                })
                .collect::<Vec<_>>();
            success_result(json!({
                "platforms": admissions,
                "accounts": accounts,
                "platformCount": admissions.len(),
                "accountCount": accounts.len(),
                "autoPublishDefault": "manual_only"
            }))
        }
        "content_list" => {
            let packages =
                content::list_content_packages(workspace).map_err(|error| McpCallError {
                    code: "CONTENT_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?;
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase();
            let kind = arguments.get("contentKind").and_then(Value::as_str);
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(20)
                .min(100) as usize;
            let items = packages
                .into_iter()
                .filter(|package| {
                    kind.is_none_or(|value| package.kind.as_str() == value)
                        && (query.is_empty()
                            || format!(
                                "{} {} {}",
                                package.title,
                                package.summary,
                                package.tags.join(" ")
                            )
                            .to_lowercase()
                            .contains(&query))
                })
                .take(limit)
                .map(|package| {
                    json!({
                        "contentId": package.id,
                        "contentKind": package.kind.as_str(),
                        "title": package.title,
                        "summary": package.summary,
                        "tags": package.tags,
                        "updatedAt": package.updated_at,
                        "hasCover": package.has_cover,
                        "mediaCount": package.media_count,
                        "contentVersion": package.content_version,
                    })
                })
                .collect::<Vec<_>>();
            success_result(json!({ "items": items, "count": items.len() }))
        }
        "content_get_detail" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let package =
                content::read_content_package(workspace, content_id).map_err(|error| {
                    McpCallError {
                        code: "CONTENT_NOT_FOUND".to_string(),
                        message: error,
                        field_errors: vec![field_error("contentId", "内容无法读取")],
                    }
                })?;
            let mut detail = json!({
                "contentId": package.manifest.id,
                "contentKind": package.manifest.kind.as_str(),
                "title": if scope.include_title { Value::String(package.manifest.title.clone()) } else { Value::Null },
                "summary": if scope.include_summary { Value::String(package.manifest.summary.clone()) } else { Value::Null },
                "tags": package.manifest.tags,
                "updatedAt": package.manifest.updated_at,
                "contentVersion": package.content_version,
                "bodyIncluded": scope.include_body && package.manifest.kind.requires_markdown(),
                "media": if scope.include_media_metadata { package.manifest.media.iter().map(|asset| json!({
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
                })).collect::<Vec<_>>() } else { Vec::new() },
            });
            if scope.include_body && package.manifest.kind.requires_markdown() {
                detail["body"] = Value::String(redact_text(&package.body));
            }
            success_result(detail)
        }
        "external_content_list" => {
            let works = operations::list_external_works(
                conn,
                &ExternalWorkFilter {
                    keyword: arguments
                        .get("keyword")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    platform_id: arguments
                        .get("platformId")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    account_id: arguments
                        .get("accountId")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    content_kind: arguments
                        .get("contentKind")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    published_from: arguments
                        .get("publishedFrom")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    published_to: arguments
                        .get("publishedTo")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                },
            )
            .map_err(|error| McpCallError {
                code: "CONTENT_SERVICE_ERROR".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
            success_result(json!({ "items": works, "count": works.len() }))
        }
        "analytics_get_trend" => {
            let query = parse_analytics_query(arguments)?;
            let result =
                operations::analytics_trend(conn, &query).map_err(|error| McpCallError {
                    code: "ANALYTICS_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?;
            success_result(serde_json::to_value(result).map_err(|error| McpCallError {
                code: "ANALYTICS_SERIALIZE_ERROR".to_string(),
                message: error.to_string(),
                field_errors: Vec::new(),
            })?)
        }
        "studio_create_content" => {
            let payload = prepare_create_content(workspace, arguments).map_err(|field_errors| {
                McpCallError {
                    code: "CONTENT_VALIDATION_FAILED".to_string(),
                    message: "内容载荷校验失败，未创建草稿".to_string(),
                    field_errors,
                }
            })?;
            let result = execute_prepared_content(workspace, &payload)?;
            success_result(result)
        }
        "studio_update_content" => {
            let payload = prepare_update_content(workspace, arguments).map_err(|field_errors| {
                McpCallError {
                    code: "CONTENT_VALIDATION_FAILED".to_string(),
                    message: "内容载荷校验失败，未写入草稿".to_string(),
                    field_errors,
                }
            })?;
            let result = execute_prepared_content(workspace, &payload)?;
            success_result(result)
        }
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
        "content_delete_draft" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            content::delete_content_package(workspace, content_id).map_err(|error| {
                McpCallError {
                    code: "CONTENT_DELETE_FAILED".to_string(),
                    message: error,
                    field_errors: vec![field_error("contentId", "内容删除失败")],
                }
            })?;
            success_result(json!({ "contentId": content_id, "deleted": true }))
        }
        _ => {
            return Err(McpCallError {
                code: "TOOL_NOT_FOUND".to_string(),
                message: format!("未注册工具：{tool_name}"),
                field_errors: Vec::new(),
            });
        }
    };
    Ok(result)
}

fn preview_details(
    conn: &Connection,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, Vec<McpFieldError>> {
    match tool_name {
        "studio_create_content" => prepare_create_content(workspace, arguments)
            .map(|payload| content_preview_details(&payload)),
        "studio_update_content" => prepare_update_content(workspace, arguments)
            .map(|payload| content_preview_details(&payload)),
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
        "content_delete_draft" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            content::read_content_package(workspace, content_id)
                .map(|package| {
                    json!({
                        "contentId": package.manifest.id,
                        "contentKind": package.manifest.kind.as_str(),
                        "title": package.manifest.title,
                        "media": package.manifest.media,
                        "hasCover": package.manifest.cover.is_some(),
                    })
                })
                .map_err(|error| vec![field_error("contentId", error)])
        }
        "content_get_detail" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            content::read_content_package(workspace, content_id)
                .map(|package| {
                    json!({
                        "contentId": package.manifest.id,
                        "contentKind": package.manifest.kind.as_str(),
                        "title": package.manifest.title,
                        "mediaCount": package.manifest.media.len(),
                    })
                })
                .map_err(|error| vec![field_error("contentId", error)])
        }
        "mcp_test_hang" => Ok(json!({
            "simulation": "Hang until timeout or user cancellation",
            "milliseconds": arguments.get("milliseconds").and_then(Value::as_u64).unwrap_or(5000)
        })),
        _ => Ok(arguments.clone()),
    }
}

pub fn preview_tool_call(
    conn: &Connection,
    workspace: &Path,
    request: McpPreviewRequest,
) -> Result<McpAuthorizationPreview, String> {
    let tool = tool_by_name(&request.tool_name)
        .ok_or_else(|| format!("未注册工具：{}", request.tool_name))?;
    let settings = get_settings(conn)?;
    let call_id = Uuid::new_v4().to_string();
    let mut field_errors = validate_tool_arguments(&tool, &request.arguments);
    if tool.permission == ToolPermission::Read && !settings.read_enabled {
        field_errors.push(field_error("permission", "用户尚未启用只读工具权限"));
    }
    let details = if field_errors.is_empty() {
        match preview_details(conn, workspace, &tool.name, &request.arguments) {
            Ok(value) => value,
            Err(mut errors) => {
                field_errors.append(&mut errors);
                json!({})
            }
        }
    } else {
        json!({})
    };
    let requires_confirmation = tool.permission.requires_confirmation(&settings);
    let authorization = if field_errors.is_empty() && requires_confirmation {
        let (token, expires_at) = issue_authorization(
            conn,
            &call_id,
            &tool.name,
            tool.permission,
            &request.arguments,
        )?;
        (Some(token), Some(expires_at))
    } else {
        (None, None)
    };
    let effects = match tool.permission {
        ToolPermission::Read => vec!["读取本地 Core 数据；不产生外部平台请求".to_string()],
        ToolPermission::Write => vec![
            "写入本地内容包、发布任务或删除本地草稿".to_string(),
            "不直接访问 SQLite、凭据引用或任意文件路径".to_string(),
        ],
        ToolPermission::ExternalAction => vec![
            "向外部平台发出发布请求".to_string(),
            "每个目标独立执行，部分失败不会回滚已完成目标".to_string(),
            "确认令牌仅可使用一次；结果不确定时不会自动重试".to_string(),
        ],
    };
    Ok(McpAuthorizationPreview {
        call_id,
        tool_name: tool.name.clone(),
        title: tool.title,
        permission: tool.permission,
        content_kind: content_kind_from_arguments(&request.arguments),
        target_summary: target_summary_for(&tool.name, &request.arguments),
        parameter_summary: summarize_arguments(&request.arguments),
        effects,
        details,
        requires_confirmation,
        authorization_token: authorization.0,
        confirmation_expires_at: authorization.1,
        field_errors,
    })
}

#[allow(clippy::too_many_arguments)]
fn tool_call_error(
    call_id: String,
    audit_id: String,
    tool: &McpToolDefinition,
    arguments: &Value,
    code: &str,
    message: impl Into<String>,
    field_errors: Vec<McpFieldError>,
    duration_ms: u128,
) -> McpCallResult {
    McpCallResult {
        call_id,
        audit_id,
        tool_name: tool.name.clone(),
        permission: tool.permission,
        status: "failed".to_string(),
        content_kind: content_kind_from_arguments(arguments),
        target_summary: target_summary_for(&tool.name, arguments),
        parameter_summary: summarize_arguments(arguments),
        result: None,
        error: Some(McpCallError {
            code: code.to_string(),
            message: message.into(),
            field_errors,
        }),
        duration_ms,
    }
}

fn call_view_from_result(result: &McpCallResult) -> McpToolCallView {
    McpToolCallView {
        call_id: result.call_id.clone(),
        tool_name: result.tool_name.clone(),
        permission: result.permission,
        target_summary: result.target_summary.clone(),
        parameter_summary: result.parameter_summary.clone(),
        arguments: None,
        status: result.status.clone(),
        error_code: result.error.as_ref().map(|error| error.code.clone()),
        error_message: result.error.as_ref().map(|error| error.message.clone()),
    }
}

fn server_forwarded_tool_response(response: &Value, tool_name: &str) -> Result<(), String> {
    if let Some(error) = response.get("error") {
        return Err(error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("MCP Server 返回错误")
            .to_string());
    }
    let structured = response
        .pointer("/result/structuredContent")
        .ok_or_else(|| "MCP Server 未返回结构化转发结果".to_string())?;
    if structured.get("forwardToHost").and_then(Value::as_bool) != Some(true) {
        return Err("MCP Server 返回了不受支持的执行模式".to_string());
    }
    if structured.get("tool").and_then(Value::as_str) != Some(tool_name) {
        return Err("MCP Server 返回的工具名与请求不一致".to_string());
    }
    Ok(())
}

pub fn call_tool(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
    host: &McpHost,
    request: McpCallRequest,
) -> Result<McpCallResult, String> {
    let started = Instant::now();
    let tool = tool_by_name(&request.tool_name)
        .ok_or_else(|| format!("未注册工具：{}", request.tool_name))?;
    let call_id = request
        .call_id
        .as_deref()
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
        .map(str::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let audit_id = insert_audit_start(
        conn,
        &call_id,
        request.thread_id.as_deref(),
        &tool,
        &request.arguments,
    )?;
    let settings = get_settings(conn)?;
    let field_errors = validate_tool_arguments(&tool, &request.arguments);
    if !field_errors.is_empty() {
        let result = tool_call_error(
            call_id,
            audit_id.clone(),
            &tool,
            &request.arguments,
            "INVALID_ARGUMENT",
            "工具参数未通过 JSON Schema 校验，未执行任何副作用",
            field_errors,
            started.elapsed().as_millis(),
        );
        finish_audit(
            conn,
            &audit_id,
            "rejected",
            result.error.as_ref().map(|error| error.code.as_str()),
            result.error.as_ref().map(|error| error.message.as_str()),
        )?;
        return Ok(result);
    }
    if let Err(business_errors) = preview_details(conn, workspace, &tool.name, &request.arguments)
        && !business_errors.is_empty()
    {
        let result = tool_call_error(
            call_id,
            audit_id.clone(),
            &tool,
            &request.arguments,
            "INVALID_ARGUMENT",
            "工具业务载荷校验失败，未执行任何本地写入或外部请求",
            business_errors,
            started.elapsed().as_millis(),
        );
        finish_audit(
            conn,
            &audit_id,
            "rejected",
            result.error.as_ref().map(|error| error.code.as_str()),
            result.error.as_ref().map(|error| error.message.as_str()),
        )?;
        return Ok(result);
    }
    if tool.permission == ToolPermission::Read && !settings.read_enabled {
        let result = tool_call_error(
            call_id,
            audit_id.clone(),
            &tool,
            &request.arguments,
            "READ_PERMISSION_DISABLED",
            "用户尚未启用只读工具权限",
            vec![field_error("permission", "请在 MCP 设置中启用只读工具")],
            started.elapsed().as_millis(),
        );
        finish_audit(
            conn,
            &audit_id,
            "rejected",
            Some("READ_PERMISSION_DISABLED"),
            result.error.as_ref().map(|error| error.message.as_str()),
        )?;
        return Ok(result);
    }
    if tool.permission.requires_confirmation(&settings) {
        let Some(token) = request.authorization_token.as_deref() else {
            let result = tool_call_error(
                call_id,
                audit_id.clone(),
                &tool,
                &request.arguments,
                "AUTHORIZATION_REQUIRED",
                "该工具需要用户明确确认，请先预览最终目标与参数",
                Vec::new(),
                started.elapsed().as_millis(),
            );
            finish_audit(
                conn,
                &audit_id,
                "rejected",
                Some("AUTHORIZATION_REQUIRED"),
                result.error.as_ref().map(|error| error.message.as_str()),
            )?;
            return Ok(result);
        };
        if let Err(error) =
            consume_authorization(conn, token, &tool.name, tool.permission, &request.arguments)
        {
            let result = tool_call_error(
                call_id,
                audit_id.clone(),
                &tool,
                &request.arguments,
                &error.code,
                error.message,
                error.field_errors,
                started.elapsed().as_millis(),
            );
            finish_audit(
                conn,
                &audit_id,
                "rejected",
                result.error.as_ref().map(|error| error.code.as_str()),
                result.error.as_ref().map(|error| error.message.as_str()),
            )?;
            return Ok(result);
        }
    }

    let cancel = host.register_cancel(&call_id);
    let timeout = StdDuration::from_millis(normalized_timeout(request.timeout_ms));
    let scope = request.context_scope.clone().unwrap_or_default();
    let outcome = (|| -> Result<(String, Value), McpCallError> {
        if tool.name == "mcp_test_hang" {
            let milliseconds = request
                .arguments
                .get("milliseconds")
                .and_then(Value::as_u64)
                .unwrap_or(5_000)
                .min(timeout.as_millis() as u64 + 1_000);
            let hang_started = Instant::now();
            loop {
                if cancel.load(Ordering::SeqCst) {
                    return Err(McpCallError {
                        code: "TOOL_CANCELLED".to_string(),
                        message: "用户已取消挂起的工具调用；未执行重复请求".to_string(),
                        field_errors: Vec::new(),
                    });
                }
                if hang_started.elapsed() >= timeout {
                    return Err(McpCallError {
                        code: "TOOL_TIMEOUT".to_string(),
                        message: "工具调用超时；系统未自动重复执行".to_string(),
                        field_errors: Vec::new(),
                    });
                }
                if hang_started.elapsed() >= StdDuration::from_millis(milliseconds) {
                    break;
                }
                thread::sleep(StdDuration::from_millis(25));
            }
            return Ok((
                "success".to_string(),
                json!({
                    "simulatedMilliseconds": milliseconds,
                    "cancelled": false,
                    "timedOut": false
                }),
            ));
        }
        let server_response = host.call_server(&request).map_err(|message| McpCallError {
            code: if message.contains("超时") {
                "TOOL_TIMEOUT".to_string()
            } else {
                "MCP_SERVER_ERROR".to_string()
            },
            message: format!("{message}；系统未自动重复执行"),
            field_errors: Vec::new(),
        })?;
        if tool.name == "mcp_test_echo" {
            if let Some(error) = server_response.get("error") {
                return Err(McpCallError {
                    code: "MCP_TOOL_ERROR".to_string(),
                    message: error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("MCP 回显工具失败")
                        .to_string(),
                    field_errors: Vec::new(),
                });
            }
            return Ok((
                "success".to_string(),
                server_response
                    .get("result")
                    .cloned()
                    .unwrap_or(Value::Null),
            ));
        }
        server_forwarded_tool_response(&server_response, &tool.name).map_err(|message| {
            McpCallError {
                code: "MCP_SERVER_CONTRACT_ERROR".to_string(),
                message,
                field_errors: Vec::new(),
            }
        })?;
        let execution = execute_core_tool(
            conn,
            runtime,
            workspace,
            &tool.name,
            &request.arguments,
            &scope,
            &cancel,
        )?;
        Ok((execution.status, execution.result))
    })();
    host.finish_cancel(&call_id);

    let mut result = match outcome {
        Ok((status, value)) => McpCallResult {
            call_id: call_id.clone(),
            audit_id: audit_id.clone(),
            tool_name: tool.name.clone(),
            permission: tool.permission,
            status,
            content_kind: content_kind_from_arguments(&request.arguments),
            target_summary: target_summary_for(&tool.name, &request.arguments),
            parameter_summary: summarize_arguments(&request.arguments),
            result: Some(value),
            error: None,
            duration_ms: started.elapsed().as_millis(),
        },
        Err(error) => {
            let status = match error.code.as_str() {
                "TOOL_CANCELLED" => "cancelled",
                "TOOL_TIMEOUT" => "timedOut",
                _ => "failed",
            };
            let mut result = tool_call_error(
                call_id.clone(),
                audit_id.clone(),
                &tool,
                &request.arguments,
                &error.code,
                error.message,
                error.field_errors,
                started.elapsed().as_millis(),
            );
            result.status = status.to_string();
            result
        }
    };
    result.result = result
        .result
        .map(|value| truncate_json(value, MAX_TOOL_RESULT_CHARS));
    finish_audit(
        conn,
        &audit_id,
        &result.status,
        result.error.as_ref().map(|error| error.code.as_str()),
        result.error.as_ref().map(|error| error.message.as_str()),
    )?;
    if let Some(thread_id) = request.thread_id.as_deref() {
        let view = call_view_from_result(&result);
        let content = match result.status.as_str() {
            "success" => format!(
                "工具 {} 已完成。目标：{}",
                result.tool_name, result.target_summary
            ),
            "partial" => format!(
                "工具 {} 部分完成。目标：{}；请查看逐目标结果。",
                result.tool_name, result.target_summary
            ),
            "cancelled" => format!("工具 {} 已取消，未继续重复执行。", result.tool_name),
            "timedOut" => format!("工具 {} 超时，未自动重复执行。", result.tool_name),
            _ => format!(
                "工具 {} 失败：{}",
                result.tool_name,
                result
                    .error
                    .as_ref()
                    .map(|error| error.message.as_str())
                    .unwrap_or("未知错误")
            ),
        };
        append_assistant_message(
            conn,
            thread_id,
            "assistant",
            &content,
            Some(&view),
            Some(&result.status),
            result.error.as_ref().map(|error| error.message.as_str()),
        )?;
    }
    Ok(result)
}

