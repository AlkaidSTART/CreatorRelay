use crate::{
    ai::{self, AiGenerateRequest, AiMediaMetadata, AiProviderSummary},
    content::{self, ContentKind, ContentSaveInput},
    ecosystem,
    operations::{
        self, AnalyticsGranularity, AnalyticsQuery, CreatePublishTasksInput, ExternalWorkFilter,
        PublishSourceType, PublishTargetInput,
    },
    platforms,
    plugins::PluginRuntime,
};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration as StdDuration, Instant},
};
use uuid::Uuid;

pub use crate::platforms::CredentialVault;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const MAX_TIMEOUT_MS: u64 = 120_000;
const AUTH_TTL_MINUTES: i64 = 10;
const MAX_TOOL_RESULT_CHARS: usize = 200_000;
const SERVER_ID: &str = "creatorrelay.test-mcp";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ToolPermission {
    Read,
    Write,
    ExternalAction,
}

impl ToolPermission {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::ExternalAction => "externalAction",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "externalAction" => Ok(Self::ExternalAction),
            _ => Err(format!("未知工具权限等级：{value}")),
        }
    }

    fn requires_confirmation(self, settings: &McpSettings) -> bool {
        match self {
            Self::Read => false,
            Self::Write => settings.write_policy == "confirm",
            Self::ExternalAction => true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDefinition {
    pub name: String,
    pub title: String,
    pub description: String,
    pub schema_version: String,
    pub permission: ToolPermission,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpHostConfig {
    pub auto_restart: bool,
    pub max_restarts: u32,
}

impl Default for McpHostConfig {
    fn default() -> Self {
        Self {
            auto_restart: true,
            max_restarts: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpHostStatus {
    pub state: String,
    pub server_id: String,
    pub server_name: String,
    pub protocol_version: String,
    pub pid: Option<u32>,
    pub started_at: Option<String>,
    pub last_health_at: Option<String>,
    pub last_error: Option<String>,
    pub restart_count: u32,
    pub config: McpHostConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    pub read_enabled: bool,
    pub write_policy: String,
    pub external_policy: String,
    pub auto_restart: bool,
    pub max_restarts: u32,
    pub default_context_scope: String,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            read_enabled: true,
            write_policy: "confirm".to_string(),
            external_policy: "confirm_each_time".to_string(),
            auto_restart: true,
            max_restarts: 3,
            default_context_scope: "title_summary_media".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolCallView {
    pub call_id: String,
    pub tool_name: String,
    pub permission: ToolPermission,
    pub target_summary: String,
    pub parameter_summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpFieldError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCallError {
    pub code: String,
    pub message: String,
    pub field_errors: Vec<McpFieldError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCallResult {
    pub call_id: String,
    pub audit_id: String,
    pub tool_name: String,
    pub permission: ToolPermission,
    pub status: String,
    pub content_kind: Option<String>,
    pub target_summary: String,
    pub parameter_summary: String,
    pub result: Option<Value>,
    pub error: Option<McpCallError>,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCallRequest {
    #[serde(default)]
    pub call_id: Option<String>,
    pub tool_name: String,
    #[serde(default)]
    pub arguments: Value,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub authorization_token: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub context_scope: Option<ModelContextScope>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpPreviewRequest {
    pub tool_name: String,
    #[serde(default)]
    pub arguments: Value,
    #[serde(default)]
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAuthorizationPreview {
    pub call_id: String,
    pub tool_name: String,
    pub title: String,
    pub permission: ToolPermission,
    pub content_kind: Option<String>,
    pub target_summary: String,
    pub parameter_summary: String,
    pub effects: Vec<String>,
    pub details: Value,
    pub requires_confirmation: bool,
    pub authorization_token: Option<String>,
    pub confirmation_expires_at: Option<String>,
    pub field_errors: Vec<McpFieldError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAuditRecord {
    pub id: String,
    pub thread_id: Option<String>,
    pub call_id: String,
    pub server_id: String,
    pub tool_name: String,
    pub permission: ToolPermission,
    pub content_kind: Option<String>,
    pub target_summary: String,
    pub parameter_summary: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSecurityEvidence {
    pub process_isolated: bool,
    pub transport: String,
    pub database_path_passed: bool,
    pub credential_reference_passed: bool,
    pub direct_database_api_available: bool,
    pub direct_credential_api_available: bool,
    pub business_tools_forward_to_core: bool,
    pub forbidden_argument_fields: Vec<String>,
    pub allowed_media_reference_fields: Vec<String>,
    pub observations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelContextScope {
    #[serde(default)]
    pub include_title: bool,
    #[serde(default)]
    pub include_summary: bool,
    #[serde(default)]
    pub include_body: bool,
    #[serde(default)]
    pub include_media_metadata: bool,
}

impl Default for ModelContextScope {
    fn default() -> Self {
        Self {
            include_title: true,
            include_summary: true,
            include_body: false,
            include_media_metadata: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantThread {
    pub id: String,
    pub title: String,
    pub provider_id: Option<String>,
    pub provider_name: String,
    pub model: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub tool_call: Option<McpToolCallView>,
    pub result_status: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantThreadDetail {
    pub thread: AssistantThread,
    pub messages: Vec<AssistantMessage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantPlan {
    pub thread: AssistantThread,
    pub user_message: AssistantMessage,
    pub assistant_message: AssistantMessage,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantContextPreview {
    pub provider_name: String,
    pub model: String,
    pub content_kind: Option<String>,
    pub context: String,
    pub character_count: usize,
    pub included_fields: Vec<String>,
    pub excluded_fields: Vec<String>,
    pub redaction_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssistantGenerateRequest {
    pub thread_id: String,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub content_id: Option<String>,
    #[serde(default)]
    pub scope: ModelContextScope,
    #[serde(default)]
    pub instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantGenerateResult {
    pub user_message: Option<AssistantMessage>,
    pub assistant_message: AssistantMessage,
    pub context_preview: AssistantContextPreview,
}

struct ServerProcess {
    child: Child,
    stdin: ChildStdin,
    responses: Arc<Mutex<mpsc::Receiver<Value>>>,
}

impl ServerProcess {
    fn pid(&self) -> Option<u32> {
        Some(self.child.id())
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child.try_wait().map_err(|error| error.to_string())
    }
}

struct HostInner {
    executable: PathBuf,
    process: Mutex<Option<ServerProcess>>,
    request_lock: Mutex<()>,
    lifecycle_lock: Mutex<()>,
    config: Mutex<McpHostConfig>,
    restarts: AtomicUsize,
    next_request_id: AtomicU64,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    started_at: Mutex<Option<String>>,
    last_health_at: Mutex<Option<String>>,
    last_error: Mutex<Option<String>>,
    server_name: Mutex<String>,
    protocol_version: Mutex<String>,
}

#[derive(Clone)]
pub struct McpHost {
    inner: Arc<HostInner>,
}

impl McpHost {
    pub fn new(executable: PathBuf) -> Self {
        Self {
            inner: Arc::new(HostInner {
                executable,
                process: Mutex::new(None),
                request_lock: Mutex::new(()),
                lifecycle_lock: Mutex::new(()),
                config: Mutex::new(McpHostConfig::default()),
                restarts: AtomicUsize::new(0),
                next_request_id: AtomicU64::new(1),
                cancels: Mutex::new(HashMap::new()),
                started_at: Mutex::new(None),
                last_health_at: Mutex::new(None),
                last_error: Mutex::new(None),
                server_name: Mutex::new("CreatorRelay 测试 MCP Server".to_string()),
                protocol_version: Mutex::new("2024-11-05".to_string()),
            }),
        }
    }

    pub fn start(&self, config: Option<McpHostConfig>) -> Result<McpHostStatus, String> {
        let _lifecycle = self
            .inner
            .lifecycle_lock
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(config) = config {
            *self
                .inner
                .config
                .lock()
                .map_err(|error| error.to_string())? = config;
        }
        self.inner.restarts.store(0, Ordering::SeqCst);
        self.start_locked()?;
        self.status()
    }

    fn start_locked(&self) -> Result<(), String> {
        self.stop_locked(false)?;
        let mut child = Command::new(&self.inner.executable)
            .arg("--mcp-test-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("无法启动本地 MCP Server：{error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "MCP Server stdin 不可用".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "MCP Server stdout 不可用".to_string())?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(value) => {
                        if sender.send(value).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        *self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())? = Some(ServerProcess {
            child,
            stdin,
            responses: Arc::new(Mutex::new(receiver)),
        });
        *self
            .inner
            .started_at
            .lock()
            .map_err(|error| error.to_string())? = Some(Utc::now().to_rfc3339());
        *self
            .inner
            .last_error
            .lock()
            .map_err(|error| error.to_string())? = None;

        let initialized = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "CreatorRelay", "version": "0.1.0" }
            }),
            StdDuration::from_secs(5),
        )?;
        let result = initialized
            .get("result")
            .ok_or_else(|| "MCP initialize 响应缺少 result".to_string())?;
        let server_name = result
            .pointer("/serverInfo/name")
            .and_then(Value::as_str)
            .unwrap_or("CreatorRelay 测试 MCP Server")
            .to_string();
        let protocol_version = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or("2024-11-05")
            .to_string();
        *self
            .inner
            .server_name
            .lock()
            .map_err(|error| error.to_string())? = server_name;
        *self
            .inner
            .protocol_version
            .lock()
            .map_err(|error| error.to_string())? = protocol_version;
        self.notify("notifications/initialized", json!({}))?;
        Ok(())
    }

    pub fn stop(&self) -> Result<McpHostStatus, String> {
        let _lifecycle = self
            .inner
            .lifecycle_lock
            .lock()
            .map_err(|error| error.to_string())?;
        self.stop_locked(true)?;
        self.status()
    }

    fn stop_locked(&self, clear_state: bool) -> Result<(), String> {
        let mut guard = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(mut process) = guard.take() {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }
        drop(guard);
        if clear_state {
            *self
                .inner
                .started_at
                .lock()
                .map_err(|error| error.to_string())? = None;
            *self
                .inner
                .last_health_at
                .lock()
                .map_err(|error| error.to_string())? = None;
        }
        Ok(())
    }

    fn process_exited(&self) -> Result<bool, String> {
        let mut guard = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        let Some(process) = guard.as_mut() else {
            return Ok(true);
        };
        match process.try_wait()? {
            Some(status) => {
                let message = if status.success() {
                    "MCP Server 已退出".to_string()
                } else {
                    format!("MCP Server 异常退出，退出码={status}")
                };
                *self
                    .inner
                    .last_error
                    .lock()
                    .map_err(|error| error.to_string())? = Some(message);
                guard.take();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn status(&self) -> Result<McpHostStatus, String> {
        let alive = !self.process_exited()?;
        let process = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        let pid = process.as_ref().and_then(ServerProcess::pid);
        let config = self
            .inner
            .config
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        let last_error = self
            .inner
            .last_error
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        Ok(McpHostStatus {
            state: if alive { "healthy" } else { "stopped" }.to_string(),
            server_id: SERVER_ID.to_string(),
            server_name: self
                .inner
                .server_name
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            protocol_version: self
                .inner
                .protocol_version
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            pid,
            started_at: self
                .inner
                .started_at
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            last_health_at: self
                .inner
                .last_health_at
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            last_error,
            restart_count: self.inner.restarts.load(Ordering::SeqCst) as u32,
            config,
        })
    }

    pub fn health_check(&self) -> Result<McpHostStatus, String> {
        if self.process_exited()? {
            let config = self
                .inner
                .config
                .lock()
                .map_err(|error| error.to_string())?
                .clone();
            if config.auto_restart
                && self.inner.restarts.load(Ordering::SeqCst) < config.max_restarts as usize
            {
                let _lifecycle = self
                    .inner
                    .lifecycle_lock
                    .lock()
                    .map_err(|error| error.to_string())?;
                if self.process_exited()? {
                    self.inner.restarts.fetch_add(1, Ordering::SeqCst);
                    self.start_locked()?;
                }
            } else {
                return self.status();
            }
        }
        let response = self.request("ping", json!({}), StdDuration::from_secs(2))?;
        if response.get("result").is_none() {
            return Err("MCP ping 未返回成功结果".to_string());
        }
        *self
            .inner
            .last_health_at
            .lock()
            .map_err(|error| error.to_string())? = Some(Utc::now().to_rfc3339());
        self.status()
    }

    pub fn configure(&self, config: McpHostConfig) -> Result<(), String> {
        *self
            .inner
            .config
            .lock()
            .map_err(|error| error.to_string())? = config;
        Ok(())
    }

    fn write_message(&self, value: &Value) -> Result<(), String> {
        let mut process = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        let process = process
            .as_mut()
            .ok_or_else(|| "MCP Server 未运行".to_string())?;
        let serialized = serde_json::to_string(value).map_err(|error| error.to_string())?;
        process
            .stdin
            .write_all(serialized.as_bytes())
            .and_then(|_| process.stdin.write_all(b"\n"))
            .and_then(|_| process.stdin.flush())
            .map_err(|error| format!("MCP 消息写入失败：{error}"))
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn request(&self, method: &str, params: Value, timeout: StdDuration) -> Result<Value, String> {
        let _request = self
            .inner
            .request_lock
            .lock()
            .map_err(|error| error.to_string())?;
        let id = self.inner.next_request_id.fetch_add(1, Ordering::SeqCst);
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;
        let receiver = {
            let process = self
                .inner
                .process
                .lock()
                .map_err(|error| error.to_string())?;
            process
                .as_ref()
                .ok_or_else(|| "MCP Server 未运行".to_string())?
                .responses
                .clone()
        };
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let _ = self.stop_locked(false);
                return Err(format!("MCP 请求超时：{method}"));
            }
            let value = receiver
                .lock()
                .map_err(|error| error.to_string())?
                .recv_timeout(remaining)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => format!("MCP 请求超时：{method}"),
                    mpsc::RecvTimeoutError::Disconnected => "MCP Server 连接已断开".to_string(),
                })?;
            if value.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(value);
            }
        }
    }

    pub fn list_tools(&self) -> Result<Vec<McpToolDefinition>, String> {
        let response = self.request("tools/list", json!({}), StdDuration::from_secs(5))?;
        let remote_tools = response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .ok_or_else(|| "MCP tools/list 响应缺少工具列表".to_string())?;
        let local = tool_catalog();
        let by_name = local
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect::<HashMap<_, _>>();
        let mut tools = Vec::new();
        for remote in remote_tools {
            let Some(name) = remote.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(mut definition) = by_name.get(name).cloned() else {
                continue;
            };
            if let Some(schema) = remote.get("inputSchema") {
                definition.input_schema = schema.clone();
            }
            if let Some(description) = remote.get("description").and_then(Value::as_str) {
                definition.description = description.to_string();
            }
            tools.push(definition);
        }
        Ok(tools)
    }

    fn call_server(&self, call: &McpCallRequest) -> Result<Value, String> {
        self.request(
            "tools/call",
            json!({
                "name": call.tool_name,
                "arguments": call.arguments,
            }),
            StdDuration::from_millis(normalized_timeout(call.timeout_ms)),
        )
    }

    pub fn cancel(&self, call_id: &str) -> Result<(), String> {
        let cancels = self
            .inner
            .cancels
            .lock()
            .map_err(|error| error.to_string())?;
        let flag = cancels
            .get(call_id)
            .ok_or_else(|| "未找到正在执行的工具调用".to_string())?;
        flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn register_cancel(&self, call_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.inner
            .cancels
            .lock()
            .expect("cancel registry lock")
            .insert(call_id.to_string(), flag.clone());
        flag
    }

    fn finish_cancel(&self, call_id: &str) {
        self.inner
            .cancels
            .lock()
            .expect("cancel registry lock")
            .remove(call_id);
    }
}

fn object_schema(properties: Value, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": required
    })
}

fn string_array_schema(max_items: usize) -> Value {
    json!({
        "type": "array",
        "minItems": 1,
        "maxItems": max_items,
        "items": { "type": "string", "minLength": 1, "maxLength": 128 }
    })
}

fn media_ref_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sourceContentId": { "type": "string", "minLength": 1, "maxLength": 128 },
            "assetId": { "type": "string", "minLength": 1, "maxLength": 128 },
            "order": { "type": "integer", "minimum": 1, "maximum": 999 },
            "caption": { "type": "string", "maxLength": 500 }
        },
        "required": ["sourceContentId", "assetId", "order"]
    })
}

fn cover_ref_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "sourceContentId": { "type": "string", "minLength": 1, "maxLength": 128 },
            "assetId": { "type": "string", "minLength": 1, "maxLength": 128 }
        },
        "required": ["sourceContentId", "assetId"]
    })
}

fn tool(
    name: &str,
    title: &str,
    description: &str,
    permission: ToolPermission,
    input_schema: Value,
) -> McpToolDefinition {
    McpToolDefinition {
        name: name.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        schema_version: "1.0".to_string(),
        permission,
        input_schema,
    }
}

pub fn tool_catalog() -> Vec<McpToolDefinition> {
    vec![
        tool(
            "account_get_info",
            "读取账号信息",
            "通过 Core 账号服务读取脱敏的账号状态与能力，不接受凭据引用。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "accountId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "platformId": { "type": "string", "minLength": 1, "maxLength": 64 }
                }),
                vec![],
            ),
        ),
        tool(
            "platform_capabilities_get",
            "读取平台与账号能力",
            "返回平台准入、官方来源、逐形态读取/发布状态和账号级降级原因；未验证能力不会显示为 supported。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "platformId": { "type": "string", "minLength": 1, "maxLength": 64 },
                    "accountId": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec![],
            ),
        ),
        tool(
            "content_list",
            "查询本地内容",
            "按关键词、形态和平台查询本地内容包，不返回素材原始路径或凭据。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "query": { "type": "string", "maxLength": 200 },
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] },
                    "platformId": { "type": "string", "maxLength": 64 },
                    "accountId": { "type": "string", "maxLength": 128 },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
                }),
                vec![],
            ),
        ),
        tool(
            "content_get_detail",
            "读取内容详情",
            "通过 Core 内容服务读取内容包详情；正文是否返回由本次模型上下文授权范围决定。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "contentId": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec!["contentId"],
            ),
        ),
        tool(
            "external_content_list",
            "查询已发布作品",
            "查询已同步的外部作品及原始指标口径，不把不同形态的指标合并为总数。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "keyword": { "type": "string", "maxLength": 200 },
                    "platformId": { "type": "string", "maxLength": 64 },
                    "accountId": { "type": "string", "maxLength": 128 },
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article", "unknown"] },
                    "publishedFrom": { "type": "string", "maxLength": 40 },
                    "publishedTo": { "type": "string", "maxLength": 40 }
                }),
                vec![],
            ),
        ),
        tool(
            "analytics_get_trend",
            "读取数据趋势",
            "按平台和内容形态读取可追溯指标趋势，保留平台原始指标口径。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "range": { "type": "string", "enum": ["7d", "30d", "90d"] },
                    "granularity": { "type": "string", "enum": ["day", "week", "month"] },
                    "platformId": { "type": "string", "maxLength": 64 },
                    "accountId": { "type": "string", "maxLength": 128 },
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] },
                    "workId": { "type": "string", "maxLength": 128 }
                }),
                vec!["range", "granularity"],
            ),
        ),
        tool(
            "studio_create_content",
            "创建内容草稿",
            "通过受控 Core 资产服务创建视频、图文、图文文章或纯文章草稿；媒体只能引用已有资产 ID。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "contentKind": { "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] },
                    "title": { "type": "string", "minLength": 1, "maxLength": 200 },
                    "summary": { "type": "string", "maxLength": 2000 },
                    "tags": { "type": "array", "maxItems": 20, "items": { "type": "string", "minLength": 1, "maxLength": 40 } },
                    "body": { "type": "string", "maxLength": 400000 },
                    "media": { "type": "array", "maxItems": 30, "items": media_ref_schema() },
                    "cover": cover_ref_schema()
                }),
                vec!["contentKind", "title"],
            ),
        ),
        tool(
            "studio_update_content",
            "更新内容草稿",
            "更新本地内容草稿，保持原内容形态；媒体引用必须来自 Core 内容包资产。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "contentId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "title": { "type": "string", "minLength": 1, "maxLength": 200 },
                    "summary": { "type": "string", "maxLength": 2000 },
                    "tags": { "type": "array", "maxItems": 20, "items": { "type": "string", "minLength": 1, "maxLength": 40 } },
                    "body": { "type": "string", "maxLength": 400000 },
                    "media": { "type": "array", "maxItems": 30, "items": media_ref_schema() },
                    "cover": cover_ref_schema(),
                    "baseHash": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec!["contentId"],
            ),
        ),
        tool(
            "publishing_create_task",
            "创建发布任务",
            "为明确的平台账号创建独立发布任务；不自动执行外部发布。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "sourceType": { "type": "string", "enum": ["content", "platformDraft"] },
                    "sourceId": { "type": "string", "minLength": 1, "maxLength": 128 },
                    "targets": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": 12,
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "platformId": { "type": "string", "minLength": 1, "maxLength": 64 },
                                "platformName": { "type": "string", "minLength": 1, "maxLength": 80 },
                                "accountId": { "type": "string", "minLength": 1, "maxLength": 128 },
                                "accountAlias": { "type": "string", "minLength": 1, "maxLength": 80 },
                                "adapterId": { "type": "string", "maxLength": 128 }
                            },
                            "required": ["platformId", "platformName", "accountId", "accountAlias"]
                        }
                    },
                    "scheduledAt": { "type": "string", "maxLength": 40 }
                }),
                vec!["sourceType", "sourceId", "targets"],
            ),
        ),
        tool(
            "publishing_publish_task",
            "发布已确认内容",
            "External Action：按任务逐个发布到平台。每次执行必须展示全部目标与最终内容并重新确认。",
            ToolPermission::ExternalAction,
            object_schema(
                json!({
                    "taskIds": string_array_schema(12)
                }),
                vec!["taskIds"],
            ),
        ),
        tool(
            "content_delete_draft",
            "删除本地草稿",
            "删除指定内容包。该操作需要确认，且只接受内容 ID，不接受任意文件路径。",
            ToolPermission::Write,
            object_schema(
                json!({
                    "contentId": { "type": "string", "minLength": 1, "maxLength": 128 }
                }),
                vec!["contentId"],
            ),
        ),
        tool(
            "mcp_test_echo",
            "MCP 回显测试",
            "验证本地 MCP Server 生命周期、工具发现和协议调用。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "message": { "type": "string", "minLength": 1, "maxLength": 500 }
                }),
                vec!["message"],
            ),
        ),
        tool(
            "mcp_test_hang",
            "MCP 超时取消测试",
            "模拟长时间运行的本地工具，用于验证超时、取消和重复执行保护。",
            ToolPermission::Read,
            object_schema(
                json!({
                    "milliseconds": { "type": "integer", "minimum": 100, "maximum": 120000 }
                }),
                vec!["milliseconds"],
            ),
        ),
    ]
}

