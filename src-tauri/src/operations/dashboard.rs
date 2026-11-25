use super::*;
use serde::{Deserialize, Serialize};

use super::analytics::range_days;
use super::external_works::parse_datetime;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DashboardFilter {
    #[serde(default)]
    pub range: Option<String>,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationsDashboard {
    pub platform_count: usize,
    pub account_count: usize,
    pub recent_publish_count: usize,
    pub pending_task_count: usize,
    pub sync_error_count: usize,
    pub recent_works: Vec<ExternalWork>,
    pub pending_tasks: Vec<PublishTask>,
    pub recent_publishes: Vec<PublishTask>,
    pub sync_runs: Vec<SyncRunResult>,
    pub filter: DashboardFilterApplied,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardFilterApplied {
    pub range: String,
    pub platform_id: Option<String>,
    pub account_id: Option<String>,
    pub content_kind: Option<String>,
    pub from: String,
    pub to: String,
}

pub fn operations_dashboard(
    conn: &Connection,
    filter: &DashboardFilter,
) -> Result<OperationsDashboard, String> {
    let range = filter.range.as_deref().unwrap_or("7d");
    let days = range_days(range)?;
    let to = Utc::now();
    let from = to - Duration::days(days - 1);
    let accounts = platforms::list_accounts(conn)?;
    let visible_accounts = accounts
        .into_iter()
        .filter(|account| {
            filter
                .platform_id
                .as_deref()
                .is_none_or(|value| account.platform_id == value)
                && filter
                    .account_id
                    .as_deref()
                    .is_none_or(|value| account.id == value)
        })
        .collect::<Vec<_>>();
    let platform_count = visible_accounts
        .iter()
        .map(|account| account.platform_id.as_str())
        .collect::<HashSet<_>>()
        .len();
    let account_count = visible_accounts.len();
    let mut tasks = list_publish_tasks(conn)?;
    tasks.retain(|task| {
        let created = parse_datetime(&task.created_at).unwrap_or(to);
        created >= from
            && filter
                .platform_id
                .as_deref()
                .is_none_or(|value| task.platform_id == value)
            && filter
                .account_id
                .as_deref()
                .is_none_or(|value| task.account_id == value)
            && filter
                .content_kind
                .as_deref()
                .is_none_or(|value| task.content_kind.as_str() == value)
    });
    let recent_publishes = tasks
        .iter()
        .filter(|task| task.status == PublishTaskStatus::Success)
        .take(10)
        .cloned()
        .collect::<Vec<_>>();
    let pending_tasks = tasks
        .iter()
        .filter(|task| {
            matches!(
                task.status,
                PublishTaskStatus::Created
                    | PublishTaskStatus::Validating
                    | PublishTaskStatus::Pending
                    | PublishTaskStatus::Scheduled
                    | PublishTaskStatus::NeedsManualAction
            )
        })
        .take(10)
        .cloned()
        .collect::<Vec<_>>();
    let work_filter = ExternalWorkFilter {
        platform_id: filter.platform_id.clone(),
        account_id: filter.account_id.clone(),
        content_kind: filter.content_kind.clone(),
        published_from: Some(from.to_rfc3339()),
        published_to: Some(to.to_rfc3339()),
        ..ExternalWorkFilter::default()
    };
    let recent_works = list_external_works(conn, &work_filter)?
        .into_iter()
        .take(10)
        .collect::<Vec<_>>();
    let sync_runs = list_sync_runs(conn, 50)?
        .into_iter()
        .filter(|run| {
            parse_datetime(&run.completed_at).is_some_and(|value| value >= from)
                && filter
                    .platform_id
                    .as_deref()
                    .is_none_or(|value| run.platform_id == value)
                && filter
                    .account_id
                    .as_deref()
                    .is_none_or(|value| run.account_id == value)
        })
        .collect::<Vec<_>>();
    let sync_error_count = sync_runs
        .iter()
        .filter(|run| run.status != "SUCCESS")
        .count();
    Ok(OperationsDashboard {
        platform_count,
        account_count,
        recent_publish_count: recent_publishes.len(),
        pending_task_count: pending_tasks.len(),
        sync_error_count,
        recent_works: recent_works.clone(),
        pending_tasks,
        recent_publishes,
        sync_runs,
        filter: DashboardFilterApplied {
            range: range.to_string(),
            platform_id: filter.platform_id.clone(),
            account_id: filter.account_id.clone(),
            content_kind: filter.content_kind.clone(),
            from: from.to_rfc3339(),
            to: to.to_rfc3339(),
        },
    })
}
