use creatorrelay_lib::database;
use creatorrelay_lib::plugins::{
    self, CapabilityLevel, ContentKind, MediaKind, PluginPermissionDecision, PluginRuntime,
    PluginSourceKind, PluginStatus, PublishInput, PublishMediaRef, TrustLevel,
};
use rusqlite::Connection;
use serde_json::json;
use std::{fs, path::PathBuf};

struct Fixture {
    temp: tempfile::TempDir,
    conn: Connection,
    runtime: PluginRuntime,
    workspace: PathBuf,
}

fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let database_path = temp.path().join("creatorrelay.sqlite3");
    let mut conn = Connection::open(&database_path).unwrap();
    database::migrate(&mut conn).unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("articles")).unwrap();
    conn.execute(
        "INSERT INTO app_settings(key, value) VALUES('workspace_path', ?1)",
        [workspace.to_string_lossy().to_string()],
    )
    .unwrap();
    let runtime = PluginRuntime::new(
        temp.path().join("plugins"),
        temp.path().to_path_buf(),
        database_path,
        PathBuf::from(env!("CARGO_BIN_EXE_creatorrelay")),
    )
    .unwrap();
    Fixture {
        temp,
        conn,
        runtime,
        workspace,
    }
}

fn install_test_adapter(fixture: &mut Fixture) -> plugins::PluginSummary {
    plugins::install_official_plugin(
        &mut fixture.conn,
        &fixture.runtime,
        "creatorrelay.official.test-adapter",
    )
    .unwrap()
}

fn grant_required(fixture: &mut Fixture, plugin_id: &str) -> plugins::PluginSummary {
    let plugin = fixture
        .conn
        .query_row("SELECT id FROM plugins WHERE id=?1", [plugin_id], |row| {
            row.get::<_, String>(0)
        })
        .ok();
    assert!(plugin.is_some());
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
        .unwrap()
}

fn image(asset_id: &str, order: u32) -> PublishMediaRef {
    PublishMediaRef {
        asset_id: asset_id.to_string(),
        kind: MediaKind::Image,
        mime_type: "image/png".to_string(),
        byte_size: 64,
        order,
        role: None,
    }
}

fn video(asset_id: &str) -> PublishMediaRef {
    PublishMediaRef {
        asset_id: asset_id.to_string(),
        kind: MediaKind::Video,
        mime_type: "video/mp4".to_string(),
        byte_size: 1024,
        order: 1,
        role: None,
    }
}

