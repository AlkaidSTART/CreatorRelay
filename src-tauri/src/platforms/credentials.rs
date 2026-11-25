use super::*;
use std::{fs, path::Path, time::Duration};

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

pub(super) fn normalize_credential(raw: &str) -> Result<String, String> {
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
pub(super) fn set_private_mode(path: &Path, mode: u32) -> Result<(), String> {
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

