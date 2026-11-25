use creatorrelay_lib::{
    content::{self, ContentKind, ContentSaveInput},
    database, ecosystem, mcp,
    operations::{
        self, CreatePublishTasksInput, ExternalWorkFilter, PublishExecutionMode, PublishSourceType,
        PublishTargetInput, SyncPlatformWorksInput,
    },
    plugins,
};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::path::PathBuf;

struct Fixture {
    _temp: tempfile::TempDir,
    conn: Connection,
    runtime: plugins::PluginRuntime,
    workspace: PathBuf,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("creatorrelay.sqlite3");
    let mut conn = Connection::open(&database_path).unwrap();
    database::migrate(&mut conn).unwrap();
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).unwrap();
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('workspace_path', ?1)",
        [workspace.to_string_lossy().to_string()],
    )
    .unwrap();
    let runtime = plugins::PluginRuntime::new(
        temp.path().join("plugins"),
        temp.path().to_path_buf(),
        database_path,
        PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay")),
    )
    .unwrap();
    Fixture {
        _temp: temp,
        conn,
        runtime,
        workspace,
    }
}

fn grant_all(fixture: &mut Fixture, plugin_id: &str, overrides: &[(&str, bool)]) {
    let plugin = plugins::list_plugins(&fixture.conn, &fixture.runtime)
        .unwrap()
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .unwrap();
    let decisions = plugin
        .permissions
        .iter()
        .map(|permission| plugins::PluginPermissionDecision {
            id: permission.id.clone(),
            granted: overrides
                .iter()
                .find(|(id, _)| *id == permission.id)
                .map(|(_, granted)| *granted)
                .unwrap_or(permission.required),
        })
        .collect::<Vec<_>>();
    plugins::set_plugin_permissions(&mut fixture.conn, &fixture.runtime, plugin_id, &decisions)
        .unwrap();
}

fn insert_active_account(fixture: &Fixture, id: &str, platform_id: &str) {
    let now = "2026-11-25T10:00:00Z";
    fixture
        .conn
        .execute(
            "INSERT INTO platform_accounts(
                id, platform_id, alias, nickname, uid, status, enabled, is_default,
                profile_id, credential_ref, last_checked_at, last_error, created_at, updated_at
             ) VALUES(?1, ?2, ?2, ?2, 'fixture-uid', 'ACTIVE', 1, 0, ?3, ?4, ?5, NULL, ?5, ?5)",
            rusqlite::params![
                id,
                platform_id,
                format!("profile-{id}"),
                format!("credential-{id}"),
                now
            ],
        )
        .unwrap();
}

#[test]
fn m8_platform_sync_preserves_raw_fields_content_mapping_and_metric_provenance() {
    let mut fixture = fixture();
    insert_active_account(&fixture, "account-sync-m8", "juejin");
    let plugin_id = "creatorrelay.official.sync-adapter";
    plugins::install_official_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    grant_all(&mut fixture, plugin_id, &[]);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();

    let result = operations::sync_platform_works(
        &mut fixture.conn,
        &fixture.runtime,
        &SyncPlatformWorksInput {
            account_id: "account-sync-m8".to_string(),
            platform_id: "juejin".to_string(),
            platform_name: "掘金".to_string(),
            account_alias: "M8 同步账号".to_string(),
            adapter_id: plugin_id.to_string(),
            as_of: Some("2026-11-25T09:30:00Z".to_string()),
        },
    )
    .unwrap();
    assert_eq!(result.status, "SUCCESS");
    assert_eq!(result.imported_count, 4);

    let works =
        operations::list_external_works(&fixture.conn, &ExternalWorkFilter::default()).unwrap();
    assert_eq!(works.len(), 4);
    let video = works
        .iter()
        .find(|work| work.content_kind == "video")
        .unwrap();
    assert_eq!(video.original_content_type, "video");
    assert_eq!(video.raw_fields["bvid"], "BV1M8Fixture");
    assert_eq!(
        video.raw_fields["coverUrl"],
        "https://example.invalid/cover/fixture-video.jpg"
    );
    assert!(video.metrics.iter().any(|metric| {
        metric.metric_name == "视频播放量"
            && metric.metric_definition == "平台后台视频播放次数"
            && metric.unit == "次"
            && metric.source == "adapter:creatorrelay.official.sync-adapter"
    }));
    let image_text = works
        .iter()
        .find(|work| work.content_kind == "imageText")
        .unwrap();
    assert_eq!(image_text.original_content_type, "笔记");
    assert_eq!(image_text.raw_fields["imageCount"], 3);
    let image_article = works
        .iter()
        .find(|work| work.content_kind == "imageArticle")
        .unwrap();
    assert_eq!(image_article.original_content_type, "图文文章");
    assert_eq!(image_article.raw_fields["column"], "技术");
    let article = works
        .iter()
        .find(|work| work.content_kind == "article")
        .unwrap();
    assert_eq!(article.original_content_type, "文章");
    assert_eq!(article.raw_fields["draftId"], "fixture-draft-id");
}