pub fn server_tool_descriptors() -> Vec<Value> {
    tool_catalog()
        .into_iter()
        .map(|item| {
            json!({
                "name": item.name,
                "title": item.title,
                "description": item.description,
                "inputSchema": item.input_schema
            })
        })
        .collect()
}

fn normalized_timeout(value: Option<u64>) -> u64 {
    value
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(100, MAX_TIMEOUT_MS)
}

fn schema_type_matches(value: &Value, expected: &str) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn validate_schema(value: &Value, schema: &Value, path: &str, errors: &mut Vec<McpFieldError>) {
    if let Some(expected) = schema.get("type").and_then(Value::as_str)
        && !schema_type_matches(value, expected)
    {
        errors.push(McpFieldError {
            path: path.to_string(),
            message: format!("字段类型应为 {expected}"),
        });
        return;
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array)
        && !values.iter().any(|candidate| candidate == value)
    {
        errors.push(McpFieldError {
            path: path.to_string(),
            message: format!(
                "字段值必须是以下之一：{}",
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if let Some(text) = value.as_str() {
        let length = text.chars().count() as i64;
        if let Some(min) = schema.get("minLength").and_then(Value::as_i64)
            && length < min
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("至少需要 {min} 个字符"),
            });
        }
        if let Some(max) = schema.get("maxLength").and_then(Value::as_i64)
            && length > max
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能超过 {max} 个字符"),
            });
        }
    }
    if let Some(number) = value.as_f64() {
        if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64)
            && number < minimum
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能小于 {minimum}"),
            });
        }
        if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64)
            && number > maximum
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能大于 {maximum}"),
            });
        }
    }
    if let Some(items) = value.as_array() {
        let length = items.len() as i64;
        if let Some(min) = schema.get("minItems").and_then(Value::as_i64)
            && length < min
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("至少需要 {min} 项"),
            });
        }
        if let Some(max) = schema.get("maxItems").and_then(Value::as_i64)
            && length > max
        {
            errors.push(McpFieldError {
                path: path.to_string(),
                message: format!("不能超过 {max} 项"),
            });
        }
        if let Some(item_schema) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                validate_schema(item, item_schema, &format!("{path}[{index}]"), errors);
            }
        }
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for field in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(field) {
                    errors.push(McpFieldError {
                        path: if path.is_empty() {
                            field.to_string()
                        } else {
                            format!("{path}.{field}")
                        },
                        message: "缺少必填字段".to_string(),
                    });
                }
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (key, child) in object {
            if let Some(property_schema) = properties.and_then(|values| values.get(key)) {
                let child_path = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                validate_schema(child, property_schema, &child_path, errors);
            } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                errors.push(McpFieldError {
                    path: if path.is_empty() {
                        key.to_string()
                    } else {
                        format!("{path}.{key}")
                    },
                    message: "不允许的字段".to_string(),
                });
            }
        }
    }
}

