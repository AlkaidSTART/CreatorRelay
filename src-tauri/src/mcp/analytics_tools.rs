use super::outcome::{CoreToolExecution, success_result};
use super::*;
use super::content_tools::field_error;

pub(super) fn parse_analytics_query(arguments: &Value) -> Result<AnalyticsQuery, McpCallError> {
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



pub(super) fn execute_analytics_tool(
    conn: &Connection,
    tool_name: &str,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let result = match tool_name {
        "external_content_list" => {
            let works = operations::list_external_works(
                conn,
                &ExternalWorkFilter {
                    keyword: arguments
                        .get("keyword")
                        .and_then(Value::as_str)
                        .map(str::to_string),
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
                    published_from: arguments
                        .get("publishedFrom")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    published_to: arguments
                        .get("publishedTo")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                },
            )
            .map_err(|error| McpCallError {
                code: "CONTENT_SERVICE_ERROR".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
            success_result(json!({ "items": works, "count": works.len() }))
        }
        "analytics_get_trend" => {
            let query = parse_analytics_query(arguments)?;
            let result =
                operations::analytics_trend(conn, &query).map_err(|error| McpCallError {
                    code: "ANALYTICS_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?;
            success_result(serde_json::to_value(result).map_err(|error| McpCallError {
                code: "ANALYTICS_SERIALIZE_ERROR".to_string(),
                message: error.to_string(),
                field_errors: Vec::new(),
            })?)
        }
        _ => return Err(McpCallError {
            code: "TOOL_NOT_FOUND".to_string(),
            message: format!("未注册数据工具：{tool_name}"),
            field_errors: Vec::new(),
        }),
    };
    Ok(result)
}
