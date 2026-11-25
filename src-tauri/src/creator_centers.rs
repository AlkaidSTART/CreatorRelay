use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatorCenterLink {
    pub platform_id: String,
    pub platform_name: String,
    pub url: String,
    pub updated_at: String,
}

pub fn list(
    conn: &Connection,
    platform_id: Option<&str>,
) -> Result<Vec<CreatorCenterLink>, String> {
    let mut statement = conn
        .prepare(
            "SELECT platform_id, platform_name, url, updated_at
             FROM creator_center_links
             WHERE (?1 IS NULL OR platform_id = ?1)
             ORDER BY sort_order, platform_id",
        )
        .map_err(|error| format!("无法读取创作者中心链接：{error}"))?;
    let rows = statement
        .query_map([platform_id], |row| {
            Ok(CreatorCenterLink {
                platform_id: row.get(0)?,
                platform_name: row.get(1)?,
                url: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })
        .map_err(|error| format!("无法查询创作者中心链接：{error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法解析创作者中心链接：{error}"))
}

pub fn save_url(
    conn: &Connection,
    platform_id: &str,
    url: &str,
) -> Result<CreatorCenterLink, String> {
    let canonical_url = validate_url(url)?;
    let updated_at = Utc::now().to_rfc3339();
    let changed = conn
        .execute(
            "UPDATE creator_center_links SET url = ?1, updated_at = ?2 WHERE platform_id = ?3",
            params![canonical_url, updated_at, platform_id],
        )
        .map_err(|error| format!("无法保存创作者中心链接：{error}"))?;
    if changed != 1 {
        return Err("未找到对应平台的创作者中心配置".to_string());
    }
    get(conn, platform_id)?.ok_or_else(|| "保存后未能重新读取创作者中心链接".to_string())
}

fn get(conn: &Connection, platform_id: &str) -> Result<Option<CreatorCenterLink>, String> {
    conn.query_row(
        "SELECT platform_id, platform_name, url, updated_at
         FROM creator_center_links WHERE platform_id = ?1",
        [platform_id],
        |row| {
            Ok(CreatorCenterLink {
                platform_id: row.get(0)?,
                platform_name: row.get(1)?,
                url: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|error| format!("无法读取创作者中心链接：{error}"))
}

fn validate_url(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 2048 || value.chars().any(char::is_control) {
        return Err("链接不能为空，且长度不能超过 2048 个字符".to_string());
    }
    let parsed = Url::parse(value).map_err(|_| "请输入有效的 HTTPS 创作者中心链接".to_string())?;
    if parsed.scheme() != "https"
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err("创作者中心链接必须使用 HTTPS，且不能包含用户名或密码".to_string());
    }
    Ok(parsed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database;

    fn migrated_connection() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        database::migrate(&mut conn).unwrap();
        conn
    }

    #[test]
    fn seeds_eight_creator_centers_and_updates_the_requested_url() {
        let conn = migrated_connection();
        let links = list(&conn, None).unwrap();
        assert_eq!(links.len(), 8);
        assert_eq!(
            links
                .iter()
                .find(|link| link.platform_id == "wechat")
                .unwrap()
                .url,
            "https://mp.weixin.qq.com/cgi-bin/home?t=home/index&lang=zh_CN"
        );

        let changed = save_url(&conn, "douyin", "https://creator.douyin.com/new/creator").unwrap();
        assert_eq!(changed.url, "https://creator.douyin.com/new/creator");
        assert_eq!(list(&conn, Some("douyin")).unwrap(), vec![changed]);
    }

    #[test]
    fn refuses_invalid_urls_and_unknown_platforms() {
        let conn = migrated_connection();
        assert!(save_url(&conn, "douyin", "http://creator.douyin.com/").is_err());
        assert!(save_url(&conn, "douyin", "https://user:pass@example.com/").is_err());
        assert!(save_url(&conn, "unknown", "https://example.com/").is_err());
    }
}