#[test]
fn m4_ac_03_05_lifecycle_trace_registers_identity_capabilities_and_events() {
    let mut fixture = fixture();
    let catalog = plugins::list_catalog(&fixture.conn, &fixture.runtime).unwrap();
    assert!(catalog.len() >= 3);
    assert!(catalog.iter().all(|entry| !entry.installed));

    let installed = install_test_adapter(&mut fixture);
    assert_eq!(installed.status, PluginStatus::ReviewRequired);
    assert_eq!(installed.adapter_contract_version, 1);
    assert!(
        installed
            .permissions
            .iter()
            .all(|permission| permission.granted.is_none())
    );

    let ready = grant_required(&mut fixture, &installed.id);
    assert_eq!(ready.status, PluginStatus::Ready);
    assert!(
        ready
            .permissions
            .iter()
            .all(|permission| permission.granted.is_some())
    );

    let enabled =
        plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    assert_eq!(enabled.status, PluginStatus::Enabled);
    assert_eq!(enabled.runtime_state, "running");
    assert_eq!(
        enabled
            .capabilities
            .iter()
            .find(|capability| capability.kind == ContentKind::Video)
            .unwrap()
            .effective_publish,
        CapabilityLevel::Unsupported
    );
    assert_eq!(
        enabled
            .capabilities
            .iter()
            .find(|capability| capability.kind == ContentKind::ImageText)
            .unwrap()
            .effective_publish,
        CapabilityLevel::Supported
    );

    let disabled =
        plugins::disable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    assert_eq!(disabled.status, PluginStatus::Disabled);
    assert_eq!(disabled.runtime_state, "stopped");
    let events = plugins::list_plugin_events(&fixture.conn, Some(&installed.id), None).unwrap();
    let actions = events
        .iter()
        .map(|event| event.action.as_str())
        .collect::<Vec<_>>();
    for expected in ["INSTALLED", "PERMISSIONS_REVIEWED", "ENABLED", "DISABLED"] {
        assert!(
            actions.contains(&expected),
            "missing event {expected}: {actions:?}"
        );
    }

    let uninstall =
        plugins::uninstall_plugin(&mut fixture.conn, &fixture.runtime, &installed.id, true)
            .unwrap();
    assert!(uninstall.deleted_plugin_files);
    assert!(
        !fixture
            .runtime
            .plugin_directory(&installed.id)
            .unwrap()
            .exists()
    );
    assert!(
        plugins::list_plugins(&fixture.conn, &fixture.runtime)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn m4_ac_02_permissions_block_enable_and_core_proxy_never_returns_raw_access() {
    let mut fixture = fixture();
    let installed = install_test_adapter(&mut fixture);
    let error =
        plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap_err();
    assert!(error.contains("权限审查"));

    let denied_decisions = installed
        .permissions
        .iter()
        .map(|permission| PluginPermissionDecision {
            id: permission.id.clone(),
            granted: !permission.required,
        })
        .collect::<Vec<_>>();
    let reviewed = plugins::set_plugin_permissions(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        &denied_decisions,
    )
    .unwrap();
    assert_ne!(reviewed.status, PluginStatus::Ready);
    let error =
        plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap_err();
    assert!(error.contains("必需权限未授权"), "{error}");

    grant_required(&mut fixture, &installed.id);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    let probe = plugins::probe_plugin_capabilities(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        Some("fixture-image-text".to_string()),
    )
    .unwrap();
    assert!(probe.ok);
    let credential_proxy = probe
        .host_access
        .iter()
        .find(|observation| observation.method == "credentials.get")
        .unwrap();
    assert!(credential_proxy.allowed);
    let raw = probe
        .host_access
        .iter()
        .find(|observation| observation.method == "credentials.readRaw")
        .unwrap();
    assert!(!raw.allowed);
    let database = probe
        .host_access
        .iter()
        .find(|observation| observation.method == "database.query")
        .unwrap();
    assert!(!database.allowed);
    let workspace = probe
        .host_access
        .iter()
        .find(|observation| observation.method == "workspace.read")
        .unwrap();
    assert!(!workspace.allowed);
}

#[test]
fn m4_ac_04_runtime_probe_failure_overrides_manifest_supported_claim() {
    let mut fixture = fixture();
    let installed = plugins::install_official_plugin(
        &mut fixture.conn,
        &fixture.runtime,
        "creatorrelay.official.probe-failure-adapter",
    )
    .unwrap();
    assert_eq!(installed.status, PluginStatus::ReviewRequired);
    assert_eq!(
        installed
            .capabilities
            .iter()
            .find(|capability| capability.kind == ContentKind::Video)
            .unwrap()
            .declared_publish,
        CapabilityLevel::Supported
    );
    grant_required(&mut fixture, &installed.id);
    let error =
        plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap_err();
    assert!(error.contains("PROBE_FAILED"), "{error}");
    let plugins = plugins::list_plugins(&fixture.conn, &fixture.runtime).unwrap();
    let failed = plugins
        .iter()
        .find(|plugin| plugin.id == installed.id)
        .unwrap();
    assert_eq!(failed.status, PluginStatus::Error);
    assert!(
        failed
            .capabilities
            .iter()
            .all(|capability| capability.effective_publish == CapabilityLevel::Unavailable)
    );
    assert!(
        failed
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("PROBE_FAILED"))
    );
}

#[test]
fn m4_ac_06_crash_and_timeout_isolate_faulty_adapter_from_healthy_adapter() {
    let mut fixture = fixture();
    let healthy = install_test_adapter(&mut fixture);
    grant_required(&mut fixture, &healthy.id);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &healthy.id).unwrap();

    let faulty = plugins::install_official_plugin(
        &mut fixture.conn,
        &fixture.runtime,
        "creatorrelay.official.fault-adapter",
    )
    .unwrap();
    grant_required(&mut fixture, &faulty.id);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &faulty.id).unwrap();

    let crash =
        plugins::simulate_plugin_fault(&mut fixture.conn, &fixture.runtime, &faulty.id, "crash")
            .unwrap();
    assert_eq!(crash.status, PluginStatus::Error);
    assert!(
        crash.error.contains("退出") || crash.error.contains("崩溃"),
        "{}",
        crash.error
    );
    let healthy_probe =
        plugins::probe_plugin_capabilities(&mut fixture.conn, &fixture.runtime, &healthy.id, None)
            .unwrap();
    assert!(healthy_probe.ok);

    let restarted =
        plugins::restart_plugin(&mut fixture.conn, &fixture.runtime, &faulty.id).unwrap();
    assert_eq!(restarted.status, PluginStatus::Enabled);
    let timeout =
        plugins::simulate_plugin_fault(&mut fixture.conn, &fixture.runtime, &faulty.id, "timeout")
            .unwrap();
    assert_eq!(timeout.status, PluginStatus::Error);
    assert!(timeout.error.contains("超时"), "{}", timeout.error);
    let healthy_probe =
        plugins::probe_plugin_capabilities(&mut fixture.conn, &fixture.runtime, &healthy.id, None)
            .unwrap();
    assert!(healthy_probe.ok);
}

