use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn assistant_generate_reply(
    request: mcp::AssistantGenerateRequest,
    state: State<'_, AppState>,
) -> Result<mcp::AssistantGenerateResult, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    let workspace = active_workspace(&conn)?.ok_or_else(|| "请先选择工作区".to_string())?;
    mcp::generate_assistant_reply(&conn, &workspace, request, &ai::keyring_vault())
}

#[tauri::command]
pub(crate) fn get_ecosystem_overview(
    state: State<'_, AppState>,
) -> Result<ecosystem::EcosystemOverview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    ecosystem::ecosystem_overview(&conn)
}
