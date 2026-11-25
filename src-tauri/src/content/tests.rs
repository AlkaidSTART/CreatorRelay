use super::drafts::*;
use super::package::*;
use super::paths::*;
use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;

fn workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("contents")).unwrap();
    fs::create_dir_all(temp.path().join("platform-drafts")).unwrap();
    temp
}

fn image_bytes() -> &'static [u8] {
    b"\x89PNG\r\n\x1a\nfixture-image"
}

fn video_bytes() -> &'static [u8] {
    b"\x00\x00\x00\x18ftypmp42fixture-video"
}

#[test]
fn creates_all_four_kinds_and_survives_restart() {
    let temp = workspace();
    for kind in [
        ContentKind::Video,
        ContentKind::ImageText,
        ContentKind::ImageArticle,
        ContentKind::Article,
    ] {
        let package = create_content_package(temp.path(), kind, kind.as_str()).unwrap();
        assert_eq!(package.manifest.kind, kind);
        if kind.requires_markdown() {
            assert!(!package.body.is_empty());
            assert_eq!(package.path, format!("contents/{}", package.manifest.id));
        } else {
            assert!(package.body.is_empty());
        }
    }
    let listed = list_content_packages(temp.path()).unwrap();
    assert_eq!(listed.len(), 4);
    let reopened = read_content_package(temp.path(), &listed[0].id).unwrap();
    assert_eq!(reopened.manifest.kind, listed[0].kind);
}

#[test]
fn video_assets_and_metadata_persist_without_markdown_body() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::Video, "视频草稿").unwrap();
    let video = import_content_asset(
        temp.path(),
        &package.manifest.id,
        "media",
        "clip.mp4",
        "video/mp4",
        video_bytes(),
    )
    .unwrap();
    let cover = import_content_asset(
        temp.path(),
        &package.manifest.id,
        "cover",
        "cover.png",
        "image/png",
        image_bytes(),
    )
    .unwrap();
    let saved = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "视频新标题".to_string(),
            summary: "视频简介".to_string(),
            tags: vec!["视频".to_string()],
            cover: Some(cover),
            media: vec![video],
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    assert!(saved.body.is_empty());
    assert!(!temp.path().join(&saved.path).join("body.md").exists());
    let reopened = read_content_package(temp.path(), &package.manifest.id).unwrap();
    assert_eq!(reopened.manifest.summary, "视频简介");
    assert_eq!(reopened.manifest.media.len(), 1);
    assert!(reopened.manifest.cover.is_some());
}

#[test]
fn imported_media_bytes_can_be_read_for_preview() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::Video, "预览").unwrap();
    let video = import_content_asset(
        temp.path(),
        &package.manifest.id,
        "media",
        "clip.mp4",
        "video/mp4",
        video_bytes(),
    )
    .unwrap();
    save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "预览".to_string(),
            summary: String::new(),
            tags: vec![],
            cover: None,
            media: vec![video.clone()],
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    let content = read_content_asset(temp.path(), &package.manifest.id, &video.id).unwrap();
    assert_eq!(content.mime_type, "video/mp4");
    assert_eq!(content.bytes, video_bytes());
}

#[test]
fn image_order_cover_and_captions_persist() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::ImageText, "图集").unwrap();
    let mut media = Vec::new();
    for index in 0..3 {
        media.push(
            import_content_asset(
                temp.path(),
                &package.manifest.id,
                "media",
                &format!("image-{index}.png"),
                "image/png",
                image_bytes(),
            )
            .unwrap(),
        );
    }
    media.reverse();
    media[0].caption = "封面说明".to_string();
    media[1].caption = "第二张".to_string();
    let cover = media[0].clone();
    let saved = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "图集".to_string(),
            summary: "短文案".to_string(),
            tags: vec![],
            cover: Some(cover.clone()),
            media,
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    assert_eq!(saved.manifest.cover.as_ref().unwrap().id, cover.id);
    assert_eq!(
        saved
            .manifest
            .media
            .iter()
            .map(|asset| asset.order)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    let reopened = read_content_package(temp.path(), &package.manifest.id).unwrap();
    assert_eq!(reopened.manifest.media[0].caption, "封面说明");
    assert_eq!(reopened.manifest.media[1].caption, "第二张");
}

