use creatorrelay_lib::{
    ai::{self, AiProviderInput},
    content::{self, ContentAsset, ContentKind, ContentSaveInput},
    database, mcp, operations,
    plugins::{self, PluginPermissionDecision, PluginRuntime, PluginStatus},
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::Mutex,
    thread,
    time::Duration,
};

struct Fixture {
    _temp: tempfile::TempDir,
    conn: Connection,
    runtime: PluginRuntime,
    host: mcp::McpHost,
    workspace: PathBuf,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("creatorrelay.sqlite3");
    let mut conn = Connection::open(&database_path).unwrap();
    database::migrate(&mut conn).unwrap();
    let workspace = temp.path().join("workspace");
    for directory in [
        "articles",
        "contents",
        "platform-drafts",
        "assets",
        "exports",
    ] {
        fs::create_dir_all(workspace.join(directory)).unwrap();
    }
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
    let host = mcp::McpHost::new(executable);
    host.start(None).unwrap();
    Fixture {
        _temp: temp,
        conn,
        runtime,
        host,
        workspace,
    }
}

fn call(
    fixture: &mut Fixture,
    tool_name: &str,
    arguments: Value,
    token: Option<String>,
) -> mcp::McpCallResult {
    mcp::call_tool(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &fixture.host,
        mcp::McpCallRequest {
            call_id: None,
            tool_name: tool_name.to_string(),
            arguments,
            thread_id: None,
            authorization_token: token,
            timeout_ms: None,
            context_scope: None,
        },
    )
    .unwrap()
}

fn authorized_call(fixture: &mut Fixture, tool_name: &str, arguments: Value) -> mcp::McpCallResult {
    let preview = mcp::preview_tool_call(
        &fixture.conn,
        &fixture.workspace,
        mcp::McpPreviewRequest {
            tool_name: tool_name.to_string(),
            arguments: arguments.clone(),
            thread_id: None,
        },
    )
    .unwrap();
    assert!(
        preview.field_errors.is_empty(),
        "{:?}",
        preview.field_errors
    );
    call(fixture, tool_name, arguments, preview.authorization_token)
}

fn source_with_asset(
    fixture: &Fixture,
    kind: ContentKind,
    title: &str,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> (String, ContentAsset) {
    let package = content::create_content_package(&fixture.workspace, kind, title).unwrap();
    let asset = content::import_content_asset(
        &fixture.workspace,
        &package.manifest.id,
        "media",
        file_name,
        mime_type,
        bytes,
    )
    .unwrap();
    content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: title.to_string(),
            summary: String::new(),
            tags: Vec::new(),
            cover: None,
            media: vec![asset.clone()],
            body: if kind.requires_markdown() {
                format!("# {title}\n\n正文")
            } else {
                String::new()
            },
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    (package.manifest.id, asset)
}

fn media_ref(source_content_id: &str, asset: &ContentAsset) -> Value {
    json!({
        "sourceContentId": source_content_id,
        "assetId": asset.id,
        "order": asset.order,
        "caption": "工具创建素材"
    })
}

fn cover_ref(source_content_id: &str, asset: &ContentAsset) -> Value {
    json!({
        "sourceContentId": source_content_id,
        "assetId": asset.id
    })
}

fn enable_adapter(fixture: &mut Fixture, plugin_id: &str) {
    plugins::install_official_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    let summary = plugins::list_plugins(&fixture.conn, &fixture.runtime)
        .unwrap()
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .unwrap();
    let decisions = summary
        .permissions
        .iter()
        .map(|permission| PluginPermissionDecision {
            id: permission.id.clone(),
            granted: permission.required,
        })
        .collect::<Vec<_>>();
    plugins::set_plugin_permissions(&mut fixture.conn, &fixture.runtime, plugin_id, &decisions)
        .unwrap();
    let enabled = plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    assert_eq!(enabled.status, PluginStatus::Enabled, "{plugin_id}");
}

fn insert_active_account(fixture: &mut Fixture, id: &str, platform_id: &str, alias: &str) {
    fixture
        .conn
        .execute(
            "INSERT INTO platform_accounts(
                id, platform_id, alias, nickname, uid, status, enabled, is_default,
                profile_id, credential_ref, last_checked_at, last_error, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?3, 'fixture-uid', 'ACTIVE', 1, 0, ?4, ?5,
                      '2026-11-25T08:00:00Z', NULL, '2026-11-25T08:00:00Z',
                      '2026-11-25T08:00:00Z')",
            rusqlite::params![
                id,
                platform_id,
                alias,
                format!("profile-{id}"),
                format!("credential-{id}")
            ],
        )
        .unwrap();
}

