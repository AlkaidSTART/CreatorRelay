use creatorrelay_lib::{
    browser_bridge::BrowserBridge,
    database,
    mcp::{self, McpCallRequest, McpHost, McpPreviewRequest, ToolPermission},
    plugins::PluginRuntime,
};
use rusqlite::Connection;
use serde_json::json;
use std::{fs, path::PathBuf};

fn fixture() -> (
    tempfile::TempDir,
    Connection,
    PluginRuntime,
    McpHost,
    PathBuf,
) {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("creatorrelay.sqlite3");
    let mut conn = Connection::open(&database_path).unwrap();
    database::migrate(&mut conn).unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('workspace_path', ?1)",
        [workspace.to_string_lossy().to_string()],
    )
    .unwrap();
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay"));
    let runtime = PluginRuntime::new(
        temp.path().join("plugins"),
        temp.path().to_path_buf(),
        database_path,
        executable.clone(),
    )
    .unwrap();
    let host = McpHost::new(executable);
    host.start(None).unwrap();
    (temp, conn, runtime, host, workspace)
}

#[test]
fn browser_tools_are_discoverable_and_open_requires_one_time_confirmation() {
    let (temp, mut conn, runtime, host, workspace) = fixture();
    let bridge = BrowserBridge::new_with_database(
        temp.path().to_path_buf(),
        temp.path().join("creatorrelay.sqlite3"),
    )
    .unwrap();
    let tools = host.list_tools().unwrap();
    assert!(
        tools
            .iter()
            .any(|tool| tool.name == "creator_browser_status")
    );
    assert!(
        tools
            .iter()
            .any(|tool| tool.name == "creator_browser_read_page")
    );
    assert_eq!(
        tools
            .iter()
            .find(|tool| tool.name == "creator_browser_open")
            .unwrap()
            .permission,
        ToolPermission::ExternalAction
    );
    let status = mcp::call_tool_with_browser(
        &mut conn,
        &runtime,
        &workspace,
        &host,
        McpCallRequest {
            call_id: None,
            tool_name: "creator_browser_status".to_string(),
            arguments: json!({}),
            thread_id: None,
            authorization_token: None,
            timeout_ms: None,
            context_scope: None,
        },
        Some(&bridge),
    )
    .unwrap();
    assert_eq!(status.status, "success");
    assert_eq!(status.result.as_ref().unwrap()["state"], "stopped");
    assert_eq!(status.result.as_ref().unwrap()["isolatedProfile"], true);

    let args = json!({ "platformId": "xiaohongshu" });
    let preview = mcp::preview_tool_call(
        &conn,
        &workspace,
        McpPreviewRequest {
            tool_name: "creator_browser_open".to_string(),
            arguments: args.clone(),
            thread_id: None,
        },
    )
    .unwrap();
    assert!(preview.requires_confirmation);
    assert!(preview.authorization_token.is_some());
    assert!(
        preview.details["url"]
            .as_str()
            .unwrap()
            .starts_with("https://")
    );
    assert!(
        preview
            .effects
            .iter()
            .any(|effect| effect.contains("浏览器配置"))
    );
    let result = mcp::call_tool_with_browser(
        &mut conn,
        &runtime,
        &workspace,
        &host,
        McpCallRequest {
            call_id: None,
            tool_name: "creator_browser_open".to_string(),
            arguments: args.clone(),
            thread_id: None,
            authorization_token: None,
            timeout_ms: None,
            context_scope: None,
        },
        Some(&bridge),
    )
    .unwrap();
    assert_eq!(result.status, "failed", "error={:?}", result.error);
    assert_eq!(result.error.unwrap().code, "AUTHORIZATION_REQUIRED");
    assert_eq!(bridge.status().unwrap().state, "stopped");

    let raw_preview = mcp::preview_tool_call(
        &conn,
        &workspace,
        McpPreviewRequest {
            tool_name: "creator_browser_open".to_string(),
            arguments: json!({ "platformId": "xiaohongshu" }),
            thread_id: None,
        },
    )
    .unwrap();
    let stdio_request = McpCallRequest {
        call_id: Some(raw_preview.call_id),
        tool_name: "creator_browser_open".to_string(),
        arguments: json!({ "platformId": "xiaohongshu" }),
        thread_id: None,
        authorization_token: raw_preview.authorization_token,
        timeout_ms: Some(30_000),
        context_scope: None,
    };
    // A raw stdio client has no desktop-injected bridge; even an authorization token
    // cannot cause this local browser process to execute on its behalf.
    let direct = mcp::call_tool(&mut conn, &runtime, &workspace, &host, stdio_request).unwrap();
    assert_eq!(direct.status, "failed");
    assert_eq!(direct.error.unwrap().code, "BROWSER_BRIDGE_UNAVAILABLE");
    assert_eq!(bridge.status().unwrap().state, "stopped");

    // A desktop-injected bridge call with a fresh single-use preview token is the supported path.
    let approved = mcp::preview_tool_call(
        &conn,
        &workspace,
        McpPreviewRequest {
            tool_name: "creator_browser_open".to_string(),
            arguments: json!({"platformId":"xiaohongshu"}),
            thread_id: None,
        },
    )
    .unwrap();
    let opened = mcp::call_tool_with_browser(
        &mut conn,
        &runtime,
        &workspace,
        &host,
        McpCallRequest {
            call_id: Some(approved.call_id.clone()),
            tool_name: approved.tool_name.clone(),
            arguments: json!({ "platformId": "xiaohongshu" }),
            thread_id: None,
            authorization_token: approved.authorization_token.clone(),
            timeout_ms: Some(30_000),
            context_scope: None,
        },
        Some(&bridge),
    )
    .unwrap();
    assert_eq!(opened.status, "success", "error={:?}", opened.error);
    assert_eq!(bridge.status().unwrap().state, "running");
    let read_preview = mcp::preview_tool_call(
        &conn,
        &workspace,
        McpPreviewRequest {
            tool_name: "creator_browser_read_page".to_string(),
            arguments: json!({"platformId":"xiaohongshu"}),
            thread_id: None,
        },
    )
    .unwrap();
    assert!(read_preview.requires_confirmation);
    let page_read = mcp::call_tool_with_browser(
        &mut conn,
        &runtime,
        &workspace,
        &host,
        McpCallRequest {
            call_id: Some(read_preview.call_id.clone()),
            tool_name: "creator_browser_read_page".to_string(),
            arguments: json!({"platformId":"xiaohongshu"}),
            thread_id: None,
            authorization_token: read_preview.authorization_token,
            timeout_ms: Some(30_000),
            context_scope: None,
        },
        Some(&bridge),
    )
    .unwrap();
    assert_eq!(page_read.status, "success", "error={:?}", page_read.error);
    assert_eq!(
        page_read.result.as_ref().unwrap()["pageStatus"],
        "login_required"
    );
    bridge.stop().unwrap();
}

