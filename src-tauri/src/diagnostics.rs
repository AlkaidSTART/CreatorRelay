use crate::{database, logging};
use rusqlite::Connection;
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnostics {
    pub app_version: String,
    pub data_dir: String,
    pub database_path: String,
    pub log_path: String,
    pub schema_version: i64,
    pub target_schema_version: i64,
    pub workspace_path: Option<String>,
    pub log_available: bool,
    pub database_integrity: String,
    pub uptime_seconds: u64,
}

pub fn collect(
    conn: &Connection,
    data_dir: &Path,
    database_path: &Path,
    log_path: &Path,
    workspace_path: Option<PathBuf>,
    uptime: Duration,
) -> Result<RuntimeDiagnostics, String> {
    let schema_version = database::current_version(conn)?;
    let database_integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| "数据库完整性检查失败".to_string())?;
    Ok(RuntimeDiagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: data_dir.to_string_lossy().to_string(),
        database_path: database_path.to_string_lossy().to_string(),
        log_path: log_path.to_string_lossy().to_string(),
        schema_version,
        target_schema_version: database::CURRENT_SCHEMA_VERSION,
        workspace_path: workspace_path.map(|path| path.to_string_lossy().to_string()),
        log_available: log_path.exists(),
        database_integrity,
        uptime_seconds: uptime.as_secs(),
    })
}

pub fn run_probe(
    conn: &Connection,
    log_path: &Path,
    probe: &str,
    request_context: Option<&str>,
) -> Result<(), String> {
    match probe {
        "ipc_payload" => {
            let payload = request_context.unwrap_or(
                r#"{"password":"m1-diagnostic-password","cookie":"SESSDATA=m1-diagnostic-cookie","token":"m1-diagnostic-token","context":"workspace-open"}"#,
            );
            logging::append_log(
                log_path,
                "error",
                "diagnostics.ipc_payload",
                &format!("模拟 IPC 参数校验失败 request={payload}"),
            )?;
            Err("诊断探针：IPC 参数校验失败，错误上下文已安全写入日志".to_string())
        }
        "database_error" => {
            let error = conn
                .query_row(
                    "SELECT value FROM __creatorrelay_missing_diagnostic_table__",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .expect_err("diagnostic table must not exist")
                .to_string();
            logging::append_log(
                log_path,
                "error",
                "diagnostics.database",
                &format!("模拟数据库查询失败 error={error}"),
            )?;
            Err("诊断探针：数据库查询失败，错误阶段已安全写入日志".to_string())
        }
        _ => {
            logging::append_log(
                log_path,
                "warn",
                "diagnostics.invalid_probe",
                &format!("未知诊断探针：{probe}"),
            )?;
            Err("未知诊断探针".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn migrated_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        database::migrate(&mut conn).unwrap();
        conn
    }

    #[test]
    fn runtime_diagnostics_report_schema_and_integrity() {
        let temp = tempfile::tempdir().unwrap();
        let conn = migrated_conn();
        let log_path = logging::initialize(temp.path()).unwrap();
        let diagnostics = collect(
            &conn,
            temp.path(),
            &temp.path().join("creatorrelay.sqlite3"),
            &log_path,
            Some(temp.path().join("workspace")),
            Duration::from_secs(12),
        )
        .unwrap();
        assert_eq!(diagnostics.schema_version, database::CURRENT_SCHEMA_VERSION);
        assert_eq!(
            diagnostics.target_schema_version,
            database::CURRENT_SCHEMA_VERSION
        );
        assert_eq!(diagnostics.database_integrity, "ok");
        assert!(diagnostics.log_available);
    }

    #[test]
    fn diagnostic_probes_log_context_and_leave_database_usable() {
        let temp = tempfile::tempdir().unwrap();
        let conn = migrated_conn();
        let log_path = logging::initialize(temp.path()).unwrap();
        let ipc_error = run_probe(
            &conn,
            &log_path,
            "ipc_payload",
            Some(r#"{"password":"probe-password","cookie":"SESSDATA=probe-cookie","token":"probe-token","context":"ipc-test"}"#),
        )
        .unwrap_err();
        assert!(ipc_error.contains("IPC 参数校验失败"));
        let database_error = run_probe(&conn, &log_path, "database_error", None).unwrap_err();
        assert!(database_error.contains("数据库查询失败"));
        assert_eq!(
            database::current_version(&conn).unwrap(),
            database::CURRENT_SCHEMA_VERSION
        );
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("diagnostics.ipc_payload"));
        assert!(content.contains("diagnostics.database"));
        assert!(content.contains("ipc-test"));
        assert!(!content.contains("probe-password"));
        assert!(!content.contains("probe-cookie"));
        assert!(!content.contains("probe-token"));
    }
}
