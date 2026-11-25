use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;
use std::{fs, path::Path, time::Duration};
use uuid::Uuid;

const KEYRING_SERVICE: &str = "com.creatorrelay.platform-accounts";
const SCHEMA_VERSION: i64 = 2;
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

#[derive(Debug, Clone)]
struct PlatformDefinition {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    category_label: &'static str,
    content_formats: &'static str,
    content_read: &'static str,
    content_publish: &'static str,
    analytics: &'static str,
    auth_kind: AuthKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AuthKind {
    BrowserCookie,
    ManualOnly,
    OAuthUnavailable,
}

fn platform_definition(platform_id: &str) -> Option<PlatformDefinition> {
    let definitions = [
        PlatformDefinition {
            id: "juejin",
            name: "掘金",
            category: "tech",
            category_label: "技术社区",
            content_formats: "技术文章",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "csdn",
            name: "CSDN",
            category: "tech",
            category_label: "技术社区",
            content_formats: "技术博客",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "zhihu",
            name: "知乎",
            category: "article",
            category_label: "深度图文",
            content_formats: "问答、文章",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "wechat",
            name: "微信公众号",
            category: "article",
            category_label: "深度图文",
            content_formats: "图文、视频",
            content_read: "restricted",
            content_publish: "unsupported",
            analytics: "restricted",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "bilibili",
            name: "Bilibili",
            category: "video",
            category_label: "视频平台",
            content_formats: "视频、专栏",
            content_read: "supported",
            content_publish: "restricted",
            analytics: "supported",
            auth_kind: AuthKind::BrowserCookie,
        },
        PlatformDefinition {
            id: "youtube",
            name: "YouTube",
            category: "video",
            category_label: "视频平台",
            content_formats: "视频、频道",
            content_read: "experimental",
            content_publish: "experimental",
            analytics: "experimental",
            auth_kind: AuthKind::OAuthUnavailable,
        },
        PlatformDefinition {
            id: "douyin",
            name: "抖音",
            category: "video",
            category_label: "视频平台",
            content_formats: "短视频、图文",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "kuaishou",
            name: "快手",
            category: "video",
            category_label: "视频平台",
            content_formats: "短视频",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "xiaohongshu",
            name: "小红书",
            category: "social",
            category_label: "社交与种草",
            content_formats: "图文、视频",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
        PlatformDefinition {
            id: "weibo",
            name: "微博",
            category: "social",
            category_label: "社交与种草",
            content_formats: "社交动态",
            content_read: "restricted",
            content_publish: "restricted",
            analytics: "experimental",
            auth_kind: AuthKind::ManualOnly,
        },
    ];
    definitions
        .into_iter()
        .find(|definition| definition.id == platform_id)
}

fn browser_steps() -> Vec<String> {
    vec![
        "点击“打开登录页”，在当前系统浏览器完成平台登录；CreatorRelay 不读取浏览器主配置目录。".to_string(),
        "登录后从平台请求中复制完整 Cookie 请求头；只复制自己账号的会话值。".to_string(),
        "返回 CreatorRelay 粘贴 Cookie。后端会先调用平台身份接口校验，校验通过后才写入系统 Keychain。".to_string(),
    ]
}

fn manual_steps(platform_name: &str) -> Vec<String> {
    vec![
        format!("在系统浏览器打开 {platform_name} 并完成人工登录。"),
        "创建、检查与发布内容需要在平台官方页面内人工完成。".to_string(),
        "在 CreatorRelay 中仅保留账号别名与运营记录；人工登录不会被标记为已验证会话。".to_string(),
    ]
}

fn auth_methods(definition: &PlatformDefinition) -> Vec<PlatformAuthMethod> {
    match definition.auth_kind {
        AuthKind::BrowserCookie => vec![
            PlatformAuthMethod {
                method: "browser".to_string(),
                available: true,
                label: "系统浏览器 + 会话校验".to_string(),
                login_url: Some("https://www.bilibili.com/".to_string()),
                manual_steps: browser_steps(),
                reason: None,
            },
            PlatformAuthMethod {
                method: "embeddedWebView".to_string(),
                available: false,
                label: "内置 WebView".to_string(),
                login_url: None,
                manual_steps: Vec::new(),
                reason: Some("当前桌面构建尚未启用账号级独立 WebView 会话。".to_string()),
            },
        ],
        AuthKind::ManualOnly => vec![PlatformAuthMethod {
            method: "manual".to_string(),
            available: true,
            label: "系统浏览器人工流程".to_string(),
            login_url: Some(format!("https://www.baidu.com/s?wd={}", definition.name)),
            manual_steps: manual_steps(definition.name),
            reason: Some("该平台尚未完成官方身份接口验证，不会伪造 ACTIVE 会话。".to_string()),
        }],
        AuthKind::OAuthUnavailable => vec![
            PlatformAuthMethod {
                method: "oauth".to_string(),
                available: false,
                label: "官方 OAuth".to_string(),
                login_url: None,
                manual_steps: Vec::new(),
                reason: Some("尚未配置官方 OAuth 客户端与回调验证。".to_string()),
            },
            PlatformAuthMethod {
                method: "manual".to_string(),
                available: true,
                label: "系统浏览器人工流程".to_string(),
                login_url: Some(format!("https://www.baidu.com/s?wd={}", definition.name)),
                manual_steps: manual_steps(definition.name),
                reason: None,
            },
        ],
    }
}

fn capabilities(definition: &PlatformDefinition) -> PlatformCapabilities {
    PlatformCapabilities {
        content_read: definition.content_read.to_string(),
        content_publish: definition.content_publish.to_string(),
        analytics: definition.analytics.to_string(),
    }
}

fn catalog_entry(definition: &PlatformDefinition) -> PlatformCatalogEntry {
    PlatformCatalogEntry {
        id: definition.id.to_string(),
        name: definition.name.to_string(),
        category: definition.category.to_string(),
        category_label: definition.category_label.to_string(),
        content_formats: definition.content_formats.to_string(),
        capabilities: capabilities(definition),
        auth_methods: auth_methods(definition),
    }
}

pub fn platform_catalog() -> Vec<PlatformCatalogEntry> {
    [
        "juejin",
        "csdn",
        "zhihu",
        "wechat",
        "bilibili",
        "youtube",
        "douyin",
        "kuaishou",
        "xiaohongshu",
        "weibo",
    ]
    .into_iter()
    .filter_map(platform_definition)
    .map(|definition| catalog_entry(&definition))
    .collect()
}

pub fn initialize_schema(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|_| "无法启用数据库外键约束".to_string())?;
    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|_| "无法读取数据库 schema 版本".to_string())?;
    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    let tx = conn
        .transaction()
        .map_err(|_| "无法开始账号 schema 迁移事务".to_string())?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS platform_accounts(
            id TEXT PRIMARY KEY,
            platform_id TEXT NOT NULL,
            alias TEXT NOT NULL,
            nickname TEXT NOT NULL DEFAULT '',
            uid TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            is_default INTEGER NOT NULL DEFAULT 0,
            profile_id TEXT NOT NULL UNIQUE,
            credential_ref TEXT NOT NULL UNIQUE,
            last_checked_at TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE UNIQUE INDEX IF NOT EXISTS platform_accounts_one_default
            ON platform_accounts(platform_id) WHERE is_default = 1;
        CREATE TABLE IF NOT EXISTS platform_sessions(
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL UNIQUE REFERENCES platform_accounts(id) ON DELETE CASCADE,
            profile_id TEXT NOT NULL UNIQUE,
            auth_mode TEXT NOT NULL,
            state TEXT NOT NULL,
            credential_ref TEXT NOT NULL,
            challenge_id TEXT,
            expires_at TEXT,
            last_verified_at TEXT,
            last_error TEXT,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS platform_works(
            id TEXT PRIMARY KEY,
            account_id TEXT REFERENCES platform_accounts(id) ON DELETE SET NULL,
            title TEXT NOT NULL,
            external_id TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS platform_works_account_id ON platform_works(account_id);
        CREATE TABLE IF NOT EXISTS account_audit_logs(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id TEXT REFERENCES platform_accounts(id) ON DELETE SET NULL,
            platform_id TEXT NOT NULL,
            account_alias TEXT NOT NULL,
            action TEXT NOT NULL,
            detail TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        INSERT OR IGNORE INTO schema_migrations(version, applied_at)
            VALUES(2, datetime('now'));",
    )
    .map_err(|_| "账号 schema 迁移失败".to_string())?;
    tx.commit()
        .map_err(|_| "账号 schema 迁移提交失败".to_string())
}

pub trait CredentialVault {
    fn set(&self, reference: &str, secret: &str) -> Result<(), String>;
    fn get(&self, reference: &str) -> Result<Option<String>, String>;
    fn delete(&self, reference: &str) -> Result<(), String>;
}

pub struct KeyringVault;

impl KeyringVault {
    fn entry(reference: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(KEYRING_SERVICE, reference)
            .map_err(|_| "系统安全凭据存储不可用，无法安全保存平台会话".to_string())
    }
}

impl CredentialVault for KeyringVault {
    fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
        Self::entry(reference)?
            .set_password(secret)
            .map_err(|_| "系统安全凭据写入失败".to_string())
    }

    fn get(&self, reference: &str) -> Result<Option<String>, String> {
        match Self::entry(reference)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err("系统安全凭据读取失败".to_string()),
        }
    }

    fn delete(&self, reference: &str) -> Result<(), String> {
        match Self::entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("系统安全凭据删除失败".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VerificationOutcome {
    Verified { uid: String, nickname: String },
    Expired,
    Failed(String),
}

pub trait IdentityVerifier {
    fn verify(&self, platform_id: &str, credential: &str) -> VerificationOutcome;
}

pub struct PlatformHttpVerifier {
    bilibili_url: String,
}

impl Default for PlatformHttpVerifier {
    fn default() -> Self {
        Self {
            bilibili_url: BILIBILI_VERIFY_URL.to_string(),
        }
    }
}

impl PlatformHttpVerifier {
    #[cfg(test)]
    fn with_bilibili_url(url: String) -> Self {
        Self { bilibili_url: url }
    }
}

impl IdentityVerifier for PlatformHttpVerifier {
    fn verify(&self, platform_id: &str, credential: &str) -> VerificationOutcome {
        if platform_id != "bilibili" {
            return VerificationOutcome::Failed("该平台尚未启用官方身份校验".to_string());
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(12)))
            .build()
            .new_agent();
        let response = agent
            .get(&self.bilibili_url)
            .header("Cookie", credential)
            .header("Accept", "application/json")
            .header("User-Agent", "CreatorRelay/0.1 platform-session-check")
            .call();
        let mut response = match response {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(401 | 403)) => return VerificationOutcome::Expired,
            Err(_) => {
                return VerificationOutcome::Failed(
                    "平台身份校验请求失败，请检查网络后重试".to_string(),
                );
            }
        };
        let body = match response.body_mut().read_to_string() {
            Ok(body) => body,
            Err(_) => return VerificationOutcome::Failed("平台身份响应无法读取".to_string()),
        };
        parse_bilibili_identity(&body)
    }
}

fn parse_bilibili_identity(body: &str) -> VerificationOutcome {
    let value: serde_json::Value = match serde_json::from_str(body) {
        Ok(value) => value,
        Err(_) => return VerificationOutcome::Failed("平台身份响应格式无效".to_string()),
    };
    let code = value
        .get("code")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(-1);
    if code == -101 {
        return VerificationOutcome::Expired;
    }
    if code != 0 {
        return VerificationOutcome::Failed("平台拒绝身份校验".to_string());
    }
    let data = value.get("data").unwrap_or(&serde_json::Value::Null);
    let is_login = data
        .get("isLogin")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if !is_login {
        return VerificationOutcome::Expired;
    }
    let uid = data
        .get("mid")
        .and_then(|value| match value {
            serde_json::Value::Number(number) => Some(number.to_string()),
            serde_json::Value::String(text) => Some(text.clone()),
            _ => None,
        })
        .filter(|value| !value.is_empty());
    let nickname = data
        .get("uname")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    match (uid, nickname) {
        (Some(uid), Some(nickname)) => VerificationOutcome::Verified { uid, nickname },
        _ => VerificationOutcome::Failed("平台未返回可验证的账号身份".to_string()),
    }
}

fn normalize_credential(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("请输入平台会话 Cookie".to_string());
    }
    if trimmed.len() > MAX_CREDENTIAL_LENGTH {
        return Err("会话凭据过长，已拒绝保存".to_string());
    }
    if trimmed.chars().any(|character| character.is_control()) {
        return Err("会话凭据包含非法控制字符".to_string());
    }
    if trimmed.contains('=') {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("SESSDATA={trimmed}"))
    }
}