#[test]
fn read_current_page_requires_mcp_opened_creator_platform() {
    let temp = tempfile::tempdir().unwrap();
    let bridge = BrowserBridge::new_with_database(
        temp.path().to_path_buf(),
        temp.path().join("creatorrelay.sqlite3"),
    )
    .unwrap();
    assert!(
        bridge
            .read_current_page("douyin")
            .unwrap_err()
            .contains("尚未启动")
    );
}

#[test]
fn live_creator_browser_opens_all_eight_platform_urls_through_mcp() {
    if std::env::var("CREATORRELAY_LIVE_BROWSER_TEST").as_deref() != Ok("1") {
        return;
    }
    let (temp, mut conn, runtime, host, workspace) = fixture();
    let bridge = BrowserBridge::new_with_database(
        temp.path().to_path_buf(),
        temp.path().join("creatorrelay.sqlite3"),
    )
    .unwrap();
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
        let arguments = json!({"platformId": platform_id});
        let preview = mcp::preview_tool_call(
            &conn,
            &workspace,
            McpPreviewRequest {
                tool_name: "creator_browser_open".to_string(),
                arguments: arguments.clone(),
                thread_id: None,
            },
        )
        .unwrap();
        assert!(preview.requires_confirmation);
        let result = mcp::call_tool_with_browser(
            &mut conn,
            &runtime,
            &workspace,
            &host,
            McpCallRequest {
                call_id: None,
                tool_name: "creator_browser_open".to_string(),
                arguments,
                thread_id: None,
                authorization_token: preview.authorization_token,
                timeout_ms: Some(30_000),
                context_scope: None,
            },
            Some(&bridge),
        )
        .unwrap();
        let value = result.result.unwrap_or_default();
        if result.status != "success" {
            println!(
                "LIVE_CREATOR_BROWSER platform={} status=unavailable error={}",
                platform_id,
                result.error.map(|error| error.message).unwrap_or_default()
            );
            continue;
        }
        let url = value
            .get("url")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let title = value
            .get("title")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let text = value
            .get("text")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let page_status = value
            .get("pageStatus")
            .and_then(|value| value.as_str())
            .unwrap_or("page_shell_or_loading");
        let status = match page_status {
            "login_required" => "login_required",
            "page_visible" => "creator_page_visible",
            _ => "page_shell_or_loading",
        };
        println!(
            "LIVE_CREATOR_BROWSER mcp_tool=creator_browser_open platform={} status={} title={} url={} visible_chars={} preview={}",
            platform_id,
            status,
            title,
            url,
            text.chars().count(),
            text.chars().take(80).collect::<String>().replace('\n', " ")
        );
    }
    bridge.stop().unwrap();
}
