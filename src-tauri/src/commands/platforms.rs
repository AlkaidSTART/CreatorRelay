use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn list_platform_catalog() -> Vec<platforms::PlatformCatalogEntry> {
    platforms::platform_catalog()
}

#[tauri::command]
pub(crate) fn list_platform_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<platforms::PlatformAccount>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::list_accounts(&conn)
}

#[tauri::command]
pub(crate) fn create_platform_account(
    platform_id: String,
    alias: String,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::create_account(
        &mut conn,
        &state.platform_session_root,
        &platform_id,
        &alias,
    )
}

#[tauri::command]
pub(crate) fn update_platform_account(
    account_id: String,
    alias: Option<String>,
    enabled: Option<bool>,
    is_default: Option<bool>,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::update_account(&mut conn, &account_id, alias, enabled, is_default)
}

#[tauri::command]
pub(crate) fn begin_platform_authentication(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::AuthChallenge, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::begin_authentication(&mut conn, &account_id)
}

#[tauri::command]
pub(crate) fn cancel_platform_authentication(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::cancel_authentication(&mut conn, &account_id)
}

#[tauri::command]
pub(crate) fn complete_platform_authentication(
    account_id: String,
    challenge_id: String,
    credential: String,
    state: State<'_, AppState>,
) -> Result<platforms::AccountCheckResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::complete_authentication(
        &mut conn,
        &platforms::KeyringVault,
        &platforms::PlatformHttpVerifier::default(),
        &account_id,
        &challenge_id,
        &credential,
    )
}

#[tauri::command]
pub(crate) fn check_platform_account_status(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::AccountCheckResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::check_account_status(
        &mut conn,
        &platforms::KeyringVault,
        &platforms::PlatformHttpVerifier::default(),
        &account_id,
    )
}

#[tauri::command]
pub(crate) fn ensure_platform_account_active(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::PlatformAccount, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::require_active_account(&conn, &account_id)
}

#[tauri::command]
pub(crate) fn preview_platform_account_deletion(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<platforms::DeletionPreview, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::deletion_preview(&conn, &state.platform_session_root, &account_id)
}

#[tauri::command]
pub(crate) fn delete_platform_account(
    account_id: String,
    confirmation: platforms::DeletionConfirmation,
    state: State<'_, AppState>,
) -> Result<platforms::DeletionResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::delete_account(
        &mut conn,
        &platforms::KeyringVault,
        &state.platform_session_root,
        &account_id,
        &confirmation,
    )
}

#[tauri::command]
pub(crate) fn list_platform_audit_logs(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<platforms::AccountAuditLog>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    platforms::list_audit_logs(&conn, limit.unwrap_or(50))
}
