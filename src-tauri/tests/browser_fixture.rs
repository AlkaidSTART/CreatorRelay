use creatorrelay_lib::browser_bridge::BrowserPageSnapshot;
use serde_json::json;
use std::{
    io::{BufRead, Write},
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
fn mcp_server_lists_creator_browser_tools() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay"));
    let mut child = Command::new(executable)
        .arg("--mcp-test-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdin = child.stdin.as_mut().unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let request = json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}});
    writeln!(stdin, "{request}").unwrap();
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let response: serde_json::Value = serde_json::from_str(&line).unwrap();
    let names = response
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .unwrap()
        .iter()
        .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
        .collect::<Vec<_>>();
    assert!(names.contains(&"creator_browser_status"));
    assert!(names.contains(&"creator_browser_open"));
    assert!(names.contains(&"creator_browser_read_page"));
    child.kill().unwrap();
    child.wait().unwrap();
}

#[test]
fn browser_snapshot_and_status_contracts_have_expected_shape() {
    let status = serde_json::json!({
        "state": "stopped",
        "devtoolsReady": false,
        "isolatedProfile": true,
        "browserVersion": null,
        "currentPlatformId": null,
        "currentUrl": null,
        "currentPageStatus": null,
        "lastError": null
    });
    assert_eq!(status["state"], "stopped");
    assert_eq!(status["isolatedProfile"], true);
    assert_eq!(status["devtoolsReady"], false);
    let page = BrowserPageSnapshot {
        platform_id: "douyin".into(),
        platform_name: "抖音".into(),
        url: "https://creator.douyin.com/".into(),
        title: "Creator".into(),
        text: "Visible page".into(),
        page_status: "page_visible".into(),
        captured_at: "2026-11-25T00:00:00Z".into(),
        note: "visible only".into(),
    };
    let page = serde_json::to_value(page).unwrap();
    assert_eq!(page["platformId"], "douyin");
    assert_eq!(page["text"], "Visible page");
    assert_eq!(page["pageStatus"], "page_visible");
}

#[test]
fn raw_stdio_browser_open_is_refused_outside_desktop_confirmation_flow() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay"));
    let mut child = Command::new(executable)
        .arg("--mcp-test-server")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdin = child.stdin.as_mut().unwrap();
    let stdout = child.stdout.as_mut().unwrap();
    let request = json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"creator_browser_open","arguments":{"platformId":"douyin"}}});
    writeln!(stdin, "{request}").unwrap();
    let mut reader = std::io::BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let response: serde_json::Value = serde_json::from_str(&line).unwrap();
    assert_eq!(
        response.pointer("/result/isError"),
        Some(&serde_json::Value::Bool(true))
    );
    assert!(
        response
            .pointer("/result/content/0/text")
            .and_then(|value| value.as_str())
            .unwrap()
            .contains("裸 stdio Server")
    );
    child.kill().unwrap();
    child.wait().unwrap();
}
