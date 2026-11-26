use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn creator_browser_status(
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserBridgeStatus, String> {
    state.browser_bridge.status()
}

#[tauri::command]
pub(crate) fn creator_browser_stop(
    state: State<'_, AppState>,
) -> Result<browser_bridge::BrowserBridgeStatus, String> {
    state.browser_bridge.stop()
}
