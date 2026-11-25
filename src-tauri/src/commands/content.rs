use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn list_content_packages(
    state: State<'_, AppState>,
) -> Result<Vec<content::ContentPackageSummary>, String> {
    content::list_content_packages(&require_active_workspace(&state)?)
}

#[tauri::command]
pub(crate) fn create_content_package(
    kind: content::ContentKind,
    title: String,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::create_content_package(&require_active_workspace(&state)?, kind, &title)
}

#[tauri::command]
pub(crate) fn read_content_package(
    package_id: String,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::read_content_package(&require_active_workspace(&state)?, &package_id)
}

#[tauri::command]
pub(crate) fn save_content_package(
    package_id: String,
    input: content::ContentSaveInput,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::save_content_package(&require_active_workspace(&state)?, &package_id, input)
}

#[tauri::command]
pub(crate) fn import_content_asset(
    package_id: String,
    role: String,
    source_name: String,
    mime_type: String,
    bytes: Vec<u8>,
    state: State<'_, AppState>,
) -> Result<content::ContentAsset, String> {
    content::import_content_asset(
        &require_active_workspace(&state)?,
        &package_id,
        &role,
        &source_name,
        &mime_type,
        &bytes,
    )
}

#[tauri::command]
pub(crate) fn read_content_asset(
    package_id: String,
    asset_id: String,
    state: State<'_, AppState>,
) -> Result<content::ContentAssetContent, String> {
    content::read_content_asset(&require_active_workspace(&state)?, &package_id, &asset_id)
}

#[tauri::command]
pub(crate) fn derive_content_package(
    source_id: String,
    target_kind: content::ContentKind,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<content::ContentPackage, String> {
    content::derive_content_package(
        &require_active_workspace(&state)?,
        &source_id,
        target_kind,
        title,
    )
}

#[tauri::command]
pub(crate) fn list_platform_drafts(
    content_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<content::PlatformDraft>, String> {
    content::list_platform_drafts(&require_active_workspace(&state)?, content_id.as_deref())
}

#[tauri::command]
pub(crate) fn create_platform_drafts(
    content_id: String,
    targets: Vec<content::DraftTarget>,
    state: State<'_, AppState>,
) -> Result<Vec<content::PlatformDraft>, String> {
    if targets.len() > 12 {
        return Err("一次最多生成 12 个平台稿".to_string());
    }
    content::create_platform_drafts(&require_active_workspace(&state)?, &content_id, &targets)
}

#[tauri::command]
pub(crate) fn read_platform_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::read_platform_draft(&require_active_workspace(&state)?, &draft_id)
}

#[tauri::command]
pub(crate) fn save_platform_draft(
    draft_id: String,
    input: content::DraftSaveInput,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::save_platform_draft(&require_active_workspace(&state)?, &draft_id, input)
}

#[tauri::command]
pub(crate) fn acknowledge_platform_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::acknowledge_platform_draft_source(&require_active_workspace(&state)?, &draft_id)
}

#[tauri::command]
pub(crate) fn regenerate_platform_draft(
    draft_id: String,
    state: State<'_, AppState>,
) -> Result<content::PlatformDraft, String> {
    content::regenerate_platform_draft(&require_active_workspace(&state)?, &draft_id)
}

#[tauri::command]
pub(crate) fn read_platform_draft_asset(
    draft_id: String,
    asset_id: String,
    state: State<'_, AppState>,
) -> Result<content::ContentAssetContent, String> {
    content::read_platform_draft_asset(&require_active_workspace(&state)?, &draft_id, &asset_id)
}