#[cfg(unix)]
fn set_private_mode(path: &Path, mode: u32) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|_| "无法读取会话容器权限".to_string())?
        .permissions();
    permissions.set_mode(mode);
    fs::set_permissions(path, permissions).map_err(|_| "无法设置会话容器权限".to_string())
}

#[cfg(not(unix))]
fn set_private_mode(_path: &Path, _mode: u32) -> Result<(), String> {
    Ok(())
}

fn session_root_directory(session_root: &Path) -> Result<(), String> {
    fs::create_dir_all(session_root).map_err(|_| "无法创建账号会话目录".to_string())?;
    set_private_mode(session_root, 0o700)
}

fn create_profile(
    session_root: &Path,
    account_id: &str,
    platform_id: &str,
) -> Result<String, String> {
    session_root_directory(session_root)?;
    let profile_id = format!("profile-{}", Uuid::new_v4());
    let profile_dir = session_root.join(&profile_id);
    fs::create_dir(&profile_dir).map_err(|_| "无法创建隔离会话容器".to_string())?;
    set_private_mode(&profile_dir, 0o700)?;
    let metadata = serde_json::json!({
        "version": 1,
        "accountId": account_id,
        "platformId": platform_id,
        "createdAt": Utc::now().to_rfc3339(),
        "sensitiveDataStoredIn": "os-keychain",
    });
    let metadata_path = profile_dir.join("profile.json");
    fs::write(
        &metadata_path,
        serde_json::to_vec_pretty(&metadata).map_err(|_| "无法序列化会话元数据")?,
    )
    .map_err(|_| "无法写入会话元数据".to_string())?;
    set_private_mode(&metadata_path, 0o600)?;
    Ok(profile_id)
}

