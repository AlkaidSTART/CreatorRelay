use super::*;

fn account_view(account: &platforms::PlatformAccount) -> Value {
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

