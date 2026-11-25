use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn list_creator_center_links(
    platform_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<creator_centers::CreatorCenterLink>, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    creator_centers::list(&conn, platform_id.as_deref())
}

#[tauri::command]
pub(crate) fn save_creator_center_link(
    platform_id: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<creator_centers::CreatorCenterLink, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    creator_centers::save_url(&conn, &platform_id, &url)
}
