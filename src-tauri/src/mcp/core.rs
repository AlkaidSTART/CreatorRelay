use super::assistant::append_assistant_message;
use super::audit::{
    content_kind_from_arguments, finish_audit, insert_audit_start, summarize_arguments,
    target_summary_for,
};
use super::authorization::{consume_authorization, issue_authorization};
use super::content_tools::field_error;
use super::outcome::CoreToolExecution;
use super::schema::{normalized_timeout, validate_tool_arguments};
use super::security::{tool_by_name, truncate_json};
use super::*;

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
    if matches!(
        tool_name,
        "account_get_info" | "platform_capabilities_get" | "creator_centers_list"
    ) {
        return account_tools::execute_account_tool(conn, tool_name, arguments);
    }
    if matches!(
        tool_name,
        "content_list"
            | "content_get_detail"
            | "studio_create_content"
            | "studio_update_content"
            | "content_delete_draft"
    ) {
        return content_tools::execute_content_tool(workspace, tool_name, arguments, scope);
    }
    if matches!(tool_name, "external_content_list" | "analytics_get_trend") {
        return analytics_tools::execute_analytics_tool(conn, tool_name, arguments);
    }
    if matches!(
        tool_name,
        "publishing_create_task" | "publishing_publish_task"
    ) {
        return publishing_tools::execute_publishing_tool(
            conn, runtime, workspace, tool_name, arguments, cancel,
        );
    }
    if super::platform_mcp::find_platform_tool(tool_name).is_some() {
        return super::platform_mcp::execute_platform_tool(conn, workspace, tool_name, arguments);
    }
    Err(McpCallError {
        code: "TOOL_NOT_FOUND".to_string(),
        message: format!("未注册工具：{tool_name}"),
        field_errors: Vec::new(),
    })
}

fn preview_details(
    conn: &Connection,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, Vec<McpFieldError>> {
    match tool_name {
        "studio_create_content"
        | "studio_update_content"
        | "content_delete_draft"
        | "content_get_detail" => {
            content_tools::preview_content_tool(workspace, tool_name, arguments)
        }
        "publishing_create_task" | "publishing_publish_task" => {
            publishing_tools::preview_publishing_tool(conn, workspace, tool_name, arguments)
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