#[test]
fn m8_marketplace_and_admission_registry_are_closed_and_traceable() {
    let admissions = ecosystem::platform_admissions().unwrap();
    assert_eq!(admissions.len(), 10);
    for admission in admissions {
        assert_eq!(admission.admission_status, "evaluation_only");
        assert!(!admission.auto_publish_allowed);
        assert_eq!(admission.publish_dispatch, "manual_only");
        assert_eq!(admission.content_kinds.len(), 4);
        assert!(!admission.official_sources.is_empty());
        assert!(
            admission
                .content_kinds
                .iter()
                .all(
                    |capability| capability.read.level != plugins::CapabilityLevel::Supported
                        && capability.publish.level != plugins::CapabilityLevel::Supported
                )
        );
    }

    let policy = ecosystem::marketplace_policy().unwrap();
    assert_eq!(policy.decision, "closed");
    assert_eq!(policy.allowed_distribution, "bundled_official_only");
    assert!(!policy.third_party_install_allowed);
    assert!(!policy.controlled_pilot_allowed);
}

#[test]
fn m8_mcp_returns_platform_and_account_capability_matrix() {
    let mut fixture = fixture();
    insert_active_account(&fixture, "account-mcp-m8", "bilibili");
    let host = mcp::McpHost::new(PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay")));
    host.start(None).unwrap();
    let result = mcp::call_tool(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &host,
        mcp::McpCallRequest {
            call_id: None,
            tool_name: "platform_capabilities_get".to_string(),
            arguments: serde_json::json!({
                "platformId": "bilibili",
                "accountId": "account-mcp-m8"
            }),
            thread_id: None,
            authorization_token: None,
            timeout_ms: None,
            context_scope: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, "success");
    let value = result.result.unwrap();
    assert_eq!(value["platformCount"], 1);
    assert_eq!(value["accountCount"], 1);
    assert_eq!(value["autoPublishDefault"], "manual_only");
    let platform = &value["platforms"][0];
    assert_eq!(platform["platformId"], "bilibili");
    assert_eq!(platform["autoPublishAllowed"], false);
    let account = &value["accounts"][0];
    assert_eq!(account["accountStatus"], "ACTIVE");
    assert!(
        account["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .all(|capability| capability["autoPublishAllowed"] == false)
    );
    host.stop().unwrap();
}

#[test]
fn m8_embedded_adapter_contract_and_payload_regression_passes_together() {
    let report = ecosystem::compatibility_report();
    assert!(report.passed, "{:?}", report.checks);
    assert_eq!(
        report.adapter_count,
        plugins::EMBEDDED_MANIFEST_SOURCES.len()
    );
    assert!(
        report
            .checks
            .iter()
            .any(|check| check.id == "shared_payload_contract" && check.passed)
    );
}

#[test]
fn m8_unverified_platform_forces_manual_publish_even_when_adapter_claims_support() {
    let mut fixture = fixture();
    let workspace = fixture
        .conn
        .query_row(
            "SELECT value FROM app_settings WHERE key='workspace_path'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap();
    let package = content::create_content_package(
        std::path::Path::new(&workspace),
        ContentKind::Video,
        "M8 发布安全",
    )
    .unwrap();
    let video = content::import_content_asset(
        std::path::Path::new(&workspace),
        &package.manifest.id,
        "media",
        "video.mp4",
        "video/mp4",
        b"fixture-video-bytes",
    )
    .unwrap();
    let cover = content::import_content_asset(
        std::path::Path::new(&workspace),
        &package.manifest.id,
        "cover",
        "cover.png",
        "image/png",
        b"fixture-cover-bytes",
    )
    .unwrap();
    content::save_content_package(
        std::path::Path::new(&workspace),
        &package.manifest.id,
        ContentSaveInput {
            title: "M8 发布安全".to_string(),
            summary: "准入未验证时只能人工发布".to_string(),
            tags: vec!["M8".to_string()],
            cover: Some(cover),
            media: vec![video],
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();

    let account_id = uuid::Uuid::new_v4().to_string();
    insert_active_account(&fixture, &account_id, "bilibili");
    let plugin_id = "creatorrelay.official.video-adapter";
    plugins::install_official_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    grant_all(&mut fixture, plugin_id, &[]);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();

    let created = operations::create_publish_tasks(
        &mut fixture.conn,
        std::path::Path::new(&workspace),
        &CreatePublishTasksInput {
            source_type: PublishSourceType::Content,
            source_id: package.manifest.id.clone(),
            targets: vec![PublishTargetInput {
                platform_id: "bilibili".to_string(),
                platform_name: "Bilibili".to_string(),
                account_id: account_id.clone(),
                account_alias: "M8 测试账号".to_string(),
                adapter_id: Some(plugin_id.to_string()),
            }],
            scheduled_at: None,
        },
    )
    .unwrap();
    let preview = operations::prepare_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        std::path::Path::new(&workspace),
        &created.tasks[0].id,
    )
    .unwrap();
    assert_eq!(preview.execution_mode, PublishExecutionMode::Manual);
    assert_ne!(preview.capability, plugins::CapabilityLevel::Supported);
    assert!(
        !preview
            .actions
            .iter()
            .find(|action| action.id == "auto_publish")
            .unwrap()
            .available
    );
    assert!(
        preview
            .capability_reason
            .as_deref()
            .unwrap()
            .contains("人工发布")
    );
}

#[test]
fn m8_plugin_upgrade_requires_review_of_new_permission_and_denial_is_effective() {
    let mut fixture = fixture();
    let plugin_id = "creatorrelay.official.video-adapter";
    plugins::install_official_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    grant_all(&mut fixture, plugin_id, &[]);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    plugins::disable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();

    let raw = plugins::EMBEDDED_MANIFEST_SOURCES
        .iter()
        .find(|raw| raw.contains("\"creatorrelay.official.video-adapter\""))
        .expect("video adapter manifest");
    let mut manifest: Value = serde_json::from_str(raw).unwrap();
    manifest["version"] = Value::String("1.1.0".to_string());
    manifest["permissions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
          "id": "network",
          "required": false,
          "reason": "M8 权限升级：读取发布状态"
        }));
    let updated = plugins::update_plugin_manifest(
        &mut fixture.conn,
        &fixture.runtime,
        &serde_json::to_string(&manifest).unwrap(),
    )
    .unwrap();
    assert_eq!(updated.version, "1.1.0");
    assert_eq!(updated.status, plugins::PluginStatus::ReviewRequired);
    let network = updated
        .permissions
        .iter()
        .find(|permission| permission.id == "network")
        .unwrap();
    assert_eq!(network.granted, None);
    assert!(
        updated
            .permissions
            .iter()
            .find(|permission| permission.id == "account.credentials")
            .unwrap()
            .granted
            == Some(true)
    );
    assert!(
        plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id)
            .unwrap_err()
            .contains("权限审查")
    );

    grant_all(&mut fixture, plugin_id, &[("network", false)]);
    let reviewed = plugins::list_plugins(&fixture.conn, &fixture.runtime)
        .unwrap()
        .into_iter()
        .find(|plugin| plugin.id == plugin_id)
        .unwrap();
    assert_eq!(reviewed.status, plugins::PluginStatus::Ready);
    assert_eq!(
        reviewed
            .permissions
            .iter()
            .find(|permission| permission.id == "network")
            .unwrap()
            .granted,
        Some(false)
    );
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, plugin_id).unwrap();
    let probe = plugins::probe_plugin_capabilities(
        &mut fixture.conn,
        &fixture.runtime,
        plugin_id,
        Some("fixture-video".to_string()),
    )
    .unwrap();
    let network_observation = probe
        .host_access
        .iter()
        .find(|observation| observation.method == "network.request")
        .expect("network host call");
    assert!(!network_observation.allowed);
    assert_eq!(
        network_observation.code.as_deref(),
        Some("PERMISSION_DENIED")
    );
    let events = plugins::list_plugin_events(&fixture.conn, Some(plugin_id), None).unwrap();
    assert!(events.iter().any(|event| event.action == "VERSION_CHANGED"));
}

#[test]
fn m8_platform_mcp_profiles_keep_native_fields_and_manual_safety_boundaries() {
    let profiles = mcp::platform_mcp_catalog();
    assert_eq!(profiles.len(), 10);
    assert!(
        profiles
            .iter()
            .all(|profile| !profile.auto_publish_allowed
                && profile.publish_dispatch == "manual_only")
    );

    let douyin = profiles
        .iter()
        .find(|profile| profile.platform_id == "douyin")
        .unwrap();
    assert_eq!(douyin.server_name, "douyin_mcp");
    assert_eq!(douyin.native_work_id_field, "itemId");
    assert!(douyin.interfaces.iter().any(|interface| {
        interface
            .endpoint
            .as_deref()
            .is_some_and(|endpoint| endpoint.contains("/api/douyin/v1/video/create_video/"))
    }));
    assert!(douyin.field_mappings.iter().any(
        |mapping| mapping.unified_field == "externalId" && mapping.platform_field == "item_id"
    ));

    let xiaohongshu = profiles
        .iter()
        .find(|profile| profile.platform_id == "xiaohongshu")
        .unwrap();
    assert_eq!(xiaohongshu.server_name, "xiaohongshu_mcp");
    assert_eq!(xiaohongshu.native_work_id_field, "noteId");
    assert!(xiaohongshu.interfaces.iter().any(|interface| {
        interface.transport == "manual_web"
            && interface
                .endpoint
                .as_deref()
                .is_some_and(|endpoint| endpoint.contains("creator.xiaohongshu.com/publish"))
    }));
    assert!(
        xiaohongshu
            .interfaces
            .iter()
            .all(|interface| interface.verification != "verified"
                || interface.name == "identity_nav")
    );

    let tools = mcp::tool_catalog();
    let property_names = |name: &str| {
        tools
            .iter()
            .find(|tool| tool.name == name)
            .unwrap()
            .input_schema
            .pointer("/properties")
            .and_then(Value::as_object)
            .unwrap()
            .keys()
            .cloned()
            .collect::<std::collections::HashSet<_>>()
    };
    assert!(property_names("douyin_work_list").contains("itemId"));
    assert!(property_names("xiaohongshu_work_list").contains("noteId"));
    assert!(property_names("youtube_work_list").contains("videoId"));
    assert!(property_names("wechat_work_list").contains("msgId"));
    assert!(property_names("bilibili_work_list").contains("bvid"));
    assert!(property_names("weibo_work_list").contains("statusId"));
}

#[test]
fn m8_platform_publish_validate_is_read_only_and_returns_field_mapping() {
    let mut fixture = fixture();
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::Video, "抖音待校验视频")
            .unwrap();
    let host = mcp::McpHost::new(PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay")));
    host.start(None).unwrap();
    let result = mcp::call_tool(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &host,
        mcp::McpCallRequest {
            call_id: None,
            tool_name: "douyin_publish_validate".to_string(),
            arguments: json!({ "contentId": package.manifest.id }),
            thread_id: None,
            authorization_token: None,
            timeout_ms: None,
            context_scope: None,
        },
    )
    .unwrap();
    assert_eq!(result.status, "success");
    let value = result.result.unwrap();
    assert_eq!(value["serverName"], "douyin_mcp");
    assert_eq!(value["platformId"], "douyin");
    assert_eq!(value["autoPublishAllowed"], false);
    assert_eq!(value["publishDispatch"], "manual_only");
    assert_eq!(value["valid"], false);
    assert!(
        value["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["field"] == "media")
    );
    assert!(
        value["fieldMappings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|mapping| mapping["unifiedField"] == "media.video"
                && mapping["platformField"] == "video_id")
    );
    assert!(
        value["handoff"]["publishUrl"]
            .as_str()
            .unwrap()
            .contains("creator.douyin.com")
    );
    host.stop().unwrap();
}
