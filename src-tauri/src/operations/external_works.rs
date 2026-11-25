use super::*;
use serde::{Deserialize, Serialize};

use super::publish::mark_account_invalid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricSnapshot {
    pub id: String,
    pub work_id: String,
    pub metric_name: String,
    pub metric_definition: String,
    pub unit: String,
    pub value: f64,
    pub captured_at: String,
    pub snapshot_day: String,
    pub source: String,
    pub canonical_key: Option<String>,
    pub comparable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalWork {
    pub id: String,
    pub source_key: String,
    pub account_id: Option<String>,
    pub platform_id: String,
    pub platform_name: String,
    pub account_alias: String,
    pub external_id: String,
    pub title: String,
    pub content_kind: String,
    pub original_content_type: String,
    pub raw_fields: Value,
    pub url: Option<String>,
    pub published_at: String,
    pub source: String,
    pub synced_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub metrics: Vec<MetricSnapshot>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalWorkFilter {
    #[serde(default)]
    pub keyword: Option<String>,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
    #[serde(default)]
    pub published_from: Option<String>,
    #[serde(default)]
    pub published_to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncPlatformWorksInput {
    pub account_id: String,
    pub platform_id: String,
    pub platform_name: String,
    pub account_alias: String,
    pub adapter_id: String,
    #[serde(default)]
    pub as_of: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRunResult {
    pub id: String,
    pub account_id: String,
    pub platform_id: String,
    pub adapter_id: String,
    pub status: String,
    pub message: String,
    pub imported_count: u32,
    pub updated_count: u32,
    pub skipped_count: u32,
    pub error_count: u32,
    pub account_invalid: bool,
    pub errors: Vec<String>,
    pub started_at: String,
    pub completed_at: String,
}

fn metric_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MetricSnapshot> {
    Ok(MetricSnapshot {
        id: row.get(0)?,
        work_id: row.get(1)?,
        metric_name: row.get(2)?,
        metric_definition: row.get(3)?,
        unit: row.get(4)?,
        value: row.get(5)?,
        captured_at: row.get(6)?,
        snapshot_day: row.get(7)?,
        source: row.get(8)?,
        canonical_key: row.get(9)?,
        comparable: row.get::<_, i64>(10)? != 0,
    })
}

fn metrics_for_work(conn: &Connection, work_id: &str) -> Result<Vec<MetricSnapshot>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, work_id, metric_name, metric_definition, unit, value, captured_at,
                    snapshot_day, source, canonical_key, comparable
             FROM metric_snapshots WHERE work_id=?1
             ORDER BY captured_at ASC, metric_name ASC",
        )
        .map_err(|error| format!("指标快照准备失败：{error}"))?;
    let rows = statement
        .query_map([work_id], metric_from_row)
        .map_err(|error| format!("指标快照读取失败：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("指标快照解析失败：{error}"))
}

type ExternalWorkRow = (
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
    String,
    String,
);

fn external_work_from_values(
    conn: &Connection,
    row: ExternalWorkRow,
) -> Result<ExternalWork, String> {
    let (
        id,
        source_key,
        account_id,
        platform_id,
        platform_name,
        account_alias,
        external_id,
        title,
        content_kind,
        original_content_type,
        raw_fields_json,
        url,
        published_at,
        source,
        synced_at,
        created_at,
        updated_at,
    ) = row;
    Ok(ExternalWork {
        metrics: metrics_for_work(conn, &id)?,
        id,
        source_key,
        account_id,
        platform_id,
        platform_name,
        account_alias,
        external_id,
        title,
        content_kind,
        original_content_type,
        raw_fields: serde_json::from_str(&raw_fields_json)
            .map_err(|error| format!("外部作品原始字段解析失败：{error}"))?,
        url,
        published_at,
        source,
        synced_at,
        created_at,
        updated_at,
    })
}

pub fn list_external_works(
    conn: &Connection,
    filter: &ExternalWorkFilter,
) -> Result<Vec<ExternalWork>, String> {
    let keyword = filter
        .keyword
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let from = filter
        .published_from
        .as_deref()
        .and_then(parse_datetime)
        .map(|value| value.date_naive());
    let to = filter
        .published_to
        .as_deref()
        .and_then(parse_datetime)
        .map(|value| value.date_naive());
    let mut statement = conn
        .prepare(
            "SELECT id, source_key, account_id, platform_id, platform_name, account_alias,
                    external_id, title, content_kind, original_content_type, raw_fields_json,
                    url, published_at, source, synced_at, created_at, updated_at
             FROM external_works ORDER BY published_at DESC, updated_at DESC",
        )
        .map_err(|error| format!("作品列表准备失败：{error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, String>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
                row.get::<_, String>(16)?,
            ))
        })
        .map_err(|error| format!("作品列表读取失败：{error}"))?;
    let mut filtered = Vec::new();
    for row in rows {
        let row = row.map_err(|error| format!("作品列表解析失败：{error}"))?;
        let account_id = &row.2;
        let platform_id = &row.3;
        let external_id = &row.6;
        let title = &row.7;
        let content_kind = &row.8;
        let content_type = &row.9;
        let published_at = &row.12;
        if filter
            .platform_id
            .as_deref()
            .is_some_and(|value| value != platform_id)
        {
            continue;
        }
        if filter
            .account_id
            .as_deref()
            .is_some_and(|value| account_id.as_deref() != Some(value))
        {
            continue;
        }
        if filter
            .content_kind
            .as_deref()
            .is_some_and(|value| value != content_kind)
        {
            continue;
        }
        let published_date = parse_datetime(published_at).map(|value| value.date_naive());
        if from.is_some_and(|from| published_date.is_none_or(|date| date < from)) {
            continue;
        }
        if to.is_some_and(|to| published_date.is_none_or(|date| date > to)) {
            continue;
        }
        if !keyword.is_empty()
            && !format!("{title} {external_id} {platform_id} {content_type}")
                .to_lowercase()
                .contains(&keyword)
        {
            continue;
        }
        filtered.push(row);
    }
    let mut works = Vec::new();
    for row in filtered {
        works.push(external_work_from_values(conn, row)?);
    }
    Ok(works)
}

pub fn external_work(conn: &Connection, work_id: &str) -> Result<ExternalWork, String> {
    let row = conn
        .query_row(
            "SELECT id, source_key, account_id, platform_id, platform_name, account_alias,
                    external_id, title, content_kind, original_content_type, raw_fields_json,
                    url, published_at, source, synced_at, created_at, updated_at
             FROM external_works WHERE id=?1",
            [work_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, String>(12)?,
                    row.get::<_, String>(13)?,
                    row.get::<_, String>(14)?,
                    row.get::<_, String>(15)?,
                    row.get::<_, String>(16)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("作品详情读取失败：{error}"))?
        .ok_or_else(|| "未找到外部作品".to_string())?;
    external_work_from_values(conn, row)
}

pub(super) fn parse_datetime(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn day_bucket(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn upsert_external_work(
    conn: &Connection,
    input: &SyncPlatformWorksInput,
    work: &plugins::AdapterSyncWork,
    synced_at: &str,
) -> Result<(String, bool), String> {
    let source_key = format!(
        "sync:{}:{}:{}",
        input.platform_id, input.account_id, work.external_id
    );
    if let Some(existing) = conn
        .query_row(
            "SELECT id FROM external_works WHERE source_key=?1",
            [&source_key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("同步作品去重查询失败：{error}"))?
    {
        conn.execute(
            "UPDATE external_works
             SET title=?1, content_kind=?2, original_content_type=?3, raw_fields_json=?4,
                 url=?5, published_at=?6, source='sync', synced_at=?7, updated_at=?7
             WHERE id=?8",
            params![
                work.title,
                work.content_kind
                    .map(|kind| kind.as_str())
                    .unwrap_or("unknown"),
                work.content_type,
                serde_json::to_string(&work.raw_fields)
                    .map_err(|error| format!("平台原始字段编码失败：{error}"))?,
                work.url,
                work.published_at,
                synced_at,
                existing
            ],
        )
        .map_err(|error| format!("同步作品更新失败：{error}"))?;
        return Ok((existing, false));
    }
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO external_works(
            id, source_key, account_id, platform_id, platform_name, account_alias,
            external_id, title, content_kind, original_content_type, raw_fields_json,
            url, published_at, source, synced_at, created_at, updated_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 'sync', ?14, ?14, ?14)",
        params![
            id,
            source_key,
            input.account_id,
            input.platform_id,
            input.platform_name,
            input.account_alias,
            work.external_id,
            work.title,
            work.content_kind
                .map(|kind| kind.as_str())
                .unwrap_or("unknown"),
            work.content_type,
            serde_json::to_string(&work.raw_fields)
                .map_err(|error| format!("平台原始字段编码失败：{error}"))?,
            work.url,
            work.published_at,
            synced_at
        ],
    )
    .map_err(|error| format!("同步作品保存失败：{error}"))?;
    Ok((id, true))
}

fn record_metric_snapshots(
    conn: &Connection,
    work_id: &str,
    metrics: &[plugins::AdapterSyncMetric],
    captured_at: DateTime<Utc>,
    source: &str,
) -> Result<u32, String> {
    let mut recorded = 0;
    let day = day_bucket(captured_at);
    for metric in metrics {
        conn.execute(
            "INSERT INTO metric_snapshots(
                id, work_id, metric_name, metric_definition, unit, value, captured_at,
                snapshot_day, source, canonical_key, comparable
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL, 0)
             ON CONFLICT(work_id, metric_name, snapshot_day) DO UPDATE SET
                metric_definition=excluded.metric_definition,
                unit=excluded.unit,
                value=excluded.value,
                captured_at=excluded.captured_at,
                source=excluded.source",
            params![
                Uuid::new_v4().to_string(),
                work_id,
                metric.name,
                metric.definition,
                metric.unit,
                metric.value,
                captured_at.to_rfc3339(),
                day,
                source
            ],
        )
        .map_err(|error| format!("指标快照保存失败：{error}"))?;
        recorded += 1;
    }
    Ok(recorded)
}

pub fn sync_platform_works(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    input: &SyncPlatformWorksInput,
) -> Result<SyncRunResult, String> {
    let started_at = Utc::now();
    let as_of = input
        .as_of
        .as_deref()
        .and_then(parse_datetime)
        .unwrap_or(started_at);
    let adapter_result = plugins::execute_plugin_sync(
        conn,
        runtime,
        &input.adapter_id,
        &input.account_id,
        Some(&as_of.to_rfc3339()),
    );
    let run_id = Uuid::new_v4().to_string();
    match adapter_result {
        Ok(output) => {
            let tx = conn
                .transaction()
                .map_err(|error| format!("无法开始同步事务：{error}"))?;
            let mut imported_count = 0;
            let mut updated_count = 0;
            let mut skipped_count = 0;
            let mut errors = output.errors;
            for work in &output.works {
                let (work_id, inserted) =
                    upsert_external_work(&tx, input, work, &started_at.to_rfc3339())?;
                if inserted {
                    imported_count += 1;
                } else {
                    updated_count += 1;
                }
                let captured_at = work
                    .captured_at
                    .as_deref()
                    .and_then(parse_datetime)
                    .unwrap_or(as_of);
                let recorded = record_metric_snapshots(
                    &tx,
                    &work_id,
                    &work.metrics,
                    captured_at,
                    &format!("adapter:{}", input.adapter_id),
                )?;
                if recorded == 0 {
                    skipped_count += 1;
                }
            }
            let error_count = errors.len() as u32;
            let status = if error_count > 0 {
                "PARTIAL"
            } else {
                "SUCCESS"
            };
            let message = if error_count > 0 {
                format!(
                    "同步完成但有 {error_count} 项失败；已导入 {imported_count} 个、更新 {updated_count} 个作品"
                )
            } else {
                format!(
                    "同步完成：导入 {imported_count} 个，更新 {updated_count} 个作品；指标按 UTC 日采样"
                )
            };
            let completed_at = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT INTO sync_runs(
                    id, account_id, platform_id, adapter_id, status, message,
                    imported_count, updated_count, skipped_count, error_count,
                    started_at, completed_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    run_id,
                    input.account_id,
                    input.platform_id,
                    input.adapter_id,
                    status,
                    message,
                    imported_count,
                    updated_count,
                    skipped_count,
                    error_count,
                    started_at.to_rfc3339(),
                    completed_at
                ],
            )
            .map_err(|error| format!("同步运行记录保存失败：{error}"))?;
            tx.commit()
                .map_err(|error| format!("同步事务提交失败：{error}"))?;
            Ok(SyncRunResult {
                id: run_id,
                account_id: input.account_id.clone(),
                platform_id: input.platform_id.clone(),
                adapter_id: input.adapter_id.clone(),
                status: status.to_string(),
                message,
                imported_count,
                updated_count,
                skipped_count,
                error_count,
                account_invalid: false,
                errors: std::mem::take(&mut errors),
                started_at: started_at.to_rfc3339(),
                completed_at: Utc::now().to_rfc3339(),
            })
        }
        Err(error) => {
            let account_invalid =
                error.code == "AUTH_EXPIRED" || error.message.contains("AUTH_EXPIRED");
            if account_invalid {
                mark_account_invalid(conn, &input.account_id)?;
            }
            let completed_at = Utc::now().to_rfc3339();
            let message = if account_invalid {
                "账号已失效，依赖该账号的发布任务已暂停；请重新授权后显式恢复".to_string()
            } else {
                format!("同步失败：{}", error.message)
            };
            conn.execute(
                "INSERT INTO sync_runs(
                    id, account_id, platform_id, adapter_id, status, message,
                    imported_count, updated_count, skipped_count, error_count,
                    started_at, completed_at
                 ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0, 1, ?7, ?8)",
                params![
                    run_id,
                    input.account_id,
                    input.platform_id,
                    input.adapter_id,
                    if account_invalid {
                        "ACCOUNT_INVALID"
                    } else {
                        "FAILED"
                    },
                    message,
                    started_at.to_rfc3339(),
                    completed_at
                ],
            )
            .map_err(|db_error| format!("同步失败记录保存失败：{db_error}"))?;
            Ok(SyncRunResult {
                id: run_id,
                account_id: input.account_id.clone(),
                platform_id: input.platform_id.clone(),
                adapter_id: input.adapter_id.clone(),
                status: if account_invalid {
                    "ACCOUNT_INVALID".to_string()
                } else {
                    "FAILED".to_string()
                },
                message,
                imported_count: 0,
                updated_count: 0,
                skipped_count: 0,
                error_count: 1,
                account_invalid,
                errors: vec![error.message],
                started_at: started_at.to_rfc3339(),
                completed_at,
            })
        }
    }
}

pub fn list_sync_runs(conn: &Connection, limit: usize) -> Result<Vec<SyncRunResult>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, account_id, platform_id, adapter_id, status, message,
                    imported_count, updated_count, skipped_count, error_count,
                    started_at, completed_at
             FROM sync_runs ORDER BY completed_at DESC LIMIT ?1",
        )
        .map_err(|error| format!("同步记录准备失败：{error}"))?;
    let rows = statement
        .query_map([limit as i64], |row| {
            Ok(SyncRunResult {
                id: row.get(0)?,
                account_id: row.get(1)?,
                platform_id: row.get(2)?,
                adapter_id: row.get(3)?,
                status: row.get(4)?,
                message: row.get(5)?,
                imported_count: u32::try_from(row.get::<_, i64>(6)?).unwrap_or(0),
                updated_count: u32::try_from(row.get::<_, i64>(7)?).unwrap_or(0),
                skipped_count: u32::try_from(row.get::<_, i64>(8)?).unwrap_or(0),
                error_count: u32::try_from(row.get::<_, i64>(9)?).unwrap_or(0),
                account_invalid: row.get::<_, String>(4)? == "ACCOUNT_INVALID",
                errors: Vec::new(),
                started_at: row.get(10)?,
                completed_at: row.get(11)?,
            })
        })
        .map_err(|error| format!("同步记录读取失败：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("同步记录解析失败：{error}"))
}