fn create_image_text_with_cover(fixture: &Fixture) -> String {
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::ImageText, "批量图文")
            .unwrap();
    let media = content::import_content_asset(
        &fixture.workspace,
        &package.manifest.id,
        "media",
        "batch-image.png",
        "image/png",
        b"\x89PNG\r\n\x1a\nbatch-image",
    )
    .unwrap();
    let cover = content::import_content_asset(
        &fixture.workspace,
        &package.manifest.id,
        "cover",
        "batch-cover.png",
        "image/png",
        b"\x89PNG\r\n\x1a\nbatch-cover",
    )
    .unwrap();
    content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: "批量图文".to_string(),
            summary: "批量发布图文".to_string(),
            tags: Vec::new(),
            cover: Some(cover),
            media: vec![media],
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    package.manifest.id
}

#[test]
fn m7_host_lifecycle_discovery_schema_and_crash_restart_are_observable() {
    let mut fixture = fixture();
    let status = fixture.host.status().unwrap();
    assert_eq!(status.state, "healthy");
    assert_eq!(status.server_id, "creatorrelay.test-mcp");
    assert_eq!(status.protocol_version, "2024-11-05");

    let tools = fixture.host.list_tools().unwrap();
    let read_tool = tools
        .iter()
        .find(|tool| tool.name == "account_get_info")
        .unwrap();
    assert_eq!(read_tool.permission, mcp::ToolPermission::Read);
    assert_eq!(read_tool.schema_version, "1.0");
    assert!(read_tool.input_schema.get("properties").is_some());

    let invalid = call(
        &mut fixture,
        "account_get_info",
        json!({ "unexpected": true }),
        None,
    );
    assert_eq!(invalid.status, "failed");
    assert_eq!(
        invalid.error.as_ref().map(|error| error.code.as_str()),
        Some("INVALID_ARGUMENT")
    );
    assert!(
        invalid
            .error
            .unwrap()
            .field_errors
            .iter()
            .any(|error| error.path == "unexpected")
    );

    let echo = call(
        &mut fixture,
        "mcp_test_echo",
        json!({ "message": "MCP ready" }),
        None,
    );
    assert_eq!(echo.status, "success");
    assert!(echo.result.unwrap().to_string().contains("MCP ready"));

    let pid = fixture.host.status().unwrap().pid.unwrap();
    std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .unwrap();
    thread::sleep(Duration::from_millis(150));
    let restarted = fixture.host.health_check().unwrap();
    assert_eq!(restarted.state, "healthy");
    assert_eq!(restarted.restart_count, 1);
    assert!(restarted.pid.is_some());
    assert!(fixture.host.list_tools().unwrap().len() >= 10);
    let stopped = fixture.host.stop().unwrap();
    assert_eq!(stopped.state, "stopped");
    assert!(stopped.pid.is_none());
}

#[test]
fn m7_core_boundary_and_security_evidence_do_not_expose_database_or_credentials() {
    let mut fixture = fixture();
    fixture
        .conn
        .execute(
            "INSERT INTO platform_accounts(
                id, platform_id, alias, nickname, uid, status, enabled, is_default,
                profile_id, credential_ref, last_checked_at, last_error, created_at, updated_at
             ) VALUES('account-juejin', 'juejin', '测试掘金号', '测试昵称', 'uid-1',
                      'ACTIVE', 1, 1, 'profile-secret-id', 'credential-secret-ref',
                      '2026-11-25T08:00:00Z', NULL, '2026-11-25T08:00:00Z',
                      '2026-11-25T08:00:00Z')",
            [],
        )
        .unwrap();
    let result = authorized_call(&mut fixture, "account_get_info", json!({}));
    assert_eq!(result.status, "success");
    let rendered = result.result.unwrap().to_string();
    assert!(rendered.contains("测试掘金号"));
    assert!(!rendered.contains("credential-secret-ref"));
    assert!(!rendered.contains("profile-secret-id"));

    let evidence = mcp::security_evidence();
    assert!(evidence.process_isolated);
    assert!(!evidence.database_path_passed);
    assert!(!evidence.credential_reference_passed);
    assert!(!evidence.direct_database_api_available);
    assert!(!evidence.direct_credential_api_available);
    assert!(evidence.business_tools_forward_to_core);
    assert!(
        evidence
            .forbidden_argument_fields
            .contains(&"filePath".to_string())
    );
}

