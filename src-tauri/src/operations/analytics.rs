use super::external_works::parse_datetime;
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnalyticsGranularity {
    Day,
    Week,
    Month,
}

impl AnalyticsGranularity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyticsQuery {
    pub range: String,
    pub granularity: AnalyticsGranularity,
    #[serde(default)]
    pub platform_id: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub content_kind: Option<String>,
    #[serde(default)]
    pub work_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsPoint {
    pub bucket: String,
    pub captured_at: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsSeries {
    pub key: String,
    pub work_id: String,
    pub title: String,
    pub platform_id: String,
    pub platform_name: String,
    pub account_alias: String,
    pub content_kind: String,
    pub metric_name: String,
    pub metric_definition: String,
    pub unit: String,
    pub points: Vec<AnalyticsPoint>,
    pub comparable: bool,
    pub comparability_note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalyticsResult {
    pub range: String,
    pub from: String,
    pub to: String,
    pub granularity: String,
    pub series: Vec<AnalyticsSeries>,
    pub warnings: Vec<String>,
}

pub(super) fn range_days(range: &str) -> Result<i64, String> {
    match range {
        "7d" => Ok(7),
        "30d" => Ok(30),
        "90d" => Ok(90),
        _ => Err("趋势范围必须是 7d、30d 或 90d".to_string()),
    }
}

fn analytics_bucket(value: DateTime<Utc>, granularity: AnalyticsGranularity) -> String {
    match granularity {
        AnalyticsGranularity::Day => value.format("%Y-%m-%d").to_string(),
        AnalyticsGranularity::Week => {
            let iso = value.iso_week();
            format!("{}-W{:02}", iso.year(), iso.week())
        }
        AnalyticsGranularity::Month => value.format("%Y-%m").to_string(),
    }
}

pub fn analytics_trend(
    conn: &Connection,
    query: &AnalyticsQuery,
) -> Result<AnalyticsResult, String> {
    let days = range_days(&query.range)?;
    let to = Utc::now();
    let from = to - Duration::days(days - 1);
    let work_filter = ExternalWorkFilter {
        platform_id: query.platform_id.clone(),
        account_id: query.account_id.clone(),
        content_kind: query.content_kind.clone(),
        ..ExternalWorkFilter::default()
    };
    let works = list_external_works(conn, &work_filter)?;
    let mut selected = Vec::new();
    for work in works {
        if query
            .work_id
            .as_deref()
            .is_some_and(|work_id| work.id != work_id)
        {
            continue;
        }
        selected.push(work);
    }
    let mut grouped: HashMap<String, BTreeMap<String, MetricSnapshot>> = HashMap::new();
    let mut metadata: HashMap<String, (ExternalWork, String)> = HashMap::new();
    for work in &selected {
        for metric in &work.metrics {
            let captured = match parse_datetime(&metric.captured_at) {
                Some(value) if value >= from && value <= to => value,
                _ => continue,
            };
            let key = format!("{}::{}", work.id, metric.metric_name);
            let bucket = analytics_bucket(captured, query.granularity);
            let bucket_map = grouped.entry(key.clone()).or_default();
            let replace = bucket_map
                .get(&bucket)
                .and_then(|existing| parse_datetime(&existing.captured_at))
                .is_none_or(|existing| captured > existing);
            if replace {
                bucket_map.insert(bucket, metric.clone());
            }
            metadata
                .entry(key)
                .or_insert_with(|| (work.clone(), metric.metric_name.clone()));
        }
    }
    let mut series = Vec::new();
    let mut warnings = Vec::new();
    for (key, buckets) in grouped {
        let Some((work, metric_name)) = metadata.get(&key) else {
            continue;
        };
        let metric = buckets.values().next().cloned();
        let Some(metric) = metric else {
            continue;
        };
        let points = buckets
            .into_iter()
            .map(|(bucket, snapshot)| AnalyticsPoint {
                bucket,
                captured_at: snapshot.captured_at,
                value: snapshot.value,
            })
            .collect::<Vec<_>>();
        if points.len() < 2 {
            warnings.push(format!(
                "《{}》的「{}」仅有 {} 个采样点，暂不绘制趋势，也不会补零",
                work.title,
                metric_name,
                points.len()
            ));
        }
        series.push(AnalyticsSeries {
            key,
            work_id: work.id.clone(),
            title: work.title.clone(),
            platform_id: work.platform_id.clone(),
            platform_name: work.platform_name.clone(),
            account_alias: work.account_alias.clone(),
            content_kind: work.content_kind.clone(),
            metric_name: metric_name.clone(),
            metric_definition: metric.metric_definition.clone(),
            unit: metric.unit.clone(),
            points,
            comparable: metric.comparable,
            comparability_note: if metric.comparable {
                "指标有明确统一字典定义，可在相同口径范围内比较".to_string()
            } else {
                "仅展示同一作品、同一平台的原始指标，不与其他平台或形态聚合".to_string()
            },
        });
    }
    series.sort_by(|left, right| {
        left.platform_name
            .cmp(&right.platform_name)
            .then(left.title.cmp(&right.title))
            .then(left.metric_name.cmp(&right.metric_name))
    });
    if series.is_empty() {
        warnings.push("所选范围内没有可追溯的指标采样；不会用 0 代替缺失数据".to_string());
    }
    Ok(AnalyticsResult {
        range: query.range.clone(),
        from: from.to_rfc3339(),
        to: to.to_rfc3339(),
        granularity: query.granularity.as_str().to_string(),
        series,
        warnings,
    })
}
