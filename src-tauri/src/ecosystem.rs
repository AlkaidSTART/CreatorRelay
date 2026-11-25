use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

use crate::plugins::{
    self, CapabilityLevel, ContentKind, ManifestValidation, SUPPORTED_ADAPTER_CONTRACT_VERSION,
    validate_manifest_json,
};

use chrono::Utc;

const ADMISSION_REGISTRY_JSON: &str =
    include_str!("../resources/ecosystem/platform-admissions.json");
const MARKETPLACE_POLICY_JSON: &str =
    include_str!("../resources/ecosystem/marketplace-policy.json");
const EXPECTED_GATE_IDS: [&str; 6] = [
    "threat_assessment",
    "source_signature",
    "strong_isolation",
    "update_rollback",
    "permission_revocation",
    "emergency_takedown",
];

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdmissionRegistry {
    pub schema_version: u32,
    pub policy_version: String,
    pub reviewed_at: String,
    pub status_semantics: StatusSemantics,
    pub platforms: Vec<PlatformAdmission>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusSemantics {
    pub supported: String,
    pub restricted: String,
    pub experimental: String,
    pub unsupported: String,
    pub unavailable: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformAdmission {
    pub platform_id: String,
    pub platform_name: String,
    pub adapter_contract_version: u32,
    pub admission_status: String,
    pub auto_publish_allowed: bool,
    pub publish_dispatch: String,
    pub account_types: Vec<String>,
    pub reviewed_at: String,
    pub official_sources: Vec<OfficialSource>,
    pub content_kinds: Vec<ContentKindCapability>,
    pub unsupported: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OfficialSource {
    pub title: String,
    pub url: String,
    pub http_status: Option<u16>,
    pub checked_at: String,
    pub result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentKindCapability {
    pub kind: ContentKind,
    pub label: String,
    pub read: CapabilityAssessment,
    pub publish: CapabilityAssessment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityAssessment {
    pub level: CapabilityLevel,
    pub evidence: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveCapability {
    pub kind: ContentKind,
    pub label: String,
    pub read: CapabilityLevel,
    pub publish: CapabilityLevel,
    pub read_evidence: String,
    pub publish_evidence: String,
    pub auto_publish_allowed: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCapabilitySnapshot {
    pub account_id: String,
    pub account_alias: String,
    pub platform_id: String,
    pub platform_name: String,
    pub account_status: String,
    pub admission_status: String,
    pub auto_publish_allowed: bool,
    pub publish_dispatch: String,
    pub degraded: bool,
    pub degradation_reason: Option<String>,
    pub capabilities: Vec<EffectiveCapability>,
    pub official_sources: Vec<OfficialSource>,
    pub computed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishPolicy {
    pub platform_id: String,
    pub account_id: String,
    pub content_kind: ContentKind,
    pub admission_status: String,
    pub capability: CapabilityLevel,
    pub auto_publish_allowed: bool,
    pub manual_publish_allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketplacePolicy {
    pub schema_version: u32,
    pub decision: String,
    pub reviewed_at: String,
    pub rationale: String,
    pub gates: Vec<MarketplaceGate>,
    pub allowed_distribution: String,
    pub third_party_install_allowed: bool,
    pub controlled_pilot_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MarketplaceGate {
    pub id: String,
    pub label: String,
    pub required: bool,
    pub passed: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityCheck {
    pub id: String,
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub contract_version: u32,
    pub checked_at: String,
    pub adapter_count: usize,
    pub passed: bool,
    pub checks: Vec<CompatibilityCheck>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EcosystemOverview {
    pub admissions: AdmissionRegistry,
    pub accounts: Vec<AccountCapabilitySnapshot>,
    pub marketplace: MarketplacePolicy,
    pub compatibility: CompatibilityReport,
}

fn parse_registry() -> Result<AdmissionRegistry, String> {
    serde_json::from_str(ADMISSION_REGISTRY_JSON)
        .map_err(|error| format!("平台准入登记表解析失败：{error}"))
}

fn parse_marketplace_policy() -> Result<MarketplacePolicy, String> {
    serde_json::from_str(MARKETPLACE_POLICY_JSON)
        .map_err(|error| format!("插件市场策略解析失败：{error}"))
}

pub fn platform_admissions() -> Result<Vec<PlatformAdmission>, String> {
    let registry = parse_registry()?;
    validate_admission_registry(&registry)?;
    Ok(registry.platforms)
}

pub fn platform_admission(platform_id: &str) -> Result<Option<PlatformAdmission>, String> {
    Ok(platform_admissions()?
        .into_iter()
        .find(|admission| admission.platform_id == platform_id))
}

pub fn marketplace_policy() -> Result<MarketplacePolicy, String> {
    let policy = parse_marketplace_policy()?;
    validate_marketplace_policy(&policy)?;
    Ok(policy)
}

pub fn ecosystem_overview(conn: &Connection) -> Result<EcosystemOverview, String> {
    let registry = parse_registry()?;
    validate_admission_registry(&registry)?;
    let marketplace = parse_marketplace_policy()?;
    validate_marketplace_policy(&marketplace)?;
    Ok(EcosystemOverview {
        admissions: registry,
        accounts: list_account_capability_snapshots_with(conn, &platform_admissions()?)?,
        marketplace,
        compatibility: compatibility_report(),
    })
}

pub fn list_account_capability_snapshots(
    conn: &Connection,
) -> Result<Vec<AccountCapabilitySnapshot>, String> {
    let admissions = platform_admissions()?;
    list_account_capability_snapshots_with(conn, &admissions)
}

fn list_account_capability_snapshots_with(
    conn: &Connection,
    admissions: &[PlatformAdmission],
) -> Result<Vec<AccountCapabilitySnapshot>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, platform_id, alias, status, enabled
             FROM platform_accounts
             ORDER BY platform_id ASC, created_at ASC",
        )
        .map_err(|error| format!("账号能力矩阵准备失败：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? != 0,
            ))
        })
        .map_err(|error| format!("账号能力矩阵读取失败：{error}"))?;
    let mut snapshots = Vec::new();
    for row in rows {
        let (account_id, platform_id, account_alias, account_status, enabled) =
            row.map_err(|error| format!("账号能力矩阵记录解析失败：{error}"))?;
        let Some(admission) = admissions
            .iter()
            .find(|admission| admission.platform_id == platform_id)
        else {
            snapshots.push(missing_admission_snapshot(
                &account_id,
                &account_alias,
                &platform_id,
                &account_status,
                enabled,
            ));
            continue;
        };
        snapshots.push(account_snapshot(
            admission,
            &account_id,
            &account_alias,
            &account_status,
            enabled,
        ));
    }
    Ok(snapshots)
}

fn missing_admission_snapshot(
    account_id: &str,
    account_alias: &str,
    platform_id: &str,
    account_status: &str,
    enabled: bool,
) -> AccountCapabilitySnapshot {
    AccountCapabilitySnapshot {
        account_id: account_id.to_string(),
        account_alias: account_alias.to_string(),
        platform_id: platform_id.to_string(),
        platform_name: platform_id.to_string(),
        account_status: account_status.to_string(),
        admission_status: "not_admitted".to_string(),
        auto_publish_allowed: false,
        publish_dispatch: "manual_only".to_string(),
        degraded: true,
        degradation_reason: Some(
            "该平台没有可追溯的准入记录；能力和自动发布均保持不可用".to_string(),
        ),
        capabilities: plugins::CONTENT_KINDS
            .into_iter()
            .map(|kind| EffectiveCapability {
                kind,
                label: kind.label().to_string(),
                read: CapabilityLevel::Unavailable,
                publish: CapabilityLevel::Unavailable,
                read_evidence: "not_verified".to_string(),
                publish_evidence: "not_verified".to_string(),
                auto_publish_allowed: false,
                reason: Some(if enabled {
                    "缺少平台准入记录".to_string()
                } else {
                    "账号已禁用且缺少平台准入记录".to_string()
                }),
            })
            .collect(),
        official_sources: Vec::new(),
        computed_at: Utc::now().to_rfc3339(),
    }
}

fn account_snapshot(
    admission: &PlatformAdmission,
    account_id: &str,
    account_alias: &str,
    account_status: &str,
    enabled: bool,
) -> AccountCapabilitySnapshot {
    let account_usable = enabled && account_status == "ACTIVE";
    let degradation_reason = if !enabled {
        Some("账号已禁用，所有平台能力降级为不可用".to_string())
    } else if account_status != "ACTIVE" {
        Some(format!(
            "账号当前为 {account_status}，需要重新授权或完成状态检测后能力才可恢复"
        ))
    } else if admission.admission_status != "verified" {
        Some(format!(
            "平台准入状态为 {}，未完成真实账号运行验证的能力不会显示为支持",
            admission.admission_status
        ))
    } else {
        None
    };
    let capabilities = admission
        .content_kinds
        .iter()
        .map(|capability| {
            let read = effective_level(capability.read.level, &capability.read.evidence, account_usable);
            let publish = effective_level(
                capability.publish.level,
                &capability.publish.evidence,
                account_usable,
            );
            let auto_publish_allowed = account_usable
                && admission.auto_publish_allowed
                && admission.publish_dispatch == "verified_auto"
                && capability.publish.level == CapabilityLevel::Supported
                && capability.publish.evidence == "runtime_verified";
            let mut reasons = Vec::new();
            if !account_usable {
                reasons.push("账号非 ACTIVE 或已禁用".to_string());
            }
            if capability.publish.evidence != "runtime_verified" {
                reasons.push(capability.publish.reason.clone());
            }
            if !auto_publish_allowed {
                reasons.push("自动发布未通过平台准入，仅保留人工流程".to_string());
            }
            EffectiveCapability {
                kind: capability.kind,
                label: capability.label.clone(),
                read,
                publish,
                read_evidence: capability.read.evidence.clone(),
                publish_evidence: capability.publish.evidence.clone(),
                auto_publish_allowed,
                reason: (!reasons.is_empty()).then(|| reasons.join("；")),
            }
        })
        .collect();
    AccountCapabilitySnapshot {
        account_id: account_id.to_string(),
        account_alias: account_alias.to_string(),
        platform_id: admission.platform_id.clone(),
        platform_name: admission.platform_name.clone(),
        account_status: account_status.to_string(),
        admission_status: admission.admission_status.clone(),
        auto_publish_allowed: admission.auto_publish_allowed,
        publish_dispatch: admission.publish_dispatch.clone(),
        degraded: !account_usable || admission.admission_status != "verified",
        degradation_reason,
        capabilities,
        official_sources: admission.official_sources.clone(),
        computed_at: Utc::now().to_rfc3339(),
    }
}

fn effective_level(
    declared: CapabilityLevel,
    evidence: &str,
    account_usable: bool,
) -> CapabilityLevel {
    if !account_usable {
        return CapabilityLevel::Unavailable;
    }
    if declared == CapabilityLevel::Supported && evidence != "runtime_verified" {
        return CapabilityLevel::Experimental;
    }
    declared
}

pub fn publish_policy(
    conn: &Connection,
    account_id: &str,
    platform_id: &str,
    content_kind: ContentKind,
) -> Result<PublishPolicy, String> {
    let row = conn
        .query_row(
            "SELECT platform_id, status, enabled FROM platform_accounts WHERE id=?1",
            params![account_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)? != 0,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("发布账号策略读取失败：{error}"))?;
    let Some((account_platform, account_status, enabled)) = row else {
        return Ok(PublishPolicy {
            platform_id: platform_id.to_string(),
            account_id: account_id.to_string(),
            content_kind,
            admission_status: "account_not_found".to_string(),
            capability: CapabilityLevel::Unavailable,
            auto_publish_allowed: false,
            manual_publish_allowed: true,
            reason: "未找到受管账号；自动发布关闭，只能人工发布".to_string(),
        });
    };
    if account_platform != platform_id {
        return Ok(PublishPolicy {
            platform_id: platform_id.to_string(),
            account_id: account_id.to_string(),
            content_kind,
            admission_status: "platform_mismatch".to_string(),
            capability: CapabilityLevel::Unavailable,
            auto_publish_allowed: false,
            manual_publish_allowed: true,
            reason: "发布任务的平台与账号所属平台不一致，自动发布已关闭".to_string(),
        });
    }
    let admission = platform_admission(platform_id)?.ok_or_else(|| {
        format!("平台 `{platform_id}` 没有准入记录，不能执行自动发布")
    })?;
    let capability = admission
        .content_kinds
        .iter()
        .find(|capability| capability.kind == content_kind)
        .ok_or_else(|| format!("平台 `{platform_id}` 缺少该内容形态的准入记录"))?;
    let account_usable = enabled && account_status == "ACTIVE";
    let effective = effective_level(
        capability.publish.level,
        &capability.publish.evidence,
        account_usable,
    );
    let auto_allowed = account_usable
        && admission.auto_publish_allowed
        && admission.publish_dispatch == "verified_auto"
        && capability.publish.level == CapabilityLevel::Supported
        && capability.publish.evidence == "runtime_verified";
    let reason = if !account_usable {
        format!("账号当前为 {account_status}，自动发布关闭")
    } else if !auto_allowed {
        format!(
            "{}；{}",
            capability.publish.reason,
            "平台准入未允许自动发布，保留人工发布路径"
        )
    } else {
        capability.publish.reason.clone()
    };
    Ok(PublishPolicy {
        platform_id: platform_id.to_string(),
        account_id: account_id.to_string(),
        content_kind,
        admission_status: admission.admission_status,
        capability: effective,
        auto_publish_allowed: auto_allowed,
        manual_publish_allowed: true,
        reason,
    })
}

pub fn validate_admission_registry(registry: &AdmissionRegistry) -> Result<(), String> {
    let mut issues = Vec::new();
    if registry.schema_version != 1 {
        issues.push(format!(
            "schemaVersion {} 不受支持",
            registry.schema_version
        ));
    }
    let mut platform_ids = HashSet::new();
    for admission in &registry.platforms {
        if !platform_ids.insert(admission.platform_id.clone()) {
            issues.push(format!("平台 `{}` 重复登记", admission.platform_id));
        }
        if admission.adapter_contract_version != SUPPORTED_ADAPTER_CONTRACT_VERSION {
            issues.push(format!(
                "平台 `{}` 的 adapterContractVersion 与当前合约不一致",
                admission.platform_id
            ));
        }
        if !matches!(
            admission.admission_status.as_str(),
            "evaluation_only" | "restricted" | "experimental" | "verified"
        ) {
            issues.push(format!(
                "平台 `{}` 的 admissionStatus 非法",
                admission.platform_id
            ));
        }
        if !matches!(
            admission.publish_dispatch.as_str(),
            "manual_only" | "verified_auto"
        ) {
            issues.push(format!(
                "平台 `{}` 的 publishDispatch 非法",
                admission.platform_id
            ));
        }
        if admission.official_sources.is_empty() {
            issues.push(format!("平台 `{}` 缺少官方来源", admission.platform_id));
        }
        for source in &admission.official_sources {
            match Url::parse(&source.url) {
                Ok(url) if url.scheme() == "https" => {}
                _ => issues.push(format!(
                    "平台 `{}` 的来源 `{}` 不是合法 HTTPS URL",
                    admission.platform_id, source.title
                )),
            }
            if source.checked_at.trim().is_empty() {
                issues.push(format!(
                    "平台 `{}` 的来源 `{}` 缺少验证日期",
                    admission.platform_id, source.title
                ));
            }
        }
        let mut kinds = HashSet::new();
        for capability in &admission.content_kinds {
            if !kinds.insert(capability.kind) {
                issues.push(format!(
                    "平台 `{}` 的内容形态 `{}` 重复",
                    admission.platform_id,
                    capability.kind.as_str()
                ));
            }
            validate_assessment(
                &mut issues,
                &admission.platform_id,
                "read",
                capability.kind,
                &capability.read,
            );
            validate_assessment(
                &mut issues,
                &admission.platform_id,
                "publish",
                capability.kind,
                &capability.publish,
            );
        }
        for kind in plugins::CONTENT_KINDS {
            if !kinds.contains(&kind) {
                issues.push(format!(
                    "平台 `{}` 缺少 `{}` 的逐形态评估",
                    admission.platform_id,
                    kind.as_str()
                ));
            }
        }
        if admission.auto_publish_allowed {
            if admission.admission_status != "verified"
                || admission.publish_dispatch != "verified_auto"
            {
                issues.push(format!(
                    "平台 `{}` 在未完成准入验证时声明自动发布",
                    admission.platform_id
                ));
            }
            for capability in &admission.content_kinds {
                if capability.publish.level != CapabilityLevel::Supported
                    || capability.publish.evidence != "runtime_verified"
                {
                    issues.push(format!(
                        "平台 `{}` 的 `{}` 自动发布缺少运行时证据",
                        admission.platform_id,
                        capability.kind.as_str()
                    ));
                }
            }
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues.join("；"))
    }
}

fn validate_assessment(
    issues: &mut Vec<String>,
    platform_id: &str,
    direction: &str,
    kind: ContentKind,
    assessment: &CapabilityAssessment,
) {
    if !matches!(
        assessment.evidence.as_str(),
        "runtime_verified" | "official_docs_only" | "official_scope" | "not_verified"
    ) {
        issues.push(format!(
            "平台 `{platform_id}` 的 `{}` {direction} evidence 非法",
            kind.as_str()
        ));
    }
    if assessment.reason.trim().is_empty() {
        issues.push(format!(
            "平台 `{platform_id}` 的 `{}` {direction} 缺少原因",
            kind.as_str()
        ));
    }
    if assessment.level == CapabilityLevel::Supported
        && assessment.evidence != "runtime_verified"
    {
        issues.push(format!(
            "平台 `{platform_id}` 的 `{}` {direction} 在无运行时证据时声明 supported",
            kind.as_str()
        ));
    }
}

pub fn validate_marketplace_policy(policy: &MarketplacePolicy) -> Result<(), String> {
    let mut issues = Vec::new();
    if policy.schema_version != 1 {
        issues.push(format!("schemaVersion {} 不受支持", policy.schema_version));
    }
    let ids = policy
        .gates
        .iter()
        .map(|gate| gate.id.as_str())
        .collect::<HashSet<_>>();
    for expected in EXPECTED_GATE_IDS {
        if !ids.contains(expected) {
            issues.push(format!("缺少市场门禁 `{expected}`"));
        }
    }
    let all_required_gates_pass = policy
        .gates
        .iter()
        .filter(|gate| gate.required)
        .all(|gate| gate.passed);
    match policy.decision.as_str() {
        "open" => {
            if !all_required_gates_pass {
                issues.push("市场决定开放但必需门禁未全部通过".to_string());
            }
            if !policy.third_party_install_allowed {
                issues.push("市场决定开放但第三方安装仍被禁用".to_string());
            }
        }
        "controlled_pilot" => {
            if !all_required_gates_pass {
                issues.push("受控试点决定缺少全部必需门禁证据".to_string());
            }
            if policy.third_party_install_allowed {
                issues.push("受控试点不能等同于开放第三方安装".to_string());
            }
        }
        "closed" => {
            if policy.third_party_install_allowed || policy.controlled_pilot_allowed {
                issues.push("市场关闭状态不能允许第三方安装或受控试点".to_string());
            }
        }
        _ => issues.push(format!("未知市场决定 `{}`", policy.decision)),
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues.join("；"))
    }
}

pub fn compatibility_report() -> CompatibilityReport {
    let checked_at = Utc::now().to_rfc3339();
    let mut checks = Vec::new();
    let validations = plugins::EMBEDDED_MANIFEST_SOURCES
        .iter()
        .enumerate()
        .map(|(index, raw)| (index, validate_manifest_json(raw)))
        .collect::<Vec<_>>();
    let invalid = validations
        .iter()
        .filter(|(_, validation)| !validation.valid)
        .map(|(index, validation)| manifest_issue_summary(index, validation))
        .collect::<Vec<_>>();
    checks.push(CompatibilityCheck {
        id: "manifest_contract".to_string(),
        label: "全部内嵌 Manifest 通过合约校验",
        passed: invalid.is_empty(),
        detail: if invalid.is_empty() {
            format!("{} 个 Manifest 全部通过字段和语义校验", validations.len())
        } else {
            invalid.join("；")
        },
    });

    let manifests = validations
        .iter()
        .filter_map(|(_, validation)| validation.manifest.clone())
        .collect::<Vec<_>>();
    let unique_ids = manifests
        .iter()
        .map(|manifest| manifest.id.as_str())
        .collect::<HashSet<_>>();
    checks.push(CompatibilityCheck {
        id: "unique_adapter_ids".to_string(),
        label: "适配器身份唯一",
        passed: unique_ids.len() == manifests.len(),
        detail: format!(
            "{} 个有效 Manifest，{} 个唯一 adapter id",
            manifests.len(),
            unique_ids.len()
        ),
    });

    let contract_ok = manifests.iter().all(|manifest| {
        manifest.adapter_contract_version == SUPPORTED_ADAPTER_CONTRACT_VERSION
    });
    checks.push(CompatibilityCheck {
        id: "contract_version".to_string(),
        label: "Adapter 合约版本兼容",
        passed: contract_ok,
        detail: format!(
            "当前支持合约 v{}，内嵌适配器必须使用同一版本",
            SUPPORTED_ADAPTER_CONTRACT_VERSION
        ),
    });

    let content_kinds_ok = manifests.iter().all(|manifest| {
        let declared = manifest
            .capabilities
            .iter()
            .map(|capability| capability.kind)
            .collect::<HashSet<_>>();
        declared.len() == plugins::CONTENT_KINDS.len()
            && plugins::CONTENT_KINDS
                .iter()
                .all(|kind| declared.contains(kind))
    });
    checks.push(CompatibilityCheck {
        id: "content_kind_matrix".to_string(),
        label: "四类内容形态能力声明完整",
        passed: content_kinds_ok,
        detail: format!(
            "{} 个适配器均需声明 video/imageText/imageArticle/article",
            manifests.len()
        ),
    });

    let payload_checks = plugins::payload_self_check();
    let payload_ok = payload_checks.iter().all(|check| check.passed);
    checks.push(CompatibilityCheck {
        id: "shared_payload_contract".to_string(),
        label: "共享发布载荷合约回归通过",
        passed: payload_ok,
        detail: format!(
            "{}/{} 个载荷检查通过",
            payload_checks.iter().filter(|check| check.passed).count(),
            payload_checks.len()
        ),
    });

    CompatibilityReport {
        contract_version: SUPPORTED_ADAPTER_CONTRACT_VERSION,
        checked_at,
        adapter_count: manifests.len(),
        passed: checks.iter().all(|check| check.passed),
        checks,
    }
}

fn manifest_issue_summary(index: usize, validation: &ManifestValidation) -> String {
    let issues = validation
        .issues
        .iter()
        .map(|issue| format!("{}:{}", issue.field, issue.code))
        .collect::<Vec<_>>()
        .join(",");
    format!("Manifest[{index}] {issues}")
}

pub fn require_auto_publish_allowed(
    conn: &Connection,
    account_id: &str,
    platform_id: &str,
    content_kind: ContentKind,
) -> Result<PublishPolicy, String> {
    let policy = publish_policy(conn, account_id, platform_id, content_kind)?;
    if policy.auto_publish_allowed {
        Ok(policy)
    } else {
        Err(policy.reason)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;
    use rusqlite::Connection;

    fn registry() -> AdmissionRegistry {
        parse_registry().unwrap()
    }

    fn policy() -> MarketplacePolicy {
        parse_marketplace_policy().unwrap()
    }

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        database::migrate(&mut conn).unwrap();
        conn
    }

    fn insert_account(conn: &Connection, id: &str, platform_id: &str, status: &str) {
        let now = "2026-11-25T00:00:00Z";
        conn.execute(
            "INSERT INTO platform_accounts(
                id, platform_id, alias, nickname, uid, status, enabled, is_default,
                profile_id, credential_ref, created_at, updated_at
             ) VALUES(?1, ?2, ?2, '', '', ?3, 1, 0, ?4, ?5, ?6, ?6)",
            params![
                id,
                platform_id,
                status,
                format!("profile-{id}"),
                format!("credential-{id}"),
                now
            ],
        )
        .unwrap();
    }

    #[test]
    fn m8_admission_registry_is_traceable_and_covers_every_content_kind() {
        let registry = registry();
        validate_admission_registry(&registry).unwrap();
        assert_eq!(registry.platforms.len(), 10);
        for admission in &registry.platforms {
            assert!(!admission.official_sources.is_empty());
            assert_eq!(admission.content_kinds.len(), 4);
            assert!(!admission.auto_publish_allowed);
            assert_eq!(admission.publish_dispatch, "manual_only");
            assert!(
                admission
                    .content_kinds
                    .iter()
                    .all(|item| item.read.level != CapabilityLevel::Supported
                        && item.publish.level != CapabilityLevel::Supported)
            );
        }
    }

    #[test]
    fn m8_unverified_adapter_contract_cannot_claim_auto_publish_support() {
        let mut registry = registry();
        let admission = registry.platforms.first_mut().unwrap();
        admission.auto_publish_allowed = true;
        admission.publish_dispatch = "verified_auto".to_string();
        let error = validate_admission_registry(&registry).unwrap_err();
        assert!(error.contains("运行时证据"));
    }

    #[test]
    fn m8_expired_account_degrades_platform_and_account_capabilities() {
        let conn = test_conn();
        insert_account(&conn, "account-expired", "bilibili", "NEEDS_REAUTH");
        let snapshots = list_account_capability_snapshots(&conn).unwrap();
        let snapshot = snapshots
            .iter()
            .find(|snapshot| snapshot.account_id == "account-expired")
            .unwrap();
        assert!(snapshot.degraded);
        assert!(snapshot
            .degradation_reason
            .as_deref()
            .unwrap()
            .contains("NEEDS_REAUTH"));
        assert!(snapshot
            .capabilities
            .iter()
            .all(|capability| capability.read == CapabilityLevel::Unavailable
                && capability.publish == CapabilityLevel::Unavailable
                && !capability.auto_publish_allowed));
        let policy = publish_policy(
            &conn,
            "account-expired",
            "bilibili",
            ContentKind::Video,
        )
        .unwrap();
        assert!(!policy.auto_publish_allowed);
        assert!(policy.manual_publish_allowed);
    }

    #[test]
    fn m8_unverified_platform_keeps_manual_publish_and_blocks_auto_publish() {
        let conn = test_conn();
        insert_account(&conn, "account-active", "juejin", "ACTIVE");
        let policy =
            publish_policy(&conn, "account-active", "juejin", ContentKind::Article).unwrap();
        assert!(!policy.auto_publish_allowed);
        assert!(policy.manual_publish_allowed);
        assert_ne!(policy.capability, CapabilityLevel::Supported);
        assert!(policy.reason.contains("人工发布"));
    }

    #[test]
    fn m8_marketplace_policy_stays_closed_until_all_gates_pass() {
        let mut policy = policy();
        validate_marketplace_policy(&policy).unwrap();
        assert_eq!(policy.decision, "closed");
        assert!(!policy.third_party_install_allowed);
        assert!(!policy.controlled_pilot_allowed);
        assert!(policy.gates.iter().any(|gate| !gate.passed));
        let mut invalid = policy.clone();
        invalid.decision = "open".to_string();
        invalid.third_party_install_allowed = true;
        assert!(validate_marketplace_policy(&invalid).is_err());
        policy.decision = "controlled_pilot".to_string();
        policy.controlled_pilot_allowed = true;
        assert!(validate_marketplace_policy(&policy).is_err());
    }

    #[test]
    fn m8_compatibility_report_covers_all_embedded_adapters() {
        let report = compatibility_report();
        assert!(report.passed, "{:?}", report.checks);
        assert_eq!(report.adapter_count, plugins::EMBEDDED_MANIFEST_SOURCES.len());
        assert_eq!(report.contract_version, SUPPORTED_ADAPTER_CONTRACT_VERSION);
    }

    #[test]
    fn m8_unknown_platform_is_never_admitted() {
        let conn = test_conn();
        let snapshots = list_account_capability_snapshots(&conn).unwrap();
        assert!(snapshots.is_empty());
        insert_account(&conn, "account-orphan", "not-a-platform", "ACTIVE");
        let snapshots = list_account_capability_snapshots(&conn).unwrap();
        let snapshot = &snapshots[0];
        assert_eq!(snapshot.admission_status, "not_admitted");
        assert!(snapshot.degraded);
        assert!(snapshot
            .capabilities
            .iter()
            .all(|capability| capability.read == CapabilityLevel::Unavailable));
    }
}