#[test]
fn m7_four_content_kinds_create_update_and_restore_with_field_level_rejection() {
    let mut fixture = fixture();
    let (video_source, video_asset) = source_with_asset(
        &fixture,
        ContentKind::Video,
        "视频来源",
        "clip.mp4",
        "video/mp4",
        b"\x00\x00\x00\x18ftypmp42fixture-video",
    );
    let (image_source, image_asset) = source_with_asset(
        &fixture,
        ContentKind::ImageText,
        "图片来源",
        "image.png",
        "image/png",
        b"\x89PNG\r\n\x1a\nfixture-image",
    );

    let cases = vec![
        (
            "video",
            json!({
                "contentKind": "video",
                "title": "工具视频草稿",
                "summary": "视频简介",
                "media": [media_ref(&video_source, &video_asset)],
                "cover": cover_ref(&image_source, &image_asset)
            }),
        ),
        (
            "imageText",
            json!({
                "contentKind": "imageText",
                "title": "工具图文草稿",
                "summary": "图集说明",
                "media": [media_ref(&image_source, &image_asset)],
                "cover": cover_ref(&image_source, &image_asset)
            }),
        ),
        (
            "imageArticle",
            json!({
                "contentKind": "imageArticle",
                "title": "工具图文文章草稿",
                "body": "# 图文文章\n\n正文与素材位置",
                "media": [media_ref(&image_source, &image_asset)],
                "cover": cover_ref(&image_source, &image_asset)
            }),
        ),
        (
            "article",
            json!({
                "contentKind": "article",
                "title": "工具纯文章草稿",
                "body": "# 纯文章\n\n工具写入正文"
            }),
        ),
    ];

    let mut created = Vec::new();
    for (expected_kind, arguments) in cases {
        let result = authorized_call(&mut fixture, "studio_create_content", arguments);
        assert_eq!(result.status, "success", "{:?}", result.error);
        let value = result.result.unwrap();
        assert_eq!(value["contentKind"], expected_kind);
        let id = value["contentId"].as_str().unwrap().to_string();
        assert_eq!(
            content::read_content_package(&fixture.workspace, &id)
                .unwrap()
                .manifest
                .kind
                .as_str(),
            expected_kind
        );
        created.push(id);
    }

    for id in &created {
        let updated = authorized_call(
            &mut fixture,
            "studio_update_content",
            json!({ "contentId": id, "title": format!("更新-{id}") }),
        );
        assert_eq!(updated.status, "success", "{:?}", updated.error);
        assert!(
            content::read_content_package(&fixture.workspace, id)
                .unwrap()
                .manifest
                .title
                .starts_with("更新-")
        );
    }

    let before = content::list_content_packages(&fixture.workspace)
        .unwrap()
        .len();
    let missing_video = call(
        &mut fixture,
        "studio_create_content",
        json!({ "contentKind": "video", "title": "缺视频" }),
        None,
    );
    assert_eq!(missing_video.status, "failed");
    assert!(
        missing_video
            .error
            .unwrap()
            .field_errors
            .iter()
            .any(|error| error.path == "media")
    );
    let empty_images = call(
        &mut fixture,
        "studio_create_content",
        json!({ "contentKind": "imageText", "title": "空图文", "media": [] }),
        None,
    );
    assert!(
        empty_images
            .error
            .unwrap()
            .field_errors
            .iter()
            .any(|error| error.path == "media")
    );
    let missing_body = call(
        &mut fixture,
        "studio_create_content",
        json!({ "contentKind": "article", "title": "缺正文" }),
        None,
    );
    assert!(
        missing_body
            .error
            .unwrap()
            .field_errors
            .iter()
            .any(|error| error.path == "body")
    );
    assert_eq!(
        content::list_content_packages(&fixture.workspace)
            .unwrap()
            .len(),
        before
    );
    for id in created {
        assert_eq!(
            content::read_content_package(&fixture.workspace, &id)
                .unwrap()
                .manifest
                .id,
            id
        );
    }
}

