use crate::*;
use tauri::State;

#[tauri::command]
pub(crate) fn create_publish_tasks(
    input: operations::CreatePublishTasksInput,
    state: State<'_, AppState>,
) -> Result<operations::CreatePublishTasksResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区后再创建发布任务")?;
    let result = operations::create_publish_tasks(&mut conn, &workspace, &input);
    log_command_outcome(
        &state,
        "publishing.create_tasks",
        &format!(
            "source={}:{} targets={}",
            input.source_type as u8,
            input.source_id,
            input.targets.len()
        ),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn list_publish_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTask>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_publish_tasks(&conn)
}

#[tauri::command]
pub(crate) fn prepare_publish_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<operations::PublishPreview, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let result =
        operations::prepare_publish_task(&mut conn, &state.plugin_runtime, &workspace, &task_id);
    log_command_outcome(
        &state,
        "publishing.prepare",
        &format!("task={task_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn confirm_publish_task(
    task_id: String,
    confirmation_token: String,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    let result = operations::confirm_publish_task(
        &mut conn,
        &state.plugin_runtime,
        &workspace,
        &task_id,
        &confirmation_token,
    );
    log_command_outcome(
        &state,
        "publishing.confirm",
        &format!("task={task_id}"),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn cancel_publish_task(
    task_id: String,
    reason: Option<String>,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::cancel_publish_task(&conn, &task_id, reason.as_deref())
}

#[tauri::command]
pub(crate) fn retry_publish_task(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::retry_publish_task(&conn, &task_id)
}

#[tauri::command]
pub(crate) fn update_publish_task_schedule(
    task_id: String,
    scheduled_at: Option<String>,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::update_publish_task_schedule(&conn, &task_id, scheduled_at.as_deref())
}

#[tauri::command]
pub(crate) fn resolve_uncertain_publish_task(
    task_id: String,
    input: operations::ResolveUncertainInput,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::resolve_uncertain_publish_task(&mut conn, &task_id, &input)
}

#[tauri::command]
pub(crate) fn export_manual_publish_package(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<operations::ManualPublishPackage, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    operations::export_manual_publish_package(&conn, &workspace, &task_id)
}

#[tauri::command]
pub(crate) fn complete_manual_publish_task(
    task_id: String,
    input: operations::ManualCompletionInput,
    state: State<'_, AppState>,
) -> Result<operations::PublishTask, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::complete_manual_publish_task(&mut conn, &task_id, &input)
}

#[tauri::command]
pub(crate) fn resume_account_publish_tasks(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTask>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::resume_account_tasks(&conn, &account_id)
}

#[tauri::command]
pub(crate) fn run_due_publish_tasks(
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTask>, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?.ok_or("请先选择工作区")?;
    operations::run_due_publish_tasks(&mut conn, &state.plugin_runtime, &workspace)
}

#[tauri::command]
pub(crate) fn sync_platform_works(
    input: operations::SyncPlatformWorksInput,
    state: State<'_, AppState>,
) -> Result<operations::SyncRunResult, String> {
    let mut conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let result = operations::sync_platform_works(&mut conn, &state.plugin_runtime, &input);
    log_command_outcome(
        &state,
        "operations.sync",
        &format!("account={} adapter={}", input.account_id, input.adapter_id),
        &result,
    );
    result
}

#[tauri::command]
pub(crate) fn list_external_works(
    filter: Option<operations::ExternalWorkFilter>,
    state: State<'_, AppState>,
) -> Result<Vec<operations::ExternalWork>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_external_works(&conn, &filter.unwrap_or_default())
}

#[tauri::command]
pub(crate) fn get_external_work_detail(
    work_id: String,
    state: State<'_, AppState>,
) -> Result<operations::ExternalWork, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::external_work(&conn, &work_id)
}

#[tauri::command]
pub(crate) fn get_analytics_trend(
    query: operations::AnalyticsQuery,
    state: State<'_, AppState>,
) -> Result<operations::AnalyticsResult, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::analytics_trend(&conn, &query)
}

#[tauri::command]
pub(crate) fn get_operations_dashboard(
    filter: Option<operations::DashboardFilter>,
    state: State<'_, AppState>,
) -> Result<operations::OperationsDashboard, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::operations_dashboard(&conn, &filter.unwrap_or_default())
}

#[tauri::command]
pub(crate) fn list_sync_runs(
    limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<Vec<operations::SyncRunResult>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_sync_runs(&conn, limit.unwrap_or(20).min(200))
}

#[tauri::command]
pub(crate) fn list_publish_task_events(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<operations::PublishTaskEvent>, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    operations::list_publish_task_events(&conn, &task_id)
}

#[tauri::command]
pub(crate) fn get_runtime_diagnostics(
    state: State<'_, AppState>,
) -> Result<diagnostics::RuntimeDiagnostics, String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    let workspace = active_workspace(&conn)?;
    diagnostics::collect(
        &conn,
        &state.data_dir,
        &state.database_path,
        &state.log_path,
        workspace,
        state.started_at.elapsed(),
    )
}

#[tauri::command]
pub(crate) fn run_diagnostic_probe(
    probe: String,
    request_context: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|_| "数据库状态不可用".to_string())?;
    diagnostics::run_probe(&conn, &state.log_path, &probe, request_context.as_deref())
}
