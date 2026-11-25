use creatorrelay_lib::content::{self, ContentKind, ContentPackage, ContentSaveInput};
use creatorrelay_lib::database;
use creatorrelay_lib::operations::{
    self, AnalyticsGranularity, AnalyticsQuery, CreatePublishTasksInput, DashboardFilter,
    ExternalWorkFilter, ManualCompletionInput, PublishSourceType, PublishTargetInput,
    PublishTaskStatus, ResolveUncertainInput, SyncPlatformWorksInput, UncertainOutcome,
};
use creatorrelay_lib::plugins::{self, PluginPermissionDecision, PluginRuntime, PluginStatus};
use rusqlite::Connection;
use std::{fs, path::PathBuf};

struct Fixture {
    _temp: tempfile::TempDir,
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
    let runtime = PluginRuntime::new(
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

fn insert_account(fixture: &mut Fixture, id: &str, platform_id: &str, alias: &str) {
    let now = "2026-11-25T08:00:00Z";
    fixture
        .conn
        .execute(
            "INSERT INTO platform_accounts(
                id, platform_id, alias, nickname, uid, status, enabled, is_default,
                profile_id, credential_ref, last_checked_at, last_error, created_at, updated_at
             ) VALUES(?1, ?2, ?3, ?3, 'fixture-uid', 'ACTIVE', 1, 0, ?4, ?5, ?6, NULL, ?6, ?6)",
            rusqlite::params![
                id,
                platform_id,
                alias,
                format!("profile-{id}"),
                format!("credential-{id}"),
                now
            ],
        )
        .unwrap();
}

fn image_bytes(index: u8) -> Vec<u8> {
    let mut value = b"\x89PNG\r\n\x1a\nfixture-image-".to_vec();
    value.push(index);
    value
}

fn create_image_text(fixture: &Fixture, count: usize) -> ContentPackage {
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::ImageText, "三图测试发布")
            .unwrap();
    let mut media = Vec::new();
    for index in 1..=count {
        media.push(
            content::import_content_asset(
                &fixture.workspace,
                &package.manifest.id,
                "media",
                &format!("image-{index}.png"),
                "image/png",
                &image_bytes(index as u8),
            )
            .unwrap(),
        );
    }
    let cover = media.get(1).cloned().unwrap_or_else(|| media[0].clone());
    content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: "三图测试发布".to_string(),
            summary: "三张图片按设置顺序发布".to_string(),
            tags: vec!["M6".to_string(), "图文".to_string()],
            cover: Some(cover),
            media,
            body: String::new(),
            base_hash: None,
        },
    )
    .unwrap()
}

fn create_article(fixture: &Fixture) -> ContentPackage {
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::Article, "多目标文章")
            .unwrap();
    content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: "多目标文章".to_string(),
            summary: "验证多目标独立任务".to_string(),
            tags: vec!["M6".to_string()],
            cover: None,
            media: vec![],
            body: "# 多目标文章\n\n每个平台独立记录结果。".to_string(),
            base_hash: None,
        },
    )
    .unwrap()
}

fn create_video(fixture: &Fixture) -> ContentPackage {
    let package =
        content::create_content_package(&fixture.workspace, ContentKind::Video, "视频往返测试")
            .unwrap();
    let video = content::import_content_asset(
        &fixture.workspace,
        &package.manifest.id,
        "media",
        "main.mp4",
        "video/mp4",
        b"\x00\x00\x00\x18ftypmp42m6-video",
    )
    .unwrap();
    let cover = content::import_content_asset(
        &fixture.workspace,
        &package.manifest.id,
        "cover",
        "cover.png",
        "image/png",
        &image_bytes(9),
    )
    .unwrap();
    content::save_content_package(
        &fixture.workspace,
        &package.manifest.id,
        ContentSaveInput {
            title: "视频往返测试".to_string(),
            summary: "视频和封面完整提交".to_string(),
            tags: vec!["M6".to_string(), "视频".to_string()],
            cover: Some(cover),
            media: vec![video],
            body: String::new(),
            base_hash: None,
        },
    )
    .unwrap()
}

