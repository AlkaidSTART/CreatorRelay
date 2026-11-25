use super::audit::{summarize_arguments, target_summary_for};
use super::security::tool_by_name;
use super::*;

fn provider_for_thread(
    conn: &Connection,
    provider_id: Option<&str>,
) -> Result<Option<AiProviderSummary>, String> {
    let providers = ai::list_providers(conn)?;
    Ok(if let Some(provider_id) = provider_id {
        providers
            .into_iter()
            .find(|provider| provider.id == provider_id)
    } else {
        providers
            .iter()
            .find(|provider| provider.is_default)
            .cloned()
            .or_else(|| providers.first().cloned())
    })
}

fn thread_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssistantThread> {
    Ok(AssistantThread {
        id: row.get(0)?,
        title: row.get(1)?,
        provider_id: row.get(2)?,
        provider_name: row.get(3)?,
        model: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn get_thread_row(conn: &Connection, thread_id: &str) -> Result<AssistantThread, String> {
    conn.query_row(
        "SELECT id, title, provider_id, provider_name, model, created_at, updated_at
         FROM assistant_threads WHERE id=?1",
        [thread_id],
        thread_from_row,
    )
    .optional()
    .map_err(|error| error.to_string())?
    .ok_or_else(|| "未找到助手对话".to_string())
}

pub fn list_assistant_threads(conn: &Connection) -> Result<Vec<AssistantThread>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, title, provider_id, provider_name, model, created_at, updated_at
             FROM assistant_threads ORDER BY updated_at DESC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], thread_from_row)
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

pub fn create_assistant_thread(
    conn: &Connection,
    title: Option<String>,
    provider_id: Option<String>,
) -> Result<AssistantThread, String> {
    let provider = provider_for_thread(conn, provider_id.as_deref())?;
    let now = Utc::now().to_rfc3339();
    let thread = AssistantThread {
        id: Uuid::new_v4().to_string(),
        title: title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.chars().take(80).collect())
            .unwrap_or_else(|| "新对话".to_string()),
        provider_id: provider.as_ref().map(|provider| provider.id.clone()),
        provider_name: provider
            .as_ref()
            .map(|provider| provider.name.clone())
            .unwrap_or_else(|| "本地编排器".to_string()),
        model: provider
            .as_ref()
            .map(|provider| provider.model.clone())
            .unwrap_or_else(|| "deterministic-v1".to_string()),
        created_at: now.clone(),
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO assistant_threads(
            id, title, provider_id, provider_name, model, created_at, updated_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            thread.id,
            thread.title,
            thread.provider_id,
            thread.provider_name,
            thread.model,
            thread.created_at,
            thread.updated_at
        ],
    )
    .map_err(|error| format!("助手对话创建失败：{error}"))?;
    get_thread_row(conn, &thread.id)
}

fn message_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssistantMessage> {
    let tool_call_json: Option<String> = row.get(6)?;
    let tool_call = tool_call_json
        .as_deref()
        .and_then(|value| serde_json::from_str::<McpToolCallView>(value).ok());
    Ok(AssistantMessage {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        role: row.get(2)?,
        content: row.get(3)?,
        provider_name: row.get(4)?,
        model: row.get(5)?,
        tool_call,
        result_status: row.get(7)?,
        error_message: row.get(8)?,
        created_at: row.get(9)?,
    })
}