fn profile_directory(session_root: &Path, profile_id: &str) -> Result<std::path::PathBuf, String> {
    if profile_id.is_empty()
        || !profile_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("账号会话容器标识无效".to_string());
    }
    Ok(session_root.join(profile_id))
}

fn remove_profile(session_root: &Path, profile_id: &str) -> Result<bool, String> {
    let path = profile_directory(session_root, profile_id)?;
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_dir_all(path).map_err(|_| "无法删除账号会话容器".to_string())?;
    Ok(true)
}

fn validate_alias(alias: &str) -> Result<String, String> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Err("账号别名不能为空".to_string());
    }
    if alias.chars().count() > 80 {
        return Err("账号别名不能超过 80 个字符".to_string());
    }
    if alias.chars().any(char::is_control) {
        return Err("账号别名包含非法控制字符".to_string());
    }
    Ok(alias.to_string())
}

#[derive(Debug, Clone)]
struct AccountRecord {
    id: String,
    platform_id: String,
    alias: String,
    nickname: String,
    uid: String,
    status: AccountStatus,
    enabled: bool,
    is_default: bool,
    profile_id: String,
    credential_ref: String,
    last_checked_at: Option<String>,
    last_error: Option<String>,
    challenge_id: Option<String>,
}

const ACCOUNT_SELECT: &str = "SELECT a.id, a.platform_id, a.alias, a.nickname, a.uid, a.status,
       a.enabled, a.is_default, a.profile_id, a.credential_ref, a.last_checked_at,
       a.last_error, s.challenge_id
  FROM platform_accounts a
  LEFT JOIN platform_sessions s ON s.account_id = a.id";

fn account_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    let status: String = row.get(5)?;
    Ok(AccountRecord {
        id: row.get(0)?,
        platform_id: row.get(1)?,
        alias: row.get(2)?,
        nickname: row.get(3)?,
        uid: row.get(4)?,
        status: AccountStatus::parse(&status).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
            )
        })?,
        enabled: row.get::<_, i64>(6)? != 0,
        is_default: row.get::<_, i64>(7)? != 0,
        profile_id: row.get(8)?,
        credential_ref: row.get(9)?,
        last_checked_at: row.get(10)?,
        last_error: row.get(11)?,
        challenge_id: row.get(12)?,
    })
}

fn account_record(conn: &Connection, account_id: &str) -> Result<AccountRecord, String> {
    conn.query_row(
        &format!("{ACCOUNT_SELECT} WHERE a.id = ?1"),
        params![account_id],
        account_record_from_row,
    )
    .optional()
    .map_err(|_| "账号记录读取失败".to_string())?
    .ok_or_else(|| "未找到平台账号".to_string())
}

fn account_from_record(record: AccountRecord) -> Result<PlatformAccount, String> {
    let definition = platform_definition(&record.platform_id)
        .ok_or_else(|| "平台目录中不存在该平台".to_string())?;
    let auth_mode = auth_methods(&definition)
        .into_iter()
        .find(|method| method.available)
        .map(|method| method.method)
        .unwrap_or_else(|| "manual".to_string());
    Ok(PlatformAccount {
        id: record.id,
        platform_id: record.platform_id,
        platform_name: definition.name.to_string(),
        category: definition.category.to_string(),
        content_formats: definition.content_formats.to_string(),
        auth_mode,
        alias: record.alias,
        nickname: record.nickname,
        uid: record.uid,
        status: record.status,
        enabled: record.enabled,
        is_default: record.is_default,
        profile_id: record.profile_id,
        credential_ref: record.credential_ref,
        last_checked_at: record.last_checked_at,
        last_error: record.last_error,
        capabilities: capabilities(&definition),
    })
}

