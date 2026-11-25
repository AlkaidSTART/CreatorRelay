use creatorrelay_lib::{database, ecosystem, plugins};
use rusqlite::Connection;
use serde_json::Value;
use std::path::PathBuf;

struct Fixture {
    _temp: tempfile::TempDir,
    conn: Connection,
    runtime: plugins::PluginRuntime,
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