pub fn get_assistant_thread(
    conn: &Connection,
    thread_id: &str,
) -> Result<AssistantThreadDetail, String> {
    let thread = get_thread_row(conn, thread_id)?;
    let mut statement = conn
        .prepare(
            "SELECT id, thread_id, role, content, provider_name, model, tool_call_json,
                    result_status, error_message, created_at
             FROM assistant_messages WHERE thread_id=?1 ORDER BY created_at ASC, id ASC",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([thread_id], message_from_row)
        .map_err(|error| error.to_string())?;
    let messages = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    Ok(AssistantThreadDetail { thread, messages })
}

pub(super) fn append_assistant_message(
    conn: &Connection,
    thread_id: &str,
    role: &str,
    content: &str,
    tool_call: Option<&McpToolCallView>,
    result_status: Option<&str>,
    error_message: Option<&str>,
) -> Result<AssistantMessage, String> {
    let thread = get_thread_row(conn, thread_id)?;
    let id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO assistant_messages(
            id, thread_id, role, content, provider_name, model, tool_call_json,
            result_status, error_message, created_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            thread_id,
            role,
            redact_text(content),
            if role == "assistant" {
                Some(thread.provider_name.clone())
            } else {
                None
            },
            if role == "assistant" {
                Some(thread.model.clone())
            } else {
                None
            },
            tool_call
                .map(|value| serde_json::to_string(value).map_err(|error| error.to_string()))
                .transpose()?,
            result_status,
            error_message.map(redact_text),
            created_at
        ],
    )
    .map_err(|error| format!("助手消息保存失败：{error}"))?;
    conn.execute(
        "UPDATE assistant_threads SET updated_at=?1 WHERE id=?2",
        params![Utc::now().to_rfc3339(), thread_id],
    )
    .map_err(|error| error.to_string())?;
    Ok(AssistantMessage {
        id,
        thread_id: thread_id.to_string(),
        role: role.to_string(),
        content: redact_text(content),
        provider_name: if role == "assistant" {
            Some(thread.provider_name)
        } else {
            None
        },
        model: if role == "assistant" {
            Some(thread.model)
        } else {
            None
        },
        tool_call: tool_call.cloned(),
        result_status: result_status.map(str::to_string),
        error_message: error_message.map(redact_text),
        created_at,
    })
}

fn plan_tool_for_prompt(prompt: &str) -> Option<(&'static str, Value, &'static str)> {
    let lower = prompt.to_lowercase();
    if lower.contains("超时") || lower.contains("挂起") || lower.contains("取消测试") {
        return Some((
            "mcp_test_hang",
            json!({ "milliseconds": 5000 }),
            "运行本地超时与取消测试，不会产生外部副作用",
        ));
    }
    if lower.contains("账号") || lower.contains("登录") || lower.contains("会话") {
        return Some((
            "account_get_info",
            json!({}),
            "通过 Core 账号服务读取脱敏账号与登录状态",
        ));
    }
    if lower.contains("作品") || lower.contains("已发布") {
        return Some((
            "external_content_list",
            json!({}),
            "查询已同步外部作品；指标保留平台原始口径",
        ));
    }
    if lower.contains("指标") || lower.contains("趋势") || lower.contains("表现") {
        return Some((
            "analytics_get_trend",
            json!({ "range": "7d", "granularity": "day" }),
            "按平台原始指标读取最近 7 天趋势，不补零也不跨口径聚合",
        ));
    }
    if lower.contains("草稿") || lower.contains("内容") || lower.contains("检索") {
        return Some((
            "content_list",
            json!({}),
            "检索本地内容包，不读取任意文件路径",
        ));
    }
    if lower.contains("回显") || lower.contains("mcp") {
        return Some((
            "mcp_test_echo",
            json!({ "message": "CreatorRelay MCP ready" }),
            "验证 MCP Server 生命周期与工具调用协议",
        ));
    }
    None
}

pub fn plan_assistant_message(
    conn: &Connection,
    prompt: &str,
    preferred_thread_id: Option<&str>,
) -> Result<AssistantPlan, String> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        return Err("消息不能为空".to_string());
    }
    let thread = if let Some(thread_id) = preferred_thread_id {
        get_thread_row(conn, thread_id)?
    } else {
        let existing = list_assistant_threads(conn)?.into_iter().next();
        existing.unwrap_or(create_assistant_thread(conn, None, None)?)
    };
    if thread.title == "新对话" {
        conn.execute(
            "UPDATE assistant_threads SET title=?1 WHERE id=?2",
            params![prompt.chars().take(40).collect::<String>(), thread.id],
        )
        .map_err(|error| error.to_string())?;
    }
    let user_message =
        append_assistant_message(conn, &thread.id, "user", prompt, None, None, None)?;
    let assistant_message = if let Some((tool_name, arguments, reason)) =
        plan_tool_for_prompt(prompt)
    {
        let tool = tool_by_name(tool_name).expect("planned tool exists");
        let sequence = Uuid::new_v4().to_string();
        let tool_call = McpToolCallView {
            call_id: format!("proposal-{sequence}"),
            tool_name: tool.name.clone(),
            permission: tool.permission,
            target_summary: target_summary_for(&tool.name, &arguments),
            parameter_summary: summarize_arguments(&arguments),
            arguments: Some(arguments.clone()),
            status: "proposed".to_string(),
            error_code: None,
            error_message: None,
        };
        let content = format!(
            "我建议调用 `{}`。{}\n\n执行前会先展示目标、参数摘要与权限；写操作和外部操作需要你确认。",
            tool.name, reason
        );
        append_assistant_message(
            conn,
            &thread.id,
            "assistant",
            &content,
            Some(&tool_call),
            Some("proposed"),
            None,
        )?;
        get_assistant_thread(conn, &thread.id)?
            .messages
            .into_iter()
            .last()
            .ok_or_else(|| "助手计划消息写入失败".to_string())?
    } else {
        append_assistant_message(
            conn,
            &thread.id,
            "assistant",
            "我没有匹配到可执行的本地工具。你可以要求查询账号、作品、指标、草稿，或运行 MCP 回显/超时自检。",
            None,
            Some("no_tool"),
            None,
        )?
    };
    Ok(AssistantPlan {
        thread: get_thread_row(conn, &thread.id)?,
        user_message,
        assistant_message,
    })
}