fn versioned_manifest(version: &str, permissions: &[(&str, bool)]) -> String {
    json!({
      "id": "creatorrelay.official.versioned-adapter",
      "name": "版本兼容测试适配器",
      "description": "版本迁移测试",
      "version": version,
      "pluginType": "adapter",
      "adapterContractVersion": 1,
      "runtime": { "kind": "coreTestRunner", "profile": "healthy", "timeoutMs": 2000 },
      "entry": "builtin://creatorrelay.official.versioned-adapter",
      "permissions": permissions.iter().map(|(id, required)| json!({
        "id": id, "required": required, "reason": "版本迁移权限测试"
      })).collect::<Vec<_>>(),
      "capabilities": [
        { "kind": "video", "read": "unsupported", "publish": "unsupported" },
        { "kind": "imageText", "read": "supported", "publish": "supported" },
        { "kind": "imageArticle", "read": "supported", "publish": "experimental" },
        { "kind": "article", "read": "supported", "publish": "supported" }
      ]
    })
    .to_string()
}

#[test]
fn m4_ac_07_version_rollback_preserves_intersected_grants_and_rejects_contract_v2() {
    let mut fixture = fixture();
    let v1 = versioned_manifest(
        "1.0.0",
        &[("account.credentials", true), ("network", false)],
    );
    let installed = plugins::install_manifest(
        &mut fixture.conn,
        &fixture.runtime,
        &v1,
        PluginSourceKind::Bundled,
        TrustLevel::Official,
    )
    .unwrap();
    plugins::set_plugin_permissions(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        &[
            PluginPermissionDecision {
                id: "account.credentials".to_string(),
                granted: true,
            },
            PluginPermissionDecision {
                id: "network".to_string(),
                granted: true,
            },
        ],
    )
    .unwrap();

    let v2 = versioned_manifest(
        "1.1.0",
        &[("account.credentials", true), ("workspace.read", false)],
    );
    let updated =
        plugins::update_plugin_manifest(&mut fixture.conn, &fixture.runtime, &v2).unwrap();
    assert_eq!(updated.version, "1.1.0");
    assert_eq!(updated.previous_version.as_deref(), Some("1.0.0"));
    let account = updated
        .permissions
        .iter()
        .find(|permission| permission.id == "account.credentials")
        .unwrap();
    assert_eq!(account.granted, Some(true));
    let workspace = updated
        .permissions
        .iter()
        .find(|permission| permission.id == "workspace.read")
        .unwrap();
    assert_eq!(workspace.granted, None);
    assert!(
        !updated
            .permissions
            .iter()
            .any(|permission| permission.id == "network")
    );

    let rolled_back =
        plugins::rollback_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    assert_eq!(rolled_back.version, "1.0.0");
    assert_eq!(rolled_back.adapter_contract_version, 1);
    let account = rolled_back
        .permissions
        .iter()
        .find(|permission| permission.id == "account.credentials")
        .unwrap();
    assert_eq!(account.granted, Some(true));
    let network = rolled_back
        .permissions
        .iter()
        .find(|permission| permission.id == "network")
        .unwrap();
    assert_eq!(network.granted, None, "被移除的权限不能自动恢复");
    assert!(
        !rolled_back
            .permissions
            .iter()
            .any(|permission| permission.id == "workspace.read")
    );

    let incompatible = versioned_manifest("2.0.0", &[("account.credentials", true)]).replace(
        "\"adapterContractVersion\":1",
        "\"adapterContractVersion\":2",
    );
    let validation = plugins::validate_manifest(&incompatible);
    assert!(!validation.valid);
    assert!(
        validation
            .issues
            .iter()
            .any(|issue| issue.code == "unsupported_contract_version")
    );
    assert!(
        plugins::install_manifest(
            &mut fixture.conn,
            &fixture.runtime,
            &incompatible,
            PluginSourceKind::Bundled,
            TrustLevel::Official,
        )
        .is_err()
    );

    fixture
        .conn
        .execute(
            "UPDATE plugins SET adapter_contract_version=2 WHERE id=?1",
            [&installed.id],
        )
        .unwrap();
    let enable_error =
        plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap_err();
    assert!(enable_error.contains("合约版本"), "{enable_error}");
}

