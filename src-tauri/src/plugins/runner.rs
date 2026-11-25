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
    PublishTimeout,
}

impl RunnerProfile {
    fn parse(value: &str) -> Self {
        match value {
            "faulty" | "fault" => Self::Faulty,
            "probeFailure" | "probe-failure" => Self::ProbeFailure,
            "publishTimeout" | "publish-timeout" => Self::PublishTimeout,
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
                  "capabilities": test_capabilities(manifest_id),
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
            if !test_publish_supported(manifest_id, input.content_kind) {
                return (
                    false,
                    Value::Null,
                    json!({
                      "code": "CAPABILITY_UNSUPPORTED",
                      "message": format!("该测试平台不支持{}发布", input.content_kind.label())
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
            if profile == RunnerProfile::PublishTimeout {
                thread::sleep(Duration::from_secs(30));
            }
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
            if !test_publish_supported(manifest_id, input.content_kind) {
                return (
                    false,
                    Value::Null,
                    json!({
                      "code": "CAPABILITY_UNSUPPORTED",
                      "message": format!("该测试平台不支持{}发布", input.content_kind.label())
                    }),
                );
            }
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
            let submitted_media = input
                .media
                .iter()
                .map(|media| json!({
                  "assetId": media.asset_id,
                  "kind": media.kind,
                  "order": media.order,
                  "role": media.role
                }))
                .collect::<Vec<_>>();
            let submitted_cover = input.cover.as_ref().map(|cover| json!({
              "assetId": cover.asset_id,
              "kind": cover.kind,
              "order": cover.order,
              "role": cover.role
            }));
            (
                true,
                json!({
                  "externalId": format!("test-{}", uuid::Uuid::new_v4()),
                  "contentKind": input.content_kind.as_str(),
                  "submittedMedia": submitted_media,
                  "submittedCover": submitted_cover,
                  "simulated": true,
                  "idempotencyObserved": true
                }),
                Value::Null,
            )
        }
        "syncWorks" => {
            let account_ref = params
                .get("accountRef")
                .and_then(Value::as_str)
                .unwrap_or("fixture-sync-account");
            if account_ref.contains("expired") || account_ref.contains("reauth") {
                return (
                    false,
                    Value::Null,
                    json!({
                      "code": "AUTH_EXPIRED",
                      "message": "测试平台会话已过期，请重新授权后同步"
                    }),
                );
            }
            let as_of = params
                .get("asOf")
                .and_then(Value::as_str)
                .unwrap_or("2026-11-25T09:30:00Z");
            let safe_account = account_ref
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '-' {
                        character
                    } else {
                        '-'
                    }
                })
                .collect::<String>();
            let works = vec![
                json!({
                  "externalId": format!("{safe_account}-video-001"),
                  "title": "测试视频作品",
                  "contentType": "video",
                  "contentKind": "video",
                  "url": "https://example.invalid/video/test-video-001",
                  "publishedAt": "2026-11-20T08:00:00Z",
                  "capturedAt": as_of,
                  "metrics": [
                    { "name": "视频播放量", "value": 4200, "unit": "次", "definition": "平台后台视频播放次数" },
                    { "name": "点赞数", "value": 320, "unit": "次", "definition": "平台后台点赞次数" }
                  ]
                }),
                json!({
                  "externalId": format!("{safe_account}-image-text-001"),
                  "title": "测试图文作品",
                  "contentType": "笔记",
                  "contentKind": "imageText",
                  "url": "https://example.invalid/note/test-image-text-001",
                  "publishedAt": "2026-11-21T08:00:00Z",
                  "capturedAt": as_of,
                  "metrics": [
                    { "name": "曝光量", "value": 6800, "unit": "次", "definition": "平台后台曝光次数" },
                    { "name": "收藏数", "value": 180, "unit": "次", "definition": "平台后台收藏次数" }
                  ]
                }),
                json!({
                  "externalId": format!("{safe_account}-image-article-001"),
                  "title": "测试图文文章",
                  "contentType": "图文文章",
                  "contentKind": "imageArticle",
                  "url": "https://example.invalid/article/test-image-article-001",
                  "publishedAt": "2026-11-22T08:00:00Z",
                  "capturedAt": as_of,
                  "metrics": [
                    { "name": "图文文章阅读量", "value": 3100, "unit": "次", "definition": "平台后台图文文章阅读次数" }
                  ]
                }),
                json!({
                  "externalId": format!("{safe_account}-article-001"),
                  "title": "测试纯文章",
                  "contentType": "文章",
                  "contentKind": "article",
                  "url": "https://example.invalid/article/test-article-001",
                  "publishedAt": "2026-11-23T08:00:00Z",
                  "capturedAt": as_of,
                  "metrics": [
                    { "name": "文章阅读量", "value": 5200, "unit": "次", "definition": "平台后台文章阅读次数" }
                  ]
                })
            ];
            (
                true,
                json!({
                  "accountRef": account_ref,
                  "works": works,
                  "partialFailure": false
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

fn test_capabilities(manifest_id: &str) -> Value {
    if manifest_id == "creatorrelay.official.video-adapter" {
        return json!([
          { "kind": "video", "read": "supported", "publish": "supported", "reason": "视频发布往返可验证" },
          { "kind": "imageText", "read": "supported", "publish": "unsupported", "reason": "该适配器只验证视频形态" },
          { "kind": "imageArticle", "read": "supported", "publish": "restricted", "reason": "图文文章需人工审核" },
          { "kind": "article", "read": "supported", "publish": "unsupported", "reason": "该适配器只验证视频形态" }
        ]);
    }
    if manifest_id == "creatorrelay.official.sync-adapter" {
        return json!([
          { "kind": "video", "read": "supported", "publish": "unsupported", "reason": "可读取视频作品，但测试平台不支持视频发布" },
          { "kind": "imageText", "read": "supported", "publish": "supported", "reason": "图文发布可验证" },
          { "kind": "imageArticle", "read": "supported", "publish": "restricted", "reason": "图文文章仍需人工审核" },
          { "kind": "article", "read": "supported", "publish": "supported", "reason": "纯文章发布可验证" }
        ]);
    }
    if manifest_id == "creatorrelay.official.publish-timeout-adapter" {
        return json!([
          { "kind": "video", "read": "supported", "publish": "unsupported" },
          { "kind": "imageText", "read": "supported", "publish": "supported", "reason": "用于验证发布超时结果不确定" },
          { "kind": "imageArticle", "read": "supported", "publish": "unsupported" },
          { "kind": "article", "read": "supported", "publish": "supported" }
        ]);
    }
    json!([
      { "kind": "video", "read": "unsupported", "publish": "unsupported", "reason": "测试适配器不模拟视频发布" },
      { "kind": "imageText", "read": "supported", "publish": "supported", "reason": "图文发布可验证" },
      { "kind": "imageArticle", "read": "supported", "publish": "restricted", "reason": "图文文章仍需人工审核" },
      { "kind": "article", "read": "supported", "publish": "supported", "reason": "纯文章发布可验证" }
    ])
}

fn test_publish_supported(manifest_id: &str, kind: ContentKind) -> bool {
    match manifest_id {
        "creatorrelay.official.video-adapter" => kind == ContentKind::Video,
        "creatorrelay.official.sync-adapter" | "creatorrelay.official.test-adapter" => {
            matches!(kind, ContentKind::ImageText | ContentKind::Article)
        }
        "creatorrelay.official.fault-adapter"
        | "creatorrelay.official.publish-timeout-adapter" => {
            matches!(kind, ContentKind::ImageText | ContentKind::Article)
        }
        _ => false,
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
