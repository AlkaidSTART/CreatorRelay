use super::*;

pub fn test_server_main() -> ! {
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            continue;
        };
        let id = message.get("id").cloned();
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        if matches!(
            method,
            "notifications/initialized" | "notifications/cancelled"
        ) {
            continue;
        }
        let response = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "CreatorRelay 测试 MCP Server", "version": "0.1.0" }
            }),
            "ping" => json!({}),
            "tools/list" => json!({ "tools": server_tool_descriptors() }),
            "tools/call" => {
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if name == "mcp_test_echo" {
                    json!({
                        "content": [{
                            "type": "text",
                            "text": params.pointer("/arguments/message").and_then(Value::as_str).unwrap_or_default()
                        }],
                        "isError": false
                    })
                } else if tool_by_name(name).is_some() {
                    json!({
                        "content": [{ "type": "text", "text": "forwarded-to-host-core" }],
                        "structuredContent": {
                            "forwardToHost": true,
                            "tool": name,
                            "arguments": params.get("arguments").cloned().unwrap_or_else(|| json!({}))
                        },
                        "isError": false
                    })
                } else {
                    json!({
                        "content": [{ "type": "text", "text": "unknown tool" }],
                        "isError": true
                    })
                }
            }
            _ => {
                if let Some(id) = id {
                    let _ = writeln!(
                        stdout,
                        "{}",
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "error": { "code": -32601, "message": "Method not found" }
                        })
                    );
                }
                continue;
            }
        };
        if let Some(id) = id {
            let payload = json!({ "jsonrpc": "2.0", "id": id, "result": response });
            if writeln!(stdout, "{payload}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    }
    std::process::exit(0)
}
