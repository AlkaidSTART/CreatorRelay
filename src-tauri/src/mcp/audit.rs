use super::*;
use super::security::{redact_json, sensitive_key};

pub(super) fn summarize_arguments(arguments: &Value) -> String {
    let Some(object) = arguments.as_object() else {
        return "[non-object arguments]".to_string();
    };
    let mut parts = Vec::new();
    for (key, value) in object {
        if key == "body" {
            let length = value.as_str().map(|text| text.chars().count()).unwrap_or(0);
            parts.push(format!("body=<{length} chars>"));
            continue;
        }
        if key == "media" {
            if let Some(items) = value.as_array() {
                let orders = items
                    .iter()
                    .filter_map(|item| item.get("order").and_then(Value::as_u64))
                    .map(|order| order.to_string())
                    .collect::<Vec<_>>();
                parts.push(format!(
                    "mediaCount={} order=[{}]",
                    items.len(),
                    orders.join(",")
                ));
            }
            continue;
        }
        if sensitive_key(key) {
            parts.push(format!("{key}=[REDACTED]"));
            continue;
        }
        let rendered = match value {
            Value::String(text) => {
                let redacted = redact_text(text);
                if redacted.chars().count() > 120 {
                    format!("{}…", redacted.chars().take(120).collect::<String>())
                } else {
                    redacted
                }
            }
            other => redact_json(other).to_string(),
        };
        if rendered.chars().count() > 160 {
            parts.push(format!(
                "{key}={}…",
                rendered.chars().take(160).collect::<String>()
            ));
        } else {
            parts.push(format!("{key}={rendered}"));
        }
    }
    parts.join(" · ")
}

pub(super) fn target_summary_for(tool_name: &str, arguments: &Value) -> String {
    match tool_name {
        "account_get_info" => {
            let account = arguments.get("accountId").and_then(Value::as_str);
            let platform = arguments.get("platformId").and_then(Value::as_str);
            match (platform, account) {
                (Some(platform), Some(account)) => format!("平台 {platform} / 账号 {account}"),
                (Some(platform), None) => format!("平台 {platform} 的全部账号"),
                (None, Some(account)) => format!("账号 {account}"),
                (None, None) => "全部脱敏账号状态".to_string(),
            }
        }
        "platform_capabilities_get" => {
            let platform = arguments.get("platformId").and_then(Value::as_str);
            let account = arguments.get("accountId").and_then(Value::as_str);
            match (platform, account) {
                (Some(platform), Some(account)) => format!("平台 {platform} / 账号 {account}"),
                (Some(platform), None) => format!("平台 {platform} 的能力矩阵"),
                (None, Some(account)) => format!("账号 {account} 的能力矩阵"),
                (None, None) => "全部平台与账号能力矩阵".to_string(),
            }
        }
        "content_get_detail" | "studio_update_content" | "content_delete_draft" => arguments
            .get("contentId")
            .and_then(Value::as_str)
            .map(|id| format!("内容 {id}"))
            .unwrap_or_else(|| "未指定内容".to_string()),
        "studio_create_content" => {
            let kind = arguments
                .get("contentKind")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let title = arguments
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("未命名");
            format!("新建 {kind}：{title}")
        }
        "publishing_create_task" => {
            let count = arguments
                .get("targets")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            format!("创建 {count} 个独立发布任务")
        }
        "publishing_publish_task" => {
            let count = arguments
                .get("taskIds")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            format!("执行 {count} 个发布目标")
        }
        "analytics_get_trend" => format!(
            "{} 趋势 / 粒度 {}",
            arguments
                .get("range")
                .and_then(Value::as_str)
                .unwrap_or("7d"),
            arguments
                .get("granularity")
                .and_then(Value::as_str)
                .unwrap_or("day")
        ),
        _ => "本地测试 MCP Server".to_string(),
    }
}

pub(super) fn content_kind_from_arguments(arguments: &Value) -> Option<String> {
    arguments
        .get("contentKind")
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub(super) fn insert_audit_start(
    conn: &Connection,
    call_id: &str,
    thread_id: Option<&str>,
    tool: &McpToolDefinition,
    arguments: &Value,
) -> Result<String, String> {
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO mcp_tool_audit(
            id, thread_id, call_id, server_id, tool_name, permission, content_kind,
            target_summary, parameter_summary, status, started_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'running', ?10)",
        params![
            id,
            thread_id,
            call_id,
            SERVER_ID,
            tool.name,
            tool.permission.as_str(),
            content_kind_from_arguments(arguments),
            target_summary_for(&tool.name, arguments),
            summarize_arguments(arguments),
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| format!("工具调用审计写入失败：{error}"))?;
    Ok(id)
}

pub(super) fn finish_audit(
    conn: &Connection,
    audit_id: &str,
    status: &str,
    error_code: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE mcp_tool_audit
         SET status=?1, error_code=?2, error_message=?3, completed_at=?4
         WHERE id=?5",
        params![
            status,
            error_code,
            error_message.map(redact_text),
            Utc::now().to_rfc3339(),
            audit_id
        ],
    )
    .map_err(|error| format!("工具调用审计完成状态写入失败：{error}"))?;
    Ok(())
}

pub fn list_audit(conn: &Connection, limit: usize) -> Result<Vec<McpAuditRecord>, String> {
    let limit = limit.clamp(1, 200) as i64;
    let mut statement = conn
        .prepare(
            "SELECT id, thread_id, call_id, server_id, tool_name, permission, content_kind,
                    target_summary, parameter_summary, status, error_code, error_message,
                    started_at, completed_at
             FROM mcp_tool_audit ORDER BY started_at DESC LIMIT ?1",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([limit], |row| {
            let permission: String = row.get(5)?;
            Ok(McpAuditRecord {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                call_id: row.get(2)?,
                server_id: row.get(3)?,
                tool_name: row.get(4)?,
                permission: ToolPermission::parse(&permission).unwrap_or(ToolPermission::Read),
                content_kind: row.get(6)?,
                target_summary: row.get(7)?,
                parameter_summary: row.get(8)?,
                status: row.get(9)?,
                error_code: row.get(10)?,
                error_message: row.get(11)?,
                started_at: row.get(12)?,
                completed_at: row.get(13)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

