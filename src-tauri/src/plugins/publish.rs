use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::manifest::{CONTENT_KINDS, ContentKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Video,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishMediaRef {
    pub asset_id: String,
    pub kind: MediaKind,
    pub mime_type: String,
    pub byte_size: u64,
    pub order: u32,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublishInput {
    pub content_kind: ContentKind,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub media: Vec<PublishMediaRef>,
    #[serde(default)]
    pub cover: Option<PublishMediaRef>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishIssue {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishValidationResult {
    pub ok: bool,
    pub content_kind: ContentKind,
    pub issues: Vec<PublishIssue>,
    pub media_order: Vec<u32>,
    pub cover_asset_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishPayloadCheckResult {
    pub id: &'static str,
    pub content_kind: ContentKind,
    pub expected_ok: bool,
    pub actual_ok: bool,
    pub passed: bool,
    pub message: String,
}

fn issue(field: &str, code: &str, message: impl Into<String>) -> PublishIssue {
    PublishIssue {
        field: field.to_string(),
        code: code.to_string(),
        message: message.into(),
    }
}

fn valid_asset_reference(media: &PublishMediaRef, field: &str, issues: &mut Vec<PublishIssue>) {
    if media.asset_id.trim().is_empty() {
        issues.push(issue(
            &format!("{field}.assetId"),
            "missing_asset_id",
            "素材引用缺少受控 assetId",
        ));
    }
    if media.asset_id.contains('/')
        || media.asset_id.contains('\\')
        || media.asset_id.contains("..")
    {
        issues.push(issue(
            &format!("{field}.assetId"),
            "invalid_asset_id",
            "assetId 必须是不透明引用，不能包含路径",
        ));
    }
    if media.mime_type.trim().is_empty() {
        issues.push(issue(
            &format!("{field}.mimeType"),
            "missing_mime_type",
            "素材引用缺少 MIME 类型",
        ));
    }
    if media.byte_size == 0 {
        issues.push(issue(
            &format!("{field}.byteSize"),
            "empty_asset",
            "素材大小必须大于 0",
        ));
    }
}

fn validate_media_order(media: &[PublishMediaRef], issues: &mut Vec<PublishIssue>) {
    let mut orders = HashSet::new();
    for (index, item) in media.iter().enumerate() {
        if item.order == 0 {
            issues.push(issue(
                &format!("media[{index}].order"),
                "invalid_order",
                "素材顺序必须从 1 开始",
            ));
        }
        if !orders.insert(item.order) {
            issues.push(issue(
                &format!("media[{index}].order"),
                "duplicate_order",
                format!("素材顺序 {} 重复", item.order),
            ));
        }
    }
    let expected = (1..=media.len() as u32).collect::<Vec<_>>();
    let mut actual = media.iter().map(|item| item.order).collect::<Vec<_>>();
    actual.sort_unstable();
    if !media.is_empty() && actual != expected {
        issues.push(issue(
            "media",
            "non_contiguous_order",
            "素材顺序必须连续，不能跳号",
        ));
    }
}

fn cover_is_image(cover: &PublishMediaRef, field: &str, issues: &mut Vec<PublishIssue>) {
    valid_asset_reference(cover, field, issues);
    if cover.kind != MediaKind::Image {
        issues.push(issue(
            &format!("{field}.kind"),
            "invalid_cover_kind",
            "封面必须是图片素材",
        ));
    }
}

pub fn validate_publish_payload(input: &PublishInput) -> PublishValidationResult {
    let mut issues = Vec::new();
    if input.title.trim().is_empty() {
        issues.push(issue("title", "missing_title", "发布标题不能为空"));
    } else if input.title.chars().count() > 200 {
        issues.push(issue(
            "title",
            "title_too_long",
            "发布标题不能超过 200 个字符",
        ));
    }

    for (index, media) in input.media.iter().enumerate() {
        valid_asset_reference(media, &format!("media[{index}]"), &mut issues);
    }
    validate_media_order(&input.media, &mut issues);

    let media_order = {
        let mut ordered = input
            .media
            .iter()
            .map(|item| item.order)
            .collect::<Vec<_>>();
        ordered.sort_unstable();
        ordered
    };

    match input.content_kind {
        ContentKind::Video => {
            let videos = input
                .media
                .iter()
                .filter(|item| item.kind == MediaKind::Video)
                .count();
            if videos == 0 {
                issues.push(issue(
                    "media",
                    "missing_video_file",
                    "视频发布必须包含至少一个视频素材",
                ));
            }
            if input.media.iter().any(|item| item.kind == MediaKind::Image) {
                issues.push(issue(
                    "media",
                    "unexpected_video_media",
                    "视频发布的正文素材必须是视频文件；图片应放在 cover",
                ));
            }
            match &input.cover {
                Some(cover) => cover_is_image(cover, "cover", &mut issues),
                None => issues.push(issue(
                    "cover",
                    "missing_video_cover",
                    "视频发布必须包含封面图片",
                )),
            }
        }
        ContentKind::ImageText | ContentKind::ImageArticle => {
            if input.media.is_empty() {
                issues.push(issue(
                    "media",
                    "missing_images",
                    "图文发布必须包含至少一张图片素材",
                ));
            }
            if input.media.iter().any(|item| item.kind != MediaKind::Image) {
                issues.push(issue(
                    "media",
                    "invalid_image_kind",
                    "图文发布只能包含图片素材",
                ));
            }
            match &input.cover {
                Some(cover) => cover_is_image(cover, "cover", &mut issues),
                None => issues.push(issue(
                    "cover",
                    "missing_image_cover",
                    "图文发布必须明确封面图片",
                )),
            }
            if input.content_kind == ContentKind::ImageArticle
                && input
                    .body
                    .as_deref()
                    .map(str::trim)
                    .filter(|body| !body.is_empty())
                    .is_none()
            {
                issues.push(issue(
                    "body",
                    "missing_article_body",
                    "图文文章必须包含正文",
                ));
            }
        }
        ContentKind::Article => {
            if input
                .body
                .as_deref()
                .map(str::trim)
                .filter(|body| !body.is_empty())
                .is_none()
            {
                issues.push(issue(
                    "body",
                    "missing_article_body",
                    "文章发布必须包含正文",
                ));
            }
            if input.media.iter().any(|item| item.kind == MediaKind::Video) {
                issues.push(issue(
                    "media",
                    "unexpected_article_video",
                    "纯文章发布不能包含视频素材",
                ));
            }
            if let Some(cover) = &input.cover {
                cover_is_image(cover, "cover", &mut issues);
            }
        }
    }

    PublishValidationResult {
        ok: issues.is_empty(),
        content_kind: input.content_kind,
        issues,
        media_order,
        cover_asset_id: input.cover.as_ref().map(|cover| cover.asset_id.clone()),
    }
}

fn image(asset_id: &str, order: u32) -> PublishMediaRef {
    PublishMediaRef {
        asset_id: asset_id.to_string(),
        kind: MediaKind::Image,
        mime_type: "image/png".to_string(),
        byte_size: 128,
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

pub fn publish_payload_self_check() -> Vec<PublishPayloadCheckResult> {
    let cases = vec![
        (
            "video-complete",
            ContentKind::Video,
            true,
            PublishInput {
                content_kind: ContentKind::Video,
                title: "视频".into(),
                body: Some("描述".into()),
                media: vec![video("asset-video")],
                cover: Some(image("asset-video-cover", 1)),
            },
        ),
        (
            "video-missing-media",
            ContentKind::Video,
            false,
            PublishInput {
                content_kind: ContentKind::Video,
                title: "视频".into(),
                body: None,
                media: vec![],
                cover: Some(image("asset-video-cover", 1)),
            },
        ),
        (
            "video-missing-cover",
            ContentKind::Video,
            false,
            PublishInput {
                content_kind: ContentKind::Video,
                title: "视频".into(),
                body: None,
                media: vec![video("asset-video")],
                cover: None,
            },
        ),
        (
            "image-text-order-and-cover",
            ContentKind::ImageText,
            true,
            PublishInput {
                content_kind: ContentKind::ImageText,
                title: "图文".into(),
                body: Some("说明".into()),
                media: vec![image("asset-image-1", 1), image("asset-image-2", 2)],
                cover: Some(image("asset-image-cover", 1)),
            },
        ),
        (
            "image-text-missing-cover",
            ContentKind::ImageText,
            false,
            PublishInput {
                content_kind: ContentKind::ImageText,
                title: "图文".into(),
                body: None,
                media: vec![image("asset-image-1", 1)],
                cover: None,
            },
        ),
        (
            "image-article-missing-body",
            ContentKind::ImageArticle,
            false,
            PublishInput {
                content_kind: ContentKind::ImageArticle,
                title: "图文文章".into(),
                body: Some("   ".into()),
                media: vec![image("asset-image-1", 1)],
                cover: Some(image("asset-image-cover", 1)),
            },
        ),
        (
            "article-complete",
            ContentKind::Article,
            true,
            PublishInput {
                content_kind: ContentKind::Article,
                title: "文章".into(),
                body: Some("正文".into()),
                media: vec![],
                cover: None,
            },
        ),
        (
            "article-missing-body",
            ContentKind::Article,
            false,
            PublishInput {
                content_kind: ContentKind::Article,
                title: "文章".into(),
                body: None,
                media: vec![],
                cover: None,
            },
        ),
        (
            "image-text-duplicate-order",
            ContentKind::ImageText,
            false,
            PublishInput {
                content_kind: ContentKind::ImageText,
                title: "图文".into(),
                body: None,
                media: vec![image("asset-image-1", 1), image("asset-image-2", 1)],
                cover: Some(image("asset-image-cover", 1)),
            },
        ),
    ];

    cases
        .into_iter()
        .map(|(id, kind, expected_ok, input)| {
            let result = validate_publish_payload(&input);
            let passed = result.ok == expected_ok;
            PublishPayloadCheckResult {
                id,
                content_kind: kind,
                expected_ok,
                actual_ok: result.ok,
                passed,
                message: if passed {
                    "符合预期".to_string()
                } else {
                    result
                        .issues
                        .iter()
                        .map(|issue| issue.message.as_str())
                        .collect::<Vec<_>>()
                        .join("；")
                },
            }
        })
        .collect()
}

pub fn all_content_kinds_covered() -> bool {
    CONTENT_KINDS.iter().all(|kind| {
        publish_payload_self_check()
            .iter()
            .any(|case| case.content_kind == *kind)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m4_ac_11_payload_checks_cover_every_content_kind() {
        assert!(all_content_kinds_covered());
        let results = publish_payload_self_check();
        assert!(results.iter().all(|result| result.passed));
        assert!(results.len() >= 8);
    }

    #[test]
    fn m4_ac_11_video_requires_file_and_cover_before_publish() {
        let missing_cover = PublishInput {
            content_kind: ContentKind::Video,
            title: "视频".into(),
            body: None,
            media: vec![video("asset-video")],
            cover: None,
        };
        let result = validate_publish_payload(&missing_cover);
        assert!(!result.ok);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == "missing_video_cover")
        );

        let missing_file = PublishInput {
            content_kind: ContentKind::Video,
            title: "视频".into(),
            body: None,
            media: vec![],
            cover: Some(image("cover", 1)),
        };
        assert!(
            validate_publish_payload(&missing_file)
                .issues
                .iter()
                .any(|issue| issue.code == "missing_video_file")
        );
    }

    #[test]
    fn m4_ac_11_image_order_and_cover_are_preserved() {
        let input = PublishInput {
            content_kind: ContentKind::ImageText,
            title: "图文".into(),
            body: None,
            media: vec![image("first", 1), image("second", 2)],
            cover: Some(image("cover", 1)),
        };
        let result = validate_publish_payload(&input);
        assert!(result.ok);
        assert_eq!(result.media_order, vec![1, 2]);
        assert_eq!(result.cover_asset_id.as_deref(), Some("cover"));
    }

    #[test]
    fn m4_ac_11_article_requires_body() {
        let result = validate_publish_payload(&PublishInput {
            content_kind: ContentKind::Article,
            title: "文章".into(),
            body: Some(" ".into()),
            media: vec![],
            cover: None,
        });
        assert!(!result.ok);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.code == "missing_article_body")
        );
    }
}
