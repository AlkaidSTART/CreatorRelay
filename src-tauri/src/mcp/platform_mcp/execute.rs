use super::*;
use crate::browser_bridge::BrowserBridge;
use crate::mcp::outcome::{CoreToolExecution, success_result};
use crate::mcp::{McpCallError, McpFieldError};
use crate::operations::{
    AnalyticsGranularity, AnalyticsQuery, ExternalWorkFilter, SyncPlatformWorksInput,
};
use crate::plugins::PluginRuntime;
use crate::{content, operations, platforms};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) fn execute_platform_tool(
    conn: &mut Connection,
    runtime: &PluginRuntime,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
    browser_bridge: Option<&BrowserBridge>,
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
        PlatformToolKind::WorkGet => execute_work_get(&profile, conn, arguments),
        PlatformToolKind::WorkChanges => execute_work_changes(&profile, conn, arguments),
        PlatformToolKind::WorkSync => execute_work_sync(&profile, conn, runtime, arguments),
        PlatformToolKind::AnalyticsTrend => execute_analytics(&profile, conn, arguments),
        PlatformToolKind::PublishValidate => {
            execute_publish_validate(&profile, workspace, arguments)
        }
        PlatformToolKind::MediaStage => {
            execute_media_stage(&profile, conn, workspace, arguments, browser_bridge)
        }
        PlatformToolKind::PublishSubmit => {
            execute_publish_submit(&profile, arguments, browser_bridge)
        }
    }
}

pub(crate) fn preview_platform_tool(
    conn: &Connection,
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, Vec<McpFieldError>> {
    let Some((profile, tool)) = find_platform_tool(tool_name) else {
        return Err(vec![McpFieldError {
            path: "toolName".to_string(),
            message: "平台工具不存在".to_string(),
        }]);
    };
    let mut preview = json!({
        "platformId": profile.platform_id,
        "platformName": profile.platform_name,
        "toolKind": tool.kind,
        "arguments": arguments,
    });
    match tool.kind {
        PlatformToolKind::WorkSync => {
            let account_id = arguments
                .get("accountId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let account = platforms::list_accounts(conn)
                .map_err(|error| {
                    vec![McpFieldError {
                        path: "accountId".to_string(),
                        message: error,
                    }]
                })?
                .into_iter()
                .find(|account| account.id == account_id)
                .ok_or_else(|| {
                    vec![McpFieldError {
                        path: "accountId".to_string(),
                        message: "账号不存在".to_string(),
                    }]
                })?;
            preview["account"] = json!({
                "id": account.id,
                "alias": account.alias,
                "status": account.status,
            });
            preview["adapterId"] = arguments.get("adapterId").cloned().unwrap_or(Value::Null);
            preview["networkRequest"] = json!(true);
        }
        PlatformToolKind::MediaStage => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let package =
                content::read_content_package(workspace, content_id).map_err(|error| {
                    vec![McpFieldError {
                        path: "contentId".to_string(),
                        message: error,
                    }]
                })?;
            preview["content"] = json!({
                "id": package.manifest.id,
                "title": package.manifest.title,
                "kind": package.manifest.kind,
                "version": package.content_version,
                "mediaCount": package.manifest.media.len(),
                "hasCover": package.manifest.cover.is_some(),
            });
            preview["publishUrl"] = json!(profile.handoff.publish_url);
            preview["operation"] = json!(
                "open isolated browser, attach local files, fill title/body; do not click save or publish"
            );
        }
        PlatformToolKind::PublishSubmit => {
            if arguments.get("mode").and_then(Value::as_str) == Some("publish")
                && arguments
                    .get("confirmFinalPublish")
                    .and_then(Value::as_bool)
                    != Some(true)
            {
                return Err(vec![McpFieldError {
                    path: "confirmFinalPublish".to_string(),
                    message: "publish 模式必须设置为 true".to_string(),
                }]);
            }
            preview["mode"] = arguments.get("mode").cloned().unwrap_or(Value::Null);
            preview["buttonLabel"] = arguments.get("buttonLabel").cloned().unwrap_or(Value::Null);
            preview["risk"] = if arguments.get("mode").and_then(Value::as_str) == Some("publish") {
                json!("最终公开发布内容；点击后不能由 CreatorRelay 自动撤销")
            } else {
                json!("保存草稿；可能与平台现有草稿发生覆盖或重复")
            };
        }
        _ => {}
    }
    Ok(preview)
}

pub(crate) fn platform_tool_effects(tool_name: &str) -> Vec<String> {
    let Some((_, tool)) = find_platform_tool(tool_name) else {
        return vec!["执行平台工具".to_string()];
    };
    match tool.kind {
        PlatformToolKind::WorkSync => vec![
            "通过指定 Adapter 向外部平台发起作品与指标同步请求".to_string(),
            "导入/更新本地 external_works 与 metric_snapshots；失败不会自动重复".to_string(),
        ],
        PlatformToolKind::MediaStage => vec![
            "在 CreatorRelay 隔离 Chrome 中打开平台官方发布页".to_string(),
            "向平台上传本地内容包媒体并填写标题、正文、标签".to_string(),
            "不会点击保存草稿或最终发布；仍需人工核对后单独确认提交".to_string(),
        ],
        PlatformToolKind::PublishSubmit => vec![
            "在隔离 Chrome 页面点击一个精确匹配且启用的按钮".to_string(),
            "publish 模式会最终公开发布；creatorrelay 不会自动重试或撤销".to_string(),
            "执行前必须核对账号、标题、正文、媒体顺序、封面和可见范围".to_string(),
        ],
        _ => vec!["读取或处理平台结构化数据".to_string()],
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
        "fieldModel": platform_field_model(profile),
    })
}

