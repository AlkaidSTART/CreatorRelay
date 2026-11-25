use crate::{browser_bridge::BrowserBridge, database};
use rusqlite::Connection;
use std::path::PathBuf;

#[test]
fn live_creator_centers_are_opened_in_isolated_chrome() {
    if std::env::var("CREATORRELAY_LIVE_BROWSER_TEST").as_deref() != Ok("1") {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("creatorrelay.sqlite3");
    let mut conn = Connection::open(&database_path).unwrap();
    database::migrate(&mut conn).unwrap();
    let bridge = BrowserBridge::new(temp.path().to_path_buf()).unwrap();
    for platform_id in [
        "douyin",
        "xiaohongshu",
        "zhihu",
        "wechat",
        "kuaishou",
        "bilibili",
        "juejin",
        "csdn",
    ] {
        match bridge.open_creator_center(&conn, platform_id) {
            Ok(page) => {
                let login = page.url.contains("login")
                    || page.url.contains("signin")
                    || page.text.contains("请重新登录")
                    || page.text.contains("扫码登录")
                    || page.text.contains("验证码登录");
                println!(
                    "LIVE_CREATOR_BROWSER platform={} status={} title={} url={} visible_chars={}",
                    platform_id,
                    if login {
                        "login_required"
                    } else {
                        "creator_page_loaded"
                    },
                    page.title,
                    page.url,
                    page.text.chars().count()
                );
            }
            Err(error) => println!(
                "LIVE_CREATOR_BROWSER platform={} status=unavailable error={}",
                platform_id, error
            ),
        }
    }
    bridge.stop().unwrap();
}