#[test]
fn m4_ac_08_security_self_test_denies_database_workspace_and_credential_canaries() {
    let mut fixture = fixture();
    let installed = install_test_adapter(&mut fixture);
    grant_required(&mut fixture, &installed.id);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    let probe = plugins::probe_plugin_capabilities(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        None,
    )
    .unwrap();
    assert!(
        !probe
            .host_access
            .iter()
            .find(|observation| observation.method == "database.query")
            .unwrap()
            .allowed
    );

    #[cfg(target_os = "macos")]
    {
        let workspace_canary = fixture.workspace.join(".m4-workspace-canary");
        fs::write(&workspace_canary, b"test-only-workspace-canary").unwrap();
        let result = plugins::security_self_test(
            &fixture.conn,
            &fixture.runtime,
            &installed.id,
            Some(workspace_canary.clone()),
        )
        .unwrap();
        assert!(result.sandboxed, "{result:?}");
        assert!(result.database_denied, "{result:?}");
        assert!(result.workspace_denied, "{result:?}");
        assert!(result.credential_denied, "{result:?}");
        assert!(result.all_denied, "{result:?}");
        let _ = fs::remove_file(workspace_canary);
    }
}

#[test]
fn m4_ac_10_11_capability_matrix_gates_each_content_kind_and_payload_completeness() {
    let mut fixture = fixture();
    let installed = install_test_adapter(&mut fixture);
    grant_required(&mut fixture, &installed.id);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    let probe = plugins::probe_plugin_capabilities(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        None,
    )
    .unwrap();
    assert!(probe.ok);
    let video_state = probe
        .capabilities
        .iter()
        .find(|capability| capability.kind == ContentKind::Video)
        .unwrap();
    let image_text_state = probe
        .capabilities
        .iter()
        .find(|capability| capability.kind == ContentKind::ImageText)
        .unwrap();
    assert_eq!(video_state.effective_publish, CapabilityLevel::Unsupported);
    assert_eq!(
        image_text_state.effective_publish,
        CapabilityLevel::Supported
    );

    let image_text_input = PublishInput {
        content_kind: ContentKind::ImageText,
        title: "图文发布".to_string(),
        body: Some("说明".to_string()),
        media: vec![image("asset-1", 1), image("asset-2", 2)],
        cover: Some(image("asset-cover", 1)),
    };
    let image_text = plugins::validate_plugin_publish_input(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        None,
        &image_text_input,
    )
    .unwrap();
    assert!(image_text.ok, "{image_text:?}");
    assert!(image_text.adapter_checked);
    assert_eq!(image_text.payload.media_order, vec![1, 2]);

    let video_input = PublishInput {
        content_kind: ContentKind::Video,
        title: "视频发布".to_string(),
        body: None,
        media: vec![video("asset-video")],
        cover: Some(image("asset-cover", 1)),
    };
    let video_result = plugins::validate_plugin_publish_input(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        None,
        &video_input,
    )
    .unwrap();
    assert!(!video_result.ok);
    assert_eq!(video_result.capability, Some(CapabilityLevel::Unsupported));
    assert!(
        video_result
            .payload
            .issues
            .iter()
            .any(|issue| issue.code == "capability_not_available")
    );

    let article_input = PublishInput {
        content_kind: ContentKind::Article,
        title: "纯文章".to_string(),
        body: Some("正文".to_string()),
        media: vec![],
        cover: None,
    };
    let article = plugins::validate_plugin_publish_input(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        None,
        &article_input,
    )
    .unwrap();
    assert!(article.ok, "{article:?}");

    let incomplete = PublishInput {
        content_kind: ContentKind::Video,
        title: "缺少封面".to_string(),
        body: None,
        media: vec![video("asset-video")],
        cover: None,
    };
    let incomplete_result = plugins::validate_plugin_publish_input(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        None,
        &incomplete,
    )
    .unwrap();
    assert!(!incomplete_result.ok);
    assert!(!incomplete_result.adapter_checked);
    assert!(
        incomplete_result
            .payload
            .issues
            .iter()
            .any(|issue| issue.code == "missing_video_cover")
    );

    let self_check = plugins::payload_self_check();
    assert!(self_check.iter().all(|case| case.passed));
    for kind in [
        ContentKind::Video,
        ContentKind::ImageText,
        ContentKind::ImageArticle,
        ContentKind::Article,
    ] {
        assert!(self_check.iter().any(|case| case.content_kind == kind));
    }
}