#[test]
fn m7_write_confirmation_is_one_time_and_rejection_has_no_side_effect() {
    let mut fixture = fixture();
    let arguments = json!({ "contentKind": "article", "title": "授权草稿", "body": "# 内容" });
    let before = content::list_content_packages(&fixture.workspace)
        .unwrap()
        .len();
    let rejected = call(
        &mut fixture,
        "studio_create_content",
        arguments.clone(),
        None,
    );
    assert_eq!(rejected.status, "failed");
    assert_eq!(
        rejected.error.as_ref().map(|error| error.code.as_str()),
        Some("AUTHORIZATION_REQUIRED")
    );
    assert_eq!(
        content::list_content_packages(&fixture.workspace)
            .unwrap()
            .len(),
        before
    );

    let preview = mcp::preview_tool_call(
        &fixture.conn,
        &fixture.workspace,
        mcp::McpPreviewRequest {
            tool_name: "studio_create_content".to_string(),
            arguments: arguments.clone(),
            thread_id: None,
        },
    )
    .unwrap();
    assert!(preview.requires_confirmation);
    let token = preview.authorization_token.unwrap();
    let success = call(
        &mut fixture,
        "studio_create_content",
        arguments.clone(),
        Some(token.clone()),
    );
    assert_eq!(success.status, "success");
    let replay = call(
        &mut fixture,
        "studio_create_content",
        arguments,
        Some(token),
    );
    assert_eq!(replay.status, "failed");
    assert_eq!(
        replay.error.as_ref().map(|error| error.code.as_str()),
        Some("AUTHORIZATION_USED")
    );
}

#[test]
fn m7_timeout_cancel_and_audit_do_not_repeat_or_leak_secrets() {
    let mut fixture = fixture();
    let timed_out = mcp::call_tool(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &fixture.host,
        mcp::McpCallRequest {
            call_id: Some("timeout-call".to_string()),
            tool_name: "mcp_test_hang".to_string(),
            arguments: json!({ "milliseconds": 5000 }),
            thread_id: None,
            authorization_token: None,
            timeout_ms: Some(250),
            context_scope: None,
        },
    )
    .unwrap();
    assert_eq!(timed_out.status, "timedOut");
    assert_eq!(timed_out.error.unwrap().code, "TOOL_TIMEOUT");

    let echo = call(
        &mut fixture,
        "mcp_test_echo",
        json!({ "message": "tool still available" }),
        None,
    );
    assert_eq!(echo.status, "success");

    let thread = mcp::create_assistant_thread(&fixture.conn, None, None).unwrap();
    let hang_request = mcp::McpCallRequest {
        call_id: Some("cancel-call".to_string()),
        tool_name: "mcp_test_hang".to_string(),
        arguments: json!({ "milliseconds": 5000 }),
        thread_id: Some(thread.id.clone()),
        authorization_token: None,
        timeout_ms: Some(10000),
        context_scope: None,
    };
    let host = fixture.host.clone();
    let workspace = fixture.workspace.clone();
    let mut conn = Connection::open(fixture._temp.path().join("creatorrelay.sqlite3")).unwrap();
    let runtime = PluginRuntime::new(
        fixture._temp.path().join("plugins-cancel"),
        fixture._temp.path().to_path_buf(),
        fixture._temp.path().join("creatorrelay.sqlite3"),
        PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay")),
    )
    .unwrap();
    let handle = thread::spawn(move || {
        mcp::call_tool(&mut conn, &runtime, &workspace, &host, hang_request).unwrap()
    });
    thread::sleep(Duration::from_millis(150));
    fixture.host.cancel("cancel-call").unwrap();
    let cancelled = handle.join().unwrap();
    assert_eq!(cancelled.status, "cancelled");
    assert_eq!(cancelled.error.unwrap().code, "TOOL_CANCELLED");
    let history = mcp::get_assistant_thread(&fixture.conn, &thread.id).unwrap();
    let tool_message = history
        .messages
        .iter()
        .find(|message| message.tool_call.is_some())
        .unwrap();
    assert_eq!(tool_message.provider_name.as_deref(), Some("本地编排器"));
    assert_eq!(tool_message.model.as_deref(), Some("deterministic-v1"));
    assert_eq!(
        tool_message
            .tool_call
            .as_ref()
            .map(|call| call.status.as_str()),
        Some("cancelled")
    );

    let audits = mcp::list_audit(&fixture.conn, 20).unwrap();
    assert!(audits.iter().any(|audit| audit.status == "timedOut"));
    assert!(audits.iter().any(|audit| audit.status == "cancelled"));
    assert!(
        audits
            .iter()
            .all(|audit| !audit.parameter_summary.contains("secret"))
    );
}