fn insert_audit(
    tx: &Transaction<'_>,
    account_id: Option<&str>,
    platform_id: &str,
    account_alias: &str,
    action: &str,
    detail: &str,
) -> Result<(), String> {
    let sanitized = detail.replace(['\r', '\n'], " ");
    tx.execute(
        "INSERT INTO account_audit_logs(account_id, platform_id, account_alias, action, detail, created_at)
         VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
        params![account_id, platform_id, account_alias, action, sanitized, Utc::now().to_rfc3339()],
    )
    .map_err(|_| "账号操作日志写入失败".to_string())?;
    Ok(())
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<PlatformAccount>, String> {
    let records = conn
        .prepare(&format!(
            "{ACCOUNT_SELECT} ORDER BY a.platform_id ASC, a.created_at ASC"
        ))
        .map_err(|_| "账号列表准备失败".to_string())?
        .query_map([], account_record_from_row)
        .map_err(|_| "账号列表读取失败".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "账号记录解析失败".to_string())?;
    records.into_iter().map(account_from_record).collect()
}

pub fn create_account(
    conn: &mut Connection,
    session_root: &Path,
    platform_id: &str,
    alias: &str,
) -> Result<PlatformAccount, String> {
    let definition = platform_definition(platform_id).ok_or_else(|| "不支持该平台".to_string())?;
    let alias = validate_alias(alias)?;
    let id = Uuid::new_v4().to_string();
    let profile_id = create_profile(session_root, &id, platform_id)?;
    let credential_ref = format!("account:{id}:session");
    let now = Utc::now().to_rfc3339();
    let auth_mode = auth_methods(&definition)
        .into_iter()
        .find(|method| method.available)
        .map(|method| method.method)
        .unwrap_or_else(|| "manual".to_string());
    let tx = conn
        .transaction()
        .map_err(|_| "无法开始创建账号事务".to_string())?;
    tx.execute(
        "INSERT INTO platform_accounts(
            id, platform_id, alias, nickname, uid, status, enabled, is_default,
            profile_id, credential_ref, last_checked_at, last_error, created_at, updated_at
         ) VALUES(?1, ?2, ?3, '', '', 'UNAUTHORIZED', 1, 0, ?4, ?5, NULL, NULL, ?6, ?6)",
        params![id, platform_id, alias, profile_id, credential_ref, now],
    )
    .map_err(|_| "账号创建失败".to_string())?;
    tx.execute(
        "INSERT INTO platform_sessions(
            id, account_id, profile_id, auth_mode, state, credential_ref,
            challenge_id, expires_at, last_verified_at, last_error, updated_at
         ) VALUES(?1, ?2, ?3, ?4, 'UNAUTHORIZED', ?5, NULL, NULL, NULL, NULL, ?6)",
        params![
            Uuid::new_v4().to_string(),
            id,
            profile_id,
            auth_mode,
            credential_ref,
            now
        ],
    )
    .map_err(|_| "账号会话初始化失败".to_string())?;
    insert_audit(
        &tx,
        Some(&id),
        platform_id,
        &alias,
        "ACCOUNT_CREATED",
        "创建平台账号及隔离会话容器",
    )?;
    if tx.commit().is_err() {
        let _ = remove_profile(session_root, &profile_id);
        return Err("账号创建提交失败".to_string());
    }
    let record = account_record(conn, &id)?;
    account_from_record(record)
}

pub fn update_account(
    conn: &mut Connection,
    account_id: &str,
    alias: Option<String>,
    enabled: Option<bool>,
    is_default: Option<bool>,
) -> Result<PlatformAccount, String> {
    let existing = account_record(conn, account_id)?;
    let next_alias = alias
        .map(|value| validate_alias(&value))
        .transpose()?
        .unwrap_or(existing.alias);
    let next_enabled = enabled.unwrap_or(existing.enabled);
    let requested_default = is_default.unwrap_or(existing.is_default);
    if requested_default && !next_enabled {
        return Err("禁用账号不能设为默认账号".to_string());
    }
    let next_status = if !next_enabled {
        AccountStatus::Disabled
    } else if !existing.enabled && next_enabled {
        AccountStatus::NeedsReauth
    } else {
        existing.status
    };
    let next_default = if next_enabled {
        requested_default
    } else {
        false
    };
    let now = Utc::now().to_rfc3339();
    let tx = conn
        .transaction()
        .map_err(|_| "无法开始账号更新事务".to_string())?;
    if next_default {
        tx.execute(
            "UPDATE platform_accounts SET is_default = 0 WHERE platform_id = ?1 AND id <> ?2",
            params![existing.platform_id, account_id],
        )
        .map_err(|_| "默认账号更新失败".to_string())?;
    }
    tx.execute(
        "UPDATE platform_accounts
            SET alias = ?1, enabled = ?2, is_default = ?3, status = ?4,
                last_error = CASE WHEN ?4 = 'NEEDS_REAUTH' THEN '账号已重新启用，请重新检测会话' ELSE last_error END,
                updated_at = ?5
          WHERE id = ?6",
        params![
            next_alias,
            i64::from(next_enabled),
            i64::from(next_default),
            next_status.as_str(),
            now,
            account_id
        ],
    )
    .map_err(|_| "账号更新失败".to_string())?;
    tx.execute(
        "UPDATE platform_sessions SET state = ?1, updated_at = ?2 WHERE account_id = ?3",
        params![next_status.as_str(), now, account_id],
    )
    .map_err(|_| "账号会话状态更新失败".to_string())?;
    insert_audit(
        &tx,
        Some(account_id),
        &existing.platform_id,
        &next_alias,
        "ACCOUNT_UPDATED",
        "更新账号别名、启用状态或默认设置",
    )?;
    tx.commit().map_err(|_| "账号更新提交失败".to_string())?;
    account_from_record(account_record(conn, account_id)?)
}

pub fn begin_authentication(
    conn: &mut Connection,
    account_id: &str,
) -> Result<AuthChallenge, String> {
    let account = account_record(conn, account_id)?;
    if !account.enabled {
        return Err("账号已禁用，请先启用".to_string());
    }
    let definition = platform_definition(&account.platform_id)
        .ok_or_else(|| "平台目录中不存在该平台".to_string())?;
    let method = auth_methods(&definition)
        .into_iter()
        .find(|method| method.available && method.method == "browser")
        .ok_or_else(|| {
            auth_methods(&definition)
                .into_iter()
                .find(|method| method.available)
                .and_then(|method| method.reason)
                .unwrap_or_else(|| {
                    "该平台尚未提供可验证的自动会话流程，请按平台目录中的人工步骤操作".to_string()
                })
        })?;
    let challenge_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let tx = conn
        .transaction()
        .map_err(|_| "无法开始身份校验事务".to_string())?;
    tx.execute(
        "UPDATE platform_accounts SET status = 'AUTHENTICATING', last_error = NULL, updated_at = ?1 WHERE id = ?2",
        params![now, account_id],
    )
    .map_err(|_| "无法进入认证中状态".to_string())?;
    tx.execute(
        "UPDATE platform_sessions
            SET state = 'AUTHENTICATING', auth_mode = ?1, challenge_id = ?2, last_error = NULL, updated_at = ?3
          WHERE account_id = ?4",
        params![method.method, challenge_id, now, account_id],
    )
    .map_err(|_| "无法创建登录挑战".to_string())?;
    insert_audit(
        &tx,
        Some(account_id),
        &account.platform_id,
        &account.alias,
        "AUTH_STARTED",
        "开始系统浏览器身份校验流程",
    )?;
    tx.commit().map_err(|_| "登录挑战提交失败".to_string())?;
    Ok(AuthChallenge {
        account_id: account_id.to_string(),
        challenge_id,
        method: "browser".to_string(),
        login_url: method.login_url.unwrap_or_default(),
        manual_steps: method.manual_steps,
        credential_label: "平台会话 Cookie".to_string(),
        credential_help: "Cookie 只会写入操作系统 Keychain，不会进入 SQLite、日志或备份文件。"
            .to_string(),
    })
}

