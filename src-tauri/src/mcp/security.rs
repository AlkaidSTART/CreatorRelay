use super::*;

pub(super) fn tool_by_name(name: &str) -> Option<McpToolDefinition> {
    tool_catalog().into_iter().find(|tool| tool.name == name)
}

pub(super) fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

pub(super) fn payload_hash(tool: &str, arguments: &Value) -> Result<String, String> {
    Ok(digest(&format!(
        "{tool}:{}",
        serde_json::to_string(arguments).map_err(|error| error.to_string())?
    )))
}

pub(super) fn sensitive_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    [
        "apikey",
        "api_key",
        "authorization",
        "cookie",
        "credential",
        "password",
        "secret",
        "token",
        "session",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(super) fn redact_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    if sensitive_key(key) {
                        (key.clone(), json!("[REDACTED]"))
                    } else {
                        (key.clone(), redact_json(value))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(redact_json).collect()),
        Value::String(text) => Value::String(redact_text(text)),
        other => other.clone(),
    }
}

pub fn redact_text(text: &str) -> String {
    let mut value = text.to_string();
    for key in [
        "api_key",
        "apiKey",
        "access_token",
        "refresh_token",
        "password",
        "cookie",
        "authorization",
    ] {
        let mut search_from = 0;
        while let Some(relative) = value[search_from..].find(key) {
            let start = search_from + relative;
            let tail = &value[start + key.len()..];
            let separator = tail
                .find([':', '='])
                .or_else(|| tail.find(char::is_whitespace));
            let Some(separator) = separator else {
                break;
            };
            let value_start = start + key.len() + separator + 1;
            let value_tail = value[value_start..].trim_start();
            let skip = value[value_start..].len() - value_tail.len();
            let end_rel = value_tail
                .find(|character: char| {
                    character.is_whitespace() || matches!(character, ',' | ';' | '&' | '}')
                })
                .unwrap_or(value_tail.len());
            if end_rel > 0 {
                let absolute_start = value_start + skip;
                value.replace_range(absolute_start..absolute_start + end_rel, "[REDACTED]");
                search_from = absolute_start + "[REDACTED]".len();
            } else {
                search_from = value_start;
            }
        }
    }
    if let Some(prefix) = value.find("Bearer ") {
        let start = prefix + "Bearer ".len();
        let end = value[start..]
            .find(char::is_whitespace)
            .map(|offset| start + offset)
            .unwrap_or(value.len());
        if end > start {
            value.replace_range(start..end, "[REDACTED]");
        }
    }
    value
}

pub fn security_evidence() -> McpSecurityEvidence {
    McpSecurityEvidence {
        process_isolated: true,
        transport: "stdio JSON-RPC; business tools return a host-forward marker".to_string(),
        database_path_passed: false,
        credential_reference_passed: false,
        direct_database_api_available: false,
        direct_credential_api_available: false,
        business_tools_forward_to_core: true,
        forbidden_argument_fields: vec![
            "path".to_string(),
            "filePath".to_string(),
            "credentialRef".to_string(),
            "cookie".to_string(),
            "token".to_string(),
            "apiKey".to_string(),
        ],
        allowed_media_reference_fields: vec![
            "sourceContentId".to_string(),
            "assetId".to_string(),
            "order".to_string(),
            "caption".to_string(),
        ],
        observations: vec![
            "测试 MCP Server 以独立进程和 stdio 启动，未接收数据库路径或凭据引用".to_string(),
            "Server 不链接 SQLite 或 Keychain 访问命令；业务工具仅返回 forwardToHost 标记"
                .to_string(),
            "Host 在 JSON Schema 校验后通过 Core 服务执行账号、内容和发布工具".to_string(),
            "媒体载荷只接受内容 ID + 素材 ID，不接受任意本地路径".to_string(),
        ],
    }
}

pub(super) fn truncate_json(value: Value, max_chars: usize) -> Value {
    let rendered = value.to_string();
    if rendered.chars().count() <= max_chars {
        return value;
    }
    json!({
        "truncated": true,
        "preview": rendered.chars().take(max_chars).collect::<String>(),
        "message": "工具结果超过传输上限，已截断；请使用更精确的筛选参数"
    })
}
