//! Platform-specific MCP profiles.
//!
//! Each platform keeps its own interface inventory, field mapping and tool
//! schemas here instead of growing a single platform-agnostic MCP file. A
//! profile is descriptive, not an authorization: the admission registry and
//! account status still decide whether any external action is allowed.

use crate::mcp::{McpToolDefinition, ToolPermission};
use serde::Serialize;
use serde_json::{Map, Value, json};

mod bilibili_mcp;
mod csdn_mcp;
mod douyin_mcp;
mod execute;
mod juejin_mcp;
mod kuaishou_mcp;
mod wechat_mcp;
mod weibo_mcp;
mod xiaohongshu_mcp;
mod youtube_mcp;
mod zhihu_mcp;

pub(crate) use execute::execute_platform_tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformToolKind {
    AccountInfo,
    WorkList,
    AnalyticsTrend,
    PublishValidate,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformToolSpec {
    pub name: String,
    pub title: String,
    pub description: String,
    pub permission: ToolPermission,
    pub kind: PlatformToolKind,
    pub input_schema: Value,
}

impl PlatformToolSpec {
    pub fn definition(&self) -> McpToolDefinition {
        McpToolDefinition {
            name: self.name.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
            schema_version: "1.0".to_string(),
            permission: self.permission,
            input_schema: self.input_schema.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformInterface {
    pub name: String,
    pub direction: String,
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub auth: String,
    pub verification: String,
    pub source_url: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFieldMapping {
    pub unified_field: String,
    pub platform_field: String,
    pub direction: String,
    pub required: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformContentRule {
    pub kind: String,
    pub allowed_media: Vec<String>,
    pub min_media: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_media: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_title_chars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_body_chars: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tags: Option<usize>,
    pub cover_required: bool,
    pub constraints_verified: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformHandoff {
    pub publish_url: String,
    pub work_list_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analytics_url: Option<String>,
    pub steps: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformMcpProfile {
    pub server_name: String,
    pub platform_id: String,
    pub platform_name: String,
    pub description: String,
    pub transport: String,
    pub auth_mode: String,
    pub admission_status: String,
    pub auto_publish_allowed: bool,
    pub publish_dispatch: String,
    pub native_work_id_field: String,
    pub interfaces: Vec<PlatformInterface>,
    pub tools: Vec<PlatformToolSpec>,
    pub field_mappings: Vec<PlatformFieldMapping>,
    pub content_rules: Vec<PlatformContentRule>,
    pub handoff: PlatformHandoff,
}

pub fn platform_mcp_catalog() -> Vec<PlatformMcpProfile> {
    vec![
        bilibili_mcp::profile(),
        csdn_mcp::profile(),
        douyin_mcp::profile(),
        juejin_mcp::profile(),
        kuaishou_mcp::profile(),
        wechat_mcp::profile(),
        weibo_mcp::profile(),
        xiaohongshu_mcp::profile(),
        youtube_mcp::profile(),
        zhihu_mcp::profile(),
    ]
}

pub fn platform_mcp_profile(platform_id: &str) -> Option<PlatformMcpProfile> {
    platform_mcp_catalog()
        .into_iter()
        .find(|profile| profile.platform_id == platform_id)
}

pub fn platform_tool_definitions() -> Vec<McpToolDefinition> {
    platform_mcp_catalog()
        .into_iter()
        .flat_map(|profile| profile.tools.into_iter().map(|tool| tool.definition()))
        .collect()
}

pub fn find_platform_tool(tool_name: &str) -> Option<(PlatformMcpProfile, PlatformToolSpec)> {
    for profile in platform_mcp_catalog() {
        if let Some(tool) = profile
            .tools
            .iter()
            .find(|tool| tool.name == tool_name)
            .cloned()
        {
            return Some((profile, tool));
        }
    }
    None
}

fn object_schema(properties: Value, required: Vec<&str>) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": required,
    })
}

fn string_property(max_length: usize) -> Value {
    json!({ "type": "string", "minLength": 1, "maxLength": max_length })
}

fn optional_string_property(max_length: usize) -> Value {
    json!({ "type": "string", "maxLength": max_length })
}

pub(super) fn account_tool(
    platform_id: &str,
    title: &str,
    description: &str,
) -> PlatformToolSpec {
    PlatformToolSpec {
        name: format!("{platform_id}_account_get_info"),
        title: title.to_string(),
        description: description.to_string(),
        permission: ToolPermission::Read,
        kind: PlatformToolKind::AccountInfo,
        input_schema: object_schema(
            json!({
                "accountId": optional_string_property(128),
            }),
            vec![],
        ),
    }
}

pub(super) fn work_list_tool(
    platform_id: &str,
    native_id_field: &str,
    native_id_label: &str,
    extra_properties: Value,
    description: &str,
) -> PlatformToolSpec {
    let mut properties = Map::new();
    properties.insert(
        "accountId".to_string(),
        optional_string_property(128),
    );
    properties.insert(
        native_id_field.to_string(),
        optional_string_property(160),
    );
    properties.insert(
        "contentKind".to_string(),
        json!({ "type": "string", "enum": ["video", "imageText", "imageArticle", "article", "unknown"] }),
    );
    properties.insert("keyword".to_string(), optional_string_property(200));
    properties.insert("publishedFrom".to_string(), optional_string_property(40));
    properties.insert("publishedTo".to_string(), optional_string_property(40));
    properties.insert(
        "limit".to_string(),
        json!({ "type": "integer", "minimum": 1, "maximum": 100 }),
    );
    if let Some(extra) = extra_properties.as_object() {
        for (key, value) in extra {
            properties.insert(key.clone(), value.clone());
        }
    }
    PlatformToolSpec {
        name: format!("{platform_id}_work_list"),
        title: format!("查询{}作品", native_id_label),
        description: description.to_string(),
        permission: ToolPermission::Read,
        kind: PlatformToolKind::WorkList,
        input_schema: object_schema(Value::Object(properties), vec![]),
    }
}

pub(super) fn analytics_tool(
    platform_id: &str,
    native_id_field: &str,
    native_id_label: &str,
    metric_names: Vec<&str>,
    extra_properties: Value,
) -> PlatformToolSpec {
    let mut properties = Map::new();
    properties.insert(
        "accountId".to_string(),
        optional_string_property(128),
    );
    properties.insert(
        native_id_field.to_string(),
        optional_string_property(160),
    );
    properties.insert(
        "range".to_string(),
        json!({ "type": "string", "enum": ["7d", "30d", "90d"] }),
    );
    properties.insert(
        "granularity".to_string(),
        json!({ "type": "string", "enum": ["day", "week", "month"] }),
    );
    properties.insert(
        "contentKind".to_string(),
        json!({ "type": "string", "enum": ["video", "imageText", "imageArticle", "article"] }),
    );
    properties.insert(
        "metrics".to_string(),
        json!({
            "type": "array",
            "maxItems": 12,
            "items": { "type": "string", "enum": metric_names },
        }),
    );
    if let Some(extra) = extra_properties.as_object() {
        for (key, value) in extra {
            properties.insert(key.clone(), value.clone());
        }
    }
    PlatformToolSpec {
        name: format!("{platform_id}_analytics_get_trend"),
        title: format!("读取{}数据趋势", native_id_label),
        description: format!(
            "读取本地已同步的{native_id_label}指标趋势，保留平台原始指标口径；不会发起未授权的外部请求。"
        ),
        permission: ToolPermission::Read,
        kind: PlatformToolKind::AnalyticsTrend,
        input_schema: object_schema(Value::Object(properties), vec!["range", "granularity"]),
    }
}

pub(super) fn publish_validate_tool(
    platform_id: &str,
    platform_name: &str,
    native_id_field: &str,
    native_id_label: &str,
) -> PlatformToolSpec {
    PlatformToolSpec {
        name: format!("{platform_id}_publish_validate"),
        title: format!("校验{}发布载荷", platform_name),
        description: format!(
            "按{platform_name}实际表单字段与内容形态校验本地内容包，返回字段映射和人工发布交接信息；不发布、不写入外部平台。{native_id_label}发布使用 `{native_id_field}` 作为回读标识。"
        ),
        permission: ToolPermission::Read,
        kind: PlatformToolKind::PublishValidate,
        input_schema: object_schema(
            json!({
                "contentId": string_property(128),
                "accountId": optional_string_property(128),
            }),
            vec!["contentId"],
        ),
    }
}

pub(super) fn interface(
    name: &str,
    direction: &str,
    transport: &str,
    method: Option<&str>,
    endpoint: Option<&str>,
    auth: &str,
    verification: &str,
    source_url: &str,
    notes: &str,
) -> PlatformInterface {
    PlatformInterface {
        name: name.to_string(),
        direction: direction.to_string(),
        transport: transport.to_string(),
        method: method.map(str::to_string),
        endpoint: endpoint.map(str::to_string),
        auth: auth.to_string(),
        verification: verification.to_string(),
        source_url: source_url.to_string(),
        notes: notes.to_string(),
    }
}

pub(super) fn mapping(
    unified_field: &str,
    platform_field: &str,
    direction: &str,
    required: bool,
    notes: &str,
) -> PlatformFieldMapping {
    PlatformFieldMapping {
        unified_field: unified_field.to_string(),
        platform_field: platform_field.to_string(),
        direction: direction.to_string(),
        required,
        notes: notes.to_string(),
    }
}

pub(super) fn content_rule(
    kind: &str,
    allowed_media: Vec<&str>,
    min_media: usize,
    max_media: Option<usize>,
    max_title_chars: Option<usize>,
    max_body_chars: Option<usize>,
    max_tags: Option<usize>,
    cover_required: bool,
    constraints_verified: bool,
    notes: &str,
) -> PlatformContentRule {
    PlatformContentRule {
        kind: kind.to_string(),
        allowed_media: allowed_media.into_iter().map(str::to_string).collect(),
        min_media,
        max_media,
        max_title_chars,
        max_body_chars,
        max_tags,
        cover_required,
        constraints_verified,
        notes: notes.to_string(),
    }
}

pub(super) fn handoff(
    publish_url: &str,
    work_list_url: &str,
    analytics_url: Option<&str>,
    steps: Vec<&str>,
    notes: &str,
) -> PlatformHandoff {
    PlatformHandoff {
        publish_url: publish_url.to_string(),
        work_list_url: work_list_url.to_string(),
        analytics_url: analytics_url.map(str::to_string),
        steps: steps.into_iter().map(str::to_string).collect(),
        notes: notes.to_string(),
    }
}
