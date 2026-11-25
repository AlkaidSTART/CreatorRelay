use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Write},
    thread,
    time::Duration,
};

use super::{
    manifest::{AdapterManifest, ContentKind, SUPPORTED_ADAPTER_CONTRACT_VERSION},
    publish::{PublishInput, validate_publish_payload},
    runtime::{HostAccessObservation, SandboxProbeObservation},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunnerProfile {
    Healthy,
    Faulty,
    ProbeFailure,
}

impl RunnerProfile {
    fn parse(value: &str) -> Self {
        match value {
            "faulty" | "fault" => Self::Faulty,
            "probeFailure" | "probe-failure" => Self::ProbeFailure,
            _ => Self::Healthy,
        }
    }
}

pub fn run_adapter_runner() {
    let profile = argument_value("--profile")
        .map(|value| RunnerProfile::parse(&value))
        .unwrap_or(RunnerProfile::Healthy);
    let manifest_id = std::env::var("CREATORRELAY_ADAPTER_ID")
        .unwrap_or_else(|_| "creatorrelay.test-adapter".to_string());
    let manifest_version =
        std::env::var("CREATORRELAY_ADAPTER_VERSION").unwrap_or_else(|_| "1.0.0".to_string());
    let sandboxed = std::env::var("CREATORRELAY_SANDBOX_ACTIVE")
        .map(|value| value == "1")
        .unwrap_or(false);

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        let mut line = String::new();
        let read = {
            let mut lock = stdin.lock();
            lock.read_line(&mut line)
        };
        match read {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            let _ = writeln!(
                stdout,
                "{}",
                json!({
                  "type": "response",
                  "id": "",
                  "ok": false,
                  "error": { "code": "INVALID_JSON", "message": "Runner 收到非法 JSON" }
                })
            );
            let _ = stdout.flush();
            continue;
        };
        if message.get("type").and_then(Value::as_str) != Some("request") {
            continue;
        }
        let request_id = message
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let method = message
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let (ok, result, error) = handle_request(
            profile,
            &manifest_id,
            &manifest_version,
            sandboxed,
            &method,
            &params,
            &stdin,
        );
        let response = if ok {
            json!({
              "type": "response",
              "id": request_id,
              "ok": true,
              "result": result
            })
        } else {
            json!({
              "type": "response",
              "id": request_id,
              "ok": false,
              "error": error
            })
        };
        let _ = writeln!(stdout, "{response}");
        let _ = stdout.flush();
        if method == "shutdown" {
            return;
        }
    }
}

