use crate::{
    content::ContentKind,
    logging,
    platforms::{CredentialVault, KeyringVault},
};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs, path::Path, time::Duration};
use uuid::Uuid;

const API_STYLE_OPENAI_COMPATIBLE: &str = "openai-compatible";
const MAX_API_KEY_LENGTH: usize = 16 * 1024;
const MAX_CONTEXT_CHARS: usize = 40_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSummary {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub api_style: String,
    pub is_default: bool,
    pub configured: bool,
    pub last_test_status: Option<String>,
    pub last_test_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderInput {
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_style: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub is_default: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTestResult {
    pub provider_id: String,
    pub ok: bool,
    pub message: String,
    pub latency_ms: u128,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMediaMetadata {
    pub kind: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub mime_type: String,
    #[serde(default)]
    pub byte_size: u64,
    #[serde(default)]
    pub caption: String,
    #[serde(default)]
    pub order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerateRequest {
    pub provider_id: String,
    pub operation: String,
    pub content_kind: ContentKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub media_metadata: Vec<AiMediaMetadata>,
    #[serde(default)]
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiGenerateResult {
    pub request_id: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub operation: String,
    pub content: String,
}

#[derive(Debug, Clone)]
struct ProviderRecord {
    id: String,
    name: String,
    base_url: String,
    model: String,
    api_style: String,
    is_default: bool,
    has_secret: bool,
    last_test_status: Option<String>,
    last_test_error: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ProviderRecord {
    fn summary(&self) -> AiProviderSummary {
        AiProviderSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            model: self.model.clone(),
            api_style: self.api_style.clone(),
            is_default: self.is_default,
            configured: self.has_secret,
            last_test_status: self.last_test_status.clone(),
            last_test_error: self.last_test_error.clone(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

fn secret_reference(provider_id: &str) -> String {
    format!("ai:{provider_id}:api-key")
}

fn normalize_base_url(value: &str) -> Result<String, String> {
    let mut value = value.trim().trim_end_matches('/').to_string();
    if value.is_empty() {
        return Err("模型端点不能为空".to_string());
    }
    if value.len() > 500 || value.chars().any(char::is_control) {
        return Err("模型端点格式无效".to_string());
    }
    let parsed = url::Url::parse(&value).map_err(|_| "模型端点不是有效 URL".to_string())?;
    if !matches!(parsed.scheme(), "https" | "http") {
        return Err("模型端点必须使用 http 或 https".to_string());
    }
    if parsed.host_str().is_none() || !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("模型端点不能包含用户信息".to_string());
    }
    if parsed.scheme() == "http"
        && !matches!(parsed.host_str(), Some("127.0.0.1" | "localhost" | "::1"))
    {
        return Err("非本地模型端点必须使用 HTTPS".to_string());
    }
    while value.ends_with("/chat/completions") {
        value.truncate(value.len() - "/chat/completions".len());
    }
    Ok(value.trim_end_matches('/').to_string())
}

fn validate_provider_input(input: &AiProviderInput) -> Result<(String, String, String, String), String> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err("供应商名称需为 1 至 80 个字符".to_string());
    }
    let model = input.model.trim();
    if model.is_empty() || model.chars().count() > 200 {
        return Err("模型名称需为 1 至 200 个字符".to_string());
    }
    let base_url = normalize_base_url(&input.base_url)?;
    let api_style = input
        .api_style
        .as_deref()
        .unwrap_or(API_STYLE_OPENAI_COMPATIBLE)
        .trim();
    if api_style != API_STYLE_OPENAI_COMPATIBLE {
        return Err("当前仅支持 OpenAI Compatible 对话端点".to_string());
    }
    Ok((
        name.to_string(),
        base_url,
        model.to_string(),
        api_style.to_string(),
    ))
}

fn provider_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProviderRecord> {
    Ok(ProviderRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        base_url: row.get(2)?,
        model: row.get(3)?,
        api_style: row.get(4)?,
        is_default: row.get::<_, i64>(5)? != 0,
        has_secret: row.get::<_, i64>(6)? != 0,
        last_test_status: row.get(7)?,
        last_test_error: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

const PROVIDER_SELECT: &str = "SELECT id, name, base_url, model, api_style, is_default, has_secret,
       last_test_status, last_test_error, created_at, updated_at FROM ai_providers";

fn provider_record(conn: &Connection, provider_id: &str) -> Result<ProviderRecord, String> {
    conn.query_row(
        &format!("{PROVIDER_SELECT} WHERE id=?1"),
        [provider_id],
        provider_from_row,
    )
    .optional()
    .map_err(|_| "AI 供应商读取失败".to_string())?
    .ok_or_else(|| "未找到 AI 供应商".to_string())
}

pub fn list_providers(conn: &Connection) -> Result<Vec<AiProviderSummary>, String> {
    let mut statement = conn
        .prepare(&format!(
            "{PROVIDER_SELECT} ORDER BY is_default DESC, updated_at DESC"
        ))
        .map_err(|_| "AI 供应商列表准备失败".to_string())?;
    let records = statement
        .query_map([], provider_from_row)
        .map_err(|_| "AI 供应商列表读取失败".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "AI 供应商记录解析失败".to_string())?;
    Ok(records.into_iter().map(|record| record.summary()).collect())
}

fn apply_default(conn: &Connection, provider_id: &str) -> Result<(), String> {
    conn.execute("UPDATE ai_providers SET is_default=0", [])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE ai_providers SET is_default=1, updated_at=?1 WHERE id=?2",
        params![Utc::now().to_rfc3339(), provider_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn save_provider_with_vault(
    conn: &Connection,
    input: &AiProviderInput,
    vault: &dyn CredentialVault,
) -> Result<AiProviderSummary, String> {
    let (name, base_url, model, api_style) = validate_provider_input(input)?;
    let now = Utc::now().to_rfc3339();
    let id = input
        .id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    if id.len() > 128
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("供应商 ID 格式无效".to_string());
    }
    let existing = conn
        .query_row(
            &format!("{PROVIDER_SELECT} WHERE id=?1"),
            [&id],
            provider_from_row,
        )
        .optional()
        .map_err(|_| "AI 供应商读取失败".to_string())?;
    let api_key = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(key) = api_key {
        if key.len() > MAX_API_KEY_LENGTH || key.chars().any(char::is_control) {
            return Err("API Key 格式无效".to_string());
        }
        vault.set(&secret_reference(&id), key)?;
    } else if existing.as_ref().is_none_or(|record| !record.has_secret) {
        return Err("请提供 API Key".to_string());
    }
    let has_secret = if api_key.is_some() {
        1
    } else {
        existing.as_ref().map(|record| record.has_secret as i64).unwrap_or(0)
    };
    let is_default = input
        .is_default
        .or_else(|| existing.as_ref().map(|record| record.is_default))
        .unwrap_or(false);
    if let Some(record) = existing {
        conn.execute(
            "UPDATE ai_providers SET name=?1, base_url=?2, model=?3, api_style=?4,
                has_secret=?5, updated_at=?6 WHERE id=?7",
            params![name, base_url, model, api_style, has_secret, now, record.id],
        )
        .map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT INTO ai_providers(id, name, base_url, model, api_style, is_default, has_secret,
                last_test_status, last_test_error, created_at, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, 0, ?6, NULL, NULL, ?7, ?7)",
            params![id, name, base_url, model, api_style, has_secret, now],
        )
        .map_err(|e| e.to_string())?;
    }
    if is_default {
        apply_default(conn, &id)?;
    }
    Ok(provider_record(conn, &id)?.summary())
}

pub fn set_default_provider(conn: &Connection, provider_id: &str) -> Result<AiProviderSummary, String> {
    let record = provider_record(conn, provider_id)?;
    if !record.has_secret {
        return Err("请先为该供应商保存 API Key".to_string());
    }
    apply_default(conn, provider_id)?;
    Ok(provider_record(conn, provider_id)?.summary())
}

pub fn delete_provider_with_vault(
    conn: &Connection,
    provider_id: &str,
    vault: &dyn CredentialVault,
) -> Result<(), String> {
    provider_record(conn, provider_id)?;
    vault.delete(&secret_reference(provider_id))?;
    conn.execute("DELETE FROM ai_providers WHERE id=?1", [provider_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn operation_instruction(operation: &str, kind: ContentKind) -> Result<&'static str, String> {
    match (operation, kind) {
        ("title", ContentKind::Video | ContentKind::ImageText | ContentKind::ImageArticle | ContentKind::Article) => {
            Ok("只生成一个可直接使用的中文标题，不要解释，不要引号。")
        }
        ("summary", ContentKind::ImageArticle | ContentKind::Article | ContentKind::Video | ContentKind::ImageText) => {
            Ok("生成一段简洁的中文摘要或短文案，不要添加标题，不要解释。")
        }
        ("description", ContentKind::Video) => Ok("生成一段视频简介，保留原意，不要使用 Markdown 标题。"),
        ("shortCopy", ContentKind::ImageText) => Ok("生成适合图集发布的短文案，可包含少量 emoji，不要编造素材。"),
        ("polish", _) => Ok("在不改变事实与结构的前提下润色文本，只返回润色后的正文。"),
        ("structure", ContentKind::ImageArticle | ContentKind::Article) => {
            Ok("重组文章结构并返回完整 Markdown 正文，保留事实与图片引用。")
        }
        ("tone", _) => Ok("按用户要求转换语气，只返回转换后的正文。"),
        ("contentCheck", _) => Ok("检查内容的事实一致性、表达与平台适配问题，输出简明问题列表。"),
        ("custom", _) => Ok("遵循用户的自定义指令，只返回可直接使用的结果。"),
        _ => Err("当前内容形态不支持该 AI 操作".to_string()),
    }
}

fn kind_label(kind: ContentKind) -> &'static str {
    match kind {
        ContentKind::Video => "视频",
        ContentKind::ImageText => "图文",
        ContentKind::ImageArticle => "图文文章",
        ContentKind::Article => "纯文章",
    }
}

pub fn build_prompt(request: &AiGenerateRequest) -> Result<String, String> {
    let instruction = operation_instruction(&request.operation, request.content_kind)?;
    if request.context.chars().count() > MAX_CONTEXT_CHARS {
        return Err(format!("发送给模型的正文不能超过 {MAX_CONTEXT_CHARS} 个字符"));
    }
    let media = request
        .media_metadata
        .iter()
        .map(|item| {
            format!(
                "- 类型：{}；原始文件名：{}；MIME：{}；大小：{} bytes；顺序：{}；说明：{}",
                item.kind,
                if item.file_name.is_empty() { "未命名" } else { &item.file_name },
                item.mime_type,
                item.byte_size,
                item.order,
                if item.caption.is_empty() { "无" } else { &item.caption }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let extra = request
        .instruction
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\n用户附加要求：{value}"))
        .unwrap_or_default();
    Ok(format!(
        "你是 CreatorRelay 的{}写作助手。\n任务：{instruction}\n内容形态：{}\n当前标题：{}\n{extra}\n\n仅基于以下用户确认发送的文本与素材元数据工作，不得编造未提供的素材：\n<context>\n{}\n</context>\n\n<media_metadata>\n{}\n</media_metadata>",
        kind_label(request.content_kind),
        kind_label(request.content_kind),
        if request.title.trim().is_empty() {
            "（无）"
        } else {
            request.title.trim()
        },
        request.context,
        if media.is_empty() { "（无素材）" } else { &media }
    ))
}

fn sanitized_provider_error(error: &str) -> String {
    logging::redact_sensitive(error)
        .replace(['\r', '\n'], " ")
        .chars()
        .take(600)
        .collect()
}

pub fn chat_completion(
    base_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    timeout: Duration,
) -> Result<String, String> {
    let base_url = normalize_base_url(base_url)?;
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .new_agent();
    let response = agent
        .post(&format!("{base_url}/chat/completions"))
        .header("Authorization", &format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .send_json(json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a concise writing assistant. Return only the requested result."
                },
                { "role": "user", "content": prompt }
            ],
            "temperature": 0.4
        }))
        .map_err(|error| match error {
            ureq::Error::StatusCode(status) => match status {
                401 | 403 => "模型服务拒绝认证，请检查 API Key 是否正确或已过期".to_string(),
                404 => "模型端点不存在，请检查 Base URL 与模型名称".to_string(),
                429 => "模型服务限流，请稍后重试".to_string(),
                _ => format!("模型服务返回 HTTP {status}"),
            },
            _ => sanitized_provider_error(&format!("模型连接失败：{error}")),
        })?;
    let body = response
        .into_body()
        .read_to_string()
        .map_err(|_| "模型响应无法读取".to_string())?;
    let value: Value = serde_json::from_str(&body).map_err(|_| "模型响应不是有效 JSON".to_string())?;
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or("模型响应缺少 choices[0].message.content")?;
    Ok(content.to_string())
}

fn record_call(
    conn: &Connection,
    request_id: &str,
    provider_id: &str,
    request: &AiGenerateRequest,
    status: &str,
    error_code: Option<&str>,
) {
    let _ = conn.execute(
        "INSERT INTO ai_call_audit(id, provider_id, operation, content_kind, status, error_code, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            request_id,
            provider_id,
            request.operation,
            request.content_kind.as_str(),
            status,
            error_code,
            Utc::now().to_rfc3339()
        ],
    );
}

fn update_test_state(
    conn: &Connection,
    provider_id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), String> {
    conn.execute(
        "UPDATE ai_providers SET last_test_status=?1, last_test_error=?2, updated_at=?3 WHERE id=?4",
        params![status, error, Utc::now().to_rfc3339(), provider_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn test_provider_with_vault(
    conn: &Connection,
    provider_id: &str,
    vault: &dyn CredentialVault,
) -> Result<AiTestResult, String> {
    let record = provider_record(conn, provider_id)?;
    let key = vault
        .get(&secret_reference(provider_id))?
        .ok_or("该供应商尚未保存 API Key")?;
    let started = std::time::Instant::now();
    let result = chat_completion(
        &record.base_url,
        &key,
        &record.model,
        "Reply with the single word OK.",
        Duration::from_secs(15),
    );
    let latency_ms = started.elapsed().as_millis();
    match result {
        Ok(_) => {
            update_test_state(conn, provider_id, "ok", None)?;
            Ok(AiTestResult {
                provider_id: provider_id.to_string(),
                ok: true,
                message: "连接成功，模型已返回响应".to_string(),
                latency_ms,
                status: Some("ok".to_string()),
            })
        }
        Err(error) => {
            update_test_state(conn, provider_id, "failed", Some(&error))?;
            Ok(AiTestResult {
                provider_id: provider_id.to_string(),
                ok: false,
                message: error,
                latency_ms,
                status: Some("failed".to_string()),
            })
        }
    }
}

pub fn generate_with_vault(
    conn: &Connection,
    request: &AiGenerateRequest,
    vault: &dyn CredentialVault,
) -> Result<AiGenerateResult, String> {
    let record = provider_record(conn, &request.provider_id)?;
    let key = vault
        .get(&secret_reference(&record.id))?
        .ok_or("该供应商尚未保存 API Key")?;
    let prompt = build_prompt(request)?;
    let request_id = Uuid::new_v4().to_string();
    match chat_completion(
        &record.base_url,
        &key,
        &record.model,
        &prompt,
        Duration::from_secs(60),
    ) {
        Ok(content) => {
            record_call(conn, &request_id, &record.id, request, "success", None);
            Ok(AiGenerateResult {
                request_id,
                provider_id: record.id,
                provider_name: record.name,
                model: record.model,
                operation: request.operation.clone(),
                content,
            })
        }
        Err(error) => {
            record_call(
                conn,
                &request_id,
                &record.id,
                request,
                "failed",
                Some("PROVIDER_ERROR"),
            );
            Err(error)
        }
    }
}

pub fn keyring_vault() -> KeyringVault {
    KeyringVault
}

pub fn audit_database_for_secret(conn: &Connection, secret: &str) -> Result<bool, String> {
    if secret.is_empty() {
        return Ok(false);
    }
    let mut statement = conn
        .prepare("SELECT id, name, base_url, model, api_style, last_test_status, last_test_error, created_at, updated_at FROM ai_providers")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            let mut parts = Vec::new();
            for index in 0..9 {
                parts.push(row.get::<_, String>(index).unwrap_or_default());
            }
            Ok(parts.join("\n"))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        if row.map_err(|e| e.to_string())?.contains(secret) {
            return Ok(true);
        }
    }
    let mut audit = conn
        .prepare("SELECT provider_id, operation, content_kind, status, error_code, created_at FROM ai_call_audit")
        .map_err(|e| e.to_string())?;
    let rows = audit
        .query_map([], |row| {
            let mut parts = Vec::new();
            for index in 0..6 {
                parts.push(row.get::<_, String>(index).unwrap_or_default());
            }
            Ok(parts.join("\n"))
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        if row.map_err(|e| e.to_string())?.contains(secret) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn assert_secret_absent_from_path(path: &Path, secret: &str) -> Result<(), String> {
    if path.exists() && fs::read_to_string(path).map_or(false, |value| value.contains(secret)) {
        return Err(format!("敏感值出现在文件：{}", path.to_string_lossy()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryVault {
        secrets: Mutex<std::collections::HashMap<String, String>>,
    }

    impl CredentialVault for MemoryVault {
        fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
            self.secrets
                .lock()
                .unwrap()
                .insert(reference.to_string(), secret.to_string());
            Ok(())
        }

        fn get(&self, reference: &str) -> Result<Option<String>, String> {
            Ok(self.secrets.lock().unwrap().get(reference).cloned())
        }

        fn delete(&self, reference: &str) -> Result<(), String> {
            self.secrets.lock().unwrap().remove(reference);
            Ok(())
        }
    }

    fn db() -> Connection {
        let mut connection = Connection::open_in_memory().unwrap();
        database::migrate(&mut connection).unwrap();
        connection
    }

    fn fixture_server(body: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut collected = Vec::new();
            let mut buffer = [0_u8; 8192];
            loop {
                let count = stream.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                collected.extend_from_slice(&buffer[..count]);
                if let Some(header_end) = collected.windows(4).position(|window| window == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&collected[..header_end + 4]).to_ascii_lowercase();
                    let content_length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if collected.len() >= header_end + 4 + content_length {
                        break;
                    }
                }
            }
            let request = String::from_utf8_lossy(&collected).to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            request
        });
        (format!("http://{address}"), handle)
    }

    fn input(base_url: &str, secret: &str) -> AiProviderInput {
        AiProviderInput {
            id: None,
            name: "测试供应商".to_string(),
            base_url: base_url.to_string(),
            model: "fixture-model".to_string(),
            api_style: Some(API_STYLE_OPENAI_COMPATIBLE.to_string()),
            api_key: Some(secret.to_string()),
            is_default: Some(true),
        }
    }

    #[test]
    fn ai_provider_secret_never_enters_sqlite_and_success_audit_is_metadata_only() {
        let conn = db();
        let vault = MemoryVault::default();
        let (base_url, server) = fixture_server(
            r#"{"choices":[{"message":{"content":"生成的标题"}}]}"#,
        );
        let provider = save_provider_with_vault(&conn, &input(&base_url, "sk-secret-do-not-store"), &vault)
            .unwrap();
        assert!(provider.configured);
        let secret = "sk-secret-do-not-store";
        assert!(!audit_database_for_secret(&conn, secret).unwrap());
        let result = generate_with_vault(
            &conn,
            &AiGenerateRequest {
                provider_id: provider.id.clone(),
                operation: "title".to_string(),
                content_kind: ContentKind::Article,
                title: "原标题".to_string(),
                context: "# 正文".to_string(),
                media_metadata: vec![],
                instruction: None,
            },
            &vault,
        )
        .unwrap();
        assert_eq!(result.content, "生成的标题");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /chat/completions"));
        let request_lower = request.to_ascii_lowercase();
        assert!(request_lower.contains("authorization: bearer sk-secret-do-not-store"));
        assert!(request.contains("fixture-model"));
        assert!(!audit_database_for_secret(&conn, secret).unwrap());
    }

    #[test]
    fn provider_key_is_absent_from_persisted_sqlite_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("creatorrelay.sqlite3");
        let mut conn = Connection::open(&path).unwrap();
        database::migrate(&mut conn).unwrap();
        let vault = MemoryVault::default();
        let secret = "sk-persisted-file-secret-3948";
        save_provider_with_vault(
            &conn,
            &AiProviderInput {
                id: None,
                name: "文件检查".to_string(),
                base_url: "https://api.example.com/v1".to_string(),
                model: "fixture-model".to_string(),
                api_style: Some(API_STYLE_OPENAI_COMPATIBLE.to_string()),
                api_key: Some(secret.to_string()),
                is_default: Some(true),
            },
            &vault,
        )
        .unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").unwrap();
        drop(conn);
        let bytes = fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains(secret));
        for suffix in ["-wal", "-shm"] {
            let sidecar = Path::new(&format!("{}{suffix}", path.to_string_lossy())).to_path_buf();
            if sidecar.exists() {
                let bytes = fs::read(sidecar).unwrap();
                assert!(!String::from_utf8_lossy(&bytes).contains(secret));
            }
        }
    }

    #[test]
    fn test_connection_distinguishes_auth_failure_and_updates_status() {
        let conn = db();
        let vault = MemoryVault::default();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer);
            stream
                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
        });
        let provider = save_provider_with_vault(
            &conn,
            &input(&format!("http://{address}"), "bad-key"),
            &vault,
        )
        .unwrap();
        let result = test_provider_with_vault(&conn, &provider.id, &vault).unwrap();
        assert!(!result.ok);
        assert!(result.message.contains("认证"));
        assert!(!result.message.contains("bad-key"));
        let refreshed = provider_record(&conn, &provider.id).unwrap();
        assert_eq!(refreshed.last_test_status.as_deref(), Some("failed"));
        server.join().unwrap();
    }

    #[test]
    fn prompt_contains_only_explicit_context_and_media_metadata() {
        let request = AiGenerateRequest {
            provider_id: "p".to_string(),
            operation: "shortCopy".to_string(),
            content_kind: ContentKind::ImageText,
            title: "图集标题".to_string(),
            context: "用户选中的短文案".to_string(),
            media_metadata: vec![AiMediaMetadata {
                kind: "image".to_string(),
                file_name: "a.png".to_string(),
                mime_type: "image/png".to_string(),
                byte_size: 128,
                caption: "第一张说明".to_string(),
                order: 1,
            }],
            instruction: None,
        };
        let prompt = build_prompt(&request).unwrap();
        assert!(prompt.contains("用户选中的短文案"));
        assert!(prompt.contains("a.png"));
        assert!(prompt.contains("第一张说明"));
        assert!(!prompt.contains("workspace"));
        assert!(!prompt.contains("Cookies"));
    }

    #[test]
    fn rejects_non_local_plain_http_endpoints() {
        let error = normalize_base_url("http://example.com/v1").unwrap_err();
        assert!(error.contains("HTTPS"));
        assert!(normalize_base_url("https://api.example.com/v1/").is_ok());
    }
}
