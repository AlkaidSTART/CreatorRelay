use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn validate_adapter_manifest(manifest_json: String) -> plugins::ManifestValidation {
    plugins::validate_manifest(&manifest_json)
}

#[tauri::command]
pub(crate) fn list_adapter_catalog(
    state: State<'_, AppState>,
) -> Result<Vec<plugins::AdapterCatalogEntry>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    plugins::list_catalog(&conn, &state.plugin_runtime)
}

#[tauri::command]
pub(crate) fn list_plugins(
    state: State<'_, AppState>,
) -> Result<Vec<plugins::PluginSummary>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    plugins::list_plugins(&conn, &state.plugin_runtime)
}

#[tauri::command]
pub(crate) fn install_official_adapter(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::install_official_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.install",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn save_plugin_permissions(
    plugin_id: String,
    decisions: Vec<plugins::PluginPermissionDecision>,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result =
        plugins::set_plugin_permissions(&mut conn, &state.plugin_runtime, &plugin_id, &decisions);
    log_command_outcome(
        &state,
        "plugin.permissions",
        &format!("plugin={plugin_id} decisions={}", decisions.len()),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn enable_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::enable_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.enable",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn disable_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::disable_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.disable",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn restart_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::restart_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.restart",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn rollback_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginSummary, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::rollback_plugin(&mut conn, &state.plugin_runtime, &plugin_id);
    log_command_outcome(
        &state,
        "plugin.rollback",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn probe_plugin_capabilities(
    plugin_id: String,
    account_ref: Option<String>,
    state: State<'_, AppState>,
) -> Result<plugins::PluginProbeResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::probe_plugin_capabilities(
        &mut conn,
        &state.plugin_runtime,
        &plugin_id,
        account_ref,
    );
    if let Ok(probe) = &result
        && !probe.ok
    {
        let _ = logging::append_log(
            &state.log_path,
            "error",
            "plugin.probe_failed",
            &format!(
                "plugin={} account={} error={}",
                plugin_id,
                probe.account_ref,
                probe.error.as_deref().unwrap_or("unknown")
            ),
        );
    } else {
        log_command_outcome(
            &state,
            "plugin.probe",
            &format!("plugin={plugin_id}"),
            &result,
        );
    }
    result
}

#[tauri::command]
pub(crate) fn simulate_plugin_fault(
    plugin_id: String,
    fault: String,
    state: State<'_, AppState>,
) -> Result<plugins::PluginFaultResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result =
        plugins::simulate_plugin_fault(&mut conn, &state.plugin_runtime, &plugin_id, &fault);
    match &result {
        Ok(fault_result) => {
            let _ = logging::append_log(
                &state.log_path,
                "error",
                "plugin.fault_detected",
                &format!(
                    "plugin={} fault={} error={}",
                    plugin_id, fault, fault_result.error
                ),
            );
        }
        Err(error) => {
            let _ = logging::append_log(
                &state.log_path,
                "error",
                "plugin.fault",
                &format!("plugin={plugin_id} fault={fault} error={error}"),
            );
        }
    }
    result
}

#[tauri::command]
pub(crate) fn uninstall_plugin(
    plugin_id: String,
    delete_plugin_data: bool,
    state: State<'_, AppState>,
) -> Result<plugins::PluginUninstallResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::uninstall_plugin(
        &mut conn,
        &state.plugin_runtime,
        &plugin_id,
        delete_plugin_data,
    );
    log_command_outcome(
        &state,
        "plugin.uninstall",
        &format!("plugin={plugin_id} deleteData={delete_plugin_data}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn list_plugin_events(
    plugin_id: Option<String>,
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<plugins::PluginEvent>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    plugins::list_plugin_events(&conn, plugin_id.as_deref(), limit)
}

#[tauri::command]
pub(crate) fn run_plugin_security_self_test(
    plugin_id: String,
    state: State<'_, AppState>,
) -> Result<plugins::SecuritySelfTestResult, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区后再运行安全边界自检")?;
    let canary = workspace.join(format!(".creatorrelay-sandbox-canary-{}", Uuid::new_v4()));
    fs::write(&canary, b"test-only-workspace-canary")
        .map_err(|error| format!("无法创建工作区测试哨兵：{error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&canary, fs::Permissions::from_mode(0o600));
    }
    let result = plugins::security_self_test(
        &conn,
        &state.plugin_runtime,
        &plugin_id,
        Some(canary.clone()),
    );
    let _ = fs::remove_file(&canary);
    log_command_outcome(
        &state,
        "plugin.security_self_test",
        &format!("plugin={plugin_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn run_publish_payload_self_check() -> Vec<plugins::PublishPayloadCheckResult> {
    plugins::payload_self_check()
}

#[tauri::command]
pub(crate) fn validate_plugin_publish_input(
    plugin_id: String,
    account_ref: Option<String>,
    input: plugins::PublishInput,
    state: State<'_, AppState>,
) -> Result<plugins::PluginPublishValidationResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = plugins::validate_plugin_publish_input(
        &mut conn,
        &state.plugin_runtime,
        &plugin_id,
        account_ref,
        &input,
    );
    log_command_outcome(
        &state,
        "plugin.validate_publish",
        &format!("plugin={plugin_id} kind={}", input.content_kind.as_str()),
        &result,
    );
    result
}