#[derive(Default)]
struct MemoryVault {
    secrets: Mutex<std::collections::HashMap<String, String>>,
}

impl mcp::CredentialVault for MemoryVault {
    fn set(&self, reference: &str, secret: &str) -> Result<(), String> {
        self.secrets
            .lock()
            .unwrap()
            .insert(reference.to_string(), secret.to_string());
        Ok(())
    }

    fn get(&self, reference: &str) -> Result<Option<String>, String> {
        Ok(self.secrets.lock().unwrap().get(reference).cloned())
    }

    fn delete(&self, reference: &str) -> Result<(), String> {
        self.secrets.lock().unwrap().remove(reference);
        Ok(())
    }
}

fn fixture_server() -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut collected = Vec::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let count = stream.read(&mut buffer).unwrap();
            if count == 0 {
                break;
            }
            collected.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = collected
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
            {
                let headers =
                    String::from_utf8_lossy(&collected[..header_end + 4]).to_ascii_lowercase();
                let content_length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if collected.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        let request = String::from_utf8_lossy(&collected).to_string();
        let body = r#"{"choices":[{"message":{"content":"已按选择范围处理"}}]}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
        request
    });
    (format!("http://{address}"), handle)
}

#[test]
fn m7_assistant_history_privacy_and_model_payload_exclude_unselected_body() {
    let fixture = fixture();
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::Article, "隐私测试标题")
            .unwrap();
    let package = content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: "隐私测试标题".to_string(),
            summary: "可发送摘要".to_string(),
            tags: vec!["测试".to_string()],
            cover: None,
            media: Vec::new(),
            body: "# 不应发送的正文\n\nSENSITIVE_BODY api_key=model-secret-value".to_string(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();

    let preview = mcp::preview_assistant_context(
        &fixture.conn,
        &fixture.workspace,
        None,
        Some(&package.manifest.id),
        &mcp::ModelContextScope {
            include_title: true,
            include_summary: true,
            include_body: false,
            include_media_metadata: true,
        },
    )
    .unwrap();
    assert!(preview.context.contains("隐私测试标题"));
    assert!(!preview.context.contains("SENSITIVE_BODY"));
    assert!(!preview.context.contains("model-secret-value"));

    let thread = mcp::create_assistant_thread(&fixture.conn, None, None).unwrap();
    let plan =
        mcp::plan_assistant_message(&fixture.conn, "查询账号状态", Some(&thread.id)).unwrap();
    assert_eq!(
        plan.assistant_message
            .tool_call
            .as_ref()
            .map(|call| call.tool_name.as_str()),
        Some("account_get_info")
    );
    let reopened = mcp::get_assistant_thread(&fixture.conn, &thread.id).unwrap();
    assert!(reopened.messages.len() >= 2);
    assert!(
        reopened
            .messages
            .iter()
            .any(|message| message.role == "user")
    );
    let proposal = reopened
        .messages
        .iter()
        .find(|message| message.tool_call.is_some())
        .unwrap();
    assert_eq!(
        proposal
            .tool_call
            .as_ref()
            .and_then(|call| call.arguments.as_ref()),
        Some(&json!({}))
    );

    let (base_url, server) = fixture_server();
    let vault = MemoryVault::default();
    let provider = ai::save_provider_with_vault(
        &fixture.conn,
        &AiProviderInput {
            id: None,
            name: "测试云端模型".to_string(),
            base_url,
            model: "fixture-model".to_string(),
            api_style: Some("openai-compatible".to_string()),
            api_key: Some("api-key-must-not-leak".to_string()),
            is_default: Some(true),
        },
        &vault,
    )
    .unwrap();
    let generated = mcp::generate_assistant_reply(
        &fixture.conn,
        &fixture.workspace,
        mcp::AssistantGenerateRequest {
            thread_id: thread.id,
            prompt: Some("总结选定内容".to_string()),
            provider_id: Some(provider.id.clone()),
            content_id: Some(package.manifest.id.clone()),
            scope: mcp::ModelContextScope {
                include_title: true,
                include_summary: true,
                include_body: false,
                include_media_metadata: false,
            },
            instruction: None,
        },
        &vault,
    )
    .unwrap();
    assert_eq!(
        generated.assistant_message.result_status.as_deref(),
        Some("success")
    );
    let request = server.join().unwrap();
    let request_body = request
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or_default();
    assert!(
        request
            .to_ascii_lowercase()
            .contains("authorization: bearer api-key-must-not-leak")
    );
    assert!(request_body.contains("隐私测试标题"));
    assert!(request_body.contains("可发送摘要"));
    assert!(!request_body.contains("SENSITIVE_BODY"));
    assert!(!request_body.contains("model-secret-value"));
    assert!(!request_body.contains("api-key-must-not-leak"));
    assert!(!ai::audit_database_for_secret(&fixture.conn, "api-key-must-not-leak").unwrap());
    assert!(
        mcp::list_audit(&fixture.conn, 50)
            .unwrap()
            .iter()
            .all(|audit| !audit.parameter_summary.contains("api-key-must-not-leak"))
    );
}