fn target(
    platform_id: &str,
    platform_name: &str,
    account_id: &str,
    alias: &str,
    adapter_id: Option<&str>,
) -> PublishTargetInput {
    PublishTargetInput {
        platform_id: platform_id.to_string(),
        platform_name: platform_name.to_string(),
        account_id: account_id.to_string(),
        account_alias: alias.to_string(),
        adapter_id: adapter_id.map(str::to_string),
    }
}

fn create_tasks(
    fixture: &mut Fixture,
    source_type: PublishSourceType,
    source_id: &str,
    targets: Vec<PublishTargetInput>,
) -> Vec<operations::PublishTask> {
    operations::create_publish_tasks(
        &mut fixture.conn,
        &fixture.workspace,
        &CreatePublishTasksInput {
            source_type,
            source_id: source_id.to_string(),
            targets,
            scheduled_at: None,
        },
    )
    .unwrap()
    .tasks
}

fn prepare_and_confirm(fixture: &mut Fixture, task_id: &str) -> operations::PublishTask {
    let preview = operations::prepare_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        task_id,
    )
    .unwrap();
    let token = preview.confirmation_token.expect("preview token");
    operations::confirm_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        task_id,
        &token,
    )
    .unwrap()
}

#[test]
fn m6_ac_01_03_05_06_13_multi_target_confirmation_timeout_and_video_roundtrip() {
    let mut fixture = fixture();
    enable_adapter(&mut fixture, "creatorrelay.official.test-adapter");
    enable_adapter(
        &mut fixture,
        "creatorrelay.official.publish-timeout-adapter",
    );
    enable_adapter(&mut fixture, "creatorrelay.official.video-adapter");
    let article = create_article(&fixture);
    let tasks = create_tasks(
        &mut fixture,
        PublishSourceType::Content,
        &article.manifest.id,
        vec![
            target(
                "creatorrelay.official.test-adapter",
                "测试平台成功目标",
                "adapter-success-account",
                "成功账号",
                Some("creatorrelay.official.test-adapter"),
            ),
            target(
                "creatorrelay.official.publish-timeout-adapter",
                "测试平台超时目标",
                "adapter-timeout-account",
                "超时账号",
                Some("creatorrelay.official.publish-timeout-adapter"),
            ),
        ],
    );
    assert_eq!(tasks.len(), 2);
    assert!(
        tasks
            .iter()
            .all(|task| task.status == PublishTaskStatus::Created)
    );
    let success_task = tasks
        .iter()
        .find(|task| task.account_id == "adapter-success-account")
        .unwrap();
    let timeout_task = tasks
        .iter()
        .find(|task| task.account_id == "adapter-timeout-account")
        .unwrap();

    let preview = operations::prepare_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &success_task.id,
    )
    .unwrap();
    assert!(preview.validation.ok);
    assert_eq!(preview.task.content_kind, ContentKind::Article);
    assert_eq!(preview.task.title, "多目标文章");
    assert_eq!(preview.task.account_id, "adapter-success-account");
    assert!(preview.confirmation_token.is_some());
    assert!(
        operations::confirm_publish_task(
            &mut fixture.conn,
            &fixture.runtime,
            &fixture.workspace,
            &success_task.id,
            "bogus-token",
        )
        .is_err()
    );
    let success = operations::confirm_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &success_task.id,
        preview.confirmation_token.as_deref().unwrap(),
    )
    .unwrap();
    assert_eq!(success.status, PublishTaskStatus::Success);
    assert!(success.external_id.as_deref().is_some());
    assert_eq!(success.attempt_count, 1);

    let timeout = prepare_and_confirm(&mut fixture, &timeout_task.id);
    assert_eq!(timeout.status, PublishTaskStatus::NeedsManualAction);
    assert_eq!(timeout.error_code.as_deref(), Some("RESULT_UNCERTAIN"));
    assert_eq!(timeout.attempt_count, 1);
    assert!(
        operations::confirm_publish_task(
            &mut fixture.conn,
            &fixture.runtime,
            &fixture.workspace,
            &timeout_task.id,
            "unused",
        )
        .is_err()
    );
    let timeout_still = operations::publish_task(&fixture.conn, &timeout_task.id).unwrap();
    assert_eq!(timeout_still.attempt_count, 1);
    let resolved = operations::resolve_uncertain_publish_task(
        &mut fixture.conn,
        &timeout_task.id,
        &ResolveUncertainInput {
            outcome: UncertainOutcome::Success,
            external_id: Some("manually-checked-001".to_string()),
            external_url: Some("https://example.invalid/checked-001".to_string()),
            note: Some("平台后台确认作品已发布".to_string()),
        },
    )
    .unwrap();
    assert_eq!(resolved.status, PublishTaskStatus::Success);
    assert_eq!(
        resolved.external_id.as_deref(),
        Some("manually-checked-001")
    );

    let video = create_video(&fixture);
    let video_task = create_tasks(
        &mut fixture,
        PublishSourceType::Content,
        &video.manifest.id,
        vec![target(
            "creatorrelay.official.video-adapter",
            "视频测试平台",
            "adapter-video-account",
            "视频账号",
            Some("creatorrelay.official.video-adapter"),
        )],
    )
    .remove(0);
    let video_success = prepare_and_confirm(&mut fixture, &video_task.id);
    assert_eq!(video_success.status, PublishTaskStatus::Success);
    assert_eq!(video_success.content_kind, ContentKind::Video);
    let events = operations::list_publish_task_events(&fixture.conn, &video_task.id).unwrap();
    let success_event = events
        .iter()
        .find(|event| event.action == "SUCCESS")
        .unwrap();
    assert!(success_event.detail.contains("mediaOrder=[1]"));
    assert!(success_event.detail.contains("cover="));
    assert!(video_success.external_id.is_some());
}