fn field_value_type(unified_field: &str) -> &'static str {
    if unified_field.starts_with("metrics.") {
        "number"
    } else if matches!(unified_field, "media" | "cover") || unified_field.starts_with("media.") {
        "object"
    } else if unified_field == "tags" {
        "array<string>"
    } else if matches!(
        unified_field,
        "publishedAt" | "syncedAt" | "updatedAt" | "createdAt"
    ) {
        "date-time"
    } else if unified_field == "contentKind" {
        "enum<video|imageText|imageArticle|article|unknown>"
    } else if unified_field == "rawFields" {
        "object"
    } else {
        "string"
    }
}

fn platform_field_model(profile: &PlatformMcpProfile) -> Vec<PlatformFieldDefinition> {
    let mut fields = profile
        .field_mappings
        .iter()
        .map(|mapping| PlatformFieldDefinition {
            unified_field: mapping.unified_field.clone(),
            platform_field: mapping.platform_field.clone(),
            direction: mapping.direction.clone(),
            value_type: field_value_type(&mapping.unified_field).to_string(),
            required: mapping.required,
            notes: mapping.notes.clone(),
        })
        .collect::<Vec<_>>();
    for (unified_field, platform_field, value_type, notes) in [
        (
            "workId",
            "creatorrelay.work_id",
            "string",
            "CreatorRelay 本地稳定作品 ID。",
        ),
        (
            "sourceKey",
            "creatorrelay.source_key",
            "string",
            "同步来源去重键。",
        ),
        (
            "accountId",
            "creatorrelay.account_id",
            "string",
            "CreatorRelay 本地账号 ID。",
        ),
        (
            "source",
            "creatorrelay.source",
            "string",
            "作品来源：publish、sync 或 manual_check。",
        ),
        (
            "syncedAt",
            "creatorrelay.synced_at",
            "date-time",
            "最后一次从平台同步的时间。",
        ),
        (
            "rawFields",
            "rawFields",
            "object",
            "平台原生字段，未知字段原样保留。",
        ),
    ] {
        fields.push(PlatformFieldDefinition {
            unified_field: unified_field.to_string(),
            platform_field: platform_field.to_string(),
            direction: "read".to_string(),
            value_type: value_type.to_string(),
            required: true,
            notes: notes.to_string(),
        });
    }
    fields
}

