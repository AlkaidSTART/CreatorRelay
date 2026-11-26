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

pub(crate) use execute::{execute_platform_tool, platform_tool_effects, preview_platform_tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PlatformToolKind {
    AccountInfo,
    WorkList,
    WorkGet,
    WorkChanges,
    WorkSync,
    AnalyticsTrend,
    PublishValidate,
    MediaStage,
    PublishSubmit,
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
pub struct PlatformFieldDefinition {
    pub unified_field: String,
    pub platform_field: String,
    pub direction: String,
    pub value_type: String,
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
    let profiles = vec![
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
    ];
    profiles
        .into_iter()
        .map(|mut profile| {
            extend_platform_profile(&mut profile);
            profile
        })
        .collect()
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

fn extend_platform_profile(profile: &mut PlatformMcpProfile) {
    let native_field = profile.native_work_id_field.clone();
    let platform_id = profile.platform_id.clone();
    let platform_name = profile.platform_name.clone();
    let native_label = native_field.clone();
    profile.tools.extend(vec![
        PlatformToolSpec {
            name: format!("{platform_id}_work_get"),
            title: format!("读取{platform_name}作品详情"),
            description: format!(
                "按本地 workId 或平台原生 {native_field} 读取结构化作品详情、原始字段和指标快照；不会发起外部请求。"
            ),
            permission: ToolPermission::Read,
            kind: PlatformToolKind::WorkGet,
            input_schema: object_schema(
                json!({
                    "workId": optional_string_property(128),
                    "accountId": optional_string_property(128),
                    native_field.clone(): optional_string_property(160),
                }),
                vec![],
            ),
        },
        PlatformToolSpec {
            name: format!("{platform_id}_work_changes"),
            title: format!("增量读取{platform_name}作品变更"),
            description: "从本地同步库按 updatedAt + id 游标增量读取作品变更；返回稳定的 nextCursor，不发起外部请求。"
                .to_string(),
            permission: ToolPermission::Read,
            kind: PlatformToolKind::WorkChanges,
            input_schema: object_schema(
                json!({
                    "accountId": optional_string_property(128),
                    "updatedAfter": optional_string_property(40),
                    "cursor": optional_string_property(2048),
                    "pageSize": { "type": "integer", "minimum": 1, "maximum": 200 },
                }),
                vec![],
            ),
        },
        PlatformToolSpec {
            name: format!("{platform_id}_work_sync"),
            title: format!("同步{platform_name}作品"),
            description: format!(
                "通过已安装且获授权的平台 Adapter 同步账号作品与指标；Adapter 必须显式提供。未验证平台不会自动改用人工或浏览器发布。{native_label} 字段进入 rawFields。"
            ),
            permission: ToolPermission::ExternalAction,
            kind: PlatformToolKind::WorkSync,
            input_schema: object_schema(
                json!({
                    "accountId": string_property(128),
                    "adapterId": string_property(160),
                    "asOf": optional_string_property(40),
                    "pageSize": { "type": "integer", "minimum": 1, "maximum": 200 },
                    "cursor": optional_string_property(2048),
                }),
                vec!["accountId", "adapterId"],
            ),
        },
        PlatformToolSpec {
            name: format!("{platform_id}_media_stage"),
            title: format!("暂存{platform_name}发布内容"),
            description: "在隔离浏览器打开平台发布页，附加本地内容包媒体并填写标题/正文；不点击保存草稿或最终发布。每次执行需确认。"
                .to_string(),
            permission: ToolPermission::ExternalAction,
            kind: PlatformToolKind::MediaStage,
            input_schema: object_schema(
                json!({
                    "contentId": string_property(128),
                    "accountId": optional_string_property(128),
                    "assetIds": {
                        "type": "array",
                        "maxItems": 20,
                        "items": string_property(128)
                    }
                }),
                vec!["contentId"],
            ),
        },
        PlatformToolSpec {
            name: format!("{platform_id}_publish_submit"),
            title: format!("提交{platform_name}发布表单"),
            description: "在已暂存的隔离浏览器页面点击用户指定的精确按钮文本。draft 模式保存草稿；publish 模式会最终公开发布，必须同时提供 confirmFinalPublish=true。"
                .to_string(),
            permission: ToolPermission::ExternalAction,
            kind: PlatformToolKind::PublishSubmit,
            input_schema: object_schema(
                json!({
                    "contentId": string_property(128),
                    "accountId": optional_string_property(128),
                    "mode": { "type": "string", "enum": ["draft", "publish"] },
                    "buttonLabel": string_property(40),
                    "confirmFinalPublish": { "type": "boolean" },
                }),
                vec!["contentId", "mode", "buttonLabel"],
            ),
        },
    ]);
}

fn string_property(max_length: usize) -> Value {
    json!({ "type": "string", "minLength": 1, "maxLength": max_length })
}

fn optional_string_property(max_length: usize) -> Value {
    json!({ "type": "string", "maxLength": max_length })
}

pub(super) fn account_tool(platform_id: &str, title: &str, description: &str) -> PlatformToolSpec {
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
    properties.insert("accountId".to_string(), optional_string_property(128));
    properties.insert(native_id_field.to_string(), optional_string_property(160));
    properties.insert(
        "contentKind".to_string(),
        json!({ "type": "string", "enum": ["video", "imageText", "imageArticle", "article", "unknown"] }),
    );
    properties.insert("keyword".to_string(), optional_string_property(200));
    properties.insert("publishedFrom".to_string(), optional_string_property(40));
    properties.insert("publishedTo".to_string(), optional_string_property(40));
    properties.insert("cursor".to_string(), optional_string_property(2048));
    properties.insert(
        "pageSize".to_string(),
        json!({ "type": "integer", "minimum": 1, "maximum": 100 }),
    );
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
    properties.insert("accountId".to_string(), optional_string_property(128));
    properties.insert(native_id_field.to_string(), optional_string_property(160));
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

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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
