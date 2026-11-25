use super::*;

fn parse_analytics_query(arguments: &Value) -> Result<AnalyticsQuery, McpCallError> {
    let granularity = match arguments
        .get("granularity")
        .and_then(Value::as_str)
        .unwrap_or("day")
    {
        "day" => AnalyticsGranularity::Day,
        "week" => AnalyticsGranularity::Week,
        "month" => AnalyticsGranularity::Month,
        _ => {
            return Err(McpCallError {
                code: "INVALID_ARGUMENT".to_string(),
                message: "granularity 无效".to_string(),
                field_errors: vec![field_error("granularity", "必须是 day、week 或 month")],
            });
        }
    };
    Ok(AnalyticsQuery {
        range: arguments
            .get("range")
            .and_then(Value::as_str)
            .unwrap_or("7d")
            .to_string(),
        granularity,
        platform_id: arguments
            .get("platformId")
            .and_then(Value::as_str)
            .map(str::to_string),
        account_id: arguments
            .get("accountId")
            .and_then(Value::as_str)
            .map(str::to_string),
        content_kind: arguments
            .get("contentKind")
            .and_then(Value::as_str)
            .map(str::to_string),
        work_id: arguments
            .get("workId")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