#[test]
fn m4_ac_02_04_permission_denials_are_visible_to_adapter_probe() {
    let mut fixture = fixture();
    let installed = install_test_adapter(&mut fixture);
    let reviewed = plugins::set_plugin_permissions(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        &installed
            .permissions
            .iter()
            .map(|permission| PluginPermissionDecision {
                id: permission.id.clone(),
                granted: permission.required,
            })
            .collect::<Vec<_>>(),
    )
    .unwrap();
    assert_eq!(reviewed.status, PluginStatus::Ready);
    plugins::enable_plugin(&mut fixture.conn, &fixture.runtime, &installed.id).unwrap();
    let probe = plugins::probe_plugin_capabilities(
        &mut fixture.conn,
        &fixture.runtime,
        &installed.id,
        Some("fixture-image-text".to_string()),
    )
    .unwrap();
    let by_method = |method: &str| {
        probe
            .host_access
            .iter()
            .find(|observation| observation.method == method)
            .unwrap()
    };
    assert_eq!(
        by_method("network.request").code.as_deref(),
        Some("PERMISSION_DENIED")
    );
    assert_eq!(
        by_method("workspace.read").code.as_deref(),
        Some("PERMISSION_DENIED")
    );
    assert_eq!(
        by_method("credentials.readRaw").code.as_deref(),
        Some("FORBIDDEN_RAW_CREDENTIAL")
    );
    assert_eq!(
        by_method("database.query").code.as_deref(),
        Some("FORBIDDEN")
    );
    assert!(
        by_method("credentials.get").allowed,
        "已授权账号代理应可用，但 rawAccess 必须为 false"
    );
    let _ = &fixture.temp;
}
