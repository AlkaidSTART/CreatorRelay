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


mod account_tools;
mod assistant;
mod audit;
mod authorization;
mod catalog;
mod content_tools;
mod core;
mod host;
mod analytics_tools;
mod publishing_tools;
mod schema;
mod security;
mod server;
mod settings;

pub mod platforms;

pub use assistant::*;
pub use audit::list_audit;
pub use catalog::{server_tool_descriptors, tool_catalog};
pub use core::{call_tool, preview_tool_call};
pub use host::McpHost;
pub use security::{redact_text, security_evidence};
pub use server::test_server_main;
pub use settings::{McpSettingsInput, get_settings, save_settings};
pub use crate::platforms::CredentialVault;