fn structured_work(work: &operations::ExternalWork) -> Value {
    json!({
        "workId": work.id,
        "sourceKey": work.source_key,
        "accountId": work.account_id,
        "platformId": work.platform_id,
        "platformName": work.platform_name,
        "accountAlias": work.account_alias,
        "externalId": work.external_id,
        "title": work.title,
        "contentKind": work.content_kind,
        "originalContentType": work.original_content_type,
        "rawFields": work.raw_fields,
        "url": work.url,
        "publishedAt": work.published_at,
        "source": work.source,
        "syncedAt": work.synced_at,
        "createdAt": work.created_at,
        "updatedAt": work.updated_at,
        "metrics": work.metrics,
    })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkPageCursor {
    version: u8,
    platform_id: String,
    offset: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkChangeCursor {
    version: u8,
    platform_id: String,
    updated_at: String,
    work_id: String,
}

fn encode_cursor<T: Serialize>(cursor: &T) -> Result<String, McpCallError> {
    let bytes = serde_json::to_vec(cursor).map_err(|error| McpCallError {
        code: "CURSOR_SERIALIZE_ERROR".to_string(),
        message: error.to_string(),
        field_errors: Vec::new(),
    })?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn decode_cursor<T: for<'de> Deserialize<'de>>(raw: &str, field: &str) -> Result<T, McpCallError> {
    let bytes = URL_SAFE_NO_PAD.decode(raw).map_err(|_| McpCallError {
        code: "INVALID_CURSOR".to_string(),
        message: "分页游标无效".to_string(),
        field_errors: vec![McpFieldError {
            path: field.to_string(),
            message: "游标无法解码，请从第一页重新调用".to_string(),
        }],
    })?;
    serde_json::from_slice(&bytes).map_err(|_| McpCallError {
        code: "INVALID_CURSOR".to_string(),
        message: "分页游标结构无效".to_string(),
        field_errors: vec![McpFieldError {
            path: field.to_string(),
            message: "游标版本或内容不匹配，请从第一页重新调用".to_string(),
        }],
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
    let page_size = arguments
        .get("pageSize")
        .or_else(|| arguments.get("limit"))
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 100) as usize;
    let offset = arguments
        .get("cursor")
        .and_then(Value::as_str)
        .map(|raw| decode_cursor::<WorkPageCursor>(raw, "cursor"))
        .transpose()?
        .map(|cursor| {
            if cursor.version != 1 || cursor.platform_id != profile.platform_id {
                Err(McpCallError {
                    code: "INVALID_CURSOR".to_string(),
                    message: "分页游标与当前平台不匹配".to_string(),
                    field_errors: vec![McpFieldError {
                        path: "cursor".to_string(),
                        message: "请使用该平台工具返回的 nextCursor".to_string(),
                    }],
                })
            } else {
                Ok(cursor.offset)
            }
        })
        .transpose()?
        .unwrap_or(0);
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
                            | "cursor"
                            | "pageSize"
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
        .collect::<Vec<_>>();
    let total_count = works.len();
    let page = works
        .iter()
        .skip(offset)
        .take(page_size)
        .collect::<Vec<_>>();
    let next_offset = offset.saturating_add(page.len());
    let has_next_page = next_offset < total_count;
    let next_cursor = has_next_page
        .then(|| {
            encode_cursor(&WorkPageCursor {
                version: 1,
                platform_id: profile.platform_id.clone(),
                offset: next_offset,
            })
        })
        .transpose()?;
    let items = page.into_iter().map(structured_work).collect::<Vec<_>>();
    let mut result = profile_summary(profile);
    result["items"] = Value::Array(items.clone());
    result["count"] = json!(items.len());
    result["pageInfo"] = json!({
        "offset": offset,
        "pageSize": page_size,
        "totalCount": total_count,
        "hasNextPage": has_next_page,
        "nextCursor": next_cursor,
        "snapshotAt": chrono::Utc::now().to_rfc3339(),
    });
    result["nativeWorkIdField"] = json!(profile.native_work_id_field);
    result["nativeFilters"] = json!(native_filter_keys);
    result["warnings"] = if native_filter_keys.is_empty() {
        json!([])
    } else {
        json!(["平台原生筛选字段仅在本地 rawFields 已包含该字段时生效；缺失字段不会伪造匹配。"])
    };
    Ok(success_result(result))
}

fn execute_work_get(
    profile: &PlatformMcpProfile,
    conn: &Connection,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let work_id = arguments.get("workId").and_then(Value::as_str);
    let native_id = arguments
        .get(&profile.native_work_id_field)
        .and_then(Value::as_str);
    if work_id.is_none() && native_id.is_none() {
        return Err(McpCallError {
            code: "INVALID_ARGUMENT".to_string(),
            message: "必须提供 workId 或平台原生作品 ID".to_string(),
            field_errors: vec![McpFieldError {
                path: "workId".to_string(),
                message: format!("workId 或 {} 至少提供一个", profile.native_work_id_field),
            }],
        });
    }
    let work = if let Some(work_id) = work_id {
        operations::external_work(conn, work_id).map_err(|error| McpCallError {
            code: "WORK_NOT_FOUND".to_string(),
            message: error,
            field_errors: vec![McpFieldError {
                path: "workId".to_string(),
                message: "未找到本地作品".to_string(),
            }],
        })?
    } else {
        let native_id = native_id.unwrap_or_default();
        operations::list_external_works(
            conn,
            &ExternalWorkFilter {
                platform_id: Some(profile.platform_id.clone()),
                account_id: arguments
                    .get("accountId")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                ..ExternalWorkFilter::default()
            },
        )
        .map_err(|error| McpCallError {
            code: "WORK_GET_FAILED".to_string(),
            message: error,
            field_errors: Vec::new(),
        })?
        .into_iter()
        .find(|work| work.external_id == native_id)
        .ok_or_else(|| McpCallError {
            code: "WORK_NOT_FOUND".to_string(),
            message: format!(
                "未找到 {}={} 的本地作品；请先同步作品数据",
                profile.native_work_id_field, native_id
            ),
            field_errors: vec![McpFieldError {
                path: profile.native_work_id_field.clone(),
                message: "本地没有匹配作品".to_string(),
            }],
        })?
    };
    if work.platform_id != profile.platform_id {
        return Err(McpCallError {
            code: "WORK_PLATFORM_MISMATCH".to_string(),
            message: "作品不属于当前平台工具".to_string(),
            field_errors: Vec::new(),
        });
    }
    let mut result = profile_summary(profile);
    result["work"] = structured_work(&work);
    Ok(success_result(result))
}

fn execute_work_changes(
    profile: &PlatformMcpProfile,
    conn: &Connection,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let page_size = arguments
        .get("pageSize")
        .and_then(Value::as_u64)
        .unwrap_or(50)
        .clamp(1, 200) as usize;
    let cursor = arguments
        .get("cursor")
        .and_then(Value::as_str)
        .map(|raw| decode_cursor::<WorkChangeCursor>(raw, "cursor"))
        .transpose()?
        .map(|cursor| {
            if cursor.version != 1 || cursor.platform_id != profile.platform_id {
                Err(McpCallError {
                    code: "INVALID_CURSOR".to_string(),
                    message: "增量游标与当前平台不匹配".to_string(),
                    field_errors: vec![McpFieldError {
                        path: "cursor".to_string(),
                        message: "请使用该平台变更工具返回的 nextCursor".to_string(),
                    }],
                })
            } else {
                Ok(cursor)
            }
        })
        .transpose()?;
    let updated_after = arguments.get("updatedAfter").and_then(Value::as_str);
    let mut works = operations::list_external_works(
        conn,
        &ExternalWorkFilter {
            platform_id: Some(profile.platform_id.clone()),
            account_id: arguments
                .get("accountId")
                .and_then(Value::as_str)
                .map(str::to_string),
            ..ExternalWorkFilter::default()
        },
    )
    .map_err(|error| McpCallError {
        code: "WORK_CHANGES_FAILED".to_string(),
        message: error,
        field_errors: Vec::new(),
    })?;
    if let Some(updated_after) = updated_after {
        works.retain(|work| work.updated_at.as_str() > updated_after);
    }
    works.sort_by(|left, right| {
        left.updated_at
            .cmp(&right.updated_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    if let Some(cursor) = &cursor {
        works.retain(|work| {
            work.updated_at.as_str() > cursor.updated_at.as_str()
                || (work.updated_at == cursor.updated_at && work.id > cursor.work_id)
        });
    }
    let has_more = works.len() > page_size;
    let page = works.into_iter().take(page_size).collect::<Vec<_>>();
    let next_cursor = page
        .last()
        .map(|work| {
            encode_cursor(&WorkChangeCursor {
                version: 1,
                platform_id: profile.platform_id.clone(),
                updated_at: work.updated_at.clone(),
                work_id: work.id.clone(),
            })
        })
        .transpose()?;
    let mut result = profile_summary(profile);
    result["changes"] = json!(
        page.iter()
            .map(|work| json!({
                "changeType": "upsert",
                "work": structured_work(work),
            }))
            .collect::<Vec<_>>()
    );
    result["pageInfo"] = json!({
        "pageSize": page_size,
        "hasNextPage": has_more,
        "nextCursor": next_cursor,
        "snapshotAt": chrono::Utc::now().to_rfc3339(),
    });
    result["warnings"] =
        json!(["当前增量源仅包含本地 upsert 记录；平台删除事件尚未同步，不会伪造 tombstone。"]);
    Ok(success_result(result))
}

fn execute_work_sync(
    profile: &PlatformMcpProfile,
    conn: &mut Connection,
    runtime: &PluginRuntime,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let account_id = arguments
        .get("accountId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let adapter_id = arguments
        .get("adapterId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let account = platforms::list_accounts(conn)
        .map_err(|error| McpCallError {
            code: "ACCOUNT_SERVICE_ERROR".to_string(),
            message: error,
            field_errors: Vec::new(),
        })?
        .into_iter()
        .find(|account| account.id == account_id)
        .ok_or_else(|| McpCallError {
            code: "ACCOUNT_NOT_FOUND".to_string(),
            message: "未找到同步目标账号".to_string(),
            field_errors: vec![McpFieldError {
                path: "accountId".to_string(),
                message: "账号不存在".to_string(),
            }],
        })?;
    if account.platform_id != profile.platform_id {
        return Err(McpCallError {
            code: "ACCOUNT_PLATFORM_MISMATCH".to_string(),
            message: "账号与当前平台工具不匹配".to_string(),
            field_errors: vec![McpFieldError {
                path: "accountId".to_string(),
                message: "请选择该平台的账号".to_string(),
            }],
        });
    }
    let sync = operations::sync_platform_works(
        conn,
        runtime,
        &SyncPlatformWorksInput {
            account_id: account.id,
            platform_id: profile.platform_id.clone(),
            platform_name: profile.platform_name.clone(),
            account_alias: account.alias,
            adapter_id: adapter_id.to_string(),
            as_of: arguments
                .get("asOf")
                .and_then(Value::as_str)
                .map(str::to_string),
            cursor: arguments
                .get("cursor")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
    )
    .map_err(|error| McpCallError {
        code: "WORK_SYNC_FAILED".to_string(),
        message: error,
        field_errors: Vec::new(),
    })?;
    let status = if sync.status == "SUCCESS" || sync.status == "PARTIAL" {
        "success"
    } else {
        "failed"
    };
    let mut result = profile_summary(profile);
    result["sync"] = serde_json::to_value(sync).map_err(|error| McpCallError {
        code: "WORK_SYNC_SERIALIZE_ERROR".to_string(),
        message: error.to_string(),
        field_errors: Vec::new(),
    })?;
    Ok(CoreToolExecution {
        status: status.to_string(),
        result,
    })
}

fn execute_media_stage(
    profile: &PlatformMcpProfile,
    conn: &Connection,
    workspace: &Path,
    arguments: &Value,
    browser_bridge: Option<&BrowserBridge>,
) -> Result<CoreToolExecution, McpCallError> {
    let bridge = browser_bridge.ok_or_else(|| McpCallError {
        code: "BROWSER_BRIDGE_UNAVAILABLE".to_string(),
        message: "发布页暂存仅能从 CreatorRelay 桌面应用内使用".to_string(),
        field_errors: Vec::new(),
    })?;
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
    let selected_assets = arguments
        .get("assetIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<std::collections::HashSet<_>>();
    let package =
        content::read_content_package(workspace, content_id).map_err(|error| McpCallError {
            code: "CONTENT_NOT_FOUND".to_string(),
            message: error,
            field_errors: vec![McpFieldError {
                path: "contentId".to_string(),
                message: "本地内容无法读取".to_string(),
            }],
        })?;
    let package_dir = workspace.join(&package.path);
    let cover_id = package
        .manifest
        .cover
        .as_ref()
        .map(|asset| asset.id.as_str());
    let mut media_files = Vec::new();
    let mut unknown_assets = Vec::new();
    for asset in &package.manifest.media {
        if selected_assets.is_empty() || selected_assets.contains(asset.id.as_str()) {
            media_files.push(package_dir.join(&asset.file));
        }
    }
    for asset_id in &selected_assets {
        let known = package
            .manifest
            .media
            .iter()
            .any(|asset| asset.id == **asset_id)
            || cover_id == Some(*asset_id);
        if !known {
            unknown_assets.push((*asset_id).to_string());
        }
    }
    if !unknown_assets.is_empty() {
        return Err(McpCallError {
            code: "ASSET_NOT_FOUND".to_string(),
            message: "内容包中不存在指定素材".to_string(),
            field_errors: vec![McpFieldError {
                path: "assetIds".to_string(),
                message: format!("未知素材：{}", unknown_assets.join(", ")),
            }],
        });
    }
    let cover_file = package
        .manifest
        .cover
        .as_ref()
        .filter(|asset| selected_assets.is_empty() || selected_assets.contains(asset.id.as_str()))
        .map(|asset| package_dir.join(&asset.file));
    let selected_kinds = package
        .manifest
        .media
        .iter()
        .filter(|asset| selected_assets.is_empty() || selected_assets.contains(asset.id.as_str()))
        .map(|asset| asset.kind)
        .collect::<Vec<_>>();
    let missing_required_media = match package.manifest.kind {
        content::ContentKind::Video => !selected_kinds.contains(&content::AssetKind::Video),
        content::ContentKind::ImageText => selected_kinds.is_empty(),
        content::ContentKind::ImageArticle | content::ContentKind::Article => false,
    };
    if missing_required_media {
        return Err(McpCallError {
            code: "ASSET_REQUIRED".to_string(),
            message: "该内容形态至少需要一个媒体素材".to_string(),
            field_errors: vec![McpFieldError {
                path: "assetIds".to_string(),
                message: "视频发布必须选择视频素材；图文发布必须至少选择一个媒体素材".to_string(),
            }],
        });
    }
    let body = if package.body.trim().is_empty() {
        package.manifest.summary.clone()
    } else {
        package.body.clone()
    };
    let receipt = bridge
        .stage_publish(
            conn,
            &profile.platform_id,
            &profile.handoff.publish_url,
            content_id,
            &package.manifest.title,
            &body,
            &package.manifest.tags,
            &media_files,
            cover_file.as_deref(),
        )
        .map_err(|error| McpCallError {
            code: "PUBLISH_STAGE_FAILED".to_string(),
            message: error,
            field_errors: Vec::new(),
        })?;
    let mut result = profile_summary(profile);
    result["stage"] = serde_json::to_value(&receipt).map_err(|error| McpCallError {
        code: "PUBLISH_STAGE_SERIALIZE_ERROR".to_string(),
        message: error.to_string(),
        field_errors: Vec::new(),
    })?;
    result["nextStep"] = json!({
        "manualReviewRequired": true,
        "submitTool": format!("{}_publish_submit", profile.platform_id),
        "warning": "页面已填写但未提交；请先在隔离浏览器核对标题、正文、媒体顺序、封面、可见范围和账号。"
    });
    let body_required = matches!(
        package.manifest.kind,
        content::ContentKind::ImageArticle | content::ContentKind::Article
    );
    let complete = receipt.title_filled
        && (!body_required || receipt.body_filled)
        && (cover_file.is_none() || receipt.uploaded_cover);
    Ok(CoreToolExecution {
        status: if complete { "success" } else { "partial" }.to_string(),
        result,
    })
}

fn execute_publish_submit(
    profile: &PlatformMcpProfile,
    arguments: &Value,
    browser_bridge: Option<&BrowserBridge>,
) -> Result<CoreToolExecution, McpCallError> {
    let bridge = browser_bridge.ok_or_else(|| McpCallError {
        code: "BROWSER_BRIDGE_UNAVAILABLE".to_string(),
        message: "发布表单提交仅能从 CreatorRelay 桌面应用内使用".to_string(),
        field_errors: Vec::new(),
    })?;
    let content_id = arguments
        .get("contentId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mode = arguments
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if mode == "publish"
        && arguments
            .get("confirmFinalPublish")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(McpCallError {
            code: "FINAL_PUBLISH_CONFIRMATION_REQUIRED".to_string(),
            message: "最终公开发布必须显式确认".to_string(),
            field_errors: vec![McpFieldError {
                path: "confirmFinalPublish".to_string(),
                message: "publish 模式必须设置为 true".to_string(),
            }],
        });
    }
    let button_label = arguments
        .get("buttonLabel")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let receipt = bridge
        .submit_current_publish(&profile.platform_id, content_id, mode, button_label)
        .map_err(|error| McpCallError {
            code: "PUBLISH_SUBMIT_FAILED".to_string(),
            message: error,
            field_errors: Vec::new(),
        })?;
    let mut result = profile_summary(profile);
    result["submit"] = serde_json::to_value(receipt).map_err(|error| McpCallError {
        code: "PUBLISH_SUBMIT_SERIALIZE_ERROR".to_string(),
        message: error.to_string(),
        field_errors: Vec::new(),
    })?;
    result["nextStep"] = json!({
        "verifyOnPlatform": true,
        "verifyWithTool": format!("{}_work_list", profile.platform_id),
        "warning": "工具不会自动重试；请在平台页面确认保存/发布结果后再同步作品。"
    });
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