#[test]
fn article_body_markdown_is_authoritative_and_base_hash_blocks_overwrite() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::Article, "文章").unwrap();
    let saved = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "文章".to_string(),
            summary: "摘要".to_string(),
            tags: vec!["Rust".to_string()],
            cover: None,
            media: vec![],
            body: "# 标题\n\n正文".to_string(),
            base_hash: Some(package.content_hash.clone()),
        },
    )
    .unwrap();
    assert_eq!(saved.body, "# 标题\n\n正文");
    let reopened = read_content_package(temp.path(), &package.manifest.id).unwrap();
    assert_eq!(reopened.body, "# 标题\n\n正文");
    let error = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "覆盖".to_string(),
            summary: String::new(),
            tags: vec![],
            cover: None,
            media: vec![],
            body: "external".to_string(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap_err();
    assert!(error.contains("其他位置修改"));
    assert_eq!(
        read_content_package(temp.path(), &package.manifest.id)
            .unwrap()
            .body,
        "# 标题\n\n正文"
    );
}

#[test]
fn disk_write_failure_returns_error_without_replacing_content() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::Article, "只读").unwrap();
    let package_dir = content_root(temp.path()).join(&package.manifest.id);
    let mut permissions = fs::metadata(&package_dir).unwrap().permissions();
    permissions.set_mode(0o500);
    fs::set_permissions(&package_dir, permissions).unwrap();
    let result = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "无法保存".to_string(),
            summary: String::new(),
            tags: vec![],
            cover: None,
            media: vec![],
            body: "# 新内容".to_string(),
            base_hash: Some(package.content_hash),
        },
    );
    let mut permissions = fs::metadata(&package_dir).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&package_dir, permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(
        read_content_package(temp.path(), &package.manifest.id)
            .unwrap()
            .body,
        "# 只读\n\n"
    );
}

#[test]
fn drafts_are_independent_and_track_source_changes() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::Article, "主稿").unwrap();
    let saved = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "主稿".to_string(),
            summary: "摘要".to_string(),
            tags: vec![],
            cover: None,
            media: vec![],
            body: "# 主稿\n\n第一版".to_string(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    let drafts = create_platform_drafts(
        temp.path(),
        &package.manifest.id,
        &[
            DraftTarget {
                platform_id: "juejin".to_string(),
                platform_name: "掘金".to_string(),
                account_id: "a1".to_string(),
                account_alias: "主号".to_string(),
                adapter_id: None,
            },
            DraftTarget {
                platform_id: "csdn".to_string(),
                platform_name: "CSDN".to_string(),
                account_id: "a2".to_string(),
                account_alias: "副号".to_string(),
                adapter_id: None,
            },
        ],
    )
    .unwrap();
    assert_eq!(drafts.len(), 2);
    let first = drafts[0].clone();
    let second = drafts[1].clone();
    let edited = save_platform_draft(
        temp.path(),
        &first.id,
        DraftSaveInput {
            title: "掘金手动标题".to_string(),
            summary: first.summary.clone(),
            tags: first.tags.clone(),
            cover: first.cover.clone(),
            media: first.media.clone(),
            body: "掘金手动正文".to_string(),
            base_hash: first.hash.clone(),
        },
    )
    .unwrap();
    assert!(edited.manually_edited);
    assert_eq!(
        read_platform_draft(temp.path(), &second.id).unwrap().body,
        second.body
    );
    assert_eq!(
        read_content_package(temp.path(), &package.manifest.id)
            .unwrap()
            .body,
        "# 主稿\n\n第一版"
    );
    let _ = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "主稿".to_string(),
            summary: "摘要".to_string(),
            tags: vec![],
            cover: None,
            media: vec![],
            body: "# 主稿\n\n第二版".to_string(),
            base_hash: Some(saved.content_hash),
        },
    )
    .unwrap();
    let changed = read_platform_draft(temp.path(), &first.id).unwrap();
    assert!(changed.source_changed);
    assert_eq!(changed.body, "掘金手动正文");
    let acknowledged = acknowledge_platform_draft_source(temp.path(), &first.id).unwrap();
    assert!(!acknowledged.source_changed);
    let regenerated = regenerate_platform_draft(temp.path(), &first.id).unwrap();
    assert!(!regenerated.source_changed);
    assert!(!regenerated.manually_edited);
    assert!(regenerated.body.contains("第二版"));
}

