use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn get_ecosystem_overview(
    state: State<'_, AppState>,
) -> Result<ecosystem::EcosystemOverview, String> {
    let conn = state.db.lock().map_err(|error| error.to_string())?;
    ecosystem::ecosystem_overview(&conn)
}