#[test]
fn m6_ac_02_04_07_to_12_and_14_capabilities_sync_analytics_dashboard_and_manual_package() {
    let mut fixture = fixture();
    enable_adapter(&mut fixture, "creatorrelay.official.sync-adapter");
    enable_adapter(&mut fixture, "creatorrelay.official.video-adapter");
    insert_account(&mut fixture, "account-sync", "juejin", "同步主号");
    insert_account(&mut fixture, "expired-account", "csdn", "过期同步号");
    let image_text = create_image_text(&fixture, 3);
    let draft = content::create_platform_drafts(
        &fixture.workspace,
        &image_text.manifest.id,
        &[content::DraftTarget {
            platform_id: "juejin".to_string(),
            platform_name: "掘金".to_string(),
            account_id: "account-sync".to_string(),
            account_alias: "同步主号".to_string(),
            adapter_id: Some("creatorrelay.official.sync-adapter".to_string()),
        }],
    )
    .unwrap()
    .remove(0);

    let image_task = create_tasks(
        &mut fixture,
        PublishSourceType::PlatformDraft,
        &draft.id,
        vec![target(
            "juejin",
            "掘金",
            "account-sync",
            "同步主号",
            Some("creatorrelay.official.sync-adapter"),
        )],
    )
    .remove(0);
    let image_preview = operations::prepare_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &image_task.id,
    )
    .unwrap();
    assert!(image_preview.validation.ok);
    assert_eq!(
        image_preview
            .task
            .media
            .iter()
            .map(|asset| asset.order)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert!(image_preview.task.cover.is_some());
    let image_success = prepare_and_confirm(&mut fixture, &image_task.id);
    assert_eq!(image_success.status, PublishTaskStatus::Success);
    let image_events = operations::list_publish_task_events(&fixture.conn, &image_task.id).unwrap();
    assert!(
        image_events
            .iter()
            .any(|event| event.action == "SUCCESS" && event.detail.contains("mediaOrder=[1, 2, 3]"))
    );

    let video = create_video(&fixture);
    let video_task = create_tasks(
        &mut fixture,
        PublishSourceType::Content,
        &video.manifest.id,
        vec![target(
            "juejin",
            "掘金",
            "account-sync",
            "同步主号",
            Some("creatorrelay.official.sync-adapter"),
        )],
    )
    .remove(0);
    let video_preview = operations::prepare_publish_task(
        &mut fixture.conn,
        &fixture.runtime,
        &fixture.workspace,
        &video_task.id,
    )
    .unwrap();
    assert_eq!(
        video_preview.execution_mode,
        operations::PublishExecutionMode::Manual
    );
    assert!(
        !video_preview
            .actions
            .iter()
            .find(|action| action.id == "auto_publish")
            .unwrap()
            .available
    );
    assert!(video_preview.capability_reason.is_some());

    let todo = create_image_text(&fixture, 3);
    let todo_draft = content::create_platform_drafts(
        &fixture.workspace,
        &todo.manifest.id,
        &[content::DraftTarget {
            platform_id: "csdn".to_string(),
            platform_name: "CSDN".to_string(),
            account_id: "expired-account".to_string(),
            account_alias: "过期同步号".to_string(),
            adapter_id: None,
        }],
    )
    .unwrap()
    .remove(0);
    let manual_task = create_tasks(
        &mut fixture,
        PublishSourceType::PlatformDraft,
        &todo_draft.id,
        vec![target(
            "csdn",
            "CSDN",
            "expired-account",
            "过期同步号",
            None,
        )],
    )
    .remove(0);
    let manual = prepare_and_confirm(&mut fixture, &manual_task.id);
    assert_eq!(manual.status, PublishTaskStatus::NeedsManualAction);
    let package = operations::export_manual_publish_package(
        &fixture.conn,
        &fixture.workspace,
        &manual_task.id,
    )
    .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&package.manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["mediaOrder"], serde_json::json!([1, 2, 3]));
    assert_eq!(manifest["assets"].as_array().unwrap().len(), 4);
    assert!(
        manifest["assets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|asset| asset["role"] == "cover")
    );
    assert_eq!(
        manifest["coverAssetId"],
        serde_json::json!(todo_draft.cover.as_ref().unwrap().id)
    );
    let completed = operations::complete_manual_publish_task(
        &mut fixture.conn,
        &manual_task.id,
        &ManualCompletionInput {
            external_id: Some("manual-platform-id".to_string()),
            external_url: Some("https://example.invalid/manual".to_string()),
        },
    )
    .unwrap();
    assert_eq!(completed.status, PublishTaskStatus::Success);
    let cancelled = operations::cancel_publish_task(
        &fixture.conn,
        &video_task.id,
        Some("用户在人工发布前取消"),
    )
    .unwrap();
    assert_eq!(cancelled.status, PublishTaskStatus::Cancelled);

    let sync_input = SyncPlatformWorksInput {
        account_id: "account-sync".to_string(),
        platform_id: "juejin".to_string(),
        platform_name: "掘金".to_string(),
        account_alias: "同步主号".to_string(),
        adapter_id: "creatorrelay.official.sync-adapter".to_string(),
        as_of: Some("2026-11-23T09:00:00Z".to_string()),
    };
    let first_sync =
        operations::sync_platform_works(&mut fixture.conn, &fixture.runtime, &sync_input).unwrap();
    assert_eq!(first_sync.status, "SUCCESS");
    assert_eq!(first_sync.imported_count, 4);
    let works =
        operations::list_external_works(&fixture.conn, &ExternalWorkFilter::default()).unwrap();
    let synced = works
        .iter()
        .filter(|work| work.account_id.as_deref() == Some("account-sync") && work.source == "sync")
        .collect::<Vec<_>>();
    assert_eq!(synced.len(), 4);
    let video_work = synced
        .iter()
        .find(|work| work.content_kind == "video")
        .unwrap();
    let article_work = synced
        .iter()
        .find(|work| work.content_kind == "article")
        .unwrap();
    assert_eq!(video_work.original_content_type, "video");
    assert_eq!(article_work.original_content_type, "文章");
    let video_views = video_work
        .metrics
        .iter()
        .find(|metric| metric.metric_name == "视频播放量")
        .unwrap();
    let article_views = article_work
        .metrics
        .iter()
        .find(|metric| metric.metric_name == "文章阅读量")
        .unwrap();
    assert!(!video_views.comparable);
    assert!(video_views.canonical_key.is_none());
    assert_ne!(video_views.metric_name, article_views.metric_name);

    let second_sync = operations::sync_platform_works(
        &mut fixture.conn,
        &fixture.runtime,
        &SyncPlatformWorksInput {
            as_of: Some("2026-11-24T09:00:00Z".to_string()),
            ..sync_input.clone()
        },
    )
    .unwrap();
    assert_eq!(second_sync.status, "SUCCESS");
    assert_eq!(second_sync.imported_count, 0);
    assert_eq!(second_sync.updated_count, 4);
    let repeated_same_day = operations::sync_platform_works(
        &mut fixture.conn,
        &fixture.runtime,
        &SyncPlatformWorksInput {
            as_of: Some("2026-11-24T18:00:00Z".to_string()),
            ..sync_input.clone()
        },
    )
    .unwrap();
    assert_eq!(repeated_same_day.updated_count, 4);
    for as_of in ["2026-11-17T09:00:00Z", "2026-10-30T09:00:00Z"] {
        let run = operations::sync_platform_works(
            &mut fixture.conn,
            &fixture.runtime,
            &SyncPlatformWorksInput {
                as_of: Some(as_of.to_string()),
                ..sync_input.clone()
            },
        )
        .unwrap();
        assert_eq!(run.status, "SUCCESS");
    }
    let works_after =
        operations::list_external_works(&fixture.conn, &ExternalWorkFilter::default()).unwrap();
    assert_eq!(
        works_after
            .iter()
            .filter(|work| {
                work.account_id.as_deref() == Some("account-sync") && work.source == "sync"
            })
            .count(),
        4
    );
    let filtered = operations::list_external_works(
        &fixture.conn,
        &ExternalWorkFilter {
            keyword: Some("测试图文".to_string()),
            content_kind: Some("imageText".to_string()),
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            published_from: Some("2026-11-01T00:00:00Z".to_string()),
            published_to: Some("2026-11-30T23:59:59Z".to_string()),
        },
    )
    .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].external_id, "account-sync-image-text-001");
    let latest_video = operations::external_work(&fixture.conn, &video_work.id).unwrap();
    let video_views = latest_video
        .metrics
        .iter()
        .find(|metric| metric.metric_name == "视频播放量")
        .unwrap();
    let video_view_count: i64 = fixture
        .conn
        .query_row(
            "SELECT COUNT(*) FROM metric_snapshots WHERE work_id=?1 AND metric_name='视频播放量'",
            [&video_work.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(video_view_count, 4);
    assert_eq!(video_views.value, 4200.0);

    let trend = operations::analytics_trend(
        &fixture.conn,
        &AnalyticsQuery {
            range: "7d".to_string(),
            granularity: AnalyticsGranularity::Day,
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            content_kind: None,
            work_id: Some(video_work.id.clone()),
        },
    )
    .unwrap();
    assert!(trend.series.iter().all(|series| series.points.len() >= 2));
    assert!(trend.series.iter().all(|series| !series.comparable));
    let weekly = operations::analytics_trend(
        &fixture.conn,
        &AnalyticsQuery {
            range: "30d".to_string(),
            granularity: AnalyticsGranularity::Week,
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            content_kind: Some("video".to_string()),
            work_id: Some(video_work.id.clone()),
        },
    )
    .unwrap();
    assert!(weekly.series.iter().all(|series| series.points.len() >= 2));
    let monthly = operations::analytics_trend(
        &fixture.conn,
        &AnalyticsQuery {
            range: "90d".to_string(),
            granularity: AnalyticsGranularity::Month,
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            content_kind: Some("video".to_string()),
            work_id: Some(video_work.id.clone()),
        },
    )
    .unwrap();
    assert!(monthly.series.iter().all(|series| series.points.len() >= 2));
    let missing = operations::analytics_trend(
        &fixture.conn,
        &AnalyticsQuery {
            range: "7d".to_string(),
            granularity: AnalyticsGranularity::Week,
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            content_kind: Some("unknown".to_string()),
            work_id: None,
        },
    )
    .unwrap();
    assert!(missing.series.is_empty());
    assert!(
        missing
            .warnings
            .iter()
            .any(|warning| warning.contains("不会用 0"))
    );

    let dashboard = operations::operations_dashboard(
        &fixture.conn,
        &DashboardFilter {
            range: Some("30d".to_string()),
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            content_kind: None,
        },
    )
    .unwrap();
    assert_eq!(dashboard.account_count, 1);
    assert_eq!(dashboard.platform_count, 1);
    assert!(
        dashboard
            .recent_works
            .iter()
            .all(|work| work.account_id.as_deref() == Some("account-sync"))
    );
    assert!(
        dashboard
            .pending_tasks
            .iter()
            .all(|task| task.account_id == "account-sync")
    );
    let video_dashboard = operations::operations_dashboard(
        &fixture.conn,
        &DashboardFilter {
            range: Some("90d".to_string()),
            platform_id: Some("juejin".to_string()),
            account_id: Some("account-sync".to_string()),
            content_kind: Some("video".to_string()),
        },
    )
    .unwrap();
    assert!(
        video_dashboard
            .recent_works
            .iter()
            .all(|work| work.content_kind == "video")
    );

    let invalid_sync = operations::sync_platform_works(
        &mut fixture.conn,
        &fixture.runtime,
        &SyncPlatformWorksInput {
            account_id: "expired-account".to_string(),
            platform_id: "csdn".to_string(),
            platform_name: "CSDN".to_string(),
            account_alias: "过期同步号".to_string(),
            adapter_id: "creatorrelay.official.sync-adapter".to_string(),
            as_of: Some("2026-11-24T09:00:00Z".to_string()),
        },
    )
    .unwrap();
    assert_eq!(invalid_sync.status, "ACCOUNT_INVALID");
    let account_status: String = fixture
        .conn
        .query_row(
            "SELECT status FROM platform_accounts WHERE id='expired-account'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(account_status, "NEEDS_REAUTH");
    let paused = operations::publish_task(&fixture.conn, &manual_task.id).unwrap();
    assert!(!paused.paused, "已成功任务不应因账号失效被改写");
    fixture
        .conn
        .execute(
            "UPDATE platform_accounts SET status='ACTIVE', last_error=NULL WHERE id='expired-account'",
            [],
        )
        .unwrap();
    let resumed = operations::resume_account_tasks(&fixture.conn, "expired-account").unwrap();
    assert!(resumed.iter().all(|task| !task.paused));
}

#[test]
fn m6_ac_12_account_invalidation_pauses_unfinished_dependent_tasks() {
    let mut fixture = fixture();
    enable_adapter(&mut fixture, "creatorrelay.official.sync-adapter");
    insert_account(&mut fixture, "expired-account", "csdn", "过期同步号");
    let article = create_article(&fixture);
    let task = create_tasks(
        &mut fixture,
        PublishSourceType::Content,
        &article.manifest.id,
        vec![target(
            "csdn",
            "CSDN",
            "expired-account",
            "过期同步号",
            Some("creatorrelay.official.sync-adapter"),
        )],
    )
    .remove(0);
    let sync = operations::sync_platform_works(
        &mut fixture.conn,
        &fixture.runtime,
        &SyncPlatformWorksInput {
            account_id: "expired-account".to_string(),
            platform_id: "csdn".to_string(),
            platform_name: "CSDN".to_string(),
            account_alias: "过期同步号".to_string(),
            adapter_id: "creatorrelay.official.sync-adapter".to_string(),
            as_of: None,
        },
    )
    .unwrap();
    assert_eq!(sync.status, "ACCOUNT_INVALID");
    let paused = operations::publish_task(&fixture.conn, &task.id).unwrap();
    assert!(paused.paused);
    assert!(paused.pause_reason.as_deref().unwrap().contains("重新授权"));
    fixture
        .conn
        .execute(
            "UPDATE platform_accounts SET status='ACTIVE', last_error=NULL WHERE id='expired-account'",
            [],
        )
        .unwrap();
    let resumed = operations::resume_account_tasks(&fixture.conn, "expired-account").unwrap();
    assert_eq!(resumed.len(), 1);
    assert!(!resumed[0].paused);
    assert_eq!(resumed[0].status, PublishTaskStatus::Created);
}