fn argument_value(flag: &str) -> Option<String> {
    let args = std::env::args().collect::<Vec<_>>();
    args.iter()
        .position(|argument| argument == flag)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn handle_request(
    profile: RunnerProfile,
    manifest_id: &str,
    manifest_version: &str,
    sandboxed: bool,
    method: &str,
    params: &Value,
    stdin: &io::Stdin,
) -> (bool, Value, Value) {
    match method {
        "describe" => (
            true,
            json!({
              "id": manifest_id,
              "version": manifest_version,
              "adapterContractVersion": SUPPORTED_ADAPTER_CONTRACT_VERSION,
              "runtime": "ok",
              "sandboxed": sandboxed
            }),
            Value::Null,
        ),
        "probe" => {
            if profile == RunnerProfile::ProbeFailure {
                return (
                    false,
                    Value::Null,
                    json!({
                      "code": "PROBE_FAILED",
                      "message": "测试适配器按配置模拟运行时能力探测失败"
                    }),
                );
            }
            let account_ref = params
                .get("accountRef")
                .and_then(Value::as_str)
                .unwrap_or("fixture-image-text");
            let host_access = probe_host_access(account_ref, stdin);
            (
                true,
                json!({
                  "accountRef": account_ref,
                  "health": "ok",
                  "sandboxed": sandboxed,
                  "capabilities": [
                    { "kind": "video", "read": "unsupported", "publish": "unsupported", "reason": "测试适配器不模拟视频发布" },
                    { "kind": "imageText", "read": "supported", "publish": "supported", "reason": "图文发布可验证" },
                    { "kind": "imageArticle", "read": "supported", "publish": "restricted", "reason": "图文文章仍需人工审核" },
                    { "kind": "article", "read": "supported", "publish": "supported", "reason": "纯文章发布可验证" }
                  ],
                  "hostAccess": host_access,
                  "isolation": {
                    "sandboxed": sandboxed,
                    "processIsolation": true
                  }
                }),
                Value::Null,
            )
        }
        "securitySelfTest" => {
            let database_path = params
                .get("databasePath")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let workspace_path = params
                .get("workspacePath")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let credential_path = params
                .get("credentialPath")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let observations = vec![
                sandbox_probe("database", database_path),
                sandbox_probe("workspace", workspace_path),
                sandbox_probe("credential", credential_path),
            ];
            (
                true,
                json!({
                  "sandboxed": sandboxed,
                  "observations": observations
                }),
                Value::Null,
            )
        }
        "validatePublish" => {
            let input = match serde_json::from_value::<PublishInput>(
                params.get("input").cloned().unwrap_or(Value::Null),
            ) {
                Ok(input) => input,
                Err(error) => {
                    return (
                        false,
                        Value::Null,
                        json!({ "code": "INVALID_PAYLOAD_SCHEMA", "message": error.to_string() }),
                    );
                }
            };
            let result = validate_publish_payload(&input);
            if !result.ok {
                return (
                    false,
                    Value::Null,
                    json!({
                      "code": "INVALID_PAYLOAD",
                      "message": result.issues.iter().map(|issue| issue.message.as_str()).collect::<Vec<_>>().join("；")
                    }),
                );
            }
            if input.content_kind == ContentKind::Video {
                return (
                    false,
                    Value::Null,
                    json!({
                      "code": "CAPABILITY_UNSUPPORTED",
                      "message": "该测试账号不支持视频发布"
                    }),
                );
            }
            (
                true,
                json!({
                  "accepted": true,
                  "contentKind": input.content_kind.as_str(),
                  "mediaOrder": result.media_order,
                  "message": "Adapter 载荷预检通过"
                }),
                Value::Null,
            )
        }
        "publish" => {
            let kind = params
                .get("input")
                .and_then(|input| input.get("contentKind"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if kind == "video" {
                return (
                    false,
                    Value::Null,
                    json!({ "code": "CAPABILITY_UNSUPPORTED", "message": "该测试账号不支持视频发布" }),
                );
            }
            (
                true,
                json!({
                  "externalId": format!("test-{}", uuid::Uuid::new_v4()),
                  "contentKind": kind,
                  "simulated": true
                }),
                Value::Null,
            )
        }
        "crash" => {
            std::process::exit(17);
        }
        "timeout" => {
            thread::sleep(Duration::from_secs(30));
            (
                true,
                json!({ "message": "timeout simulation finished" }),
                Value::Null,
            )
        }
        "shutdown" => (true, json!({ "shutdown": true }), Value::Null),
        _ => (
            false,
            Value::Null,
            json!({ "code": "METHOD_NOT_FOUND", "message": format!("未知 Adapter 方法：{method}") }),
        ),
    }
}

fn sandbox_probe(target: &str, path: &str) -> SandboxProbeObservation {
    if path.is_empty() {
        return SandboxProbeObservation {
            target: target.to_string(),
            allowed: false,
            error: Some("测试路径为空".to_string()),
        };
    }
    match std::fs::read(path) {
        Ok(_) => SandboxProbeObservation {
            target: target.to_string(),
            allowed: true,
            error: None,
        },
        Err(error) => SandboxProbeObservation {
            target: target.to_string(),
            allowed: false,
            error: Some(error.kind().to_string()),
        },
    }
}

fn probe_host_access(account_ref: &str, stdin: &io::Stdin) -> Vec<HostAccessObservation> {
    let calls = [
        ("credentials.get", json!({ "accountRef": account_ref })),
        ("credentials.readRaw", json!({ "accountRef": account_ref })),
        (
            "workspace.read",
            json!({ "path": "/__creatorrelay_forbidden_workspace__/article.md" }),
        ),
        (
            "network.request",
            json!({ "url": "https://example.invalid/probe" }),
        ),
        (
            "database.query",
            json!({ "sql": "SELECT * FROM platform_accounts" }),
        ),
    ];
    calls
        .into_iter()
        .map(|(method, params)| host_call(method, params, stdin))
        .collect()
}

fn host_call(method: &str, params: Value, stdin: &io::Stdin) -> HostAccessObservation {
    let call_id = uuid::Uuid::new_v4().to_string();
    let message = json!({
      "type": "hostCall",
      "callId": call_id,
      "method": method,
      "params": params
    });
    {
        let mut stdout = io::stdout();
        let _ = writeln!(stdout, "{message}");
        let _ = stdout.flush();
    }
    let mut response = String::new();
    let read = stdin.lock().read_line(&mut response);
    if read.is_err() || response.trim().is_empty() {
        return HostAccessObservation {
            method: method.to_string(),
            allowed: false,
            code: Some("HOST_PROTOCOL_ERROR".to_string()),
            detail: Some("未收到 Core hostResult".to_string()),
        };
    }
    let value: Value = serde_json::from_str(response.trim()).unwrap_or(Value::Null);
    if value.get("type").and_then(Value::as_str) != Some("hostResult") {
        return HostAccessObservation {
            method: method.to_string(),
            allowed: false,
            code: Some("HOST_PROTOCOL_ERROR".to_string()),
            detail: Some("hostResult 类型不匹配".to_string()),
        };
    }
    let allowed = value.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let code = value
        .get("error")
        .and_then(|error| error.get("code"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let detail = value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string);
    HostAccessObservation {
        method: method.to_string(),
        allowed,
        code,
        detail,
    }
}

#[allow(dead_code)]
fn manifest_security_hint(manifest: &AdapterManifest) -> String {
    format!("{} v{}", manifest.id, manifest.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m4_profile_parser_keeps_failure_profiles_explicit() {
        assert_eq!(RunnerProfile::parse("healthy"), RunnerProfile::Healthy);
        assert_eq!(RunnerProfile::parse("faulty"), RunnerProfile::Faulty);
        assert_eq!(
            RunnerProfile::parse("probe-failure"),
            RunnerProfile::ProbeFailure
        );
    }
}