fn validate_tool_arguments(tool: &McpToolDefinition, arguments: &Value) -> Vec<McpFieldError> {
    let mut errors = Vec::new();
    validate_schema(arguments, &tool.input_schema, "", &mut errors);
    errors
}

pub fn get_settings(conn: &Connection) -> Result<McpSettings, String> {
    let fallback = McpSettings::default();
    conn.query_row(
        "SELECT read_enabled, write_policy, external_policy, auto_restart,
                max_restarts, default_context_scope
         FROM mcp_settings WHERE id=1",
        [],
        |row| {
            Ok(McpSettings {
                read_enabled: row.get::<_, i64>(0)? != 0,
                write_policy: row.get(1)?,
                external_policy: row.get(2)?,
                auto_restart: row.get::<_, i64>(3)? != 0,
                max_restarts: u32::try_from(row.get::<_, i64>(4)?).unwrap_or(3),
                default_context_scope: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
    .map(|value| value.unwrap_or(fallback))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpSettingsInput {
    pub read_enabled: bool,
    pub write_policy: String,
    pub auto_restart: bool,
    pub max_restarts: u32,
    #[serde(default = "default_context_scope_setting")]
    pub default_context_scope: String,
}

fn default_context_scope_setting() -> String {
    "title_summary_media".to_string()
}

pub fn save_settings(conn: &Connection, input: McpSettingsInput) -> Result<McpSettings, String> {
    if !matches!(input.write_policy.as_str(), "confirm" | "allow") {
        return Err("写入授权策略必须是 confirm 或 allow".to_string());
    }
    if input.max_restarts > 10 {
        return Err("自动重启次数不能超过 10".to_string());
    }
    if !matches!(
        input.default_context_scope.as_str(),
        "title_only" | "title_summary" | "title_summary_media" | "selected"
    ) {
        return Err("默认模型上下文范围无效".to_string());
    }
    conn.execute(
        "UPDATE mcp_settings
         SET read_enabled=?1, write_policy=?2, external_policy='confirm_each_time',
             auto_restart=?3, max_restarts=?4, default_context_scope=?5, updated_at=?6
         WHERE id=1",
        params![
            if input.read_enabled { 1 } else { 0 },
            input.write_policy,
            if input.auto_restart { 1 } else { 0 },
            input.max_restarts,
            input.default_context_scope,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| error.to_string())?;
    get_settings(conn)
}

fn tool_by_name(name: &str) -> Option<McpToolDefinition> {
    tool_catalog().into_iter().find(|tool| tool.name == name)
}

fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn payload_hash(tool: &str, arguments: &Value) -> Result<String, String> {
    Ok(digest(&format!(
        "{tool}:{}",
        serde_json::to_string(arguments).map_err(|error| error.to_string())?
    )))
}

fn sensitive_key(key: &str) -> bool {
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

fn redact_json(value: &Value) -> Value {
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

fn summarize_arguments(arguments: &Value) -> String {
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

fn target_summary_for(tool_name: &str, arguments: &Value) -> String {
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

fn content_kind_from_arguments(arguments: &Value) -> Option<String> {
    arguments
        .get("contentKind")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn insert_audit_start(
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

fn finish_audit(
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

fn issue_authorization(
    conn: &Connection,
    call_id: &str,
    tool_name: &str,
    permission: ToolPermission,
    arguments: &Value,
) -> Result<(String, String), String> {
    let token = Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::minutes(AUTH_TTL_MINUTES)).to_rfc3339();
    conn.execute(
        "INSERT INTO mcp_authorization_tokens(
            token, call_id, tool_name, payload_hash, permission, expires_at, created_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            token,
            call_id,
            tool_name,
            payload_hash(tool_name, arguments)?,
            permission.as_str(),
            expires_at,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| format!("工具授权令牌写入失败：{error}"))?;
    Ok((token, expires_at))
}

fn consume_authorization(
    conn: &Connection,
    token: &str,
    tool_name: &str,
    permission: ToolPermission,
    arguments: &Value,
) -> Result<(), McpCallError> {
    let row = conn
        .query_row(
            "SELECT tool_name, payload_hash, permission, expires_at, used_at
             FROM mcp_authorization_tokens WHERE token=?1",
            [token],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| McpCallError {
            code: "AUTHORIZATION_STORE_ERROR".to_string(),
            message: error.to_string(),
            field_errors: Vec::new(),
        })?
        .ok_or_else(|| McpCallError {
            code: "AUTHORIZATION_REQUIRED".to_string(),
            message: "该工具需要一次明确的用户确认，请先预览最终目标与参数".to_string(),
            field_errors: Vec::new(),
        })?;
    if row.4.is_some() {
        return Err(McpCallError {
            code: "AUTHORIZATION_USED".to_string(),
            message: "确认令牌已使用；外部操作不会自动重试".to_string(),
            field_errors: Vec::new(),
        });
    }
    if row.2 != permission.as_str() || row.0 != tool_name {
        return Err(McpCallError {
            code: "AUTHORIZATION_MISMATCH".to_string(),
            message: "确认令牌与本次工具或权限等级不一致".to_string(),
            field_errors: Vec::new(),
        });
    }
    let expected_hash = payload_hash(tool_name, arguments).map_err(|message| McpCallError {
        code: "AUTHORIZATION_HASH_ERROR".to_string(),
        message,
        field_errors: Vec::new(),
    })?;
    if row.1 != expected_hash {
        return Err(McpCallError {
            code: "AUTHORIZATION_PAYLOAD_CHANGED".to_string(),
            message: "预览后参数或目标已变化，请重新预览并确认".to_string(),
            field_errors: Vec::new(),
        });
    }
    let expires = DateTime::parse_from_rfc3339(&row.3).map_err(|_| McpCallError {
        code: "AUTHORIZATION_INVALID".to_string(),
        message: "确认过期时间格式无效".to_string(),
        field_errors: Vec::new(),
    })?;
    if expires <= Utc::now() {
        return Err(McpCallError {
            code: "AUTHORIZATION_EXPIRED".to_string(),
            message: "确认已过期，请重新预览最终目标与内容".to_string(),
            field_errors: Vec::new(),
        });
    }
    let changed = conn
        .execute(
            "UPDATE mcp_authorization_tokens SET used_at=?1 WHERE token=?2 AND used_at IS NULL",
            params![Utc::now().to_rfc3339(), token],
        )
        .map_err(|error| McpCallError {
            code: "AUTHORIZATION_STORE_ERROR".to_string(),
            message: error.to_string(),
            field_errors: Vec::new(),
        })?;
    if changed != 1 {
        return Err(McpCallError {
            code: "AUTHORIZATION_USED".to_string(),
            message: "确认令牌已被使用".to_string(),
            field_errors: Vec::new(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct AssetReference {
    source_content_id: String,
    asset: content::ContentAsset,
    order: u32,
    caption: String,
}

#[derive(Debug, Clone)]
struct PreparedContent {
    content_id: Option<String>,
    kind: ContentKind,
    title: String,
    summary: String,
    tags: Vec<String>,
    body: String,
    base_hash: Option<String>,
    media_refs: Option<Vec<AssetReference>>,
    cover_ref: Option<AssetReference>,
    existing_media: Vec<content::ContentAsset>,
    existing_cover: Option<content::ContentAsset>,
}

fn field_error(path: &str, message: impl Into<String>) -> McpFieldError {
    McpFieldError {
        path: path.to_string(),
        message: message.into(),
    }
}

fn parse_asset_reference(
    workspace: &Path,
    value: &Value,
    path: &str,
    require_order: bool,
) -> Result<AssetReference, McpFieldError> {
    let source_content_id = value
        .get("sourceContentId")
        .and_then(Value::as_str)
        .ok_or_else(|| field_error(&format!("{path}.sourceContentId"), "缺少来源内容 ID"))?;
    let asset_id = value
        .get("assetId")
        .and_then(Value::as_str)
        .ok_or_else(|| field_error(&format!("{path}.assetId"), "缺少素材 ID"))?;
    let order = if require_order {
        value
            .get("order")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| field_error(&format!("{path}.order"), "缺少有效素材顺序"))?
    } else {
        1
    };
    let caption = value
        .get("caption")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let package = content::read_content_package(workspace, source_content_id).map_err(|error| {
        field_error(
            &format!("{path}.sourceContentId"),
            format!("来源内容不可用：{error}"),
        )
    })?;
    let asset = package
        .manifest
        .media
        .iter()
        .chain(package.manifest.cover.iter())
        .find(|asset| asset.id == asset_id)
        .cloned()
        .ok_or_else(|| field_error(&format!("{path}.assetId"), "素材 ID 不属于来源内容包"))?;
    Ok(AssetReference {
        source_content_id: source_content_id.to_string(),
        asset,
        order,
        caption,
    })
}

fn parse_media_references(
    workspace: &Path,
    arguments: &Value,
    _existing: bool,
) -> Result<Option<Vec<AssetReference>>, Vec<McpFieldError>> {
    let Some(media) = arguments.get("media") else {
        return Ok(None);
    };
    let Some(items) = media.as_array() else {
        return Err(vec![field_error("media", "media 必须是数组")]);
    };
    let mut errors = Vec::new();
    let mut refs = Vec::new();
    let mut orders = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        match parse_asset_reference(workspace, item, &format!("media[{index}]"), true) {
            Ok(reference) => {
                if !orders.insert(reference.order) {
                    errors.push(field_error(
                        &format!("media[{index}].order"),
                        "素材顺序不能重复",
                    ));
                }
                refs.push(reference);
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(Some(refs))
    } else {
        Err(errors)
    }
}

fn parse_cover_reference(
    workspace: &Path,
    arguments: &Value,
) -> Result<Option<AssetReference>, Vec<McpFieldError>> {
    let Some(cover) = arguments.get("cover") else {
        return Ok(None);
    };
    if cover.is_null() {
        return Ok(None);
    }
    parse_asset_reference(workspace, cover, "cover", false)
        .map(Some)
        .map_err(|error| vec![error])
}

fn parse_content_kind(value: &str) -> ContentKind {
    match value {
        "video" => ContentKind::Video,
        "imageText" => ContentKind::ImageText,
        "imageArticle" => ContentKind::ImageArticle,
        "article" => ContentKind::Article,
        _ => ContentKind::Article,
    }
}

fn validate_content_payload(payload: &PreparedContent) -> Vec<McpFieldError> {
    let mut errors = Vec::new();
    let media = payload
        .media_refs
        .as_deref()
        .map(|refs| {
            refs.iter()
                .map(|reference| &reference.asset)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| payload.existing_media.iter().collect::<Vec<_>>());
    let has_video = media
        .iter()
        .any(|asset| asset.kind == content::AssetKind::Video);
    let has_image = media
        .iter()
        .any(|asset| asset.kind == content::AssetKind::Image);
    let cover = payload
        .cover_ref
        .as_ref()
        .map(|reference| &reference.asset)
        .or(payload.existing_cover.as_ref());
    match payload.kind {
        ContentKind::Video => {
            if !has_video {
                errors.push(field_error("media", "视频内容必须包含至少一个视频文件"));
            }
            if has_image {
                errors.push(field_error(
                    "media",
                    "视频内容的媒体列表只能包含视频文件；图片请通过 cover 提供",
                ));
            }
            if !payload.body.trim().is_empty() {
                errors.push(field_error(
                    "body",
                    "视频内容不使用 Markdown 正文，请写入 summary",
                ));
            }
            if cover.is_none() {
                errors.push(field_error("cover", "视频内容必须提供封面图片"));
            }
        }
        ContentKind::ImageText => {
            if media.is_empty() {
                errors.push(field_error("media", "图文内容至少需要一张图片"));
            }
            if !has_image || has_video {
                errors.push(field_error("media", "图文媒体顺序只能包含图片"));
            }
            if !payload.body.trim().is_empty() {
                errors.push(field_error(
                    "body",
                    "图文内容不使用 Markdown 正文，请写入 summary 和逐图说明",
                ));
            }
            if cover.is_none() {
                errors.push(field_error("cover", "图文内容必须明确封面图片"));
            }
        }
        ContentKind::ImageArticle => {
            if payload.body.trim().is_empty() {
                errors.push(field_error("body", "图文文章正文不能为空"));
            }
            if media.is_empty() {
                errors.push(field_error("media", "图文文章至少需要一张图片"));
            }
            if !has_image || has_video {
                errors.push(field_error("media", "图文文章媒体顺序只能包含图片"));
            }
            if cover.is_none() {
                errors.push(field_error("cover", "图文文章必须明确封面图片"));
            }
        }
        ContentKind::Article => {
            if payload.body.trim().is_empty() {
                errors.push(field_error("body", "纯文章正文不能为空"));
            }
        }
    }
    if let Some(cover) = cover
        && cover.kind != content::AssetKind::Image
    {
        errors.push(field_error("cover", "封面必须是图片素材"));
    }
    errors
}

fn prepare_create_content(
    workspace: &Path,
    arguments: &Value,
) -> Result<PreparedContent, Vec<McpFieldError>> {
    let kind_value = arguments
        .get("contentKind")
        .and_then(Value::as_str)
        .unwrap_or("article");
    let kind = parse_content_kind(kind_value);
    let title = arguments
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let mut errors = Vec::new();
    if title.is_empty() {
        errors.push(field_error("title", "标题不能为空"));
    }
    let media_refs = match parse_media_references(workspace, arguments, false) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            None
        }
    };
    let cover_ref = match parse_cover_reference(workspace, arguments) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            None
        }
    };
    let payload = PreparedContent {
        content_id: None,
        kind,
        title,
        summary: arguments
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
        tags: arguments
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        body: arguments
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        base_hash: None,
        media_refs,
        cover_ref,
        existing_media: Vec::new(),
        existing_cover: None,
    };
    errors.extend(validate_content_payload(&payload));
    if errors.is_empty() {
        Ok(payload)
    } else {
        Err(errors)
    }
}

fn prepare_update_content(
    workspace: &Path,
    arguments: &Value,
) -> Result<PreparedContent, Vec<McpFieldError>> {
    let content_id = arguments
        .get("contentId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let package = content::read_content_package(workspace, content_id)
        .map_err(|error| vec![field_error("contentId", error)])?;
    let mut errors = Vec::new();
    let media_refs = match parse_media_references(workspace, arguments, true) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            Some(Vec::new())
        }
    };
    let cover_ref = match parse_cover_reference(workspace, arguments) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            None
        }
    };
    let payload = PreparedContent {
        content_id: Some(package.manifest.id.clone()),
        kind: package.manifest.kind,
        title: arguments
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| package.manifest.title.clone()),
        summary: arguments
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| package.manifest.summary.clone()),
        tags: arguments
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_else(|| package.manifest.tags.clone()),
        body: arguments
            .get("body")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| package.body.clone()),
        base_hash: arguments
            .get("baseHash")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some(package.content_hash.clone())),
        media_refs,
        cover_ref,
        existing_media: package.manifest.media.clone(),
        existing_cover: package.manifest.cover.clone(),
    };
    errors.extend(validate_content_payload(&payload));
    if errors.is_empty() {
        Ok(payload)
    } else {
        Err(errors)
    }
}

fn prepared_media_summary(payload: &PreparedContent) -> Value {
    let media = payload
        .media_refs
        .as_ref()
        .map(|refs| {
            refs.iter()
                .map(|reference| {
                    json!({
                        "assetId": reference.asset.id,
                        "kind": match reference.asset.kind {
                            content::AssetKind::Video => "video",
                            content::AssetKind::Image => "image",
                        },
                        "originalName": reference.asset.original_name,
                        "mimeType": reference.asset.mime_type,
                        "byteSize": reference.asset.byte_size,
                        "order": reference.order,
                        "caption": reference.caption,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            payload
                .existing_media
                .iter()
                .map(|asset| {
                    json!({
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
                    })
                })
                .collect::<Vec<_>>()
        });
    Value::Array(media)
}

fn content_preview_details(payload: &PreparedContent) -> Value {
    let cover = payload
        .cover_ref
        .as_ref()
        .map(|reference| &reference.asset)
        .or(payload.existing_cover.as_ref());
    json!({
        "contentId": payload.content_id,
        "contentKind": payload.kind.as_str(),
        "title": payload.title,
        "summary": payload.summary,
        "body": payload.body,
        "bodyPreview": payload.body.chars().take(600).collect::<String>(),
        "bodyTruncated": payload.body.chars().count() > 600,
        "media": prepared_media_summary(payload),
        "cover": cover.map(|asset| json!({
            "assetId": asset.id,
            "originalName": asset.original_name,
            "mimeType": asset.mime_type,
            "byteSize": asset.byte_size,
        })),
    })
}

fn materialize_reference(
    workspace: &Path,
    target_id: &str,
    reference: &AssetReference,
    role: &str,
) -> Result<content::ContentAsset, String> {
    let bytes =
        content::read_content_asset(workspace, &reference.source_content_id, &reference.asset.id)?;
    let mut asset = content::import_content_asset(
        workspace,
        target_id,
        role,
        &reference.asset.original_name,
        &reference.asset.mime_type,
        &bytes.bytes,
    )?;
    asset.order = reference.order;
    asset.caption = reference.caption.clone();
    Ok(asset)
}

fn execute_prepared_content(
    workspace: &Path,
    payload: &PreparedContent,
) -> Result<Value, McpCallError> {
    let (target_id, cleanup_on_error) = if let Some(id) = payload.content_id.as_deref() {
        (id.to_string(), false)
    } else {
        let package = content::create_content_package(workspace, payload.kind, &payload.title)
            .map_err(|error| McpCallError {
                code: "CONTENT_CREATE_FAILED".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
        (package.manifest.id, true)
    };
    let result = (|| {
        let media = if let Some(references) = &payload.media_refs {
            let mut assets = Vec::new();
            for reference in references {
                assets.push(materialize_reference(
                    workspace, &target_id, reference, "media",
                )?);
            }
            assets
        } else {
            payload.existing_media.clone()
        };
        let cover = if let Some(reference) = &payload.cover_ref {
            Some(materialize_reference(
                workspace, &target_id, reference, "cover",
            )?)
        } else {
            payload.existing_cover.clone()
        };
        let package = content::save_content_package(
            workspace,
            &target_id,
            ContentSaveInput {
                title: payload.title.clone(),
                summary: payload.summary.clone(),
                tags: payload.tags.clone(),
                cover,
                media,
                body: payload.body.clone(),
                base_hash: payload.base_hash.clone(),
            },
        )?;
        Ok::<_, String>(json!({
            "contentId": package.manifest.id,
            "contentKind": package.manifest.kind.as_str(),
            "title": package.manifest.title,
            "contentVersion": package.content_version,
            "mediaCount": package.manifest.media.len(),
            "hasCover": package.manifest.cover.is_some(),
            "saved": true,
        }))
    })();
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            if cleanup_on_error {
                let _ = content::delete_content_package(workspace, &target_id);
            }
            Err(McpCallError {
                code: "CONTENT_SAVE_FAILED".to_string(),
                message: error,
                field_errors: Vec::new(),
            })
        }
    }
}

fn account_view(account: &platforms::PlatformAccount) -> Value {
    json!({
        "id": account.id,
        "platformId": account.platform_id,
        "platformName": account.platform_name,
        "category": account.category,
        "alias": account.alias,
        "nickname": account.nickname,
        "uid": account.uid,
        "status": account.status,
        "enabled": account.enabled,
        "isDefault": account.is_default,
        "lastCheckedAt": account.last_checked_at,
        "lastError": account.last_error,
        "capabilities": account.capabilities,
    })
}

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

fn parse_analytics_query(arguments: &Value) -> Result<AnalyticsQuery, McpCallError> {
    let granularity = match arguments
        .get("granularity")
        .and_then(Value::as_str)
        .unwrap_or("day")
    {
        "day" => AnalyticsGranularity::Day,
        "week" => AnalyticsGranularity::Week,
        "month" => AnalyticsGranularity::Month,
        _ => {
            return Err(McpCallError {
                code: "INVALID_ARGUMENT".to_string(),
                message: "granularity 无效".to_string(),
                field_errors: vec![field_error("granularity", "必须是 day、week 或 month")],
            });
        }
    };
    Ok(AnalyticsQuery {
        range: arguments
            .get("range")
            .and_then(Value::as_str)
            .unwrap_or("7d")
            .to_string(),
        granularity,
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
        work_id: arguments
            .get("workId")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

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

fn truncate_json(value: Value, max_chars: usize) -> Value {
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

fn append_assistant_message(
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

pub fn test_server_main() -> ! {
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        if matches!(
            method,
            "notifications/initialized" | "notifications/cancelled"
        ) {
            continue;
        }
        let response = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "CreatorRelay 测试 MCP Server", "version": "0.1.0" }
            }),
            "ping" => json!({}),
            "tools/list" => json!({ "tools": server_tool_descriptors() }),
            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if name == "mcp_test_echo" {
                    json!({
                        "content": [{
                            "type": "text",
                            "text": params.pointer("/arguments/message").and_then(Value::as_str).unwrap_or_default()
                        }],
                        "isError": false
                    })
                } else if tool_by_name(name).is_some() {
                    json!({
                        "content": [{ "type": "text", "text": "forwarded-to-host-core" }],
                        "structuredContent": {
                            "forwardToHost": true,
                            "tool": name,
                            "arguments": params.get("arguments").cloned().unwrap_or_else(|| json!({}))
                        },
                        "isError": false
                    })
                } else {
                    json!({
                        "content": [{ "type": "text", "text": "unknown tool" }],
                        "isError": true
                    })
                }
            }
            _ => {
                if let Some(id) = id {
                    let _ = writeln!(
                        stdout,
                        "{}",
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": "Method not found" }
                        })
                    );
                }
                continue;
            }
        };
        if let Some(id) = id {
            let payload = json!({ "jsonrpc": "2.0", "id": id, "result": response });
            if writeln!(stdout, "{payload}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    }
    std::process::exit(0)
}
