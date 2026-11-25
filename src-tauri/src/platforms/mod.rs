use serde::Serialize;

const KEYRING_SERVICE: &str = "com.creatorrelay.platform-accounts";
const MAX_CREDENTIAL_LENGTH: usize = 32 * 1024;
const BILIBILI_VERIFY_URL: &str = "https://api.bilibili.com/x/web-interface/nav";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountStatus {
    Unauthorized,
    Authenticating,
    Active,
    Expired,
    NeedsReauth,
    Error,
    Disabled,
}

impl AccountStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Authenticating => "AUTHENTICATING",
            Self::Active => "ACTIVE",
            Self::Expired => "EXPIRED",
            Self::NeedsReauth => "NEEDS_REAUTH",
            Self::Error => "ERROR",
            Self::Disabled => "DISABLED",
        }
    }

    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "UNAUTHORIZED" => Ok(Self::Unauthorized),
            "AUTHENTICATING" => Ok(Self::Authenticating),
            "ACTIVE" => Ok(Self::Active),
            "EXPIRED" => Ok(Self::Expired),
            "NEEDS_REAUTH" => Ok(Self::NeedsReauth),
            "ERROR" => Ok(Self::Error),
            "DISABLED" => Ok(Self::Disabled),
            _ => Err(format!("未知账号状态：{value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub content_read: String,
    pub content_publish: String,
    pub analytics: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAuthMethod {
    pub method: String,
    pub available: bool,
    pub label: String,
    pub login_url: Option<String>,
    pub manual_steps: Vec<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCatalogEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub category_label: String,
    pub content_formats: String,
    pub capabilities: PlatformCapabilities,
    pub auth_methods: Vec<PlatformAuthMethod>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAccount {
    pub id: String,
    pub platform_id: String,
    pub platform_name: String,
    pub category: String,
    pub content_formats: String,
    pub auth_mode: String,
    pub alias: String,
    pub nickname: String,
    pub uid: String,
    pub status: AccountStatus,
    pub enabled: bool,
    pub is_default: bool,
    pub profile_id: String,
    pub credential_ref: String,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub capabilities: PlatformCapabilities,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallenge {
    pub account_id: String,
    pub challenge_id: String,
    pub method: String,
    pub login_url: String,
    pub manual_steps: Vec<String>,
    pub credential_label: String,
    pub credential_help: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountCheckResult {
    pub account: PlatformAccount,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionPreview {
    pub account_id: String,
    pub platform_name: String,
    pub alias: String,
    pub will_delete_credentials: bool,
    pub will_delete_session_profile: String,
    pub retained_history_count: u64,
    pub retained_data: Vec<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionConfirmation {
    pub credentials: bool,
    pub sessions: bool,
    pub history_retained_understood: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletionResult {
    pub account_id: String,
    pub deleted_credentials: bool,
    pub deleted_session_profile: bool,
    pub retained_history_count: u64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountAuditLog {
    pub id: i64,
    pub platform_id: String,
    pub account_alias: String,
    pub action: String,
    pub detail: String,
    pub created_at: String,
}

mod accounts;
mod catalog;
mod credentials;
#[cfg(test)]
mod tests;

pub use accounts::{
    begin_authentication, cancel_authentication, check_account_status, complete_authentication,
    create_account, delete_account, deletion_preview, list_accounts, list_audit_logs,
    require_active_account, update_account,
};
pub use catalog::platform_catalog;
pub use credentials::{
    CredentialVault, IdentityVerifier, KeyringVault, PlatformHttpVerifier, VerificationOutcome,
};