#[test]
fn m7_publish_preview_and_authorization_preserve_task_on_rejection() {
    let mut fixture = fixture();
    fixture
        .conn
        .execute(
            "INSERT INTO platform_accounts(
                id, platform_id, alias, nickname, uid, status, enabled, is_default,
                profile_id, credential_ref, last_checked_at, last_error, created_at, updated_at
             ) VALUES('account-juejin', 'juejin', '测试掘金号', '', '', 'ACTIVE', 1, 1,
                      'profile-juejin', 'credential-juejin', NULL, NULL,
                      '2026-11-25T08:00:00Z', '2026-11-25T08:00:00Z')",
            [],
        )
        .unwrap();
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::Article, "批量确认文章")
            .unwrap();
    let package = content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: "批量确认文章".to_string(),
            summary: String::new(),
            tags: Vec::new(),
            cover: None,
            media: Vec::new(),
            body: "# 最终正文\n\n确认后发布".to_string(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    let created = operations::create_publish_tasks(
        &mut fixture.conn,
        &fixture.workspace,
        &operations::CreatePublishTasksInput {
            source_type: operations::PublishSourceType::Content,
            source_id: package.manifest.id,
            targets: vec![operations::PublishTargetInput {
                platform_id: "juejin".to_string(),
                platform_name: "掘金".to_string(),
                account_id: "account-juejin".to_string(),
                account_alias: "测试掘金号".to_string(),
                adapter_id: None,
            }],
            scheduled_at: None,
        },
    )
    .unwrap();
    let task_id = created.tasks[0].id.clone();
    let before = operations::publish_task(&fixture.conn, &task_id).unwrap();
    let arguments = json!({ "taskIds": [task_id.clone()] });
    let preview = mcp::preview_tool_call(
        &fixture.conn,
        &fixture.workspace,
        mcp::McpPreviewRequest {
            tool_name: "publishing_publish_task".to_string(),
            arguments: arguments.clone(),
            thread_id: None,
        },
    )
    .unwrap();
    assert_eq!(preview.permission, mcp::ToolPermission::ExternalAction);
    assert!(preview.requires_confirmation);
    assert!(
        preview
            .effects
            .iter()
            .any(|effect| effect.contains("外部平台"))
    );
    assert_eq!(preview.details["targets"][0]["accountAlias"], "测试掘金号");
    assert!(
        preview.details["targets"][0]["bodyPreview"]
            .as_str()
            .unwrap()
            .contains("最终正文")
    );

    let rejected = call(&mut fixture, "publishing_publish_task", arguments, None);
    assert_eq!(
        rejected.error.as_ref().map(|error| error.code.as_str()),
        Some("AUTHORIZATION_REQUIRED")
    );
    let after = operations::publish_task(&fixture.conn, &task_id).unwrap();
    assert_eq!(after.status, before.status);
    assert_eq!(after.updated_at, before.updated_at);
    assert_eq!(after.attempt_count, before.attempt_count);
}

