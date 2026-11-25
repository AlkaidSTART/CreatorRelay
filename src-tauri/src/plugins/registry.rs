use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
};

use super::{
    manifest::{
        AdapterManifest, CONTENT_KINDS, CapabilityDeclaration, CapabilityLevel, ContentKind,
        ManifestValidation, PermissionRequest, SUPPORTED_ADAPTER_CONTRACT_VERSION,
        permission_definition, validate_manifest_json,
    },
    publish::{
        PublishInput, PublishIssue, PublishValidationResult, publish_payload_self_check,
        validate_publish_payload,
    },
    runtime::{HostAccessObservation, IsolationSummary, PluginRuntime, SecuritySelfTestResult},
};

pub const EMBEDDED_MANIFEST_SOURCES: [&str; 6] = [
    include_str!("../../resources/plugins/test-adapter/manifest.json"),
    include_str!("../../resources/plugins/video-adapter/manifest.json"),
    include_str!("../../resources/plugins/sync-adapter/manifest.json"),
    include_str!("../../resources/plugins/timeout-adapter/manifest.json"),
    include_str!("../../resources/plugins/fault-adapter/manifest.json"),
    include_str!("../../resources/plugins/probe-failure-adapter/manifest.json"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PluginStatus {
    ReviewRequired,
    Ready,
    Enabled,
    Disabled,
    Error,
}

impl PluginStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::ReviewRequired => "REVIEW_REQUIRED",
            Self::Ready => "READY",
            Self::Enabled => "ENABLED",
            Self::Disabled => "DISABLED",
            Self::Error => "ERROR",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "REVIEW_REQUIRED" => Ok(Self::ReviewRequired),
            "READY" => Ok(Self::Ready),
            "ENABLED" => Ok(Self::Enabled),
            "DISABLED" => Ok(Self::Disabled),
            "ERROR" => Ok(Self::Error),
            _ => Err(format!("未知插件状态：{value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSourceKind {
    Bundled,
    Local,
}

impl PluginSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "BUNDLED",
            Self::Local => "LOCAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    Official,
    LocalUntrusted,
}

impl TrustLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Official => "OFFICIAL",
            Self::LocalUntrusted => "LOCAL_UNTRUSTED",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "OFFICIAL" => Ok(Self::Official),
            "LOCAL_UNTRUSTED" => Ok(Self::LocalUntrusted),
            _ => Err(format!("未知插件信任级别：{value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPermissionView {
    pub id: String,
    pub label: String,
    pub risk: String,
    pub description: String,
    pub reason: String,
    pub required: bool,
    pub granted: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityStateView {
    pub kind: ContentKind,
    pub label: String,
    pub declared_read: CapabilityLevel,
    pub declared_publish: CapabilityLevel,
    pub effective_read: CapabilityLevel,
    pub effective_publish: CapabilityLevel,
    pub source: String,
    pub reason: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterCatalogEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub adapter_contract_version: u32,
    pub plugin_type: String,
    pub source: String,
    pub trust_level: String,
    pub permissions: Vec<PluginPermissionView>,
    pub capabilities: Vec<CapabilityStateView>,
    pub installed: bool,
    pub installable: bool,
    pub blocked_reason: Option<String>,
    pub isolation: IsolationSummary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub adapter_contract_version: u32,
    pub plugin_type: String,
    pub runtime_kind: String,
    pub runtime_profile: Option<String>,
    pub entry: String,
    pub source: String,
    pub trust_level: String,
    pub status: PluginStatus,
    pub enabled: bool,
    pub runtime_state: String,
    pub sandboxed: bool,
    pub isolation: IsolationSummary,
    pub manifest_hash: String,
    pub install_path: String,
    pub installed_at: String,
    pub updated_at: String,
    pub last_error: Option<String>,
    pub previous_version: Option<String>,
    pub last_probe_at: Option<String>,
    pub permissions: Vec<PluginPermissionView>,
    pub capabilities: Vec<CapabilityStateView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginProbeResult {
    pub plugin_id: String,
    pub account_ref: String,
    pub ok: bool,
    pub error: Option<String>,
    pub checked_at: String,
    pub sandboxed: bool,
    pub host_access: Vec<HostAccessObservation>,
    pub capabilities: Vec<CapabilityStateView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginFaultResult {
    pub plugin_id: String,
    pub fault: String,
    pub error: String,
    pub status: PluginStatus,
    pub runtime_state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUninstallResult {
    pub plugin_id: String,
    pub deleted_plugin_files: bool,
    pub deleted_plugin_data: bool,
    pub retained_events: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginEvent {
    pub id: i64,
    pub plugin_id: String,
    pub plugin_version: String,
    pub action: String,
    pub detail: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPermissionDecision {
    pub id: String,
    pub granted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPublishValidationResult {
    pub ok: bool,
    pub payload: PublishValidationResult,
    pub capability: Option<CapabilityLevel>,
    pub adapter_checked: bool,
    pub adapter_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginCapabilityStatus {
    pub level: CapabilityLevel,
    pub reason: Option<String>,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterPublishOutput {
    pub external_id: String,
    pub external_url: Option<String>,
    pub content_kind: ContentKind,
    pub media_order: Vec<u32>,
    pub cover_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSyncMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSyncWork {
    pub external_id: String,
    pub title: String,
    pub content_type: String,
    pub content_kind: Option<ContentKind>,
    pub url: Option<String>,
    pub published_at: String,
    pub captured_at: Option<String>,
    pub metrics: Vec<AdapterSyncMetric>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterSyncOutput {
    pub works: Vec<AdapterSyncWork>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AdapterOperationError {
    pub code: String,
    pub message: String,
    pub uncertain: bool,
}

impl std::fmt::Display for AdapterOperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

#[derive(Debug, Clone)]
struct PluginRow {
    id: String,
    name: String,
    version: String,
    adapter_contract_version: u32,
    runtime_kind: String,
    runtime_profile: Option<String>,
    entry: String,
    source: String,
    trust_level: TrustLevel,
    status: PluginStatus,
    enabled: bool,
    manifest_json: String,
    manifest_hash: String,
    previous_manifest_json: Option<String>,
    installed_at: String,
    updated_at: String,
    last_error: Option<String>,
    install_path: String,
    manifest: AdapterManifest,
    previous_manifest: Option<AdapterManifest>,
}

#[derive(Debug, Clone)]
struct CapabilityCheck {
    read: CapabilityLevel,
    publish: CapabilityLevel,
    reason: Option<String>,
    checked_at: String,
}

pub fn embedded_manifests() -> Vec<AdapterManifest> {
    EMBEDDED_MANIFEST_SOURCES
        .iter()
        .map(|raw| {
            let validation = validate_manifest_json(raw);
            validation
                .manifest
                .unwrap_or_else(|| panic!("内置 Manifest 不合法：{:?}", validation.issues))
        })
        .collect()
}

pub fn validate_manifest(raw: &str) -> ManifestValidation {
    validate_manifest_json(raw)
}

pub fn list_catalog(
    conn: &Connection,
    runtime: &PluginRuntime,
) -> Result<Vec<AdapterCatalogEntry>, String> {
    embedded_manifests()
        .into_iter()
        .map(|manifest| {
            let installed = plugin_row(conn, &manifest.id)?.is_some();
            let isolation = runtime.isolation_summary(&manifest);
            Ok(AdapterCatalogEntry {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                description: manifest.description.clone().unwrap_or_default(),
                version: manifest.version.clone(),
                adapter_contract_version: manifest.adapter_contract_version,
                plugin_type: manifest.plugin_type.as_str().to_string(),
                source: PluginSourceKind::Bundled.as_str().to_string(),
                trust_level: TrustLevel::Official.as_str().to_string(),
                permissions: permission_views(&manifest, &HashMap::new()),
                capabilities: capability_views(&manifest, &HashMap::new()),
                installed,
                installable: !installed
                    && manifest.adapter_contract_version == SUPPORTED_ADAPTER_CONTRACT_VERSION,
                blocked_reason: if installed {
                    Some("该官方适配器已安装".to_string())
                } else {
                    None
                },
                isolation,
            })
        })
        .collect()
}

pub fn list_plugins(
    conn: &Connection,
    runtime: &PluginRuntime,
) -> Result<Vec<PluginSummary>, String> {
    let rows = plugin_rows(conn)?;
    rows.into_iter()
        .map(|row| plugin_summary(conn, runtime, &row))
        .collect()
}

pub fn install_official_plugin(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
) -> Result<PluginSummary, String> {
    let source = EMBEDDED_MANIFEST_SOURCES
        .iter()
        .find_map(|raw| {
            let validation = validate_manifest_json(raw);
            validation
                .manifest
                .filter(|manifest| manifest.id == plugin_id)
                .map(|_| *raw)
        })
        .ok_or_else(|| format!("未找到官方适配器 `{plugin_id}`"))?;
    install_manifest(
        conn,
        runtime,
        source,
        PluginSourceKind::Bundled,
        TrustLevel::Official,
    )
}

pub fn install_manifest(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    raw: &str,
    source: PluginSourceKind,
    trust: TrustLevel,
) -> Result<PluginSummary, String> {
    let validation = validate_manifest_json(raw);
    if !validation.valid {
        return Err(format_manifest_issues(&validation));
    }
    let manifest = validation
        .manifest
        .ok_or_else(|| "Manifest 校验后没有可用对象".to_string())?;
    if plugin_row(conn, &manifest.id)?.is_some() {
        return Err(format!(
            "适配器 `{}` 已安装，请先卸载或使用版本更新",
            manifest.id
        ));
    }
    let isolation = runtime.isolation_summary(&manifest);
    if trust != TrustLevel::Official && !isolation.strong {
        return Err(
            "当前平台没有强隔离能力，不能安装或启用非官方第三方代码；只能安装官方适配器"
                .to_string(),
        );
    }

    let plugin_dir = runtime.plugin_directory(&manifest.id)?;
    fs::create_dir_all(&plugin_dir).map_err(|error| format!("无法创建插件目录：{error}"))?;
    fs::write(plugin_dir.join("manifest.json"), raw)
        .map_err(|error| format!("无法写入插件 Manifest：{error}"))?;
    if let Err(error) = fs::create_dir_all(runtime.plugin_data_directory(&manifest.id)?) {
        let _ = fs::remove_dir_all(&plugin_dir);
        return Err(format!("无法创建插件数据目录：{error}"));
    }

    let now = Utc::now().to_rfc3339();
    let manifest_hash = validation.manifest_hash.unwrap_or_default();
    let transaction = conn
        .transaction()
        .map_err(|error| format!("无法开始插件安装事务：{error}"))?;
    transaction
        .execute(
            "INSERT INTO plugins(
                id, name, version, adapter_contract_version, runtime_kind, runtime_profile,
                entry, source, trust_level, status, enabled, manifest_json, manifest_hash,
                previous_manifest_json, previous_manifest_hash, installed_at, updated_at,
                last_error, install_path
            ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12, NULL, NULL, ?13, ?13, NULL, ?14)",
            params![
                manifest.id,
                manifest.name,
                manifest.version,
                manifest.adapter_contract_version,
                manifest.runtime.kind.as_str(),
                manifest.runtime.profile,
                manifest.entry,
                source.as_str(),
                trust.as_str(),
                PluginStatus::ReviewRequired.as_str(),
                raw,
                manifest_hash,
                now,
                plugin_dir.to_string_lossy().to_string(),
            ],
        )
        .map_err(|error| format!("无法写入插件注册表：{error}"))?;
    record_event_tx(
        &transaction,
        &manifest.id,
        &manifest.version,
        "INSTALLED",
        &format!(
            "source={} trust={} contract={}",
            source.as_str(),
            trust.as_str(),
            manifest.adapter_contract_version
        ),
    )?;
    transaction
        .commit()
        .map_err(|error| format!("无法提交插件安装：{error}"))?;
    let row = plugin_row(conn, &manifest.id)?.ok_or_else(|| "安装后未找到插件记录".to_string())?;
    plugin_summary(conn, runtime, &row)
}

pub fn update_plugin_manifest(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    raw: &str,
) -> Result<PluginSummary, String> {
    let validation = validate_manifest_json(raw);
    if !validation.valid {
        return Err(format_manifest_issues(&validation));
    }
    let manifest = validation
        .manifest
        .ok_or_else(|| "Manifest 校验后没有可用对象".to_string())?;
    let current = plugin_row(conn, &manifest.id)?
        .ok_or_else(|| format!("适配器 `{}` 尚未安装", manifest.id))?;
    if current.enabled || current.status == PluginStatus::Enabled {
        return Err("请先禁用适配器再更新或回退版本".to_string());
    }
    let plugin_dir = runtime.plugin_directory(&manifest.id)?;
    fs::write(plugin_dir.join("manifest.json"), raw)
        .map_err(|error| format!("无法写入新版本 Manifest：{error}"))?;
    migrate_grants(conn, &manifest.id, &current.manifest, &manifest)?;
    let now = Utc::now().to_rfc3339();
    let status = status_after_review(conn, &manifest)?;
    conn.execute(
        "UPDATE plugins SET
            name=?1, version=?2, adapter_contract_version=?3, runtime_kind=?4, runtime_profile=?5,
            entry=?6, status=?7, enabled=0, manifest_json=?8, manifest_hash=?9,
            previous_manifest_json=?10, previous_manifest_hash=?11, updated_at=?12, last_error=NULL
         WHERE id=?13",
        params![
            manifest.name,
            manifest.version,
            manifest.adapter_contract_version,
            manifest.runtime.kind.as_str(),
            manifest.runtime.profile,
            manifest.entry,
            status.as_str(),
            raw,
            validation.manifest_hash.unwrap_or_default(),
            current.manifest_json,
            current.manifest_hash,
            now,
            manifest.id,
        ],
    )
    .map_err(|error| format!("无法更新插件注册表：{error}"))?;
    record_event(
        conn,
        &manifest.id,
        &manifest.version,
        "VERSION_CHANGED",
        &format!(
            "from={} to={} contract={}",
            current.version, manifest.version, manifest.adapter_contract_version
        ),
    )?;
    let row = plugin_row(conn, &manifest.id)?.ok_or_else(|| "更新后未找到插件记录".to_string())?;
    plugin_summary(conn, runtime, &row)
}

pub fn rollback_plugin(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
) -> Result<PluginSummary, String> {
    let current =
        plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    if current.enabled || current.status == PluginStatus::Enabled {
        return Err("请先禁用适配器再回退版本".to_string());
    }
    let previous_json = current
        .previous_manifest_json
        .clone()
        .ok_or_else(|| "没有可回退的上一个版本".to_string())?;
    let validation = validate_manifest_json(&previous_json);
    if !validation.valid {
        return Err(format_manifest_issues(&validation));
    }
    let previous_manifest = validation
        .manifest
        .ok_or_else(|| "回退 Manifest 无效".to_string())?;
    if previous_manifest.id != current.id {
        return Err("回退 Manifest 的 id 与当前插件不一致".to_string());
    }
    let plugin_dir = runtime.plugin_directory(plugin_id)?;
    fs::write(plugin_dir.join("manifest.json"), &previous_json)
        .map_err(|error| format!("无法写入回退 Manifest：{error}"))?;
    migrate_grants(conn, plugin_id, &current.manifest, &previous_manifest)?;
    let now = Utc::now().to_rfc3339();
    let status = status_after_review(conn, &previous_manifest)?;
    conn.execute(
        "UPDATE plugins SET
            name=?1, version=?2, adapter_contract_version=?3, runtime_kind=?4, runtime_profile=?5,
            entry=?6, status=?7, enabled=0, manifest_json=?8, manifest_hash=?9,
            previous_manifest_json=?10, previous_manifest_hash=?11, updated_at=?12, last_error=NULL
         WHERE id=?13",
        params![
            previous_manifest.name,
            previous_manifest.version,
            previous_manifest.adapter_contract_version,
            previous_manifest.runtime.kind.as_str(),
            previous_manifest.runtime.profile,
            previous_manifest.entry,
            status.as_str(),
            previous_json,
            validation.manifest_hash.unwrap_or_default(),
            current.manifest_json,
            current.manifest_hash,
            now,
            plugin_id,
        ],
    )
    .map_err(|error| format!("无法回退插件注册表：{error}"))?;
    record_event(
        conn,
        plugin_id,
        &previous_manifest.version,
        "ROLLED_BACK",
        &format!(
            "from={} to={} contract={} grants=intersection-only",
            current.version, previous_manifest.version, previous_manifest.adapter_contract_version
        ),
    )?;
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| "回退后未找到插件记录".to_string())?;
    plugin_summary(conn, runtime, &row)
}

pub fn set_plugin_permissions(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    decisions: &[PluginPermissionDecision],
) -> Result<PluginSummary, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    if row.enabled || row.status == PluginStatus::Enabled {
        return Err("请先禁用适配器再修改权限".to_string());
    }
    let requested = row
        .manifest
        .permissions
        .iter()
        .map(|permission| permission.id.as_str())
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    let mut granted_ids = Vec::new();
    let transaction = conn
        .transaction()
        .map_err(|error| format!("无法开始权限事务：{error}"))?;
    for decision in decisions {
        if !requested.contains(decision.id.as_str()) {
            return Err(format!("Manifest 未申请权限 `{}`", decision.id));
        }
        if !seen.insert(decision.id.as_str()) {
            return Err(format!("权限 `{}` 重复提交", decision.id));
        }
        transaction
            .execute(
                "INSERT INTO plugin_grants(plugin_id, permission, granted, updated_at)
                 VALUES(?1, ?2, ?3, ?4)
                 ON CONFLICT(plugin_id, permission)
                 DO UPDATE SET granted=excluded.granted, updated_at=excluded.updated_at",
                params![
                    plugin_id,
                    decision.id,
                    decision.granted as i64,
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(|error| format!("无法保存权限决定：{error}"))?;
        if decision.granted {
            granted_ids.push(decision.id.clone());
        }
    }
    let status = status_after_review(&transaction, &row.manifest)?;
    transaction
        .execute(
            "UPDATE plugins SET status=?1, updated_at=?2 WHERE id=?3",
            params![status.as_str(), Utc::now().to_rfc3339(), plugin_id],
        )
        .map_err(|error| format!("无法更新权限审查状态：{error}"))?;
    record_event_tx(
        &transaction,
        plugin_id,
        &row.version,
        "PERMISSIONS_REVIEWED",
        &if granted_ids.is_empty() {
            "granted=none".to_string()
        } else {
            format!("granted={}", granted_ids.join(","))
        },
    )?;
    transaction
        .commit()
        .map_err(|error| format!("无法提交权限审查：{error}"))?;
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| "权限更新后未找到插件".to_string())?;
    plugin_summary(conn, runtime, &row)
}

pub fn enable_plugin(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
) -> Result<PluginSummary, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    if row.status == PluginStatus::Enabled && runtime.is_running(plugin_id) {
        return plugin_summary(conn, runtime, &row);
    }
    if row.adapter_contract_version != SUPPORTED_ADAPTER_CONTRACT_VERSION
        || row.manifest.adapter_contract_version != SUPPORTED_ADAPTER_CONTRACT_VERSION
    {
        return Err(format!(
            "Adapter 合约版本 {} 不受支持；当前支持 {}",
            row.adapter_contract_version, SUPPORTED_ADAPTER_CONTRACT_VERSION
        ));
    }
    let grants = load_grants(conn, plugin_id)?;
    if !permissions_complete(&row.manifest, &grants) {
        return Err("请先完成全部权限审查".to_string());
    }
    let missing_required = row
        .manifest
        .permissions
        .iter()
        .filter(|permission| permission.required)
        .filter(|permission| grants.get(&permission.id) != Some(&true))
        .map(|permission| permission.id.clone())
        .collect::<Vec<_>>();
    if !missing_required.is_empty() {
        return Err(format!(
            "必需权限未授权，不能启用：{}",
            missing_required.join("、")
        ));
    }
    let isolation = runtime.isolation_summary(&row.manifest);
    if row.trust_level != TrustLevel::Official && !isolation.strong {
        return Err("当前平台没有强隔离，不能启用非官方适配器".to_string());
    }

    runtime.ensure_running(&row.manifest, &granted_set(&grants))?;
    let workspace = active_workspace(conn);
    let probe = runtime.request(
        &row.manifest,
        "probe",
        json!({ "accountRef": "fixture-image-text", "requestHostAccess": true }),
        &granted_set(&grants),
        workspace,
        Some("fixture-image-text".to_string()),
    );
    let checked_at = Utc::now().to_rfc3339();
    match probe.and_then(|value| parse_probe_response(&row.manifest, value, &checked_at)) {
        Ok(parsed) => {
            persist_probe(conn, plugin_id, &parsed)?;
            conn.execute(
                "UPDATE plugins SET status=?1, enabled=1, last_error=NULL, updated_at=?2 WHERE id=?3",
                params![PluginStatus::Enabled.as_str(), Utc::now().to_rfc3339(), plugin_id],
            )
            .map_err(|error| format!("无法更新插件启用状态：{error}"))?;
            record_event(
                conn,
                plugin_id,
                &row.version,
                "ENABLED",
                &format!(
                    "contract={} isolation={} account={}",
                    row.manifest.adapter_contract_version, isolation.mode, parsed.account_ref
                ),
            )?;
            let row = plugin_row(conn, plugin_id)?.ok_or_else(|| "启用后未找到插件".to_string())?;
            plugin_summary(conn, runtime, &row)
        }
        Err(error) => {
            runtime.stop(plugin_id);
            persist_unavailable(conn, plugin_id, "fixture-image-text", &error, &checked_at)?;
            conn.execute(
                "UPDATE plugins SET status=?1, enabled=1, last_error=?2, updated_at=?3 WHERE id=?4",
                params![
                    PluginStatus::Error.as_str(),
                    error,
                    Utc::now().to_rfc3339(),
                    plugin_id
                ],
            )
            .map_err(|db_error| format!("无法记录插件启用失败：{db_error}"))?;
            record_event(conn, plugin_id, &row.version, "ENABLE_FAILED", &error)?;
            Err(error)
        }
    }
}

pub fn disable_plugin(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
) -> Result<PluginSummary, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    runtime.stop(plugin_id);
    conn.execute(
        "UPDATE plugins SET status=?1, enabled=0, updated_at=?2 WHERE id=?3",
        params![
            PluginStatus::Disabled.as_str(),
            Utc::now().to_rfc3339(),
            plugin_id
        ],
    )
    .map_err(|error| format!("无法停用插件：{error}"))?;
    record_event(conn, plugin_id, &row.version, "DISABLED", "process stopped")?;
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| "停用后未找到插件".to_string())?;
    plugin_summary(conn, runtime, &row)
}

pub fn restart_plugin(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
) -> Result<PluginSummary, String> {
    runtime.stop(plugin_id);
    enable_plugin(conn, runtime, plugin_id)
}

pub fn probe_plugin_capabilities(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    account_ref: Option<String>,
) -> Result<PluginProbeResult, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    let account_ref = account_ref
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "fixture-image-text".to_string());
    let grants = load_grants(conn, plugin_id)?;
    let was_running = runtime.is_running(plugin_id);
    let checked_at = Utc::now().to_rfc3339();
    let spawn_result = runtime.ensure_running(&row.manifest, &granted_set(&grants));
    if let Err(error) = spawn_result {
        persist_unavailable(conn, plugin_id, &account_ref, &error, &checked_at)?;
        record_event(conn, plugin_id, &row.version, "PROBE_FAILED", &error)?;
        return Ok(PluginProbeResult {
            plugin_id: plugin_id.to_string(),
            account_ref: account_ref.clone(),
            ok: false,
            error: Some(error),
            checked_at,
            sandboxed: runtime.isolation_summary(&row.manifest).strong,
            host_access: Vec::new(),
            capabilities: capability_views(
                &row.manifest,
                &load_capability_checks(conn, plugin_id, Some(&account_ref))?,
            ),
        });
    }
    let workspace = active_workspace(conn);
    let probe = runtime.request(
        &row.manifest,
        "probe",
        json!({ "accountRef": account_ref, "requestHostAccess": true }),
        &granted_set(&grants),
        workspace,
        Some(account_ref.clone()),
    );
    if !was_running && row.status != PluginStatus::Enabled {
        runtime.stop(plugin_id);
    }
    match probe.and_then(|value| parse_probe_response(&row.manifest, value, &checked_at)) {
        Ok(parsed) => {
            persist_probe(conn, plugin_id, &parsed)?;
            if row.status == PluginStatus::Enabled {
                conn.execute(
                    "UPDATE plugins SET last_error=NULL, updated_at=?1 WHERE id=?2",
                    params![Utc::now().to_rfc3339(), plugin_id],
                )
                .map_err(|error| format!("无法更新探测时间：{error}"))?;
            }
            record_event(
                conn,
                plugin_id,
                &row.version,
                "CAPABILITIES_PROBED",
                &format!(
                    "account={} supported={}",
                    parsed.account_ref,
                    parsed
                        .capabilities
                        .iter()
                        .filter(|capability| capability.publish == CapabilityLevel::Supported)
                        .map(|capability| capability.kind.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            )?;
            Ok(PluginProbeResult {
                plugin_id: plugin_id.to_string(),
                account_ref: parsed.account_ref.clone(),
                ok: true,
                error: None,
                checked_at,
                sandboxed: parsed.sandboxed,
                host_access: parsed.host_access,
                capabilities: capability_views(
                    &row.manifest,
                    &load_capability_checks(conn, plugin_id, Some(&parsed.account_ref))?,
                ),
            })
        }
        Err(error) => {
            runtime.stop(plugin_id);
            persist_unavailable(conn, plugin_id, &account_ref, &error, &checked_at)?;
            let status = if row.status == PluginStatus::Enabled {
                conn.execute(
                    "UPDATE plugins SET status=?1, last_error=?2, updated_at=?3 WHERE id=?4",
                    params![
                        PluginStatus::Error.as_str(),
                        error,
                        Utc::now().to_rfc3339(),
                        plugin_id
                    ],
                )
                .ok();
                PluginStatus::Error
            } else {
                row.status
            };
            record_event(conn, plugin_id, &row.version, "PROBE_FAILED", &error)?;
            let _ = status;
            Ok(PluginProbeResult {
                plugin_id: plugin_id.to_string(),
                account_ref: account_ref.clone(),
                ok: false,
                error: Some(error),
                checked_at,
                sandboxed: runtime.isolation_summary(&row.manifest).strong,
                host_access: Vec::new(),
                capabilities: capability_views(
                    &row.manifest,
                    &load_capability_checks(conn, plugin_id, Some(&account_ref))?,
                ),
            })
        }
    }
}

pub fn simulate_plugin_fault(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    fault: &str,
) -> Result<PluginFaultResult, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    let method = match fault {
        "crash" => "crash",
        "timeout" => "timeout",
        _ => return Err("故障类型必须是 crash 或 timeout".to_string()),
    };
    if !runtime.is_running(plugin_id) {
        return Err("插件进程未运行；请先启用插件".to_string());
    }
    let grants = load_grants(conn, plugin_id)?;
    let error = runtime
        .request(
            &row.manifest,
            method,
            json!({}),
            &granted_set(&grants),
            active_workspace(conn),
            None,
        )
        .err()
        .unwrap_or_else(|| "故障注入未产生错误".to_string());
    runtime.stop(plugin_id);
    conn.execute(
        "UPDATE plugins SET status=?1, last_error=?2, updated_at=?3 WHERE id=?4",
        params![
            PluginStatus::Error.as_str(),
            error,
            Utc::now().to_rfc3339(),
            plugin_id
        ],
    )
    .map_err(|db_error| format!("无法记录插件故障：{db_error}"))?;
    record_event(conn, plugin_id, &row.version, "FAULT_DETECTED", &error)?;
    Ok(PluginFaultResult {
        plugin_id: plugin_id.to_string(),
        fault: fault.to_string(),
        error,
        status: PluginStatus::Error,
        runtime_state: "stopped".to_string(),
    })
}

pub fn uninstall_plugin(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    delete_plugin_data: bool,
) -> Result<PluginUninstallResult, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    runtime.stop(plugin_id);
    let plugin_dir = runtime.plugin_directory(plugin_id)?;
    let data_dir = runtime.plugin_data_directory(plugin_id)?;
    if plugin_dir.exists() {
        fs::remove_dir_all(&plugin_dir).map_err(|error| format!("无法删除插件文件：{error}"))?;
    }
    let deleted_data = if delete_plugin_data && data_dir.exists() {
        fs::remove_dir_all(&data_dir).map_err(|error| format!("无法删除插件数据：{error}"))?;
        true
    } else {
        false
    };
    record_event(
        conn,
        plugin_id,
        &row.version,
        "UNINSTALLED",
        &format!("deleteData={delete_plugin_data}"),
    )?;
    conn.execute("DELETE FROM plugins WHERE id=?1", [plugin_id])
        .map_err(|error| format!("无法删除插件注册记录：{error}"))?;
    let retained_events: u64 =
        conn.query_row(
            "SELECT COUNT(*) FROM plugin_events WHERE plugin_id=?1",
            [plugin_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("无法读取插件审计事件：{error}"))? as u64;
    Ok(PluginUninstallResult {
        plugin_id: plugin_id.to_string(),
        deleted_plugin_files: true,
        deleted_plugin_data: deleted_data,
        retained_events,
        message: if delete_plugin_data {
            "已卸载插件并删除插件数据；审计事件保留".to_string()
        } else {
            "已卸载插件；插件数据按确认规则保留，审计事件保留".to_string()
        },
    })
}

pub fn list_plugin_events(
    conn: &Connection,
    plugin_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<PluginEvent>, String> {
    let limit = limit.unwrap_or(100).clamp(1, 500) as i64;
    let mut statement = if plugin_id.is_some() {
        conn.prepare(
            "SELECT id, plugin_id, plugin_version, action, detail, created_at
             FROM plugin_events WHERE plugin_id=?1 ORDER BY id DESC LIMIT ?2",
        )
        .map_err(|error| format!("无法读取插件事件：{error}"))?
    } else {
        conn.prepare(
            "SELECT id, plugin_id, plugin_version, action, detail, created_at
             FROM plugin_events ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|error| format!("无法读取插件事件：{error}"))?
    };
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(PluginEvent {
            id: row.get(0)?,
            plugin_id: row.get(1)?,
            plugin_version: row.get(2)?,
            action: row.get(3)?,
            detail: row.get(4)?,
            created_at: row.get(5)?,
        })
    };
    let rows = if let Some(plugin_id) = plugin_id {
        statement.query_map(params![plugin_id, limit], map_row)
    } else {
        statement.query_map(params![limit], map_row)
    }
    .map_err(|error| format!("无法读取插件事件：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法解析插件事件：{error}"))
}

pub fn security_self_test(
    conn: &Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    workspace_path: Option<PathBuf>,
) -> Result<SecuritySelfTestResult, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    let canary = runtime
        .data_directory()
        .join(format!(".m4-credential-canary-{}", uuid::Uuid::new_v4()));
    fs::write(&canary, b"test-only-credential-canary")
        .map_err(|error| format!("无法创建凭据测试哨兵：{error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&canary, fs::Permissions::from_mode(0o600));
    }
    let result = runtime.security_self_test(&row.manifest, workspace_path, &canary);
    let _ = fs::remove_file(&canary);
    result
}

pub fn validate_plugin_publish_input(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    account_ref: Option<String>,
    input: &PublishInput,
) -> Result<PluginPublishValidationResult, String> {
    let payload = validate_publish_payload(input);
    if !payload.ok {
        return Ok(PluginPublishValidationResult {
            ok: false,
            payload,
            capability: None,
            adapter_checked: false,
            adapter_message: None,
        });
    }
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    let account_ref = account_ref.unwrap_or_else(|| "fixture-image-text".to_string());
    let mut checks = load_capability_checks(conn, plugin_id, Some(&account_ref))?;
    if !checks.contains_key(&input.content_kind) {
        let _ = probe_plugin_capabilities(conn, runtime, plugin_id, Some(account_ref.clone()))?;
        checks = load_capability_checks(conn, plugin_id, Some(&account_ref))?;
    }
    let Some(check) = checks.get(&input.content_kind).cloned() else {
        return Ok(PluginPublishValidationResult {
            ok: false,
            payload,
            capability: Some(CapabilityLevel::Unavailable),
            adapter_checked: false,
            adapter_message: Some("适配器未返回该形态的能力状态".to_string()),
        });
    };
    if !check.publish.can_publish() {
        let mut payload_with_capability = payload;
        payload_with_capability.issues.push(PublishIssue {
            field: "contentKind".to_string(),
            code: "capability_not_available".to_string(),
            message: format!(
                "适配器对该账号的{}发布能力为 {}，Core 已拒绝调用平台",
                input.content_kind.label(),
                check.publish.as_str()
            ),
        });
        payload_with_capability.ok = false;
        return Ok(PluginPublishValidationResult {
            ok: false,
            payload: payload_with_capability,
            capability: Some(check.publish),
            adapter_checked: false,
            adapter_message: check.reason,
        });
    }
    if row.status != PluginStatus::Enabled || !runtime.is_running(plugin_id) {
        return Ok(PluginPublishValidationResult {
            ok: false,
            payload,
            capability: Some(check.publish),
            adapter_checked: false,
            adapter_message: Some("插件未启用，不能调用 Adapter 预检".to_string()),
        });
    }
    let grants = load_grants(conn, plugin_id)?;
    let adapter_result = runtime.request(
        &row.manifest,
        "validatePublish",
        json!({ "input": input }),
        &granted_set(&grants),
        active_workspace(conn),
        Some(account_ref),
    );
    match adapter_result {
        Ok(value) => Ok(PluginPublishValidationResult {
            ok: true,
            payload,
            capability: Some(check.publish),
            adapter_checked: true,
            adapter_message: value
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        Err(error) => {
            let mut payload_with_error = payload;
            payload_with_error.ok = false;
            payload_with_error.issues.push(PublishIssue {
                field: "adapter".to_string(),
                code: "adapter_rejected".to_string(),
                message: error.clone(),
            });
            Ok(PluginPublishValidationResult {
                ok: false,
                payload: payload_with_error,
                capability: Some(check.publish),
                adapter_checked: true,
                adapter_message: Some(error),
            })
        }
    }
}

pub fn plugin_publish_capability(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    account_ref: &str,
    content_kind: ContentKind,
) -> Result<PluginCapabilityStatus, String> {
    let row = plugin_row(conn, plugin_id)?.ok_or_else(|| format!("未找到插件 `{plugin_id}`"))?;
    let mut checks = load_capability_checks(conn, plugin_id, Some(account_ref))?;
    if !checks.contains_key(&content_kind) {
        let _ = probe_plugin_capabilities(
            conn,
            runtime,
            plugin_id,
            Some(account_ref.to_string()),
        )?;
        checks = load_capability_checks(conn, plugin_id, Some(account_ref))?;
    }
    let check = checks.get(&content_kind);
    let declaration = row.manifest.capability(content_kind);
    Ok(PluginCapabilityStatus {
        level: check
            .map(|check| check.publish)
            .or_else(|| declaration.map(|item| item.publish))
            .unwrap_or(CapabilityLevel::Unavailable),
        reason: check
            .and_then(|check| check.reason.clone())
            .or_else(|| declaration.and_then(|item| item.note.clone())),
        checked_at: check.map(|check| check.checked_at.clone()),
    })
}

pub fn execute_plugin_publish(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    account_ref: &str,
    input: &PublishInput,
) -> Result<AdapterPublishOutput, AdapterOperationError> {
    let validation = validate_plugin_publish_input(
        conn,
        runtime,
        plugin_id,
        Some(account_ref.to_string()),
        input,
    )
    .map_err(|message| AdapterOperationError {
        code: "ADAPTER_PREFLIGHT_FAILED".to_string(),
        message,
        uncertain: false,
    })?;
    if !validation.ok {
        return Err(AdapterOperationError {
            code: validation
                .payload
                .issues
                .first()
                .map(|issue| issue.code.clone())
                .unwrap_or_else(|| "ADAPTER_REJECTED".to_string()),
            message: validation
                .adapter_message
                .or_else(|| {
                    validation
                        .payload
                        .issues
                        .first()
                        .map(|issue| issue.message.clone())
                })
                .unwrap_or_else(|| "Adapter 拒绝发布载荷".to_string()),
            uncertain: false,
        });
    }
    let row = plugin_row(conn, plugin_id)
        .map_err(|message| AdapterOperationError {
            code: "PLUGIN_LOOKUP_FAILED".to_string(),
            message,
            uncertain: false,
        })?
        .ok_or_else(|| AdapterOperationError {
            code: "PLUGIN_NOT_FOUND".to_string(),
            message: format!("未找到插件 `{plugin_id}`"),
            uncertain: false,
        })?;
    if row.status != PluginStatus::Enabled || !runtime.is_running(plugin_id) {
        return Err(AdapterOperationError {
            code: "PLUGIN_NOT_ENABLED".to_string(),
            message: "插件未启用，不能执行外部发布".to_string(),
            uncertain: false,
        });
    }
    let grants = load_grants(conn, plugin_id).map_err(|message| AdapterOperationError {
        code: "PLUGIN_GRANTS_FAILED".to_string(),
        message,
        uncertain: false,
    })?;
    let value = runtime
        .request(
            &row.manifest,
            "publish",
            json!({ "input": input }),
            &granted_set(&grants),
            active_workspace(conn),
            Some(account_ref.to_string()),
        )
        .map_err(|message| {
            let uncertain = message.contains("超时");
            AdapterOperationError {
                code: if uncertain {
                    "RESULT_UNCERTAIN".to_string()
                } else {
                    "ADAPTER_PUBLISH_FAILED".to_string()
                },
                message,
                uncertain,
            }
        })?;
    parse_adapter_publish_output(&value, input.content_kind).map_err(|message| {
        AdapterOperationError {
            code: "INVALID_ADAPTER_RESPONSE".to_string(),
            message,
            uncertain: true,
        }
    })
}

fn parse_adapter_publish_output(
    value: &Value,
    content_kind: ContentKind,
) -> Result<AdapterPublishOutput, String> {
    let external_id = value
        .get("externalId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Adapter 发布结果缺少 externalId".to_string())?
        .to_string();
    let returned_kind = value
        .get("contentKind")
        .and_then(Value::as_str)
        .and_then(ContentKind::parse)
        .ok_or_else(|| "Adapter 发布结果缺少有效 contentKind".to_string())?;
    if returned_kind != content_kind {
        return Err(format!(
            "Adapter 发布结果内容形态不一致：期望 {}，实际 {}",
            content_kind.as_str(),
            returned_kind.as_str()
        ));
    }
    let media_order = value
        .get("submittedMedia")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("order").and_then(Value::as_u64))
                .filter_map(|order| u32::try_from(order).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let cover_asset_id = value
        .get("submittedCover")
        .and_then(|cover| cover.get("assetId"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(AdapterPublishOutput {
        external_id,
        external_url: value
            .get("externalUrl")
            .and_then(Value::as_str)
            .map(str::to_string),
        content_kind,
        media_order,
        cover_asset_id,
    })
}

pub fn execute_plugin_sync(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    plugin_id: &str,
    account_ref: &str,
    as_of: Option<&str>,
) -> Result<AdapterSyncOutput, AdapterOperationError> {
    let row = plugin_row(conn, plugin_id)
        .map_err(|message| AdapterOperationError {
            code: "PLUGIN_LOOKUP_FAILED".to_string(),
            message,
            uncertain: false,
        })?
        .ok_or_else(|| AdapterOperationError {
            code: "PLUGIN_NOT_FOUND".to_string(),
            message: format!("未找到插件 `{plugin_id}`"),
            uncertain: false,
        })?;
    if row.status != PluginStatus::Enabled || !runtime.is_running(plugin_id) {
        return Err(AdapterOperationError {
            code: "PLUGIN_NOT_ENABLED".to_string(),
            message: "插件未启用，不能执行平台作品同步".to_string(),
            uncertain: false,
        });
    }
    let grants = load_grants(conn, plugin_id).map_err(|message| AdapterOperationError {
        code: "PLUGIN_GRANTS_FAILED".to_string(),
        message,
        uncertain: false,
    })?;
    let value = runtime
        .request(
            &row.manifest,
            "syncWorks",
            json!({ "accountRef": account_ref, "asOf": as_of }),
            &granted_set(&grants),
            active_workspace(conn),
            Some(account_ref.to_string()),
        )
        .map_err(|message| {
            let uncertain = message.contains("超时");
            let account_invalid = message.starts_with("AUTH_EXPIRED");
            AdapterOperationError {
                code: if uncertain {
                    "SYNC_UNCERTAIN".to_string()
                } else if account_invalid {
                    "AUTH_EXPIRED".to_string()
                } else {
                    "ADAPTER_SYNC_FAILED".to_string()
                },
                message,
                uncertain,
            }
        })?;
    parse_adapter_sync_output(&value).map_err(|message| AdapterOperationError {
        code: "INVALID_ADAPTER_RESPONSE".to_string(),
        message,
        uncertain: false,
    })
}

fn parse_adapter_sync_output(value: &Value) -> Result<AdapterSyncOutput, String> {
    let works = value
        .get("works")
        .and_then(Value::as_array)
        .ok_or_else(|| "Adapter 同步结果缺少 works 数组".to_string())?;
    let mut parsed = Vec::new();
    for (index, work) in works.iter().enumerate() {
        let external_id = work
            .get("externalId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("works[{index}] 缺少 externalId"))?
            .to_string();
        let title = work
            .get("title")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("works[{index}] 缺少 title"))?
            .to_string();
        let content_type = work
            .get("contentType")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let content_kind = work
            .get("contentKind")
            .and_then(Value::as_str)
            .and_then(ContentKind::parse);
        let published_at = work
            .get("publishedAt")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("works[{index}] 缺少 publishedAt"))?
            .to_string();
        let metrics = work
            .get("metrics")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|metric| {
                        let name = metric.get("name").and_then(Value::as_str)?.to_string();
                        let value = metric.get("value").and_then(Value::as_f64)?;
                        Some(AdapterSyncMetric {
                            name,
                            value,
                            unit: metric
                                .get("unit")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                            definition: metric
                                .get("definition")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        parsed.push(AdapterSyncWork {
            external_id,
            title,
            content_type,
            content_kind,
            url: work.get("url").and_then(Value::as_str).map(str::to_string),
            published_at,
            captured_at: work
                .get("capturedAt")
                .and_then(Value::as_str)
                .map(str::to_string),
            metrics,
        });
    }
    let errors = value
        .get("errors")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(AdapterSyncOutput {
        works: parsed,
        errors,
    })
}

pub fn payload_self_check() -> Vec<super::publish::PublishPayloadCheckResult> {
    publish_payload_self_check()
}

fn plugin_rows(conn: &Connection) -> Result<Vec<PluginRow>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, name, version, adapter_contract_version, runtime_kind, runtime_profile,
                    entry, source, trust_level, status, enabled, manifest_json, manifest_hash,
                    previous_manifest_json, previous_manifest_hash, installed_at, updated_at,
                    last_error, install_path
             FROM plugins ORDER BY installed_at ASC",
        )
        .map_err(|error| format!("无法读取插件注册表：{error}"))?;
    let rows = statement
        .query_map([], plugin_row_from_sql)
        .map_err(|error| format!("无法读取插件注册表：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法解析插件注册表：{error}"))
}

fn plugin_row(conn: &Connection, plugin_id: &str) -> Result<Option<PluginRow>, String> {
    conn.query_row(
        "SELECT id, name, version, adapter_contract_version, runtime_kind, runtime_profile,
                entry, source, trust_level, status, enabled, manifest_json, manifest_hash,
                previous_manifest_json, previous_manifest_hash, installed_at, updated_at,
                last_error, install_path
         FROM plugins WHERE id=?1",
        [plugin_id],
        plugin_row_from_sql,
    )
    .optional()
    .map_err(|error| format!("无法读取插件 `{plugin_id}`：{error}"))
}

fn plugin_row_from_sql(row: &rusqlite::Row<'_>) -> rusqlite::Result<PluginRow> {
    let manifest_json: String = row.get(11)?;
    let manifest = serde_json::from_str::<AdapterManifest>(&manifest_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(11, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let previous_manifest_json: Option<String> = row.get(13)?;
    let previous_manifest = previous_manifest_json
        .as_deref()
        .and_then(|raw| serde_json::from_str::<AdapterManifest>(raw).ok());
    let status: String = row.get(9)?;
    let trust_level: String = row.get(8)?;
    Ok(PluginRow {
        id: row.get(0)?,
        name: row.get(1)?,
        version: row.get(2)?,
        adapter_contract_version: row.get::<_, i64>(3)? as u32,
        runtime_kind: row.get(4)?,
        runtime_profile: row.get(5)?,
        entry: row.get(6)?,
        source: row.get(7)?,
        trust_level: TrustLevel::parse(&trust_level).map_err(|error| {
            rusqlite::Error::InvalidColumnType(8, error, rusqlite::types::Type::Text)
        })?,
        status: PluginStatus::parse(&status).map_err(|error| {
            rusqlite::Error::InvalidColumnType(9, error, rusqlite::types::Type::Text)
        })?,
        enabled: row.get::<_, i64>(10)? != 0,
        manifest_json,
        manifest_hash: row.get(12)?,
        previous_manifest_json,
        installed_at: row.get(15)?,
        updated_at: row.get(16)?,
        last_error: row.get(17)?,
        install_path: row.get(18)?,
        manifest,
        previous_manifest,
    })
}

fn plugin_summary(
    conn: &Connection,
    runtime: &PluginRuntime,
    row: &PluginRow,
) -> Result<PluginSummary, String> {
    let grants = load_grants(conn, &row.id)?;
    let checks = load_capability_checks(conn, &row.id, None)?;
    let isolation = runtime.isolation_summary(&row.manifest);
    let runtime_state = if runtime.is_running(&row.id) {
        "running"
    } else {
        "stopped"
    };
    let last_probe_at = checks.values().map(|check| check.checked_at.clone()).max();
    Ok(PluginSummary {
        id: row.id.clone(),
        name: row.name.clone(),
        description: row.manifest.description.clone().unwrap_or_default(),
        version: row.version.clone(),
        adapter_contract_version: row.adapter_contract_version,
        plugin_type: row.manifest.plugin_type.as_str().to_string(),
        runtime_kind: row.runtime_kind.clone(),
        runtime_profile: row.runtime_profile.clone(),
        entry: row.entry.clone(),
        source: row.source.clone(),
        trust_level: row.trust_level.as_str().to_string(),
        status: row.status,
        enabled: row.enabled,
        runtime_state: runtime_state.to_string(),
        sandboxed: isolation.strong,
        isolation,
        manifest_hash: row.manifest_hash.clone(),
        install_path: row.install_path.clone(),
        installed_at: row.installed_at.clone(),
        updated_at: row.updated_at.clone(),
        last_error: row.last_error.clone(),
        previous_version: row
            .previous_manifest
            .as_ref()
            .map(|manifest| manifest.version.clone()),
        last_probe_at,
        permissions: permission_views(&row.manifest, &grants),
        capabilities: capability_views(&row.manifest, &checks),
    })
}

fn permission_views(
    manifest: &AdapterManifest,
    grants: &HashMap<String, bool>,
) -> Vec<PluginPermissionView> {
    manifest
        .permissions
        .iter()
        .map(|permission| {
            let definition = permission_definition(&permission.id);
            PluginPermissionView {
                id: permission.id.clone(),
                label: definition
                    .as_ref()
                    .map(|definition| definition.label.to_string())
                    .unwrap_or_else(|| permission.id.clone()),
                risk: definition
                    .as_ref()
                    .map(|definition| definition.risk.to_string())
                    .unwrap_or_else(|| "high".to_string()),
                description: definition
                    .as_ref()
                    .map(|definition| definition.description.to_string())
                    .unwrap_or_default(),
                reason: permission.reason.clone(),
                required: permission.required,
                granted: grants.get(&permission.id).copied(),
            }
        })
        .collect()
}

fn capability_views(
    manifest: &AdapterManifest,
    checks: &HashMap<ContentKind, CapabilityCheck>,
) -> Vec<CapabilityStateView> {
    CONTENT_KINDS
        .iter()
        .map(|kind| {
            let declaration =
                manifest
                    .capability(*kind)
                    .cloned()
                    .unwrap_or(CapabilityDeclaration {
                        kind: *kind,
                        read: CapabilityLevel::Unsupported,
                        publish: CapabilityLevel::Unsupported,
                        note: None,
                    });
            let check = checks.get(kind);
            CapabilityStateView {
                kind: *kind,
                label: kind.label().to_string(),
                declared_read: declaration.read,
                declared_publish: declaration.publish,
                effective_read: check.map(|check| check.read).unwrap_or(declaration.read),
                effective_publish: check
                    .map(|check| check.publish)
                    .unwrap_or(declaration.publish),
                source: if check.is_some() {
                    "probe".to_string()
                } else {
                    "declared".to_string()
                },
                reason: check
                    .and_then(|check| check.reason.clone())
                    .or(declaration.note),
                checked_at: check.map(|check| check.checked_at.clone()),
            }
        })
        .collect()
}

fn load_grants(conn: &Connection, plugin_id: &str) -> Result<HashMap<String, bool>, String> {
    let mut statement = conn
        .prepare("SELECT permission, granted FROM plugin_grants WHERE plugin_id=?1")
        .map_err(|error| format!("无法读取插件权限：{error}"))?;
    let rows = statement
        .query_map([plugin_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
        })
        .map_err(|error| format!("无法读取插件权限：{error}"))?;
    let mut grants = HashMap::new();
    for row in rows {
        let (permission, granted) = row.map_err(|error| format!("无法解析插件权限：{error}"))?;
        grants.insert(permission, granted);
    }
    Ok(grants)
}

fn load_capability_checks(
    conn: &Connection,
    plugin_id: &str,
    account_ref: Option<&str>,
) -> Result<HashMap<ContentKind, CapabilityCheck>, String> {
    let map_row = |row: &rusqlite::Row<'_>| {
        let kind: String = row.get(0)?;
        let read: String = row.get(1)?;
        let publish: String = row.get(2)?;
        Ok((
            kind,
            read,
            publish,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
        ))
    };
    let mut checks = HashMap::new();
    if let Some(account_ref) = account_ref {
        let mut statement = conn
            .prepare(
                "SELECT content_kind, read_state, publish_state, reason, checked_at
                 FROM plugin_capability_checks WHERE plugin_id=?1 AND account_ref=?2",
            )
            .map_err(|error| format!("无法读取能力探测结果：{error}"))?;
        let rows = statement
            .query_map(params![plugin_id, account_ref], map_row)
            .map_err(|error| format!("无法读取能力探测结果：{error}"))?;
        for row in rows {
            insert_capability_check(
                &mut checks,
                row.map_err(|error| format!("无法解析能力探测结果：{error}"))?,
            );
        }
    } else {
        let mut statement = conn
            .prepare(
                "SELECT content_kind, read_state, publish_state, reason, checked_at
                 FROM plugin_capability_checks WHERE plugin_id=?1",
            )
            .map_err(|error| format!("无法读取能力探测结果：{error}"))?;
        let rows = statement
            .query_map([plugin_id], map_row)
            .map_err(|error| format!("无法读取能力探测结果：{error}"))?;
        for row in rows {
            insert_capability_check(
                &mut checks,
                row.map_err(|error| format!("无法解析能力探测结果：{error}"))?,
            );
        }
    }
    Ok(checks)
}

fn insert_capability_check(
    checks: &mut HashMap<ContentKind, CapabilityCheck>,
    row: (String, String, String, Option<String>, String),
) {
    let (kind, read, publish, reason, checked_at) = row;
    let Some(kind) = ContentKind::parse(&kind) else {
        return;
    };
    if checks
        .get(&kind)
        .is_some_and(|existing| existing.checked_at >= checked_at)
    {
        return;
    }
    checks.insert(
        kind,
        CapabilityCheck {
            read: CapabilityLevel::parse(&read).unwrap_or(CapabilityLevel::Unavailable),
            publish: CapabilityLevel::parse(&publish).unwrap_or(CapabilityLevel::Unavailable),
            reason,
            checked_at,
        },
    );
}

fn permissions_complete(manifest: &AdapterManifest, grants: &HashMap<String, bool>) -> bool {
    manifest
        .permissions
        .iter()
        .all(|permission| grants.contains_key(&permission.id))
}

fn status_after_review(
    conn: &Connection,
    manifest: &AdapterManifest,
) -> Result<PluginStatus, String> {
    let grants = load_grants(conn, &manifest.id)?;
    if !permissions_complete(manifest, &grants) {
        return Ok(PluginStatus::ReviewRequired);
    }
    if manifest
        .permissions
        .iter()
        .filter(|permission| permission.required)
        .all(|permission| grants.get(&permission.id) == Some(&true))
    {
        Ok(PluginStatus::Ready)
    } else {
        Ok(PluginStatus::ReviewRequired)
    }
}

fn migrate_grants(
    conn: &mut Connection,
    plugin_id: &str,
    old_manifest: &AdapterManifest,
    new_manifest: &AdapterManifest,
) -> Result<(), String> {
    let old_permissions = old_manifest
        .permissions
        .iter()
        .map(|permission| permission.id.as_str())
        .collect::<HashSet<_>>();
    let new_permissions = new_manifest
        .permissions
        .iter()
        .map(|permission| permission.id.as_str())
        .collect::<HashSet<_>>();
    for permission in old_permissions.difference(&new_permissions) {
        conn.execute(
            "DELETE FROM plugin_grants WHERE plugin_id=?1 AND permission=?2",
            params![plugin_id, permission],
        )
        .map_err(|error| format!("无法迁移被移除权限：{error}"))?;
    }
    Ok(())
}

fn granted_set(grants: &HashMap<String, bool>) -> HashSet<String> {
    grants
        .iter()
        .filter(|(_, granted)| **granted)
        .map(|(permission, _)| permission.clone())
        .collect()
}

fn active_workspace(conn: &Connection) -> Option<PathBuf> {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key='workspace_path'",
        [],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .ok()
    .flatten()
    .map(PathBuf::from)
}

fn record_event(
    conn: &Connection,
    plugin_id: &str,
    plugin_version: &str,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO plugin_events(plugin_id, plugin_version, action, detail, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            plugin_id,
            plugin_version,
            action,
            detail,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| format!("无法写入插件事件：{error}"))?;
    Ok(())
}

fn record_event_tx(
    conn: &rusqlite::Transaction<'_>,
    plugin_id: &str,
    plugin_version: &str,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO plugin_events(plugin_id, plugin_version, action, detail, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5)",
        params![
            plugin_id,
            plugin_version,
            action,
            detail,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| format!("无法写入插件事件：{error}"))?;
    Ok(())
}

fn format_manifest_issues(validation: &ManifestValidation) -> String {
    validation
        .issues
        .iter()
        .map(|issue| format!("{} [{}] {}", issue.field, issue.code, issue.message))
        .collect::<Vec<_>>()
        .join("；")
}

#[derive(Debug, Clone)]
struct ProbedCapability {
    kind: ContentKind,
    read: CapabilityLevel,
    publish: CapabilityLevel,
    reason: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedProbe {
    account_ref: String,
    sandboxed: bool,
    host_access: Vec<HostAccessObservation>,
    capabilities: Vec<ProbedCapability>,
}

fn parse_probe_response(
    manifest: &AdapterManifest,
    value: Value,
    checked_at: &str,
) -> Result<ParsedProbe, String> {
    if value.get("health").and_then(Value::as_str) != Some("ok") {
        return Err("Adapter 健康检查未返回 ok".to_string());
    }
    let account_ref = value
        .get("accountRef")
        .and_then(Value::as_str)
        .unwrap_or("fixture-image-text")
        .to_string();
    let sandboxed = value
        .get("sandboxed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let host_access = value
        .get("hostAccess")
        .cloned()
        .and_then(|host| serde_json::from_value::<Vec<HostAccessObservation>>(host).ok())
        .unwrap_or_default();
    let capabilities = value
        .get("capabilities")
        .and_then(Value::as_array)
        .ok_or_else(|| "Adapter probe 缺少 capabilities 数组".to_string())?;
    let mut parsed = Vec::new();
    let mut seen = HashSet::new();
    for item in capabilities {
        let kind = item
            .get("kind")
            .and_then(Value::as_str)
            .and_then(ContentKind::parse)
            .ok_or_else(|| "Adapter probe 返回未知内容形态".to_string())?;
        if !seen.insert(kind) {
            return Err("Adapter probe 返回重复内容形态".to_string());
        }
        let read = item
            .get("read")
            .and_then(Value::as_str)
            .and_then(CapabilityLevel::parse)
            .ok_or_else(|| format!("Adapter probe 返回非法 read 状态：{kind:?}"))?;
        let publish = item
            .get("publish")
            .and_then(Value::as_str)
            .and_then(CapabilityLevel::parse)
            .ok_or_else(|| format!("Adapter probe 返回非法 publish 状态：{kind:?}"))?;
        parsed.push(ProbedCapability {
            kind,
            read,
            publish,
            reason: item
                .get("reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    for kind in CONTENT_KINDS {
        if !seen.contains(&kind) {
            return Err(format!(
                "Adapter probe 缺少内容形态 `{}`，不能仅复制 Manifest 声明",
                kind.as_str()
            ));
        }
    }
    // 运行时探测不能把 Manifest 中明确不支持的形态翻转为支持，也不能隐藏受限标记。
    for capability in &parsed {
        if let Some(declaration) = manifest.capability(capability.kind)
            && declaration.publish == CapabilityLevel::Unsupported
            && capability.publish != CapabilityLevel::Unsupported
        {
            return Err(format!(
                "Adapter probe 与 Manifest 冲突：{} 声明 unsupported 但探测为 {}",
                capability.kind.as_str(),
                capability.publish.as_str()
            ));
        }
    }
    let _ = checked_at;
    Ok(ParsedProbe {
        account_ref,
        sandboxed,
        host_access,
        capabilities: parsed,
    })
}

fn persist_probe(
    conn: &mut Connection,
    plugin_id: &str,
    parsed: &ParsedProbe,
) -> Result<(), String> {
    let transaction = conn
        .transaction()
        .map_err(|error| format!("无法开始能力结果事务：{error}"))?;
    let checked_at = Utc::now().to_rfc3339();
    transaction
        .execute(
            "DELETE FROM plugin_capability_checks WHERE plugin_id=?1 AND account_ref=?2",
            params![plugin_id, parsed.account_ref],
        )
        .map_err(|error| format!("无法刷新能力结果：{error}"))?;
    for capability in &parsed.capabilities {
        transaction
            .execute(
                "INSERT INTO plugin_capability_checks(
                    plugin_id, account_ref, content_kind, read_state, publish_state, reason, checked_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    plugin_id,
                    parsed.account_ref,
                    capability.kind.as_str(),
                    capability.read.as_str(),
                    capability.publish.as_str(),
                    capability.reason,
                    checked_at
                ],
            )
            .map_err(|error| format!("无法保存能力结果：{error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("无法提交能力结果：{error}"))
}

fn persist_unavailable(
    conn: &mut Connection,
    plugin_id: &str,
    account_ref: &str,
    reason: &str,
    checked_at: &str,
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM plugin_capability_checks WHERE plugin_id=?1 AND account_ref=?2",
        params![plugin_id, account_ref],
    )
    .map_err(|error| format!("无法刷新能力错误状态：{error}"))?;
    for kind in CONTENT_KINDS {
        conn.execute(
            "INSERT INTO plugin_capability_checks(
                plugin_id, account_ref, content_kind, read_state, publish_state, reason, checked_at
             ) VALUES(?1, ?2, ?3, 'unavailable', 'unavailable', ?4, ?5)",
            params![plugin_id, account_ref, kind.as_str(), reason, checked_at],
        )
        .map_err(|error| format!("无法保存能力错误状态：{error}"))?;
    }
    Ok(())
}

#[allow(dead_code)]
fn _assert_permission_request_type(_: &PermissionRequest) {}
