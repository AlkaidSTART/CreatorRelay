use super::*;
use crate::mcp::outcome::{CoreToolExecution, success_result};
use crate::mcp::{McpCallError, McpFieldError};
use crate::operations::{AnalyticsGranularity, AnalyticsQuery, ExternalWorkFilter};
use crate::{content, operations, platforms};
use rusqlite::Connection;
use std::path::Path;

pub(crate) fn execute_platform_tool(
    conn: &mut Connection,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let Some((profile, tool)) = find_platform_tool(tool_name) else {
        return Err(McpCallError {
            code: "TOOL_NOT_FOUND".to_string(),
            message: format!("未注册平台工具：{tool_name}"),
            field_errors: Vec::new(),
        });
    };
    match tool.kind {
        PlatformToolKind::AccountInfo => execute_account(&profile, conn, arguments),
        PlatformToolKind::WorkList => execute_work_list(&profile, conn, arguments),
        PlatformToolKind::AnalyticsTrend => execute_analytics(&profile, conn, arguments),
        PlatformToolKind::PublishValidate => {
            execute_publish_validate(&profile, workspace, arguments)
        }
    }
}

fn profile_summary(profile: &PlatformMcpProfile) -> Value {
    json!({
        "serverName": profile.server_name,
        "platformId": profile.platform_id,
        "platformName": profile.platform_name,
        "transport": profile.transport,
        "authMode": profile.auth_mode,
        "admissionStatus": profile.admission_status,
        "autoPublishAllowed": profile.auto_publish_allowed,
        "publishDispatch": profile.publish_dispatch,
        "nativeWorkIdField": profile.native_work_id_field,
    })
}

fn execute_account(
    profile: &PlatformMcpProfile,
    conn: &Connection,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let accounts = platforms::list_accounts(conn).map_err(|error| McpCallError {
        code: "ACCOUNT_SERVICE_ERROR".to_string(),
        message: error,
        field_errors: Vec::new(),
    })?;
    let account_id = arguments.get("accountId").and_then(Value::as_str);
    let accounts = accounts
        .into_iter()
        .filter(|account| account.platform_id == profile.platform_id)
        .filter(|account| account_id.is_none_or(|value| account.id == value))
        .map(|account| {
            json!({
                "id": account.id,
                "platformId": account.platform_id,
                "platformName": account.platform_name,
                "alias": account.alias,
                "nickname": account.nickname,
                "uid": account.uid,
                "status": account.status,
                "enabled": account.enabled,
                "isDefault": account.is_default,
                "lastCheckedAt": account.last_checked_at,
                "lastError": account.last_error,
                "capabilities": account.capabilities,
                "authMode": profile.auth_mode,
            })
        })
        .collect::<Vec<_>>();
    let mut result = profile_summary(profile);
    result["accounts"] = Value::Array(accounts.clone());
    result["count"] = json!(accounts.len());
    result["warnings"] = json!([]);
    if accounts.is_empty() && account_id.is_some() {
        result["warnings"] = json!(["未找到匹配账号；请先在账号页创建并完成对应平台授权。"]);
    }
    Ok(success_result(result))
}

