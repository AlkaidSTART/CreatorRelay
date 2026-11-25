use super::accounts::{account_record, profile_directory};
use super::credentials::{PlatformHttpVerifier, parse_bilibili_identity};
use super::*;
use rusqlite::{Connection, params};
use std::fs;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

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
    crate::database::migrate(&mut conn).unwrap();
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
fn schema_v2_migration_is_versioned_and_idempotent() {
    let mut conn = test_conn();
    crate::database::migrate(&mut conn).unwrap();
    let version: i64 = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, crate::database::CURRENT_SCHEMA_VERSION);
    for table in [
        "platform_accounts",
        "platform_sessions",
        "platform_works",
        "account_audit_logs",
    ] {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "missing table {table}");
    }
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

    let failed_account = create_account(&mut conn, root.path(), "bilibili", "失败账号").unwrap();
    let failing = FakeVerifier::new(VerificationOutcome::Failed("平台拒绝身份校验".to_string()));
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
    crate::database::migrate(&mut conn).unwrap();
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
    let rejected = DeletionConfirmation {
        credentials: true,
        sessions: false,
        history_retained_understood: true,
    };
    assert!(delete_account(&mut conn, &vault, root.path(), &first.id, &rejected).is_err());
    assert!(account_record(&conn, &first.id).is_ok());
    let confirmation = DeletionConfirmation {
        credentials: true,
        sessions: true,
        history_retained_understood: true,
    };
    let result = delete_account(&mut conn, &vault, root.path(), &first.id, &confirmation).unwrap();
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
        parse_bilibili_identity(r#"{"code":-101,"message":"账号未登录","data":{"isLogin":false}}"#),
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
