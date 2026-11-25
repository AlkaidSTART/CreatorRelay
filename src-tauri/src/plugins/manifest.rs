use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::sync::LazyLock;

pub const SUPPORTED_ADAPTER_CONTRACT_VERSION: u32 = 1;
pub const DEFAULT_RUNTIME_TIMEOUT_MS: u64 = 5_000;

pub const CONTENT_KINDS: [ContentKind; 4] = [
    ContentKind::Video,
    ContentKind::ImageText,
    ContentKind::ImageArticle,
    ContentKind::Article,
];

pub const KNOWN_PERMISSIONS: [&str; 5] = [
    "network",
    "account.credentials",
    "workspace.read",
    "workspace.write",
    "platform.publish",
];

static MANIFEST_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z][a-z0-9]*(?:[.-][a-z0-9]+)+$").expect("valid manifest id regex")
});
static SEMVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$").expect("valid semver regex")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Video,
    ImageText,
    ImageArticle,
    Article,
}

impl ContentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::ImageText => "imageText",
            Self::ImageArticle => "imageArticle",
            Self::Article => "article",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "video" => Some(Self::Video),
            "imageText" => Some(Self::ImageText),
            "imageArticle" => Some(Self::ImageArticle),
            "article" => Some(Self::Article),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Video => "视频",
            Self::ImageText => "图文",
            Self::ImageArticle => "图文文章",
            Self::Article => "纯文章",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityLevel {
    Supported,
    Unsupported,
    Restricted,
    Experimental,
    Unavailable,
}

impl CapabilityLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::Restricted => "restricted",
            Self::Experimental => "experimental",
            Self::Unavailable => "unavailable",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "supported" => Some(Self::Supported),
            "unsupported" => Some(Self::Unsupported),
            "restricted" => Some(Self::Restricted),
            "experimental" => Some(Self::Experimental),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }

    pub fn is_declarable(self) -> bool {
        !matches!(self, Self::Unavailable)
    }

    pub fn can_publish(self) -> bool {
        matches!(self, Self::Supported | Self::Experimental)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    Adapter,
}

impl PluginType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Adapter => "adapter",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeKind {
    CoreTestRunner,
    Executable,
}