#[test]
fn video_drafts_copy_video_and_cover_metadata() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::Video, "视频主稿").unwrap();
    let video = import_content_asset(
        temp.path(),
        &package.manifest.id,
        "media",
        "clip.mp4",
        "video/mp4",
        video_bytes(),
    )
    .unwrap();
    let cover = import_content_asset(
        temp.path(),
        &package.manifest.id,
        "cover",
        "cover.png",
        "image/png",
        image_bytes(),
    )
    .unwrap();
    let saved = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "视频主稿".to_string(),
            summary: "简介".to_string(),
            tags: vec![],
            cover: Some(cover),
            media: vec![video],
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    let drafts = create_platform_drafts(
        temp.path(),
        &saved.manifest.id,
        &[DraftTarget {
            platform_id: "bilibili".to_string(),
            platform_name: "Bilibili".to_string(),
            account_id: "bili-1".to_string(),
            account_alias: "视频号".to_string(),
            adapter_id: None,
        }],
    )
    .unwrap();
    let draft = &drafts[0];
    assert_eq!(draft.media.len(), 1);
    assert_eq!(draft.media[0].kind, AssetKind::Video);
    assert_eq!(draft.media[0].mime_type, "video/mp4");
    assert_eq!(draft.media[0].byte_size, video_bytes().len() as u64);
    assert_eq!(draft.cover.as_ref().unwrap().mime_type, "image/png");
    assert!(!draft.body.is_empty());
    assert!(
        temp.path()
            .join("platform-drafts")
            .join(&draft.id)
            .join(&draft.media[0].file)
            .is_file()
    );
}

#[test]
fn image_text_drafts_keep_media_order_and_cover_independently() {
    let temp = workspace();
    let package = create_content_package(temp.path(), ContentKind::ImageText, "图集主稿").unwrap();
    let mut media = Vec::new();
    for index in 0..3 {
        media.push(
            import_content_asset(
                temp.path(),
                &package.manifest.id,
                "media",
                &format!("image-{index}.png"),
                "image/png",
                image_bytes(),
            )
            .unwrap(),
        );
    }
    let cover = media[1].clone();
    let saved = save_content_package(
        temp.path(),
        &package.manifest.id,
        ContentSaveInput {
            title: "图集主稿".to_string(),
            summary: "短文案".to_string(),
            tags: vec![],
            cover: Some(cover.clone()),
            media,
            body: String::new(),
            base_hash: Some(package.content_hash),
        },
    )
    .unwrap();
    let drafts = create_platform_drafts(
        temp.path(),
        &saved.manifest.id,
        &[
            DraftTarget {
                platform_id: "douyin".to_string(),
                platform_name: "抖音".to_string(),
                account_id: "douyin-1".to_string(),
                account_alias: "视频号".to_string(),
                adapter_id: None,
            },
            DraftTarget {
                platform_id: "xiaohongshu".to_string(),
                platform_name: "小红书".to_string(),
                account_id: "xhs-1".to_string(),
                account_alias: "图集号".to_string(),
                adapter_id: None,
            },
        ],
    )
    .unwrap();
    for draft in &drafts {
        assert_eq!(
            draft
                .media
                .iter()
                .map(|asset| asset.order)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(draft.cover.as_ref().unwrap().order, cover.order);
        assert_eq!(draft.media.len(), 3);
    }
    let edited = save_platform_draft(
        temp.path(),
        &drafts[0].id,
        DraftSaveInput {
            title: "抖音独立标题".to_string(),
            summary: drafts[0].summary.clone(),
            tags: drafts[0].tags.clone(),
            cover: drafts[0].cover.clone(),
            media: drafts[0].media.clone(),
            body: "抖音独立正文".to_string(),
            base_hash: drafts[0].hash.clone(),
        },
    )
    .unwrap();
    assert_eq!(edited.media.len(), 3);
    assert_eq!(
        read_platform_draft(temp.path(), &drafts[1].id)
            .unwrap()
            .title,
        "图集主稿"
    );
    assert_eq!(
        read_content_package(temp.path(), &saved.manifest.id)
            .unwrap()
            .manifest
            .title,
        "图集主稿"
    );
}

#[test]
fn derived_content_is_a_new_package_with_source_reference() {
    let temp = workspace();
    let source = create_content_package(temp.path(), ContentKind::Article, "原始文章").unwrap();
    let source = save_content_package(
        temp.path(),
        &source.manifest.id,
        ContentSaveInput {
            title: "原始文章".to_string(),
            summary: "摘要".to_string(),
            tags: vec!["标签".to_string()],
            cover: None,
            media: vec![],
            body: "# 原始文章\n\n正文".to_string(),
            base_hash: Some(source.content_hash),
        },
    )
    .unwrap();
    let derived = derive_content_package(
        temp.path(),
        &source.manifest.id,
        ContentKind::ImageText,
        Some("派生图集".to_string()),
    )
    .unwrap();
    assert_ne!(derived.manifest.id, source.manifest.id);
    assert_eq!(derived.manifest.kind, ContentKind::ImageText);
    assert_eq!(
        derived.manifest.source.as_ref().unwrap().content_id,
        source.manifest.id
    );
    assert_eq!(
        read_content_package(temp.path(), &source.manifest.id)
            .unwrap()
            .body,
        "# 原始文章\n\n正文"
    );
}