fn context_scope_from_setting(value: &str) -> ModelContextScope {
    match value {
        "title_only" => ModelContextScope {
            include_title: true,
            include_summary: false,
            include_body: false,
            include_media_metadata: false,
        },
        "title_summary" => ModelContextScope {
            include_title: true,
            include_summary: true,
            include_body: false,
            include_media_metadata: false,
        },
        "title_summary_media" => ModelContextScope::default(),
        _ => ModelContextScope::default(),
    }
}

pub fn preview_assistant_context(
    conn: &Connection,
    workspace: &Path,
    provider_id: Option<&str>,
    content_id: Option<&str>,
    scope: &ModelContextScope,
) -> Result<AssistantContextPreview, String> {
    let provider = provider_for_thread(conn, provider_id)?;
    let (provider_name, model) = provider
        .as_ref()
        .map(|provider| (provider.name.clone(), provider.model.clone()))
        .unwrap_or_else(|| ("本地编排器".to_string(), "deterministic-v1".to_string()));
    let mut included_fields = Vec::new();
    let mut excluded_fields = Vec::new();
    let mut parts = Vec::new();
    let mut content_kind = None;
    let mut redaction_count = 0;
    if let Some(content_id) = content_id {
        let package = content::read_content_package(workspace, content_id)?;
        content_kind = Some(package.manifest.kind.as_str().to_string());
        if scope.include_title {
            parts.push(format!("标题：{}", package.manifest.title));
            included_fields.push("title".to_string());
        } else {
            excluded_fields.push("title".to_string());
        }
        if scope.include_summary {
            if !package.manifest.summary.trim().is_empty() {
                parts.push(format!("摘要：{}", package.manifest.summary));
            }
            if !package.manifest.tags.is_empty() {
                parts.push(format!("标签：{}", package.manifest.tags.join("、")));
            }
            included_fields.push("summaryAndTags".to_string());
        } else {
            excluded_fields.push("summaryAndTags".to_string());
        }
        if scope.include_body && package.manifest.kind.requires_markdown() {
            parts.push(format!("正文：\n{}", package.body));
            included_fields.push("body".to_string());
        } else {
            excluded_fields.push("body".to_string());
        }
        if scope.include_media_metadata {
            if !package.manifest.media.is_empty() {
                let media = package
                    .manifest
                    .media
                    .iter()
                    .map(|asset| {
                        format!(
                            "{}. {} ({}, {} bytes){}{}",
                            asset.order,
                            asset.original_name,
                            asset.mime_type,
                            asset.byte_size,
                            if asset.caption.trim().is_empty() {
                                ""
                            } else {
                                "： "
                            },
                            asset.caption
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                parts.push(format!("素材元数据：\n{media}"));
            }
            included_fields.push("mediaMetadata".to_string());
        } else {
            excluded_fields.push("mediaMetadata".to_string());
        }
    }
    let raw = parts.join("\n\n");
    let context = redact_text(&raw);
    if context != raw {
        redaction_count += 1;
    }
    Ok(AssistantContextPreview {
        provider_name,
        model,
        content_kind,
        character_count: context.chars().count(),
        context,
        included_fields,
        excluded_fields,
        redaction_count,
    })
}

pub fn generate_assistant_reply(
    conn: &Connection,
    workspace: &Path,
    request: AssistantGenerateRequest,
    vault: &dyn CredentialVault,
) -> Result<AssistantGenerateResult, String> {
    let thread = get_thread_row(conn, &request.thread_id)?;
    let user_message = if let Some(prompt) = request
        .prompt
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(append_assistant_message(
            conn, &thread.id, "user", prompt, None, None, None,
        )?)
    } else {
        None
    };
    let provider = provider_for_thread(
        conn,
        request
            .provider_id
            .as_deref()
            .or(thread.provider_id.as_deref()),
    )?;
    let mut preview = preview_assistant_context(
        conn,
        workspace,
        provider.as_ref().map(|provider| provider.id.as_str()),
        request.content_id.as_deref(),
        &request.scope,
    )?;
    let prompt = request.prompt.as_deref().unwrap_or_default().trim();
    if !prompt.is_empty() {
        preview.context = format!("用户请求：{prompt}\n\n{}", preview.context);
        preview.character_count = preview.context.chars().count();
    }
    let Some(provider) = provider else {
        conn.execute(
            "UPDATE assistant_threads
             SET provider_id=NULL, provider_name='本地编排器', model='deterministic-v1', updated_at=?1
             WHERE id=?2",
            params![Utc::now().to_rfc3339(), thread.id],
        )
        .map_err(|error| error.to_string())?;
        let assistant = append_assistant_message(
            conn,
            &thread.id,
            "assistant",
            "尚未配置 BYOK 模型供应商。你可以继续使用本地 MCP 工具，或前往系统设置配置模型；本地工具调用不会把上下文发送到云端。",
            None,
            Some("no_provider"),
            None,
        )?;
        return Ok(AssistantGenerateResult {
            user_message,
            assistant_message: assistant,
            context_preview: preview,
        });
    };
    conn.execute(
        "UPDATE assistant_threads
         SET provider_id=?1, provider_name=?2, model=?3, updated_at=?4
         WHERE id=?5",
        params![
            provider.id,
            provider.name,
            provider.model,
            Utc::now().to_rfc3339(),
            thread.id
        ],
    )
    .map_err(|error| error.to_string())?;
    let package_kind = request
        .content_id
        .as_deref()
        .and_then(|content_id| content::read_content_package(workspace, content_id).ok())
        .map(|package| package.manifest.kind)
        .unwrap_or(ContentKind::Article);
    let request_title = request
        .content_id
        .as_deref()
        .filter(|_| request.scope.include_title)
        .and_then(|content_id| content::read_content_package(workspace, content_id).ok())
        .map(|package| package.manifest.title)
        .unwrap_or_default();
    let media_metadata = if request.scope.include_media_metadata {
        request
            .content_id
            .as_deref()
            .and_then(|content_id| content::read_content_package(workspace, content_id).ok())
            .map(|package| {
                package
                    .manifest
                    .media
                    .into_iter()
                    .map(|asset| AiMediaMetadata {
                        kind: match asset.kind {
                            content::AssetKind::Video => "video".to_string(),
                            content::AssetKind::Image => "image".to_string(),
                        },
                        file_name: asset.original_name,
                        mime_type: asset.mime_type,
                        byte_size: asset.byte_size,
                        caption: asset.caption,
                        order: asset.order,
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let ai_request = AiGenerateRequest {
        provider_id: provider.id.clone(),
        operation: "custom".to_string(),
        content_kind: package_kind,
        title: request_title,
        context: preview.context.clone(),
        media_metadata,
        instruction: request.instruction.clone(),
    };
    match ai::generate_with_vault(conn, &ai_request, vault) {
        Ok(result) => {
            let assistant = append_assistant_message(
                conn,
                &thread.id,
                "assistant",
                &result.content,
                None,
                Some("success"),
                None,
            )?;
            Ok(AssistantGenerateResult {
                user_message,
                assistant_message: assistant,
                context_preview: preview,
            })
        }
        Err(error) => {
            let assistant = append_assistant_message(
                conn,
                &thread.id,
                "assistant",
                "模型请求失败；未自动重试，也未把失败伪装为成功。",
                None,
                Some("failed"),
                Some(&error),
            )?;
            Ok(AssistantGenerateResult {
                user_message,
                assistant_message: assistant,
                context_preview: preview,
            })
        }
    }
}

pub fn default_context_scope(conn: &Connection) -> Result<ModelContextScope, String> {
    Ok(context_scope_from_setting(
        &get_settings(conn)?.default_context_scope,
    ))
}
