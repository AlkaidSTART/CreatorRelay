use super::outcome::{CoreToolExecution, success_result};
use super::*;

pub(super) fn account_view(account: &platforms::PlatformAccount) -> Value {
    json!({
        "id": account.id,
        "platformId": account.platform_id,
        "platformName": account.platform_name,
        "category": account.category,
        "alias": account.alias,
        "nickname": account.nickname,
        "uid": account.uid,
        "status": account.status,
        "enabled": account.enabled,
        "isDefault": account.is_default,
        "lastCheckedAt": account.last_checked_at,
        "lastError": account.last_error,
        "capabilities": account.capabilities,
    })
}

pub(super) fn execute_account_tool(
    conn: &Connection,
    tool_name: &str,
    arguments: &Value,
) -> Result<CoreToolExecution, McpCallError> {
    let result = match tool_name {
        "account_get_info" => {
            let accounts = platforms::list_accounts(conn).map_err(|error| McpCallError {
                code: "ACCOUNT_SERVICE_ERROR".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
            let account_id = arguments.get("accountId").and_then(Value::as_str);
            let platform_id = arguments.get("platformId").and_then(Value::as_str);
            let views = accounts
                .iter()
                .filter(|account| {
                    account_id.is_none_or(|value| account.id == value)
                        && platform_id.is_none_or(|value| account.platform_id == value)
                })
                .map(account_view)
                .collect::<Vec<_>>();
            success_result(json!({ "accounts": views, "count": views.len() }))
        }
        "platform_capabilities_get" => {
            let platform_id = arguments.get("platformId").and_then(Value::as_str);
            let account_id = arguments.get("accountId").and_then(Value::as_str);
            let admissions = ecosystem::platform_admissions()
                .map_err(|error| McpCallError {
                    code: "ECOSYSTEM_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?
                .into_iter()
                .filter(|admission| platform_id.is_none_or(|value| admission.platform_id == value))
                .collect::<Vec<_>>();
            let accounts = ecosystem::list_account_capability_snapshots(conn)
                .map_err(|error| McpCallError {
                    code: "ECOSYSTEM_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?
                .into_iter()
                .filter(|account| {
                    platform_id.is_none_or(|value| account.platform_id == value)
                        && account_id.is_none_or(|value| account.account_id == value)
                })
                .collect::<Vec<_>>();
            success_result(json!({
                "platforms": admissions,
                "accounts": accounts,
                "platformCount": admissions.len(),
                "accountCount": accounts.len(),
                "autoPublishDefault": "manual_only"
            }))
        }
        _ => {
            return Err(McpCallError {
                code: "TOOL_NOT_FOUND".to_string(),
                message: format!("未注册账号工具：{tool_name}"),
                field_errors: Vec::new(),
            });
        }
    };
    Ok(result)
}
