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
        "creator_centers_list" => {
            let platform_id = arguments.get("platformId").and_then(Value::as_str);
            let links =
                crate::creator_centers::list(conn, platform_id).map_err(|error| McpCallError {
                    code: "CREATOR_CENTER_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?;
            success_result(json!({
                "creatorCenters": links,
                "count": links.len(),
                "scope": "configured_links_only",
                "platformDataAccess": "not_connected",
                "automatedPublishing": "disabled",
            }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{database, mcp::catalog};

    #[test]
    fn creator_centers_mcp_tool_only_returns_configured_links_and_clear_capability_scope() {
        let mut conn = Connection::open_in_memory().unwrap();
        database::migrate(&mut conn).unwrap();
        let result = execute_account_tool(
            &conn,
            "creator_centers_list",
            &json!({ "platformId": "douyin" }),
        )
        .unwrap();

        assert_eq!(result.status, "success");
        assert_eq!(result.result["count"], 1);
        assert_eq!(
            result.result["creatorCenters"][0]["url"],
            "https://creator.douyin.com/"
        );
        assert_eq!(result.result["platformDataAccess"], "not_connected");
        assert_eq!(result.result["automatedPublishing"], "disabled");
        let definition = catalog::tool_catalog()
            .into_iter()
            .find(|tool| tool.name == "creator_centers_list")
            .unwrap();
        assert_eq!(definition.permission, ToolPermission::Read);
        assert!(definition.description.contains("不会登录平台"));
    }
}
