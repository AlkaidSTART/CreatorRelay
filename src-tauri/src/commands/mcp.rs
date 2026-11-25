use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn get_mcp_host_status(
    state: State<'_, AppState>,
) -> Result<mcp::McpHostStatus, String> {
    state.mcp_host.status()
}

#[tauri::command]
pub(crate) fn start_mcp_server(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let settings = mcp::get_settings(&conn)?;
    state.mcp_host.start(Some(mcp::McpHostConfig {
        auto_restart: settings.auto_restart,
        max_restarts: settings.max_restarts,
    }))
}

#[tauri::command]
pub(crate) fn stop_mcp_server(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    state.mcp_host.stop()
}

#[tauri::command]
pub(crate) fn mcp_health_check(state: State<'_, AppState>) -> Result<mcp::McpHostStatus, String> {
    state.mcp_host.health_check()
}

#[tauri::command]
pub(crate) fn list_platform_mcp_servers() -> Vec<mcp::PlatformMcpProfile> {
    mcp::platform_mcp_catalog()
}

#[tauri::command]
pub(crate) fn mcp_list_tools(
    state: State<'_, AppState>,
) -> Result<Vec<mcp::McpToolDefinition>, String> {
    state.mcp_host.list_tools()
}

#[tauri::command]
pub(crate) fn mcp_cancel_tool(call_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.mcp_host.cancel(&call_id)
}

#[tauri::command]
pub(crate) fn get_mcp_settings(state: State<'_, AppState>) -> Result<mcp::McpSettings, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::get_settings(&conn)
}

#[tauri::command]
pub(crate) fn save_mcp_settings(
    input: mcp::McpSettingsInput,
    state: State<'_, AppState>,
) -> Result<mcp::McpSettings, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let settings = mcp::save_settings(&conn, input)?;
    state.mcp_host.configure(mcp::McpHostConfig {
        auto_restart: settings.auto_restart,
        max_restarts: settings.max_restarts,
    })?;
    Ok(settings)
}

#[tauri::command]
pub(crate) fn mcp_preview_tool_call(
    request: mcp::McpPreviewRequest,
    state: State<'_, AppState>,
) -> Result<mcp::McpAuthorizationPreview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    mcp::preview_tool_call(&conn, &workspace, request)
}

#[tauri::command]
pub(crate) fn mcp_call_tool(
    request: mcp::McpCallRequest,
    state: State<'_, AppState>,
) -> Result<mcp::McpCallResult, String> {
    let mut conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    mcp::call_tool(
        &mut conn,
        &state.plugin_runtime,
        &workspace,
        &state.mcp_host,
        request,
    )
}

#[tauri::command]
pub(crate) fn list_mcp_audit(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<mcp::McpAuditRecord>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::list_audit(&conn, limit.unwrap_or(50))
}

#[tauri::command]
pub(crate) fn mcp_security_self_test() -> mcp::McpSecurityEvidence {
    mcp::security_evidence()
}

#[tauri::command]
pub(crate) fn list_assistant_threads(
    state: State<'_, AppState>,
) -> Result<Vec<mcp::AssistantThread>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::list_assistant_threads(&conn)
}

#[tauri::command]
pub(crate) fn create_assistant_thread(
    title: Option<String>,
    provider_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantThread, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::create_assistant_thread(&conn, title, provider_id)
}

#[tauri::command]
pub(crate) fn get_assistant_thread(
    thread_id: String,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantThreadDetail, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::get_assistant_thread(&conn, &thread_id)
}

#[tauri::command]
pub(crate) fn assistant_plan_message(
    prompt: String,
    thread_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantPlan, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    mcp::plan_assistant_message(&conn, &prompt, thread_id.as_deref())
}

#[tauri::command]
pub(crate) fn assistant_context_preview(
    provider_id: Option<String>,
    content_id: Option<String>,
    scope: Option<mcp::ModelContextScope>,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantContextPreview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    let scope = match scope {
        Some(scope) => scope,
        None => mcp::default_context_scope(&conn)?,
    };
    mcp::preview_assistant_context(
        &conn,
        &workspace,
        provider_id.as_deref(),
        content_id.as_deref(),
        &scope,
    )
}
