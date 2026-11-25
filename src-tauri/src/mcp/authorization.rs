use super::*;

fn issue_authorization(
    conn: &Connection,
    call_id: &str,
    tool_name: &str,
    permission: ToolPermission,
    arguments: &Value,
) -> Result<(String, String), String> {
    let token = Uuid::new_v4().to_string();
    let expires_at = (Utc::now() + Duration::minutes(AUTH_TTL_MINUTES)).to_rfc3339();
    conn.execute(
        "INSERT INTO mcp_authorization_tokens(
            token, call_id, tool_name, payload_hash, permission, expires_at, created_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            token,
            call_id,
            tool_name,
            payload_hash(tool_name, arguments)?,
            permission.as_str(),
            expires_at,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| format!("工具授权令牌写入失败：{error}"))?;
    Ok((token, expires_at))
}

fn consume_authorization(
    conn: &Connection,
    token: &str,
    tool_name: &str,
    permission: ToolPermission,
    arguments: &Value,
) -> Result<(), McpCallError> {
    let row = conn
        .query_row(
            "SELECT tool_name, payload_hash, permission, expires_at, used_at
             FROM mcp_authorization_tokens WHERE token=?1",
            [token],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(|error| McpCallError {
            code: "AUTHORIZATION_STORE_ERROR".to_string(),
            message: error.to_string(),
            field_errors: Vec::new(),
        })?
        .ok_or_else(|| McpCallError {
            code: "AUTHORIZATION_REQUIRED".to_string(),
            message: "该工具需要一次明确的用户确认，请先预览最终目标与参数".to_string(),
            field_errors: Vec::new(),
        })?;
    if row.4.is_some() {
        return Err(McpCallError {
            code: "AUTHORIZATION_USED".to_string(),
            message: "确认令牌已使用；外部操作不会自动重试".to_string(),
            field_errors: Vec::new(),
        });
    }
    if row.2 != permission.as_str() || row.0 != tool_name {
        return Err(McpCallError {
            code: "AUTHORIZATION_MISMATCH".to_string(),
            message: "确认令牌与本次工具或权限等级不一致".to_string(),
            field_errors: Vec::new(),
        });
    }
    let expected_hash = payload_hash(tool_name, arguments).map_err(|message| McpCallError {
        code: "AUTHORIZATION_HASH_ERROR".to_string(),
        message,
        field_errors: Vec::new(),
    })?;
    if row.1 != expected_hash {
        return Err(McpCallError {
            code: "AUTHORIZATION_PAYLOAD_CHANGED".to_string(),
            message: "预览后参数或目标已变化，请重新预览并确认".to_string(),
            field_errors: Vec::new(),
        });
    }
    let expires = DateTime::parse_from_rfc3339(&row.3).map_err(|_| McpCallError {
        code: "AUTHORIZATION_INVALID".to_string(),
        message: "确认过期时间格式无效".to_string(),
        field_errors: Vec::new(),
    })?;
    if expires <= Utc::now() {
        return Err(McpCallError {
            code: "AUTHORIZATION_EXPIRED".to_string(),
            message: "确认已过期，请重新预览最终目标与内容".to_string(),
            field_errors: Vec::new(),
        });
    }
    let changed = conn
        .execute(
            "UPDATE mcp_authorization_tokens SET used_at=?1 WHERE token=?2 AND used_at IS NULL",
            params![Utc::now().to_rfc3339(), token],
        )
        .map_err(|error| McpCallError {
            code: "AUTHORIZATION_STORE_ERROR".to_string(),
            message: error.to_string(),
            field_errors: Vec::new(),
        })?;
    if changed != 1 {
        return Err(McpCallError {
            code: "AUTHORIZATION_USED".to_string(),
            message: "确认令牌已被使用".to_string(),
            field_errors: Vec::new(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