pub fn cancel_authentication(conn: &mut Connection, account_id: &str) -> Result<PlatformAccount, String> {
    let account = account_record(conn, account_id)?;
    if account.status != AccountStatus::Authenticating {
        return account_from_record(account);
    }
    let next_status = if account.uid.is_empty() {
        AccountStatus::Unauthorized
    } else {
        AccountStatus::NeedsReauth
    };
    let now = Utc::now().to_rfc3339();
    let tx = conn
        .transaction()
        .map_err(|_| "无法取消身份校验事务".to_string())?;
    tx.execute(
        "UPDATE platform_accounts SET status = ?1, last_error = NULL, updated_at = ?2 WHERE id = ?3",
        params![next_status.as_str(), now, account_id],
    )
    .map_err(|_| "账号状态恢复失败".to_string())?;
    tx.execute(
        "UPDATE platform_sessions
            SET state = ?1, challenge_id = NULL, last_error = NULL, updated_at = ?2
          WHERE account_id = ?3",
        params![next_status.as_str(), now, account_id],
    )
    .map_err(|_| "登录挑战取消失败".to_string())?;
    insert_audit(
        &tx,
        Some(account_id),
        &account.platform_id,
        &account.alias,
        "AUTH_CANCELLED",
        "用户取消身份校验",
    )?;
    tx.commit().map_err(|_| "取消身份校验提交失败".to_string())?;
    account_from_record(account_record(conn, account_id)?)
}

fn set_failure_status(
    conn: &mut Connection,
    account_id: &str,
    status: AccountStatus,
    message: &str,
) -> Result<(), String> {
    let account = account_record(conn, account_id)?;
    let now = Utc::now().to_rfc3339();
    let tx = conn
        .transaction()
        .map_err(|_| "无法记录身份校验结果".to_string())?;
    tx.execute(
        "UPDATE platform_accounts
            SET status = ?1, last_checked_at = ?2, last_error = ?3, updated_at = ?2
          WHERE id = ?4",
        params![status.as_str(), now, message, account_id],
    )
    .map_err(|_| "账号状态更新失败".to_string())?;
    tx.execute(
        "UPDATE platform_sessions
            SET state = ?1, challenge_id = NULL, last_error = ?2, updated_at = ?3
          WHERE account_id = ?4",
        params![status.as_str(), message, now, account_id],
    )
    .map_err(|_| "会话状态更新失败".to_string())?;
    insert_audit(
        &tx,
        Some(account_id),
        &account.platform_id,
        &account.alias,
        "AUTH_FAILED",
        message,
    )?;
    tx.commit().map_err(|_| "身份校验结果提交失败".to_string())
}

pub fn complete_authentication<V: CredentialVault, I: IdentityVerifier>(
    conn: &mut Connection,
    vault: &V,
    verifier: &I,
    account_id: &str,
    challenge_id: &str,
    raw_credential: &str,
) -> Result<AccountCheckResult, String> {
    let credential = normalize_credential(raw_credential)?;
    let account = account_record(conn, account_id)?;
    if account.status != AccountStatus::Authenticating {
        return Err("登录挑战已失效，请重新开始身份校验".to_string());
    }
    if account.challenge_id.as_deref() != Some(challenge_id) {
        return Err("登录挑战不匹配，请重新开始身份校验".to_string());
    }
    match verifier.verify(&account.platform_id, &credential) {
        VerificationOutcome::Expired => {
            set_failure_status(
                conn,
                account_id,
                AccountStatus::NeedsReauth,
                "平台会话已过期或无效",
            )?;
            Err("平台会话无效或已过期，账号已标记为需要重新授权".to_string())
        }
        VerificationOutcome::Failed(message) => {
            set_failure_status(conn, account_id, AccountStatus::Error, &message)?;
            Err(message)
        }
        VerificationOutcome::Verified { uid, nickname } => {
            if vault.set(&account.credential_ref, &credential).is_err() {
                set_failure_status(
                    conn,
                    account_id,
                    AccountStatus::Error,
                    "系统安全凭据写入失败",
                )?;
                return Err("身份校验通过，但系统安全凭据写入失败".to_string());
            }
            let now = Utc::now().to_rfc3339();
            let tx = conn
                .transaction()
                .map_err(|_| "无法保存身份校验结果".to_string())?;
            let update = tx
                .execute(
                    "UPDATE platform_accounts
                        SET status = 'ACTIVE', uid = ?1, nickname = ?2, last_checked_at = ?3,
                            last_error = NULL, updated_at = ?3
                      WHERE id = ?4",
                    params![uid, nickname, now, account_id],
                )
                .and_then(|_| {
                    tx.execute(
                        "UPDATE platform_sessions
                            SET state = 'ACTIVE', challenge_id = NULL, last_verified_at = ?1,
                                last_error = NULL, updated_at = ?1
                          WHERE account_id = ?2",
                        params![now, account_id],
                    )
                })
                .and_then(|_| {
                    insert_audit(
                        &tx,
                        Some(account_id),
                        &account.platform_id,
                        &account.alias,
                        "AUTH_VERIFIED",
                        "平台身份校验通过并经系统 Keychain 保存会话",
                    )
                    .map_err(|_| rusqlite::Error::InvalidQuery)
                });
            if update.is_err() || tx.commit().is_err() {
                let _ = vault.delete(&account.credential_ref);
                return Err("身份校验通过，但账号状态保存失败".to_string());
            }
            Ok(AccountCheckResult {
                account: account_from_record(account_record(conn, account_id)?)?,
                message: "平台身份校验通过，账号已连接".to_string(),
            })
        }
    }
}

