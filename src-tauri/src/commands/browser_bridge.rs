use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn creator_browser_status(
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserBridgeStatus, String> {
    state.browser_bridge.status()
}

#[tauri::command]
pub(crate) fn creator_browser_start(
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserBridgeStatus, String> {
    state.browser_bridge.start()
}

#[tauri::command]
pub(crate) fn creator_browser_stop(
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserBridgeStatus, String> {
    state.browser_bridge.stop()
}

#[tauri::command]
pub(crate) fn creator_browser_open(
    platform_id: String,
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserPageSnapshot, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    state
        .browser_bridge
        .open_creator_center(&conn, &platform_id)
}

#[tauri::command]
pub(crate) fn creator_browser_read_page(
    platform_id: String,
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserPageSnapshot, String> {
    state.browser_bridge.read_current_page(&platform_id)
}
