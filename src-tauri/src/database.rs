use chrono::Utc;
use rusqlite::{Connection, params};
use serde::Serialize;

pub const CURRENT_SCHEMA_VERSION: i64 = 7;

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
    migrate_to_version(conn, CURRENT_SCHEMA_VERSION)
}

fn migrate_to_version(
    conn: &mut Connection,
    target_version: i64,
) -> Result<MigrationReport, String> {
    if target_version > CURRENT_SCHEMA_VERSION || target_version < 0 {
        return Err(format!("不支持迁移到 schema v{target_version}"));
    }
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
    if from_version > target_version {
        return Err(format!(
            "数据库 schema 版本 {from_version} 高于本次迁移目标 {target_version}"
        ));
    }

    let mut applied_versions = Vec::new();
    for version in (from_version + 1)..=target_version {
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
    if to_version != target_version {
        return Err(format!(
            "数据库迁移结束版本异常：期望 {target_version}，实际 {to_version}"
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
        3 => Ok("CREATE TABLE IF NOT EXISTS plugins(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                version TEXT NOT NULL,
                adapter_contract_version INTEGER NOT NULL,
                runtime_kind TEXT NOT NULL,
                runtime_profile TEXT,
                entry TEXT NOT NULL,
                source TEXT NOT NULL,
                trust_level TEXT NOT NULL,
                status TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 0,
                manifest_json TEXT NOT NULL,
                manifest_hash TEXT NOT NULL,
                previous_manifest_json TEXT,
                previous_manifest_hash TEXT,
                installed_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_error TEXT,
                install_path TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS plugin_grants(
                plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
                permission TEXT NOT NULL,
                granted INTEGER NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY(plugin_id, permission)
            );
            CREATE TABLE IF NOT EXISTS plugin_capability_checks(
                plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
                account_ref TEXT NOT NULL,
                content_kind TEXT NOT NULL,
                read_state TEXT NOT NULL,
                publish_state TEXT NOT NULL,
                reason TEXT,
                checked_at TEXT NOT NULL,
                PRIMARY KEY(plugin_id, account_ref, content_kind)
            );
            CREATE TABLE IF NOT EXISTS plugin_events(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                plugin_id TEXT NOT NULL,
                plugin_version TEXT NOT NULL,
                action TEXT NOT NULL,
                detail TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS plugin_events_plugin_id ON plugin_events(plugin_id, id);
            CREATE INDEX IF NOT EXISTS plugin_capability_checks_plugin_id ON plugin_capability_checks(plugin_id);"),
        4 => Ok("CREATE TABLE IF NOT EXISTS ai_providers(
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                model TEXT NOT NULL,
                api_style TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                has_secret INTEGER NOT NULL DEFAULT 0,
                last_test_status TEXT,
                last_test_error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS ai_providers_one_default
                ON ai_providers(is_default) WHERE is_default = 1;
            CREATE TABLE IF NOT EXISTS ai_call_audit(
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL,
                operation TEXT NOT NULL,
                content_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                error_code TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS ai_call_audit_provider_id ON ai_call_audit(provider_id, created_at);"),
        5 => Ok("CREATE TABLE IF NOT EXISTS publish_tasks(
                id TEXT PRIMARY KEY,
                source_type TEXT NOT NULL,
                source_id TEXT NOT NULL,
                content_id TEXT NOT NULL,
                content_version TEXT NOT NULL,
                content_kind TEXT NOT NULL,
                platform_id TEXT NOT NULL,
                platform_name TEXT NOT NULL,
                account_id TEXT NOT NULL,
                account_alias TEXT NOT NULL,
                adapter_id TEXT,
                title TEXT NOT NULL,
                body TEXT NOT NULL DEFAULT '',
                tags_json TEXT NOT NULL DEFAULT '[]',
                cover_json TEXT,
                media_json TEXT NOT NULL DEFAULT '[]',
                execution_mode TEXT NOT NULL DEFAULT 'manual',
                status TEXT NOT NULL,
                is_paused INTEGER NOT NULL DEFAULT 0,
                pause_reason TEXT,
                scheduled_at TEXT,
                confirmation_token TEXT,
                confirmation_hash TEXT,
                confirmation_expires_at TEXT,
                confirmed_at TEXT,
                external_id TEXT,
                external_url TEXT,
                error_code TEXT,
                error_message TEXT,
                next_action TEXT,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                idempotency_key TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS publish_tasks_account_id
                ON publish_tasks(account_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS publish_tasks_platform_status
                ON publish_tasks(platform_id, status, created_at DESC);
            CREATE TABLE IF NOT EXISTS publish_task_events(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task_id TEXT NOT NULL REFERENCES publish_tasks(id) ON DELETE CASCADE,
                action TEXT NOT NULL,
                detail TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS publish_task_events_task_id
                ON publish_task_events(task_id, id);
            CREATE TABLE IF NOT EXISTS external_works(
                id TEXT PRIMARY KEY,
                source_key TEXT NOT NULL UNIQUE,
                account_id TEXT,
                platform_id TEXT NOT NULL,
                platform_name TEXT NOT NULL,
                account_alias TEXT NOT NULL,
                external_id TEXT NOT NULL,
                title TEXT NOT NULL,
                content_kind TEXT NOT NULL,
                original_content_type TEXT NOT NULL,
                url TEXT,
                published_at TEXT NOT NULL,
                source TEXT NOT NULL,
                synced_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(platform_id, account_id, external_id)
            );
            CREATE INDEX IF NOT EXISTS external_works_filter
                ON external_works(platform_id, content_kind, published_at DESC);
            CREATE INDEX IF NOT EXISTS external_works_account
                ON external_works(account_id, published_at DESC);
            CREATE TABLE IF NOT EXISTS metric_snapshots(
                id TEXT PRIMARY KEY,
                work_id TEXT NOT NULL REFERENCES external_works(id) ON DELETE CASCADE,
                metric_name TEXT NOT NULL,
                metric_definition TEXT NOT NULL DEFAULT '',
                unit TEXT NOT NULL DEFAULT '',
                value REAL NOT NULL,
                captured_at TEXT NOT NULL,
                snapshot_day TEXT NOT NULL,
                source TEXT NOT NULL,
                canonical_key TEXT,
                comparable INTEGER NOT NULL DEFAULT 0,
                UNIQUE(work_id, metric_name, snapshot_day)
            );
            CREATE INDEX IF NOT EXISTS metric_snapshots_work
                ON metric_snapshots(work_id, captured_at);
            CREATE INDEX IF NOT EXISTS metric_snapshots_time
                ON metric_snapshots(captured_at);
            CREATE TABLE IF NOT EXISTS sync_runs(
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                platform_id TEXT NOT NULL,
                adapter_id TEXT NOT NULL,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                imported_count INTEGER NOT NULL DEFAULT 0,
                updated_count INTEGER NOT NULL DEFAULT 0,
                skipped_count INTEGER NOT NULL DEFAULT 0,
                error_count INTEGER NOT NULL DEFAULT 0,
                started_at TEXT NOT NULL,
                completed_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS sync_runs_account
                ON sync_runs(account_id, completed_at DESC);"),
        6 => Ok("CREATE TABLE IF NOT EXISTS assistant_threads(
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                provider_id TEXT,
                provider_name TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS assistant_messages(
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL REFERENCES assistant_threads(id) ON DELETE CASCADE,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                provider_name TEXT,
                model TEXT,
                tool_call_json TEXT,
                result_status TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS assistant_messages_thread
                ON assistant_messages(thread_id, created_at, id);
            CREATE TABLE IF NOT EXISTS mcp_tool_audit(
                id TEXT PRIMARY KEY,
                thread_id TEXT REFERENCES assistant_threads(id) ON DELETE SET NULL,
                call_id TEXT NOT NULL UNIQUE,
                server_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                permission TEXT NOT NULL,
                content_kind TEXT,
                target_summary TEXT NOT NULL DEFAULT '',
                parameter_summary TEXT NOT NULL DEFAULT '',
                status TEXT NOT NULL,
                error_code TEXT,
                error_message TEXT,
                started_at TEXT NOT NULL,
                completed_at TEXT
            );
            CREATE INDEX IF NOT EXISTS mcp_tool_audit_thread
                ON mcp_tool_audit(thread_id, started_at DESC);
            CREATE INDEX IF NOT EXISTS mcp_tool_audit_status
                ON mcp_tool_audit(status, started_at DESC);
            CREATE TABLE IF NOT EXISTS mcp_authorization_tokens(
                token TEXT PRIMARY KEY,
                call_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                payload_hash TEXT NOT NULL,
                permission TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                used_at TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS mcp_settings(
                id INTEGER PRIMARY KEY CHECK(id = 1),
                read_enabled INTEGER NOT NULL DEFAULT 1,
                write_policy TEXT NOT NULL DEFAULT 'confirm',
                external_policy TEXT NOT NULL DEFAULT 'confirm_each_time',
                auto_restart INTEGER NOT NULL DEFAULT 1,
                max_restarts INTEGER NOT NULL DEFAULT 3,
                default_context_scope TEXT NOT NULL DEFAULT 'title_summary_media',
                updated_at TEXT NOT NULL
            );
            INSERT OR IGNORE INTO mcp_settings(
                id, read_enabled, write_policy, external_policy, auto_restart,
                max_restarts, default_context_scope, updated_at
            ) VALUES(1, 1, 'confirm', 'confirm_each_time', 1, 3, 'title_summary_media', '2026-11-25T00:00:00Z');"),
        7 => Ok("ALTER TABLE external_works
                ADD COLUMN raw_fields_json TEXT NOT NULL DEFAULT '{}';"),
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
        assert_eq!(report.applied_versions, vec![1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(current_version(&conn).unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn migrates_v1_fixture_without_losing_data() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = v1_fixture(&temp.path().join("legacy.sqlite3"));
        let report = migrate(&mut conn).unwrap();
        assert_eq!(report.from_version, 1);
        assert_eq!(report.to_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(report.applied_versions, vec![2, 3, 4, 5, 6, 7]);
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
    fn migrates_v2_fixture_and_preserves_existing_account_data() {
        let temp = tempfile::tempdir().unwrap();
        let mut conn = Connection::open(temp.path().join("m3.sqlite3")).unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL);
             INSERT INTO schema_migrations(version, applied_at) VALUES(1, '2026-01-01T00:00:00Z');
             INSERT INTO schema_migrations(version, applied_at) VALUES(2, '2026-01-01T00:00:00Z');
             CREATE TABLE platform_accounts(
                id TEXT PRIMARY KEY,
                platform_id TEXT NOT NULL,
                alias TEXT NOT NULL
             );
             INSERT INTO platform_accounts(id, platform_id, alias)
             VALUES('account-m3', 'bilibili', '主账号');",
        )
        .unwrap();
        let report = migrate(&mut conn).unwrap();
        assert_eq!(report.from_version, 2);
        assert_eq!(report.to_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(report.applied_versions, vec![3, 4, 5, 6, 7]);
        let alias: String = conn
            .query_row(
                "SELECT alias FROM platform_accounts WHERE id='account-m3'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(alias, "主账号");
        let plugin_tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='plugins'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(plugin_tables, 1);
    }

    #[test]
    fn migration_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        let report = migrate(&mut conn).unwrap();
        assert_eq!(report.from_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(report.to_version, CURRENT_SCHEMA_VERSION);
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