pub fn check_account_status<V: CredentialVault, I: IdentityVerifier>(
    conn: &mut Connection,
    vault: &V,
    verifier: &I,
    account_id: &str,
) -> Result<AccountCheckResult, String> {
    let account = account_record(conn, account_id)?;
    if !account.enabled {
        return Ok(AccountCheckResult {
            account: account_from_record(account)?,
            message: "账号已禁用".to_string(),
        });
    }
    let Some(credential) = vault
        .get(&account.credential_ref)
        .map_err(|_| "系统安全凭据读取失败，未改变账号状态".to_string())?
    else {
        set_failure_status(
            conn,
            account_id,
            AccountStatus::NeedsReauth,
            "未找到安全的平台会话凭据",
        )?;
        return Ok(AccountCheckResult {
            account: account_from_record(account_record(conn, account_id)?)?,
            message: "未找到会话凭据，请重新授权".to_string(),
        });
    };
    match verifier.verify(&account.platform_id, &credential) {
        VerificationOutcome::Expired => {
            set_failure_status(
                conn,
                account_id,
                AccountStatus::NeedsReauth,
                "平台会话已过期或无效",
            )?;
            Ok(AccountCheckResult {
                account: account_from_record(account_record(conn, account_id)?)?,
                message: "平台会话已过期，账号已进入需要重新授权状态".to_string(),
            })
        }
        VerificationOutcome::Failed(message) => {
            set_failure_status(conn, account_id, AccountStatus::Error, &message)?;
            Ok(AccountCheckResult {
                account: account_from_record(account_record(conn, account_id)?)?,
                message,
            })
        }
        VerificationOutcome::Verified { uid, nickname } => {
            let now = Utc::now().to_rfc3339();
            let tx = conn
                .transaction()
                .map_err(|_| "无法保存会话检测结果".to_string())?;
            tx.execute(
                "UPDATE platform_accounts
                    SET status = 'ACTIVE', uid = ?1, nickname = ?2, last_checked_at = ?3,
                        last_error = NULL, updated_at = ?3
                  WHERE id = ?4",
                params![uid, nickname, now, account_id],
            )
            .map_err(|_| "会话检测状态更新失败".to_string())?;
            tx.execute(
                "UPDATE platform_sessions
                    SET state = 'ACTIVE', last_verified_at = ?1, last_error = NULL, updated_at = ?1
                  WHERE account_id = ?2",
                params![now, account_id],
            )
            .map_err(|_| "会话检测记录更新失败".to_string())?;
            insert_audit(
                &tx,
                Some(account_id),
                &account.platform_id,
                &account.alias,
                "SESSION_CHECKED",
                "会话检测通过，账号保持 ACTIVE",
            )?;
            tx.commit().map_err(|_| "会话检测提交失败".to_string())?;
            Ok(AccountCheckResult {
                account: account_from_record(account_record(conn, account_id)?)?,
                message: "平台身份校验通过，会话有效".to_string(),
            })
        }
    }
}

pub fn require_active_account(
    conn: &Connection,
    account_id: &str,
) -> Result<PlatformAccount, String> {
    let account = account_from_record(account_record(conn, account_id)?)?;
    if !account.enabled {
        return Err("账号已禁用，无法执行依赖该账号的任务".to_string());
    }
    if account.status != AccountStatus::Active {
        return Err("账号当前不是 ACTIVE 状态，无法执行依赖该账号的任务".to_string());
    }
    Ok(account)
}

pub fn deletion_preview(
    conn: &Connection,
    session_root: &Path,
    account_id: &str,
) -> Result<DeletionPreview, String> {
    let account = account_record(conn, account_id)?;
    let definition = platform_definition(&account.platform_id)
        .ok_or_else(|| "平台目录中不存在该平台".to_string())?;
    let history_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM platform_works WHERE account_id = ?1",
            params![account_id],
            |row| row.get(0),
        )
        .map_err(|_| "历史作品数量读取失败".to_string())?;
    let profile_path = profile_directory(session_root, &account.profile_id)?;
    Ok(DeletionPreview {
        account_id: account.id,
        platform_name: definition.name.to_string(),
        alias: account.alias,
        will_delete_credentials: true,
        will_delete_session_profile: profile_path.to_string_lossy().to_string(),
        retained_history_count: history_count.max(0) as u64,
        retained_data: vec![
            "运营历史与平台作品记录会保留，并解除与已删除账号的关联".to_string(),
            "本地主稿、Markdown 正文与素材不会被删除".to_string(),
        ],
    })
}

pub fn delete_account<V: CredentialVault>(
    conn: &mut Connection,
    vault: &V,
    session_root: &Path,
    account_id: &str,
    confirmation: &DeletionConfirmation,
) -> Result<DeletionResult, String> {
    if !confirmation.credentials
        || !confirmation.sessions
        || !confirmation.history_retained_understood
    {
        return Err("请确认凭据删除、会话清理和历史作品保留范围".to_string());
    }
    let account = account_record(conn, account_id)?;
    let definition = platform_definition(&account.platform_id)
        .ok_or_else(|| "平台目录中不存在该平台".to_string())?;
    let retained_history_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM platform_works WHERE account_id = ?1",
            params![account_id],
            |row| row.get(0),
        )
        .map_err(|_| "历史作品数量读取失败".to_string())?;
    let credential_existed = vault
        .get(&account.credential_ref)
        .map_err(|_| "无法确认安全凭据状态，账号未删除".to_string())?
        .is_some();
    vault
        .delete(&account.credential_ref)
        .map_err(|_| "系统安全凭据删除失败，账号未删除".to_string())?;
    let deleted_profile = remove_profile(session_root, &account.profile_id)
        .map_err(|_| "会话容器删除失败，账号记录已保留以便重试")?;
    let tx = conn
        .transaction()
        .map_err(|_| "无法开始账号删除事务".to_string())?;
    tx.execute(
        "DELETE FROM platform_accounts WHERE id = ?1",
        params![account_id],
    )
    .map_err(|_| "账号记录删除失败".to_string())?;
    insert_audit(
        &tx,
        None,
        &account.platform_id,
        &account.alias,
        "ACCOUNT_DELETED",
        &format!(
            "删除平台账号；清理系统 Keychain 凭据与隔离会话容器；保留 {retained_history_count} 条历史作品记录"
        ),
    )?;
    tx.commit().map_err(|_| "账号删除提交失败".to_string())?;
    Ok(DeletionResult {
        account_id: account_id.to_string(),
        deleted_credentials: credential_existed,
        deleted_session_profile: deleted_profile,
        retained_history_count: retained_history_count.max(0) as u64,
        message: format!(
            "已删除 {} · {}；凭据和会话已清理，历史作品记录已保留",
            definition.name, account.alias
        ),
    })
}