impl RuntimeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CoreTestRunner => "coreTestRunner",
            Self::Executable => "executable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterRuntimeManifest {
    pub kind: RuntimeKind,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    DEFAULT_RUNTIME_TIMEOUT_MS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionRequest {
    pub id: String,
    pub required: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityDeclaration {
    pub kind: ContentKind,
    pub read: CapabilityLevel,
    pub publish: CapabilityLevel,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpManifest {
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterManifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub version: String,
    pub plugin_type: PluginType,
    pub adapter_contract_version: u32,
    pub runtime: AdapterRuntimeManifest,
    pub entry: String,
    #[serde(default)]
    pub permissions: Vec<PermissionRequest>,
    pub capabilities: Vec<CapabilityDeclaration>,
    #[serde(default)]
    pub mcp: Option<McpManifest>,
}

impl AdapterManifest {
    pub fn permission(&self, permission_id: &str) -> Option<&PermissionRequest> {
        self.permissions
            .iter()
            .find(|item| item.id == permission_id)
    }

    pub fn capability(&self, kind: ContentKind) -> Option<&CapabilityDeclaration> {
        self.capabilities.iter().find(|item| item.kind == kind)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestIssue {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestValidation {
    pub valid: bool,
    pub issues: Vec<ManifestIssue>,
    pub manifest: Option<AdapterManifest>,
    pub manifest_hash: Option<String>,
    pub supported_contract_version: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub risk: &'static str,
    pub description: &'static str,
}

pub fn permission_definition(permission_id: &str) -> Option<PermissionDefinition> {
    match permission_id {
        "network" => Some(PermissionDefinition {
            id: "network",
            label: "网络访问",
            risk: "medium",
            description: "仅通过 Core 网络代理发起已批准请求；插件进程默认不能直接联网。",
        }),
        "account.credentials" => Some(PermissionDefinition {
            id: "account.credentials",
            label: "账号会话代理",
            risk: "high",
            description: "仅可获得 Core 签发的账号作用域引用，不能读取 Cookie、Token 或 Keychain 原文。",
        }),
        "workspace.read" => Some(PermissionDefinition {
            id: "workspace.read",
            label: "读取工作区素材",
            risk: "high",
            description: "通过受控资产服务读取用户选择的文档/素材，不会获得任意工作区路径。",
        }),
        "workspace.write" => Some(PermissionDefinition {
            id: "workspace.write",
            label: "写入工作区素材",
            risk: "high",
            description: "通过 Core 写入受控导出目录；插件进程不能直接修改工作区。",
        }),
        "platform.publish" => Some(PermissionDefinition {
            id: "platform.publish",
            label: "平台发布",
            risk: "high",
            description: "调用已授权账号对应的发布能力；每次载荷仍需经过 Core 的形态与完整性校验。",
        }),
        _ => None,
    }
}

pub fn manifest_hash(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

pub fn validate_manifest_json(raw: &str) -> ManifestValidation {
    let mut issues = Vec::new();
    let value = match serde_json::from_str::<Value>(raw) {
        Ok(value) => value,
        Err(error) => {
            return ManifestValidation {
                valid: false,
                issues: vec![ManifestIssue {
                    field: "$".to_string(),
                    code: "invalid_json".to_string(),
                    message: format!("Manifest 不是合法 JSON：{error}"),
                }],
                manifest: None,
                manifest_hash: None,
                supported_contract_version: SUPPORTED_ADAPTER_CONTRACT_VERSION,
            };
        }
    };

    let Some(object) = value.as_object() else {
        return ManifestValidation {
            valid: false,
            issues: vec![ManifestIssue {
                field: "$".to_string(),
                code: "invalid_type".to_string(),
                message: "Manifest 顶层必须是对象".to_string(),
            }],
            manifest: None,
            manifest_hash: None,
            supported_contract_version: SUPPORTED_ADAPTER_CONTRACT_VERSION,
        };
    };

    const TOP_LEVEL_FIELDS: [&str; 11] = [
        "id",
        "name",
        "description",
        "version",
        "pluginType",
        "adapterContractVersion",
        "runtime",
        "entry",
        "permissions",
        "capabilities",
        "mcp",
    ];
    for key in object.keys() {
        if !TOP_LEVEL_FIELDS.contains(&key.as_str()) {
            issues.push(issue(
                key,
                "unknown_field",
                format!("未知 Manifest 字段 `{key}`"),
            ));
        }
    }

    validate_required_string(object, "id", &mut issues);
    validate_required_string(object, "name", &mut issues);
    validate_optional_string(object, "description", &mut issues);
    validate_required_string(object, "version", &mut issues);
    validate_required_string(object, "entry", &mut issues);

    if let Some(id) = object.get("id").and_then(Value::as_str)
        && !MANIFEST_ID_RE.is_match(id)
    {
        issues.push(issue(
            "id",
            "invalid_format",
            "id 必须为小写点分/短横线标识，例如 creatorrelay.test-adapter",
        ));
    }
    if let Some(version) = object.get("version").and_then(Value::as_str)
        && !SEMVER_RE.is_match(version)
    {
        issues.push(issue(
            "version",
            "invalid_semver",
            "version 必须为 SemVer，例如 1.0.0",
        ));
    }

    match object.get("pluginType").and_then(Value::as_str) {
        Some("adapter") => {}
        Some(other) => issues.push(issue(
            "pluginType",
            "unsupported_plugin_type",
            format!("不支持的插件类型 `{other}`，当前仅支持 adapter"),
        )),
        None => issues.push(issue(
            "pluginType",
            "missing_field",
            "缺少字段 `pluginType` 或类型不是字符串",
        )),
    }

    match object.get("adapterContractVersion") {
        Some(Value::Number(number)) => match number.as_u64() {
            Some(version) if version == SUPPORTED_ADAPTER_CONTRACT_VERSION as u64 => {}
            Some(version) => issues.push(issue(
                "adapterContractVersion",
                "unsupported_contract_version",
                format!(
                    "Adapter 合约版本 {version} 不受支持；当前只支持 {SUPPORTED_ADAPTER_CONTRACT_VERSION}"
                ),
            )),
            None => issues.push(issue(
                "adapterContractVersion",
                "invalid_type",
                "adapterContractVersion 必须是非负整数",
            )),
        },
        Some(_) => issues.push(issue(
            "adapterContractVersion",
            "invalid_type",
            "adapterContractVersion 必须是整数",
        )),
        None => issues.push(issue(
            "adapterContractVersion",
            "missing_field",
            "缺少字段 `adapterContractVersion`",
        )),
    }

    validate_runtime(object.get("runtime"), &mut issues);
    validate_permissions(object.get("permissions"), &mut issues);
    validate_capabilities(object.get("capabilities"), &mut issues);
    validate_mcp(object.get("mcp"), &mut issues);

    if !issues.is_empty() {
        return ManifestValidation {
            valid: false,
            issues,
            manifest: None,
            manifest_hash: None,
            supported_contract_version: SUPPORTED_ADAPTER_CONTRACT_VERSION,
        };
    }

    match serde_json::from_str::<AdapterManifest>(raw) {
        Ok(manifest) => ManifestValidation {
            valid: true,
            issues: Vec::new(),
            manifest: Some(manifest),
            manifest_hash: Some(manifest_hash(raw)),
            supported_contract_version: SUPPORTED_ADAPTER_CONTRACT_VERSION,
        },
        Err(error) => ManifestValidation {
            valid: false,
            issues: vec![issue(
                "$",
                "schema_error",
                format!("Manifest 结构不符合合约：{error}"),
            )],
            manifest: None,
            manifest_hash: None,
            supported_contract_version: SUPPORTED_ADAPTER_CONTRACT_VERSION,
        },
    }
}

fn validate_required_string(
    object: &Map<String, Value>,
    field: &str,
    issues: &mut Vec<ManifestIssue>,
) {
    match object.get(field) {
        Some(Value::String(value)) if !value.trim().is_empty() => {
            if value.len() > 200 {
                issues.push(issue(field, "too_long", "字段长度不能超过 200 个字符"));
            }
        }
        Some(Value::String(_)) => issues.push(issue(field, "empty", "字段不能为空")),
        Some(_) => issues.push(issue(field, "invalid_type", "字段必须是字符串")),
        None => issues.push(issue(field, "missing_field", format!("缺少字段 `{field}`"))),
    }
}

fn validate_optional_string(
    object: &Map<String, Value>,
    field: &str,
    issues: &mut Vec<ManifestIssue>,
) {
    match object.get(field) {
        None | Some(Value::Null) => {}
        Some(Value::String(value)) if value.len() <= 500 => {}
        Some(Value::String(_)) => {
            issues.push(issue(field, "too_long", "字段长度不能超过 500 个字符"))
        }
        Some(_) => issues.push(issue(field, "invalid_type", "字段必须是字符串")),
    }
}

fn validate_runtime(value: Option<&Value>, issues: &mut Vec<ManifestIssue>) {
    let Some(value) = value else {
        issues.push(issue("runtime", "missing_field", "缺少字段 `runtime`"));
        return;
    };
    let Some(object) = value.as_object() else {
        issues.push(issue("runtime", "invalid_type", "runtime 必须是对象"));
        return;
    };
    for key in object.keys() {
        if !["kind", "profile", "timeoutMs"].contains(&key.as_str()) {
            issues.push(issue(
                &format!("runtime.{key}"),
                "unknown_field",
                format!("未知 runtime 字段 `{key}`"),
            ));
        }
    }
    match object.get("kind").and_then(Value::as_str) {
        Some("coreTestRunner") | Some("executable") => {}
        Some(other) => issues.push(issue(
            "runtime.kind",
            "unsupported_runtime",
            format!("不支持的 runtime.kind `{other}`"),
        )),
        None => issues.push(issue("runtime.kind", "missing_field", "缺少 runtime.kind")),
    }
    if let Some(profile) = object.get("profile")
        && !profile.is_null()
        && (profile.as_str().is_none() || profile.as_str().is_some_and(str::is_empty))
    {
        issues.push(issue(
            "runtime.profile",
            "invalid_type",
            "runtime.profile 必须是非空字符串",
        ));
    }
    match object.get("timeoutMs") {
        None => {}
        Some(Value::Number(number)) => match number.as_u64() {
            Some(timeout) if (100..=120_000).contains(&timeout) => {}
            _ => issues.push(issue(
                "runtime.timeoutMs",
                "out_of_range",
                "runtime.timeoutMs 必须在 100 到 120000 之间",
            )),
        },
        Some(_) => issues.push(issue(
            "runtime.timeoutMs",
            "invalid_type",
            "runtime.timeoutMs 必须是整数",
        )),
    }
}

fn validate_permissions(value: Option<&Value>, issues: &mut Vec<ManifestIssue>) {
    let Some(value) = value else {
        issues.push(issue(
            "permissions",
            "missing_field",
            "缺少字段 `permissions`",
        ));
        return;
    };
    let Some(items) = value.as_array() else {
        issues.push(issue(
            "permissions",
            "invalid_type",
            "permissions 必须是数组",
        ));
        return;
    };
    let mut seen = std::collections::HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let field = format!("permissions[{index}]");
        let Some(object) = item.as_object() else {
            issues.push(issue(&field, "invalid_type", "权限项必须是对象"));
            continue;
        };
        for key in object.keys() {
            if !["id", "required", "reason"].contains(&key.as_str()) {
                issues.push(issue(
                    &format!("{field}.{key}"),
                    "unknown_field",
                    format!("未知权限字段 `{key}`"),
                ));
            }
        }
        let Some(id) = object.get("id").and_then(Value::as_str) else {
            issues.push(issue(
                &format!("{field}.id"),
                "missing_field",
                "权限项缺少 id",
            ));
            continue;
        };
        if permission_definition(id).is_none() {
            issues.push(issue(
                &format!("{field}.id"),
                "unsupported_permission",
                format!("未知或不支持的权限 `{id}`"),
            ));
        }
        if !seen.insert(id.to_string()) {
            issues.push(issue(
                &format!("{field}.id"),
                "duplicate_permission",
                format!("权限 `{id}` 重复声明"),
            ));
        }
        match object.get("required") {
            Some(Value::Bool(_)) => {}
            Some(_) => issues.push(issue(
                &format!("{field}.required"),
                "invalid_type",
                "required 必须是布尔值",
            )),
            None => issues.push(issue(
                &format!("{field}.required"),
                "missing_field",
                "权限项缺少 required",
            )),
        }
        match object.get("reason").and_then(Value::as_str) {
            Some(reason) if !reason.trim().is_empty() && reason.len() <= 200 => {}
            Some(_) => issues.push(issue(
                &format!("{field}.reason"),
                "invalid_value",
                "权限说明必须为 1 至 200 个字符",
            )),
            None => issues.push(issue(
                &format!("{field}.reason"),
                "missing_field",
                "权限项缺少 reason",
            )),
        }
    }
}

fn validate_capabilities(value: Option<&Value>, issues: &mut Vec<ManifestIssue>) {
    let Some(value) = value else {
        issues.push(issue(
            "capabilities",
            "missing_field",
            "缺少字段 `capabilities`",
        ));
        return;
    };
    let Some(items) = value.as_array() else {
        issues.push(issue(
            "capabilities",
            "invalid_type",
            "capabilities 必须是数组",
        ));
        return;
    };
    let mut seen = std::collections::HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let field = format!("capabilities[{index}]");
        let Some(object) = item.as_object() else {
            issues.push(issue(&field, "invalid_type", "能力项必须是对象"));
            continue;
        };
        for key in object.keys() {
            if !["kind", "read", "publish", "note"].contains(&key.as_str()) {
                issues.push(issue(
                    &format!("{field}.{key}"),
                    "unknown_field",
                    format!("未知能力字段 `{key}`"),
                ));
            }
        }
        let Some(kind) = object.get("kind").and_then(Value::as_str) else {
            issues.push(issue(
                &format!("{field}.kind"),
                "missing_field",
                "能力项缺少 kind",
            ));
            continue;
        };
        if ContentKind::parse(kind).is_none() {
            issues.push(issue(
                &format!("{field}.kind"),
                "unsupported_content_kind",
                format!("未知内容形态 `{kind}`"),
            ));
        } else if !seen.insert(kind.to_string()) {
            issues.push(issue(
                &format!("{field}.kind"),
                "duplicate_content_kind",
                format!("内容形态 `{kind}` 重复声明"),
            ));
        }
        validate_capability_level(object.get("read"), &format!("{field}.read"), issues);
        validate_capability_level(object.get("publish"), &format!("{field}.publish"), issues);
        if let Some(note) = object.get("note")
            && !note.is_null()
            && (note.as_str().is_none() || note.as_str().is_some_and(|text| text.len() > 300))
        {
            issues.push(issue(
                &format!("{field}.note"),
                "invalid_value",
                "note 必须是不超过 300 个字符的字符串",
            ));
        }
    }
    for kind in CONTENT_KINDS {
        if !seen.contains(kind.as_str()) {
            issues.push(issue(
                "capabilities",
                "missing_content_kind",
                format!(
                    "能力声明缺少内容形态 `{}`（{}）",
                    kind.as_str(),
                    kind.label()
                ),
            ));
        }
    }
}

fn validate_capability_level(value: Option<&Value>, field: &str, issues: &mut Vec<ManifestIssue>) {
    match value.and_then(Value::as_str) {
        Some(level)
            if CapabilityLevel::parse(level).is_some_and(CapabilityLevel::is_declarable) => {}
        Some(level) if CapabilityLevel::parse(level).is_some() => issues.push(issue(
            field,
            "invalid_declaration",
            format!("能力状态 `{level}` 只能由运行时探测产生，不能由 Manifest 直接声明"),
        )),
        Some(level) => issues.push(issue(
            field,
            "unsupported_capability",
            format!("未知能力状态 `{level}`"),
        )),
        None => issues.push(issue(field, "missing_field", "能力状态必须是字符串")),
    }
}

fn validate_mcp(value: Option<&Value>, issues: &mut Vec<ManifestIssue>) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() {
        return;
    }
    let Some(object) = value.as_object() else {
        issues.push(issue("mcp", "invalid_type", "mcp 必须是对象或 null"));
        return;
    };
    for key in object.keys() {
        if !["transport", "command", "args"].contains(&key.as_str()) {
            issues.push(issue(
                &format!("mcp.{key}"),
                "unknown_field",
                format!("未知 mcp 字段 `{key}`"),
            ));
        }
    }
    match object.get("transport").and_then(Value::as_str) {
        Some("stdio") | Some("http") => {}
        Some(other) => issues.push(issue(
            "mcp.transport",
            "unsupported_transport",
            format!("不支持的 MCP transport `{other}`"),
        )),
        None => issues.push(issue(
            "mcp.transport",
            "missing_field",
            "mcp.transport 必须是 stdio 或 http",
        )),
    }
    if let Some(command) = object.get("command")
        && !command.is_null()
        && command.as_str().is_none()
    {
        issues.push(issue(
            "mcp.command",
            "invalid_type",
            "mcp.command 必须是字符串",
        ));
    }
    if let Some(args) = object.get("args")
        && !args.is_null()
        && (args.as_array().is_none()
            || !args
                .as_array()
                .is_some_and(|list| list.iter().all(Value::is_string)))
    {
        issues.push(issue(
            "mcp.args",
            "invalid_type",
            "mcp.args 必须是字符串数组",
        ));
    }
}

fn issue(field: &str, code: &str, message: impl Into<String>) -> ManifestIssue {
    ManifestIssue {
        field: field.to_string(),
        code: code.to_string(),
        message: message.into(),
    }
}

pub fn semver_parts(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn valid_manifest() -> String {
        r#"{
          "id": "creatorrelay.test-adapter",
          "name": "测试适配器",
          "description": "M4 测试",
          "version": "1.0.0",
          "pluginType": "adapter",
          "adapterContractVersion": 1,
          "runtime": { "kind": "coreTestRunner", "profile": "healthy", "timeoutMs": 1000 },
          "entry": "builtin://creatorrelay.test-adapter",
          "permissions": [
            { "id": "account.credentials", "required": true, "reason": "验证账号代理" },
            { "id": "network", "required": false, "reason": "验证网络授权" }
          ],
          "capabilities": [
            { "kind": "video", "read": "unsupported", "publish": "unsupported" },
            { "kind": "imageText", "read": "supported", "publish": "supported" },
            { "kind": "imageArticle", "read": "supported", "publish": "restricted" },
            { "kind": "article", "read": "supported", "publish": "supported" }
          ],
          "mcp": { "transport": "stdio", "command": "test-adapter", "args": [] }
        }"#
        .to_string()
    }

    #[test]
    fn m4_ac_01_accepts_valid_manifest() {
        let result = validate_manifest_json(&valid_manifest());
        assert!(result.valid, "{:?}", result.issues);
        let manifest = result.manifest.unwrap();
        assert_eq!(manifest.id, "creatorrelay.test-adapter");
        assert_eq!(manifest.adapter_contract_version, 1);
        assert!(result.manifest_hash.is_some());
    }

    #[test]
    fn m4_ac_01_rejects_missing_unknown_and_illegal_fields_with_field_paths() {
        let missing: Value = serde_json::from_str(&valid_manifest()).unwrap();
        let mut missing = missing;
        missing.as_object_mut().unwrap().remove("name");
        let result = validate_manifest_json(&missing.to_string());
        assert!(!result.valid);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.field == "name" && issue.code == "missing_field")
        );

        let unknown: Value = serde_json::from_str(&valid_manifest()).unwrap();
        let mut unknown = unknown;
        unknown
            .as_object_mut()
            .unwrap()
            .insert("surprise".to_string(), Value::Bool(true));
        let result = validate_manifest_json(&unknown.to_string());
        assert!(!result.valid);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.field == "surprise" && issue.code == "unknown_field")
        );

        let mut illegal: Value = serde_json::from_str(&valid_manifest()).unwrap();
        illegal.as_object_mut().unwrap().insert(
            "adapterContractVersion".to_string(),
            Value::Number(99.into()),
        );
        illegal["capabilities"][0]["publish"] = Value::String("superpower".to_string());
        let result = validate_manifest_json(&illegal.to_string());
        assert!(!result.valid);
        assert!(result.issues.iter().any(|issue| {
            issue.field == "adapterContractVersion" && issue.code == "unsupported_contract_version"
        }));
        assert!(result.issues.iter().any(|issue| {
            issue.field == "capabilities[0].publish" && issue.code == "unsupported_capability"
        }));
    }

    #[test]
    fn m4_ac_07_rejects_unsupported_contract_and_invalid_semver() {
        let mut value: Value = serde_json::from_str(&valid_manifest()).unwrap();
        value["adapterContractVersion"] = Value::Number(2.into());
        let result = validate_manifest_json(&value.to_string());
        assert!(!result.valid);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == "unsupported_contract_version")
        );

        let mut value: Value = serde_json::from_str(&valid_manifest()).unwrap();
        value["version"] = Value::String("one".to_string());
        let result = validate_manifest_json(&value.to_string());
        assert!(!result.valid);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.field == "version" && issue.code == "invalid_semver")
        );
    }

    #[test]
    fn m4_ac_10_requires_every_content_kind() {
        let mut value: Value = serde_json::from_str(&valid_manifest()).unwrap();
        value["capabilities"].as_array_mut().unwrap().pop();
        let result = validate_manifest_json(&value.to_string());
        assert!(!result.valid);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == "missing_content_kind")
        );
    }
}