#[test]
fn m7_batch_publish_previews_every_target_and_reports_independent_partial_result() {
    let mut fixture = fixture();
    enable_adapter(&mut fixture, "creatorrelay.official.test-adapter");
    enable_adapter(
        &mut fixture,
        "creatorrelay.official.publish-timeout-adapter",
    );
    insert_active_account(
        &mut fixture,
        "adapter-success-account",
        "juejin",
        "成功账号",
    );
    insert_active_account(&mut fixture, "adapter-timeout-account", "csdn", "超时账号");

    let article =
        content::create_content_package(&fixture.workspace, ContentKind::Article, "批量文章")
            .unwrap();
    let article = content::save_content_package(
        &fixture.workspace,
        &article.manifest.id,
        ContentSaveInput {
            title: "批量文章".to_string(),
            summary: String::new(),
            tags: Vec::new(),
            cover: None,
            media: Vec::new(),
            body: "# 批量文章\n\n成功目标正文".to_string(),
            base_hash: Some(article.content_hash),
        },
    )
    .unwrap();
    let image_text_id = create_image_text_with_cover(&fixture);

    let create_task = |fixture: &mut Fixture,
                       source_id: &str,
                       platform_id: &str,
                       platform_name: &str,
                       account_id: &str,
                       account_alias: &str,
                       adapter_id: &str| {
        operations::create_publish_tasks(
            &mut fixture.conn,
            &fixture.workspace,
            &operations::CreatePublishTasksInput {
                source_type: operations::PublishSourceType::Content,
                source_id: source_id.to_string(),
                targets: vec![operations::PublishTargetInput {
                    platform_id: platform_id.to_string(),
                    platform_name: platform_name.to_string(),
                    account_id: account_id.to_string(),
                    account_alias: account_alias.to_string(),
                    adapter_id: Some(adapter_id.to_string()),
                }],
                scheduled_at: None,
            },
        )
        .unwrap()
        .tasks
        .remove(0)
        .id
    };
    let success_task = create_task(
        &mut fixture,
        &article.manifest.id,
        "juejin",
        "测试成功平台",
        "adapter-success-account",
        "成功账号",
        "creatorrelay.official.test-adapter",
    );
    let timeout_task = create_task(
        &mut fixture,
        &image_text_id,
        "csdn",
        "测试超时平台",
        "adapter-timeout-account",
        "超时账号",
        "creatorrelay.official.publish-timeout-adapter",
    );
    let arguments = json!({ "taskIds": [success_task, timeout_task] });
    let preview = mcp::preview_tool_call(
        &fixture.conn,
        &fixture.workspace,
        mcp::McpPreviewRequest {
            tool_name: "publishing_publish_task".to_string(),
            arguments: arguments.clone(),
            thread_id: None,
        },
    )
    .unwrap();
    assert!(
        preview.field_errors.is_empty(),
        "{:?}",
        preview.field_errors
    );
    let targets = preview.details["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 2);
    assert!(targets.iter().any(|target| {
        target["accountAlias"] == "成功账号"
            && target["contentKind"] == "article"
            && target["bodyPreview"]
                .as_str()
                .is_some_and(|body| body.contains("成功目标正文"))
    }));
    assert!(targets.iter().any(|target| {
        target["accountAlias"] == "超时账号"
            && target["contentKind"] == "imageText"
            && target["media"]
                .as_array()
                .is_some_and(|media| media.len() == 1)
    }));

    let result = call(
        &mut fixture,
        "publishing_publish_task",
        arguments,
        preview.authorization_token,
    );
    assert_eq!(result.status, "partial", "{result:?}");
    let payload = result.result.unwrap();
    assert_eq!(payload["successCount"], 1);
    assert_eq!(payload["failureCount"], 1);
    let results = payload["targets"].as_array().unwrap();
    assert!(
        results
            .iter()
            .any(|target| { target["status"] == "SUCCESS" && target["published"] == true })
    );
    assert!(results.iter().any(|target| {
        target["published"] == false
            && target["status"] != "SUCCESS"
            && target["errorCode"].as_str().is_some()
    }));

    let first = operations::publish_task(&fixture.conn, &success_task).unwrap();
    let second = operations::publish_task(&fixture.conn, &timeout_task).unwrap();
    assert_eq!(first.status, operations::PublishTaskStatus::Success);
    assert_ne!(second.status, operations::PublishTaskStatus::Success);
    assert_ne!(second.status, operations::PublishTaskStatus::Cancelled);
}