pub fn list_audit_logs(conn: &Connection, limit: usize) -> Result<Vec<AccountAuditLog>, String> {
    let limit = limit.clamp(1, 200) as i64;
    conn.prepare(
        "SELECT id, platform_id, account_alias, action, detail, created_at
           FROM account_audit_logs ORDER BY id DESC LIMIT ?1",
    )
    .map_err(|_| "账号日志查询准备失败".to_string())?
    .query_map(params![limit], |row| {
        Ok(AccountAuditLog {
            id: row.get(0)?,
            platform_id: row.get(1)?,
            account_alias: row.get(2)?,
            action: row.get(3)?,
            detail: row.get(4)?,
            created_at: row.get(5)?,
        })
    })
    .map_err(|_| "账号日志读取失败".to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|_| "账号日志解析失败".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    #[derive(Clone, Default)]
    struct MemoryVault {
        values: Arc<Mutex<HashMap<String, String>>>,
    }

    impl CredentialVault for MemoryVault {
        fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
            self.values
                .lock()
                .map_err(|_| "vault poisoned".to_string())?
                .insert(reference.to_string(), secret.to_string());
            Ok(())
        }

        fn get(&self, reference: &str) -> Result<Option<String>, String> {
            Ok(self
                .values
                .lock()
                .map_err(|_| "vault poisoned".to_string())?
                .get(reference)
                .cloned())
        }

        fn delete(&self, reference: &str) -> Result<(), String> {
            self.values
                .lock()
                .map_err(|_| "vault poisoned".to_string())?
                .remove(reference);
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FakeVerifier {
        outcome: Arc<Mutex<VerificationOutcome>>,
    }

    impl FakeVerifier {
        fn new(outcome: VerificationOutcome) -> Self {
            Self {
                outcome: Arc::new(Mutex::new(outcome)),
            }
        }

        fn set(&self, outcome: VerificationOutcome) {
            if let Ok(mut current) = self.outcome.lock() {
                *current = outcome;
            }
        }
    }

    impl IdentityVerifier for FakeVerifier {
        fn verify(&self, _platform_id: &str, _credential: &str) -> VerificationOutcome {
            self.outcome.lock().expect("fake verifier lock").clone()
        }
    }

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES(1, datetime('now'));",
        )
        .unwrap();
        initialize_schema(&mut conn).unwrap();
        conn
    }

    fn verified() -> VerificationOutcome {
        VerificationOutcome::Verified {
            uid: "mid-1001".to_string(),
            nickname: "主账号".to_string(),
        }
    }

    fn authenticate(
        conn: &mut Connection,
        vault: &MemoryVault,
        verifier: &FakeVerifier,
        account_id: &str,
        secret: &str,
    ) -> AccountCheckResult {
        let challenge = begin_authentication(conn, account_id).unwrap();
        complete_authentication(
            conn,
            vault,
            verifier,
            account_id,
            &challenge.challenge_id,
            secret,
        )
        .unwrap()
    }

    #[test]
    fn m3_ac_01_multiple_accounts_are_independent() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let first = create_account(&mut conn, root.path(), "bilibili", "主号").unwrap();
        let second = create_account(&mut conn, root.path(), "bilibili", "副号").unwrap();
        let updated = update_account(
            &mut conn,
            &first.id,
            Some("改名主号".to_string()),
            Some(false),
            None,
        )
        .unwrap();
        assert_eq!(updated.alias, "改名主号");
        assert_eq!(updated.status, AccountStatus::Disabled);
        let untouched = account_record(&conn, &second.id).unwrap();
        assert_eq!(untouched.alias, "副号");
        assert_eq!(untouched.status, AccountStatus::Unauthorized);
        assert_eq!(list_accounts(&conn).unwrap().len(), 2);
    }

    #[test]
    fn m3_ac_02_identity_verification_controls_active_state() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let vault = MemoryVault::default();
        let account = create_account(&mut conn, root.path(), "bilibili", "待验证").unwrap();
        let verifier = FakeVerifier::new(verified());
        let checked = authenticate(
            &mut conn,
            &vault,
            &verifier,
            &account.id,
            "SESSDATA=valid-secret",
        );
        assert_eq!(checked.account.status, AccountStatus::Active);
        assert_eq!(checked.account.uid, "mid-1001");
        assert_eq!(checked.account.nickname, "主账号");

        let failed_account =
            create_account(&mut conn, root.path(), "bilibili", "失败账号").unwrap();
        let failing =
            FakeVerifier::new(VerificationOutcome::Failed("平台拒绝身份校验".to_string()));
        let challenge = begin_authentication(&mut conn, &failed_account.id).unwrap();
        let error = complete_authentication(
            &mut conn,
            &vault,
            &failing,
            &failed_account.id,
            &challenge.challenge_id,
            "SESSDATA=bad-secret",
        )
        .unwrap_err();
        assert!(error.contains("平台拒绝身份校验"));
        let record = account_record(&conn, &failed_account.id).unwrap();
        assert_eq!(record.status, AccountStatus::Error);
        assert_eq!(record.uid, "");
    }

    #[test]
    fn m3_ac_03_expired_session_blocks_dependent_tasks() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let vault = MemoryVault::default();
        let account = create_account(&mut conn, root.path(), "bilibili", "过期测试").unwrap();
        let verifier = FakeVerifier::new(verified());
        authenticate(
            &mut conn,
            &vault,
            &verifier,
            &account.id,
            "SESSDATA=valid-secret",
        );
        require_active_account(&conn, &account.id).unwrap();

        verifier.set(VerificationOutcome::Expired);
        let checked = check_account_status(&mut conn, &vault, &verifier, &account.id).unwrap();
        assert_eq!(checked.account.status, AccountStatus::NeedsReauth);
        assert!(require_active_account(&conn, &account.id).is_err());
    }

    #[test]
    fn m3_ac_04_sqlite_and_audit_logs_do_not_contain_secret() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("accounts.sqlite3");
        let mut conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES(1, datetime('now'));",
        )
        .unwrap();
        initialize_schema(&mut conn).unwrap();
        let vault = MemoryVault::default();
        let secret = "SESSDATA=super-secret-cookie-value-8fd91";
        let account = create_account(&mut conn, temp.path(), "bilibili", "凭据测试").unwrap();
        let verifier = FakeVerifier::new(verified());
        authenticate(&mut conn, &vault, &verifier, &account.id, secret);
        let logs = list_audit_logs(&conn, 50).unwrap();
        assert!(logs.iter().all(|entry| !entry.detail.contains(secret)));
        let account_dump = format!("{:?}", list_accounts(&conn).unwrap());
        assert!(!account_dump.contains(secret));
        drop(conn);
        let bytes = fs::read(&db_path).unwrap();
        assert!(
            !bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }

    #[test]
    fn m3_ac_05_accounts_have_isolated_profiles_and_credentials() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let vault = MemoryVault::default();
        let first = create_account(&mut conn, root.path(), "bilibili", "账号 A").unwrap();
        let second = create_account(&mut conn, root.path(), "bilibili", "账号 B").unwrap();
        assert_ne!(first.profile_id, second.profile_id);
        assert_ne!(first.credential_ref, second.credential_ref);
        let first_dir = profile_directory(root.path(), &first.profile_id).unwrap();
        let second_dir = profile_directory(root.path(), &second.profile_id).unwrap();
        assert_ne!(first_dir, second_dir);
        vault
            .set(&first.credential_ref, "SESSDATA=account-a")
            .unwrap();
        assert_eq!(
            vault.get(&first.credential_ref).unwrap().as_deref(),
            Some("SESSDATA=account-a")
        );
        assert_eq!(vault.get(&second.credential_ref).unwrap(), None);
        fs::write(first_dir.join("cookie-test.marker"), b"only-a").unwrap();
        assert!(!second_dir.join("cookie-test.marker").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&first_dir).unwrap().permissions().mode() & 0o077,
                0
            );
        }
    }

    #[test]
    fn m3_ac_06_browser_fallback_exposes_manual_steps() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let account = create_account(&mut conn, root.path(), "bilibili", "浏览器回退").unwrap();
        let challenge = begin_authentication(&mut conn, &account.id).unwrap();
        assert!(challenge.login_url.starts_with("https://www.bilibili.com"));
        assert!(!challenge.manual_steps.is_empty());
        assert_eq!(challenge.credential_label, "平台会话 Cookie");
        let catalog = platform_catalog();
        let bilibili = catalog.iter().find(|entry| entry.id == "bilibili").unwrap();
        assert!(
            bilibili
                .auth_methods
                .iter()
                .any(|method| method.available && method.method == "browser")
        );
    }

    #[test]
    fn m3_ac_07_delete_cleans_session_but_retains_history() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let vault = MemoryVault::default();
        let first = create_account(&mut conn, root.path(), "bilibili", "待删除").unwrap();
        let second = create_account(&mut conn, root.path(), "bilibili", "保留账号").unwrap();
        let verifier = FakeVerifier::new(verified());
        authenticate(
            &mut conn,
            &vault,
            &verifier,
            &first.id,
            "SESSDATA=delete-me",
        );
        conn.execute(
            "INSERT INTO platform_works(id, account_id, title, external_id, created_at)
             VALUES('work-a', ?1, '历史作品 A', 'bv1', datetime('now'))",
            params![first.id],
        )
        .unwrap();
        let preview = deletion_preview(&conn, root.path(), &first.id).unwrap();
        assert_eq!(preview.retained_history_count, 1);
        assert!(preview.will_delete_credentials);
        let confirmation = DeletionConfirmation {
            credentials: true,
            sessions: true,
            history_retained_understood: true,
        };
        let result =
            delete_account(&mut conn, &vault, root.path(), &first.id, &confirmation).unwrap();
        assert_eq!(result.retained_history_count, 1);
        assert!(account_record(&conn, &first.id).is_err());
        assert!(account_record(&conn, &second.id).is_ok());
        assert_eq!(vault.get(&first.credential_ref).unwrap(), None);
        let work_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM platform_works WHERE id = 'work-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(work_count, 1);
        let orphan_account: Option<String> = conn
            .query_row(
                "SELECT account_id FROM platform_works WHERE id = 'work-a'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(orphan_account, None);
    }

    #[test]
    fn m3_ac_08_profile_presence_does_not_imply_active() {
        let mut conn = test_conn();
        let root = tempfile::tempdir().unwrap();
        let vault = MemoryVault::default();
        let account = create_account(&mut conn, root.path(), "bilibili", "状态真实").unwrap();
        let verifier = FakeVerifier::new(verified());
        authenticate(
            &mut conn,
            &vault,
            &verifier,
            &account.id,
            "SESSDATA=valid-secret",
        );
        assert!(
            profile_directory(root.path(), &account.profile_id)
                .unwrap()
                .exists()
        );

        verifier.set(VerificationOutcome::Failed("平台验证失败".to_string()));
        let checked = check_account_status(&mut conn, &vault, &verifier, &account.id).unwrap();
        assert_eq!(checked.account.status, AccountStatus::Error);
        assert_ne!(checked.account.status, AccountStatus::Active);
    }

    #[test]
    fn parses_bilibili_identity_and_expiry_fixtures() {
        assert!(matches!(
            parse_bilibili_identity(r#"{"code":0,"data":{"isLogin":true,"mid":123,"uname":"测试用户"}}"#),
            VerificationOutcome::Verified { uid, nickname } if uid == "123" && nickname == "测试用户"
        ));
        assert!(matches!(
            parse_bilibili_identity(
                r#"{"code":-101,"message":"账号未登录","data":{"isLogin":false}}"#
            ),
            VerificationOutcome::Expired
        ));
    }

    #[test]
    fn real_http_verifier_uses_fixture_server() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 4096];
            let _ = stream.read(&mut buffer).unwrap();
            let body = r#"{"code":0,"data":{"isLogin":true,"mid":42,"uname":"HTTP 测试"}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let verifier = PlatformHttpVerifier::with_bilibili_url(format!("http://{address}/nav"));
        let outcome = verifier.verify("bilibili", "SESSDATA=fixture");
        server.join().unwrap();
        assert!(matches!(
            outcome,
            VerificationOutcome::Verified { uid, nickname } if uid == "42" && nickname == "HTTP 测试"
        ));
    }

    #[test]
    fn system_keyring_smoke_is_explicitly_opt_in() {
        if std::env::var("CREATORRELAY_TEST_KEYCHAIN").as_deref() != Ok("1") {
            return;
        }
        let vault = KeyringVault;
        let reference = format!("test:{}", Uuid::new_v4());
        let secret = "creatorrelay-keychain-smoke-secret";
        vault.set(&reference, secret).unwrap();
        assert_eq!(vault.get(&reference).unwrap().as_deref(), Some(secret));
        vault.delete(&reference).unwrap();
        assert_eq!(vault.get(&reference).unwrap(), None);
    }
}
