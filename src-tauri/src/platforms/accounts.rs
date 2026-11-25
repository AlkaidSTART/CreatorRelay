use super::catalog::{auth_methods, capabilities, platform_definition};
use super::credentials::{normalize_credential, set_private_mode};
use super::*;
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::{fs, path::Path};
use uuid::Uuid;

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

pub(super) fn profile_directory(
    session_root: &Path,
    profile_id: &str,
) -> Result<std::path::PathBuf, String> {
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
pub(super) struct AccountRecord {
    pub(super) id: String,
    pub(super) platform_id: String,
    pub(super) alias: String,
    pub(super) nickname: String,
    pub(super) uid: String,
    pub(super) status: AccountStatus,
    pub(super) enabled: bool,
    pub(super) is_default: bool,
    pub(super) profile_id: String,
    pub(super) credential_ref: String,
    pub(super) last_checked_at: Option<String>,
    pub(super) last_error: Option<String>,
    pub(super) challenge_id: Option<String>,
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

pub(super) fn account_record(conn: &Connection, account_id: &str) -> Result<AccountRecord, String> {
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

pub fn cancel_authentication(
    conn: &mut Connection,
    account_id: &str,
) -> Result<PlatformAccount, String> {
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
    tx.commit()
        .map_err(|_| "取消身份校验提交失败".to_string())?;
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