fn execute_work_list(
    profile: &PlatformMcpProfile,
    conn: &Connection,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(50)
        .clamp(1, 100) as usize;
    let native_id = arguments
        .get(&profile.native_work_id_field)
        .and_then(Value::as_str);
    let filter = ExternalWorkFilter {
        keyword: arguments
            .get("keyword")
            .and_then(Value::as_str)
            .map(str::to_string),
        platform_id: Some(profile.platform_id.clone()),
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
    };
    let works = operations::list_external_works(conn, &filter).map_err(|error| McpCallError {
        code: "WORK_LIST_FAILED".to_string(),
        message: error,
        field_errors: Vec::new(),
    })?;
    let native_filter_keys = arguments
        .as_object()
        .map(|object| {
            object
                .keys()
                .filter(|key| {
                    !matches!(
                        key.as_str(),
                        "accountId"
                            | "contentKind"
                            | "keyword"
                            | "publishedFrom"
                            | "publishedTo"
                            | "limit"
                    ) && key.as_str() != profile.native_work_id_field
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let works = works
        .into_iter()
        .filter(|work| native_id.is_none_or(|value| work.external_id == value))
        .filter(|work| {
            native_filter_keys.iter().all(|key| {
                let Some(expected) = arguments.get(key) else {
                    return true;
                };
                let Some(actual) = work.raw_fields.get(key) else {
                    // Local records may not carry every platform field. Missing
                    // native fields are disclosed rather than treated as a match.
                    return true;
                };
                actual == expected
                    || actual.to_string().trim_matches('"')
                        == expected.to_string().trim_matches('"')
            })
        })
        .take(limit)
        .collect::<Vec<_>>();
    let items = works
        .iter()
        .map(|work| {
            json!({
                "workId": work.id,
                "externalId": work.external_id,
                "platformId": work.platform_id,
                "title": work.title,
                "contentKind": work.content_kind,
                "originalContentType": work.original_content_type,
                "url": work.url,
                "publishedAt": work.published_at,
                "source": work.source,
                "rawFields": work.raw_fields,
                "metrics": work.metrics,
            })
        })
        .collect::<Vec<_>>();
    let mut result = profile_summary(profile);
    result["items"] = Value::Array(items.clone());
    result["count"] = json!(items.len());
    result["nativeWorkIdField"] = json!(profile.native_work_id_field);
    result["nativeFilters"] = json!(native_filter_keys);
    result["warnings"] = if native_filter_keys.is_empty() {
        json!([])
    } else {
        json!(["平台原生筛选字段仅在本地 rawFields 已包含该字段时生效；缺失字段不会伪造匹配。"])
    };
    Ok(success_result(result))
}

fn execute_analytics(
    profile: &PlatformMcpProfile,
    conn: &Connection,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let range = arguments
        .get("range")
        .and_then(Value::as_str)
        .unwrap_or("7d");
    let granularity = match arguments
        .get("granularity")
        .and_then(Value::as_str)
        .unwrap_or("day")
    {
        "day" => AnalyticsGranularity::Day,
        "week" => AnalyticsGranularity::Week,
        "month" => AnalyticsGranularity::Month,
        other => {
            return Err(McpCallError {
                code: "INVALID_ARGUMENT".to_string(),
                message: "granularity 无效".to_string(),
                field_errors: vec![McpFieldError {
                    path: "granularity".to_string(),
                    message: format!("未知粒度：{other}"),
                }],
            });
        }
    };
    let native_id = arguments
        .get(&profile.native_work_id_field)
        .and_then(Value::as_str);
    let work_filter = ExternalWorkFilter {
        platform_id: Some(profile.platform_id.clone()),
        account_id: arguments
            .get("accountId")
            .and_then(Value::as_str)
            .map(str::to_string),
        content_kind: arguments
            .get("contentKind")
            .and_then(Value::as_str)
            .map(str::to_string),
        ..ExternalWorkFilter::default()
    };
    let works =
        operations::list_external_works(conn, &work_filter).map_err(|error| McpCallError {
            code: "ANALYTICS_SERVICE_ERROR".to_string(),
            message: error,
            field_errors: Vec::new(),
        })?;
    let work_id = native_id.and_then(|native_id| {
        works
            .iter()
            .find(|work| work.external_id == native_id)
            .map(|work| work.id.clone())
    });
    if native_id.is_some() && work_id.is_none() {
        return Err(McpCallError {
            code: "WORK_NOT_FOUND".to_string(),
            message: format!(
                "未找到 {}={} 的本地作品；请先同步作品数据",
                profile.native_work_id_field,
                native_id.unwrap_or_default()
            ),
            field_errors: vec![McpFieldError {
                path: profile.native_work_id_field.clone(),
                message: "本地没有匹配作品".to_string(),
            }],
        });
    }
    let mut analytics = operations::analytics_trend(
        conn,
        &AnalyticsQuery {
            range: range.to_string(),
            granularity,
            platform_id: Some(profile.platform_id.clone()),
            account_id: arguments
                .get("accountId")
                .and_then(Value::as_str)
                .map(str::to_string),
            content_kind: arguments
                .get("contentKind")
                .and_then(Value::as_str)
                .map(str::to_string),
            work_id,
        },
    )
    .map_err(|error| McpCallError {
        code: "ANALYTICS_SERVICE_ERROR".to_string(),
        message: error,
        field_errors: Vec::new(),
    })?;
    if let Some(metrics) = arguments.get("metrics").and_then(Value::as_array) {
        let names = metrics
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::HashSet<_>>();
        if !names.is_empty() {
            analytics
                .series
                .retain(|series| names.contains(series.metric_name.as_str()));
        }
    }
    let mut result = profile_summary(profile);
    result["analytics"] = serde_json::to_value(analytics).map_err(|error| McpCallError {
        code: "ANALYTICS_SERIALIZE_ERROR".to_string(),
        message: error.to_string(),
        field_errors: Vec::new(),
    })?;
    result["metricMappings"] = json!(
        profile
            .field_mappings
            .iter()
            .filter(|mapping| mapping.direction == "read")
            .map(|mapping| json!({
                "unifiedField": mapping.unified_field,
                "platformField": mapping.platform_field,
                "notes": mapping.notes,
            }))
            .collect::<Vec<_>>()
    );
    Ok(success_result(result))
}

fn execute_publish_validate(
    profile: &PlatformMcpProfile,
    workspace: &Path,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let content_id = arguments
        .get("contentId")
        .and_then(Value::as_str)
        .ok_or_else(|| McpCallError {
            code: "INVALID_ARGUMENT".to_string(),
            message: "缺少 contentId".to_string(),
            field_errors: vec![McpFieldError {
                path: "contentId".to_string(),
                message: "必须提供本地内容 ID".to_string(),
            }],
        })?;
    let package =
        content::read_content_package(workspace, content_id).map_err(|error| McpCallError {
            code: "CONTENT_NOT_FOUND".to_string(),
            message: error,
            field_errors: vec![McpFieldError {
                path: "contentId".to_string(),
                message: "本地内容无法读取".to_string(),
            }],
        })?;
    let content_kind = package.manifest.kind.as_str();
    let Some(rule) = profile
        .content_rules
        .iter()
        .find(|rule| rule.kind == content_kind)
    else {
        let mut result = profile_summary(profile);
        result["contentId"] = json!(content_id);
        result["contentKind"] = json!(content_kind);
        result["valid"] = json!(false);
        result["issues"] = json!([{
            "severity": "error",
            "field": "contentKind",
            "message": format!("{} 未声明 {} 形态的发布字段；请改用该平台支持的内容形态或人工发布。", profile.platform_name, content_kind),
        }]);
        result["fieldMappings"] = json!(profile.field_mappings);
        result["contentRules"] = json!(profile.content_rules);
        result["handoff"] = json!(profile.handoff);
        return Ok(success_result(result));
    };
    let mut issues = Vec::new();
    let video_count = package
        .manifest
        .media
        .iter()
        .filter(|asset| asset.kind == content::AssetKind::Video)
        .count();
    let image_count = package
        .manifest
        .media
        .iter()
        .filter(|asset| asset.kind == content::AssetKind::Image)
        .count();
    let media_count = package.manifest.media.len();
    if media_count < rule.min_media {
        issues.push(json!({
            "severity": "error",
            "field": "media",
            "message": format!("{} 需要至少 {} 个媒体素材，当前 {} 个", profile.platform_name, rule.min_media, media_count),
        }));
    }
    if let Some(max_media) = rule.max_media
        && media_count > max_media
    {
        issues.push(json!({
            "severity": if rule.constraints_verified { "error" } else { "warning" },
            "field": "media",
            "message": format!("{} 媒体数量 {} 超过{}上限 {}", profile.platform_name, media_count, if rule.constraints_verified { "平台" } else { "参考" }, max_media),
        }));
    }
    if rule.allowed_media.iter().all(|kind| kind == "video") && image_count > 0 {
        issues.push(json!({
            "severity": "error",
            "field": "media",
            "message": "该平台视频作品不能混入图片素材",
        }));
    }
    if rule.allowed_media.iter().all(|kind| kind == "image") && video_count > 0 {
        issues.push(json!({
            "severity": "error",
            "field": "media",
            "message": "该平台图文作品不能混入视频素材",
        }));
    }
    if rule.cover_required && package.manifest.cover.is_none() {
        issues.push(json!({
            "severity": if rule.constraints_verified { "error" } else { "warning" },
            "field": "cover",
            "message": format!("{} 未提供封面{}", profile.platform_name, if rule.constraints_verified { "" } else { "（平台封面要求尚未账号级验证）" }),
        }));
    }
    if matches!(content_kind, "article" | "imageArticle") && package.body.trim().is_empty() {
        issues.push(json!({
            "severity": "error",
            "field": "body",
            "message": "文章正文不能为空",
        }));
    }
    if let Some(max_title) = rule.max_title_chars {
        let title_len = package.manifest.title.chars().count();
        if title_len > max_title {
            issues.push(json!({
                "severity": if rule.constraints_verified { "error" } else { "warning" },
                "field": "title",
                "message": format!("标题 {} 字符，超过{}建议上限 {}", title_len, if rule.constraints_verified { "平台" } else { "参考" }, max_title),
            }));
        }
    }
    if let Some(max_body) = rule.max_body_chars {
        let body_len = package.body.chars().count();
        if body_len > max_body {
            issues.push(json!({
                "severity": if rule.constraints_verified { "error" } else { "warning" },
                "field": "body",
                "message": format!("正文 {} 字符，超过{}建议上限 {}", body_len, if rule.constraints_verified { "平台" } else { "参考" }, max_body),
            }));
        }
    }
    if let Some(max_tags) = rule.max_tags
        && package.manifest.tags.len() > max_tags
    {
        issues.push(json!({
            "severity": if rule.constraints_verified { "error" } else { "warning" },
            "field": "tags",
            "message": format!("标签 {} 个，超过建议上限 {}", package.manifest.tags.len(), max_tags),
        }));
    }
    if !rule.constraints_verified {
        issues.push(json!({
            "severity": "warning",
            "field": "constraints",
            "message": "该平台长度、媒体规格与审核规则尚未完成账号级验证；仅执行内容形态和本地结构校验。",
        }));
    }
    let valid = !issues
        .iter()
        .any(|issue| issue.get("severity").and_then(Value::as_str) == Some("error"));
    let mut platform_fields = json!({
        "title": package.manifest.title,
        "summary": package.manifest.summary,
        "body": package.body,
        "tags": package.manifest.tags,
        "media": package.manifest.media,
        "cover": package.manifest.cover,
    });
    platform_fields[&profile.native_work_id_field] = Value::Null;
    let mut result = profile_summary(profile);
    result["contentId"] = json!(content_id);
    result["contentKind"] = json!(content_kind);
    result["valid"] = json!(valid);
    result["issues"] = Value::Array(issues);
    result["fieldMappings"] = json!(
        profile
            .field_mappings
            .iter()
            .filter(|mapping| mapping.direction == "write")
            .collect::<Vec<_>>()
    );
    result["contentRule"] = serde_json::to_value(rule).map_err(|error| McpCallError {
        code: "PLATFORM_PROFILE_SERIALIZE_ERROR".to_string(),
        message: error.to_string(),
        field_errors: Vec::new(),
    })?;
    result["platformFields"] = platform_fields;
    result["handoff"] = json!(profile.handoff);
    result["interfaces"] = json!(
        profile
            .interfaces
            .iter()
            .filter(|interface| interface.direction != "read")
            .collect::<Vec<_>>()
    );
    Ok(success_result(result))
}
