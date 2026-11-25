use chrono::Utc;
use rusqlite::{Connection, params};
use serde::Serialize;

pub const CURRENT_SCHEMA_VERSION: i64 = 2;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationReport {
    pub from_version: i64,
    pub to_version: i64,
    pub applied_versions: Vec<i64>,
}

pub fn current_version(conn: &Connection) -> Result<i64, String> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )
    .map_err(|_| "无法读取数据库 schema 版本".to_string())
}

pub fn migrate(conn: &mut Connection) -> Result<MigrationReport, String> {
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|_| "无法初始化数据库连接参数".to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations(
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        );",
    )
    .map_err(|_| "无法创建 schema 迁移记录表".to_string())?;

    let from_version = current_version(conn)?;
    if from_version > CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "数据库 schema 版本 {from_version} 高于当前应用支持的版本 {CURRENT_SCHEMA_VERSION}"
        ));
    }

    let mut applied_versions = Vec::new();
    for version in (from_version + 1)..=CURRENT_SCHEMA_VERSION {
        let tx = conn
            .transaction()
            .map_err(|_| format!("无法开始 schema v{version} 迁移事务"))?;
        tx.execute_batch(migration_sql(version)?)
            .map_err(|error| format!("schema v{version} 迁移失败：{error}"))?;
        tx.execute(
            "INSERT INTO schema_migrations(version, applied_at) VALUES(?1, ?2)",
            params![version, Utc::now().to_rfc3339()],
        )
        .map_err(|error| format!("schema v{version} 迁移版本记录失败：{error}"))?;
        tx.commit()
            .map_err(|_| format!("schema v{version} 迁移提交失败"))?;
        applied_versions.push(version);
    }

    let to_version = current_version(conn)?;
    if to_version != CURRENT_SCHEMA_VERSION {
        return Err(format!(
            "数据库迁移结束版本异常：期望 {CURRENT_SCHEMA_VERSION}，实际 {to_version}"
        ));
    }
    Ok(MigrationReport {
        from_version,
        to_version,
        applied_versions,
    })
}

fn migration_sql(version: i64) -> Result<&'static str, String> {
    match version {
        1 => Ok("CREATE TABLE IF NOT EXISTS app_settings(
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS documents(
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                content_hash TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS revisions(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                document_id TEXT NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                label TEXT NOT NULL,
                created_at TEXT NOT NULL
            );"),
        2 => Ok("CREATE TABLE IF NOT EXISTS platform_accounts(
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
            );"),
        _ => Err(format!("没有定义 schema v{version} 的迁移脚本")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v1_fixture(path: &std::path::Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES(1, '2026-01-01T00:00:00Z');
             CREATE TABLE app_settings(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE documents(id TEXT PRIMARY KEY, path TEXT NOT NULL UNIQUE, title TEXT NOT NULL, updated_at TEXT NOT NULL, content_hash TEXT NOT NULL);
             CREATE TABLE revisions(id INTEGER PRIMARY KEY AUTOINCREMENT, document_id TEXT NOT NULL, content TEXT NOT NULL, content_hash TEXT NOT NULL, label TEXT NOT NULL, created_at TEXT NOT NULL);
             INSERT INTO app_settings(key, value) VALUES('workspace_path', '/tmp/legacy-workspace');
             INSERT INTO documents(id, path, title, updated_at, content_hash) VALUES('doc-legacy', '/tmp/legacy-workspace/articles/doc/article.md', 'Legacy', '2026-01-01T00:00:00Z', 'hash-legacy');
             INSERT INTO revisions(document_id, content, content_hash, label, created_at) VALUES('doc-legacy', '# Legacy', 'hash-legacy', 'Initial', '2026-01-01T00:00:00Z');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn migrates_fresh_database_to_current_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        let report = migrate(&mut conn).unwrap();
        assert_eq!(report.from_version, 0);
        assert_eq!(report.to_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(report.applied_versions, vec![1, 2]);
        assert_eq!(current_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrates_v1_fixture_without_losing_data() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = v1_fixture(&temp.path().join("legacy.sqlite3"));
        let report = migrate(&mut conn).unwrap();
        assert_eq!(report.from_version, 1);
        assert_eq!(report.to_version, 2);
        assert_eq!(report.applied_versions, vec![2]);
        let workspace: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key='workspace_path'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(workspace, "/tmp/legacy-workspace");
        let title: String = conn
            .query_row(
                "SELECT title FROM documents WHERE id='doc-legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(title, "Legacy");
        let revision_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM revisions WHERE document_id='doc-legacy'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(revision_count, 1);
    }

    #[test]
    fn migration_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        let report = migrate(&mut conn).unwrap();
        assert_eq!(report.from_version, 2);
        assert_eq!(report.to_version, 2);
        assert!(report.applied_versions.is_empty());
    }

    #[test]
    fn failed_migration_rolls_back_and_preserves_database() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = v1_fixture(&temp.path().join("conflict.sqlite3"));
        conn.execute_batch("CREATE TABLE platform_accounts(id TEXT PRIMARY KEY);")
            .unwrap();
        let error = migrate(&mut conn).unwrap_err();
        assert!(error.contains("schema v2 迁移失败"));
        assert_eq!(current_version(&conn).unwrap(), 1);
        let workspace: String = conn
            .query_row(
                "SELECT value FROM app_settings WHERE key='workspace_path'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(workspace, "/tmp/legacy-workspace");
        let platform_sessions_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='platform_sessions'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(platform_sessions_exists, 0);
    }
}
