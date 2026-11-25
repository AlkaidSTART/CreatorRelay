use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn list_ai_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ai::AiProviderSummary>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::list_providers(&conn)
}

#[tauri::command]
pub(crate) fn save_ai_provider(
    input: ai::AiProviderInput,
    state: State<'_, AppState>,
) -> Result<ai::AiProviderSummary, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::save_provider_with_vault(&conn, &input, &ai::keyring_vault())
}

#[tauri::command]
pub(crate) fn delete_ai_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::delete_provider_with_vault(&conn, &provider_id, &ai::keyring_vault())
}

#[tauri::command]
pub(crate) fn set_default_ai_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ai::AiProviderSummary, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::set_default_provider(&conn, &provider_id)
}

#[tauri::command]
pub(crate) fn test_ai_provider(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<ai::AiTestResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::test_provider_with_vault(&conn, &provider_id, &ai::keyring_vault())
}

#[tauri::command]
pub(crate) fn ai_generate(
    request: ai::AiGenerateRequest,
    state: State<'_, AppState>,
) -> Result<ai::AiGenerateResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    ai::generate_with_vault(&conn, &request, &ai::keyring_vault())
}
