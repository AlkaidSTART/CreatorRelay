use super::*;

pub fn get_settings(conn: &Connection) -> Result<McpSettings, String> {
    let fallback = McpSettings::default();
    conn.query_row(
        "SELECT read_enabled, write_policy, external_policy, auto_restart,
                max_restarts, default_context_scope
         FROM mcp_settings WHERE id=1",
        [],
        |row| {
            Ok(McpSettings {
                read_enabled: row.get::<_, i64>(0)? != 0,
                write_policy: row.get(1)?,
                external_policy: row.get(2)?,
                auto_restart: row.get::<_, i64>(3)? != 0,
                max_restarts: u32::try_from(row.get::<_, i64>(4)?).unwrap_or(3),
                default_context_scope: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(|error| error.to_string())
    .map(|value| value.unwrap_or(fallback))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpSettingsInput {
    pub read_enabled: bool,
    pub write_policy: String,
    pub auto_restart: bool,
    pub max_restarts: u32,
    #[serde(default = "default_context_scope_setting")]
    pub default_context_scope: String,
}

fn default_context_scope_setting() -> String {
    "title_summary_media".to_string()
}

pub fn save_settings(conn: &Connection, input: McpSettingsInput) -> Result<McpSettings, String> {
    if !matches!(input.write_policy.as_str(), "confirm" | "allow") {
        return Err("写入授权策略必须是 confirm 或 allow".to_string());
    }
    if input.max_restarts > 10 {
        return Err("自动重启次数不能超过 10".to_string());
    }
    if !matches!(
        input.default_context_scope.as_str(),
        "title_only" | "title_summary" | "title_summary_media" | "selected"
    ) {
        return Err("默认模型上下文范围无效".to_string());
    }
    conn.execute(
        "UPDATE mcp_settings
         SET read_enabled=?1, write_policy=?2, external_policy='confirm_each_time',
             auto_restart=?3, max_restarts=?4, default_context_scope=?5, updated_at=?6
         WHERE id=1",
        params![
            if input.read_enabled { 1 } else { 0 },
            input.write_policy,
            if input.auto_restart { 1 } else { 0 },
            input.max_restarts,
            input.default_context_scope,
            Utc::now().to_rfc3339()
        ],
    )
    .map_err(|error| error.to_string())?;
    get_settings(conn)
}
