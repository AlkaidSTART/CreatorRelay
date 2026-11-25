use super::outcome::{CoreToolExecution, success_result};
use super::*;

#[derive(Debug, Clone)]
struct AssetReference {
    source_content_id: String,
    asset: content::ContentAsset,
    order: u32,
    caption: String,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedContent {
    content_id: Option<String>,
    kind: ContentKind,
    title: String,
    summary: String,
    tags: Vec<String>,
    body: String,
    base_hash: Option<String>,
    media_refs: Option<Vec<AssetReference>>,
    cover_ref: Option<AssetReference>,
    existing_media: Vec<content::ContentAsset>,
    existing_cover: Option<content::ContentAsset>,
}

pub(super) fn field_error(path: &str, message: impl Into<String>) -> McpFieldError {
    McpFieldError {
        path: path.to_string(),
        message: message.into(),
    }
}

fn parse_asset_reference(
    workspace: &Path,
    value: &Value,
    path: &str,
    require_order: bool,
) -> Result<AssetReference, McpFieldError> {
    let source_content_id = value
        .get("sourceContentId")
        .and_then(Value::as_str)
        .ok_or_else(|| field_error(&format!("{path}.sourceContentId"), "缺少来源内容 ID"))?;
    let asset_id = value
        .get("assetId")
        .and_then(Value::as_str)
        .ok_or_else(|| field_error(&format!("{path}.assetId"), "缺少素材 ID"))?;
    let order = if require_order {
        value
            .get("order")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| field_error(&format!("{path}.order"), "缺少有效素材顺序"))?
    } else {
        1
    };
    let caption = value
        .get("caption")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let package = content::read_content_package(workspace, source_content_id).map_err(|error| {
        field_error(
            &format!("{path}.sourceContentId"),
            format!("来源内容不可用：{error}"),
        )
    })?;
    let asset = package
        .manifest
        .media
        .iter()
        .chain(package.manifest.cover.iter())
        .find(|asset| asset.id == asset_id)
        .cloned()
        .ok_or_else(|| field_error(&format!("{path}.assetId"), "素材 ID 不属于来源内容包"))?;
    Ok(AssetReference {
        source_content_id: source_content_id.to_string(),
        asset,
        order,
        caption,
    })
}

fn parse_media_references(
    workspace: &Path,
    arguments: &Value,
    _existing: bool,
) -> Result<Option<Vec<AssetReference>>, Vec<McpFieldError>> {
    let Some(media) = arguments.get("media") else {
        return Ok(None);
    };
    let Some(items) = media.as_array() else {
        return Err(vec![field_error("media", "media 必须是数组")]);
    };
    let mut errors = Vec::new();
    let mut refs = Vec::new();
    let mut orders = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        match parse_asset_reference(workspace, item, &format!("media[{index}]"), true) {
            Ok(reference) => {
                if !orders.insert(reference.order) {
                    errors.push(field_error(
                        &format!("media[{index}].order"),
                        "素材顺序不能重复",
                    ));
                }
                refs.push(reference);
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(Some(refs))
    } else {
        Err(errors)
    }
}

fn parse_cover_reference(
    workspace: &Path,
    arguments: &Value,
) -> Result<Option<AssetReference>, Vec<McpFieldError>> {
    let Some(cover) = arguments.get("cover") else {
        return Ok(None);
    };
    if cover.is_null() {
        return Ok(None);
    }
    parse_asset_reference(workspace, cover, "cover", false)
        .map(Some)
        .map_err(|error| vec![error])
}

fn parse_content_kind(value: &str) -> ContentKind {
    match value {
        "video" => ContentKind::Video,
        "imageText" => ContentKind::ImageText,
        "imageArticle" => ContentKind::ImageArticle,
        "article" => ContentKind::Article,
        _ => ContentKind::Article,
    }
}

fn validate_content_payload(payload: &PreparedContent) -> Vec<McpFieldError> {
    let mut errors = Vec::new();
    let media = payload
        .media_refs
        .as_deref()
        .map(|refs| {
            refs.iter()
                .map(|reference| &reference.asset)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| payload.existing_media.iter().collect::<Vec<_>>());
    let has_video = media
        .iter()
        .any(|asset| asset.kind == content::AssetKind::Video);
    let has_image = media
        .iter()
        .any(|asset| asset.kind == content::AssetKind::Image);
    let cover = payload
        .cover_ref
        .as_ref()
        .map(|reference| &reference.asset)
        .or(payload.existing_cover.as_ref());
    match payload.kind {
        ContentKind::Video => {
            if !has_video {
                errors.push(field_error("media", "视频内容必须包含至少一个视频文件"));
            }
            if has_image {
                errors.push(field_error(
                    "media",
                    "视频内容的媒体列表只能包含视频文件；图片请通过 cover 提供",
                ));
            }
            if !payload.body.trim().is_empty() {
                errors.push(field_error(
                    "body",
                    "视频内容不使用 Markdown 正文，请写入 summary",
                ));
            }
            if cover.is_none() {
                errors.push(field_error("cover", "视频内容必须提供封面图片"));
            }
        }
        ContentKind::ImageText => {
            if media.is_empty() {
                errors.push(field_error("media", "图文内容至少需要一张图片"));
            }
            if !has_image || has_video {
                errors.push(field_error("media", "图文媒体顺序只能包含图片"));
            }
            if !payload.body.trim().is_empty() {
                errors.push(field_error(
                    "body",
                    "图文内容不使用 Markdown 正文，请写入 summary 和逐图说明",
                ));
            }
            if cover.is_none() {
                errors.push(field_error("cover", "图文内容必须明确封面图片"));
            }
        }
        ContentKind::ImageArticle => {
            if payload.body.trim().is_empty() {
                errors.push(field_error("body", "图文文章正文不能为空"));
            }
            if media.is_empty() {
                errors.push(field_error("media", "图文文章至少需要一张图片"));
            }
            if !has_image || has_video {
                errors.push(field_error("media", "图文文章媒体顺序只能包含图片"));
            }
            if cover.is_none() {
                errors.push(field_error("cover", "图文文章必须明确封面图片"));
            }
        }
        ContentKind::Article => {
            if payload.body.trim().is_empty() {
                errors.push(field_error("body", "纯文章正文不能为空"));
            }
        }
    }
    if let Some(cover) = cover
        && cover.kind != content::AssetKind::Image
    {
        errors.push(field_error("cover", "封面必须是图片素材"));
    }
    errors
}

pub(super) fn prepare_create_content(
    workspace: &Path,
    arguments: &Value,
) -> Result<PreparedContent, Vec<McpFieldError>> {
    let kind_value = arguments
        .get("contentKind")
        .and_then(Value::as_str)
        .unwrap_or("article");
    let kind = parse_content_kind(kind_value);
    let title = arguments
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string();
    let mut errors = Vec::new();
    if title.is_empty() {
        errors.push(field_error("title", "标题不能为空"));
    }
    let media_refs = match parse_media_references(workspace, arguments, false) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            None
        }
    };
    let cover_ref = match parse_cover_reference(workspace, arguments) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            None
        }
    };
    let payload = PreparedContent {
        content_id: None,
        kind,
        title,
        summary: arguments
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
        tags: arguments
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        body: arguments
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        base_hash: None,
        media_refs,
        cover_ref,
        existing_media: Vec::new(),
        existing_cover: None,
    };
    errors.extend(validate_content_payload(&payload));
    if errors.is_empty() {
        Ok(payload)
    } else {
        Err(errors)
    }
}

pub(super) fn prepare_update_content(
    workspace: &Path,
    arguments: &Value,
) -> Result<PreparedContent, Vec<McpFieldError>> {
    let content_id = arguments
        .get("contentId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let package = content::read_content_package(workspace, content_id)
        .map_err(|error| vec![field_error("contentId", error)])?;
    let mut errors = Vec::new();
    let media_refs = match parse_media_references(workspace, arguments, true) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            Some(Vec::new())
        }
    };
    let cover_ref = match parse_cover_reference(workspace, arguments) {
        Ok(value) => value,
        Err(mut value) => {
            errors.append(&mut value);
            None
        }
    };
    let payload = PreparedContent {
        content_id: Some(package.manifest.id.clone()),
        kind: package.manifest.kind,
        title: arguments
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| package.manifest.title.clone()),
        summary: arguments
            .get("summary")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| package.manifest.summary.clone()),
        tags: arguments
            .get("tags")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_else(|| package.manifest.tags.clone()),
        body: arguments
            .get("body")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| package.body.clone()),
        base_hash: arguments
            .get("baseHash")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some(package.content_hash.clone())),
        media_refs,
        cover_ref,
        existing_media: package.manifest.media.clone(),
        existing_cover: package.manifest.cover.clone(),
    };
    errors.extend(validate_content_payload(&payload));
    if errors.is_empty() {
        Ok(payload)
    } else {
        Err(errors)
    }
}

fn prepared_media_summary(payload: &PreparedContent) -> Value {
    let media = payload
        .media_refs
        .as_ref()
        .map(|refs| {
            refs.iter()
                .map(|reference| {
                    json!({
                        "assetId": reference.asset.id,
                        "kind": match reference.asset.kind {
                            content::AssetKind::Video => "video",
                            content::AssetKind::Image => "image",
                        },
                        "originalName": reference.asset.original_name,
                        "mimeType": reference.asset.mime_type,
                        "byteSize": reference.asset.byte_size,
                        "order": reference.order,
                        "caption": reference.caption,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| {
            payload
                .existing_media
                .iter()
                .map(|asset| {
                    json!({
                        "assetId": asset.id,
                        "kind": match asset.kind {
                            content::AssetKind::Video => "video",
                            content::AssetKind::Image => "image",
                        },
                        "originalName": asset.original_name,
                        "mimeType": asset.mime_type,
                        "byteSize": asset.byte_size,
                        "order": asset.order,
                        "caption": asset.caption,
                    })
                })
                .collect::<Vec<_>>()
        });
    Value::Array(media)
}

pub(super) fn content_preview_details(payload: &PreparedContent) -> Value {
    let cover = payload
        .cover_ref
        .as_ref()
        .map(|reference| &reference.asset)
        .or(payload.existing_cover.as_ref());
    json!({
        "contentId": payload.content_id,
        "contentKind": payload.kind.as_str(),
        "title": payload.title,
        "summary": payload.summary,
        "body": payload.body,
        "bodyPreview": payload.body.chars().take(600).collect::<String>(),
        "bodyTruncated": payload.body.chars().count() > 600,
        "media": prepared_media_summary(payload),
        "cover": cover.map(|asset| json!({
            "assetId": asset.id,
            "originalName": asset.original_name,
            "mimeType": asset.mime_type,
            "byteSize": asset.byte_size,
        })),
    })
}

fn materialize_reference(
    workspace: &Path,
    target_id: &str,
    reference: &AssetReference,
    role: &str,
) -> Result<content::ContentAsset, String> {
    let bytes =
        content::read_content_asset(workspace, &reference.source_content_id, &reference.asset.id)?;
    let mut asset = content::import_content_asset(
        workspace,
        target_id,
        role,
        &reference.asset.original_name,
        &reference.asset.mime_type,
        &bytes.bytes,
    )?;
    asset.order = reference.order;
    asset.caption = reference.caption.clone();
    Ok(asset)
}

pub(super) fn execute_prepared_content(
    workspace: &Path,
    payload: &PreparedContent,
) -> Result<Value, McpCallError> {
    let (target_id, cleanup_on_error) = if let Some(id) = payload.content_id.as_deref() {
        (id.to_string(), false)
    } else {
        let package = content::create_content_package(workspace, payload.kind, &payload.title)
            .map_err(|error| McpCallError {
                code: "CONTENT_CREATE_FAILED".to_string(),
                message: error,
                field_errors: Vec::new(),
            })?;
        (package.manifest.id, true)
    };
    let result = (|| {
        let media = if let Some(references) = &payload.media_refs {
            let mut assets = Vec::new();
            for reference in references {
                assets.push(materialize_reference(
                    workspace, &target_id, reference, "media",
                )?);
            }
            assets
        } else {
            payload.existing_media.clone()
        };
        let cover = if let Some(reference) = &payload.cover_ref {
            Some(materialize_reference(
                workspace, &target_id, reference, "cover",
            )?)
        } else {
            payload.existing_cover.clone()
        };
        let package = content::save_content_package(
            workspace,
            &target_id,
            ContentSaveInput {
                title: payload.title.clone(),
                summary: payload.summary.clone(),
                tags: payload.tags.clone(),
                cover,
                media,
                body: payload.body.clone(),
                base_hash: payload.base_hash.clone(),
            },
        )?;
        Ok::<_, String>(json!({
            "contentId": package.manifest.id,
            "contentKind": package.manifest.kind.as_str(),
            "title": package.manifest.title,
            "contentVersion": package.content_version,
            "mediaCount": package.manifest.media.len(),
            "hasCover": package.manifest.cover.is_some(),
            "saved": true,
        }))
    })();
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            if cleanup_on_error {
                let _ = content::delete_content_package(workspace, &target_id);
            }
            Err(McpCallError {
                code: "CONTENT_SAVE_FAILED".to_string(),
                message: error,
                field_errors: Vec::new(),
            })
        }
    }
}

pub(super) fn execute_content_tool(
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
    scope: &ModelContextScope,
) -> Result<CoreToolExecution, McpCallError> {
    let result = match tool_name {
        "content_list" => {
            let packages =
                content::list_content_packages(workspace).map_err(|error| McpCallError {
                    code: "CONTENT_SERVICE_ERROR".to_string(),
                    message: error,
                    field_errors: Vec::new(),
                })?;
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase();
            let kind = arguments.get("contentKind").and_then(Value::as_str);
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(20)
                .min(100) as usize;
            let items = packages
                .into_iter()
                .filter(|package| {
                    kind.is_none_or(|value| package.kind.as_str() == value)
                        && (query.is_empty()
                            || format!(
                                "{} {} {}",
                                package.title,
                                package.summary,
                                package.tags.join(" ")
                            )
                            .to_lowercase()
                            .contains(&query))
                })
                .take(limit)
                .map(|package| {
                    json!({
                        "contentId": package.id,
                        "contentKind": package.kind.as_str(),
                        "title": package.title,
                        "summary": package.summary,
                        "tags": package.tags,
                        "updatedAt": package.updated_at,
                        "hasCover": package.has_cover,
                        "mediaCount": package.media_count,
                        "contentVersion": package.content_version,
                    })
                })
                .collect::<Vec<_>>();
            success_result(json!({ "items": items, "count": items.len() }))
        }
        "content_get_detail" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let package =
                content::read_content_package(workspace, content_id).map_err(|error| {
                    McpCallError {
                        code: "CONTENT_NOT_FOUND".to_string(),
                        message: error,
                        field_errors: vec![field_error("contentId", "内容无法读取")],
                    }
                })?;
            let mut detail = json!({
                "contentId": package.manifest.id,
                "contentKind": package.manifest.kind.as_str(),
                "title": if scope.include_title { Value::String(package.manifest.title.clone()) } else { Value::Null },
                "summary": if scope.include_summary { Value::String(package.manifest.summary.clone()) } else { Value::Null },
                "tags": package.manifest.tags,
                "updatedAt": package.manifest.updated_at,
                "contentVersion": package.content_version,
                "bodyIncluded": scope.include_body && package.manifest.kind.requires_markdown(),
                "media": if scope.include_media_metadata { package.manifest.media.iter().map(|asset| json!({
                    "assetId": asset.id,
                    "kind": match asset.kind {
                        content::AssetKind::Video => "video",
                        content::AssetKind::Image => "image",
                    },
                    "originalName": asset.original_name,
                    "mimeType": asset.mime_type,
                    "byteSize": asset.byte_size,
                    "order": asset.order,
                    "caption": asset.caption,
                })).collect::<Vec<_>>() } else { Vec::new() },
            });
            if scope.include_body && package.manifest.kind.requires_markdown() {
                detail["body"] = Value::String(redact_text(&package.body));
            }
            success_result(detail)
        }
        "studio_create_content" => {
            let payload = prepare_create_content(workspace, arguments).map_err(|field_errors| {
                McpCallError {
                    code: "CONTENT_VALIDATION_FAILED".to_string(),
                    message: "内容载荷校验失败，未创建草稿".to_string(),
                    field_errors,
                }
            })?;
            let result = execute_prepared_content(workspace, &payload)?;
            success_result(result)
        }
        "studio_update_content" => {
            let payload = prepare_update_content(workspace, arguments).map_err(|field_errors| {
                McpCallError {
                    code: "CONTENT_VALIDATION_FAILED".to_string(),
                    message: "内容载荷校验失败，未写入草稿".to_string(),
                    field_errors,
                }
            })?;
            let result = execute_prepared_content(workspace, &payload)?;
            success_result(result)
        }
        "content_delete_draft" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            content::delete_content_package(workspace, content_id).map_err(|error| {
                McpCallError {
                    code: "CONTENT_DELETE_FAILED".to_string(),
                    message: error,
                    field_errors: vec![field_error("contentId", "内容删除失败")],
                }
            })?;
            success_result(json!({ "contentId": content_id, "deleted": true }))
        }
        _ => {
            return Err(McpCallError {
                code: "TOOL_NOT_FOUND".to_string(),
                message: format!("未注册内容工具：{tool_name}"),
                field_errors: Vec::new(),
            });
        }
    };
    Ok(result)
}

pub(super) fn preview_content_tool(
    workspace: &Path,
    tool_name: &str,
    arguments: &Value,
) -> Result<Value, Vec<McpFieldError>> {
    match tool_name {
        "studio_create_content" => prepare_create_content(workspace, arguments)
            .map(|payload| content_preview_details(&payload)),
        "studio_update_content" => prepare_update_content(workspace, arguments)
            .map(|payload| content_preview_details(&payload)),
        "content_delete_draft" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            content::read_content_package(workspace, content_id)
                .map(|package| {
                    json!({
                        "contentId": package.manifest.id,
                        "contentKind": package.manifest.kind.as_str(),
                        "title": package.manifest.title,
                        "media": package.manifest.media,
                        "hasCover": package.manifest.cover.is_some(),
                    })
                })
                .map_err(|error| vec![field_error("contentId", error)])
        }
        "content_get_detail" => {
            let content_id = arguments
                .get("contentId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            content::read_content_package(workspace, content_id)
                .map(|package| {
                    json!({
                        "contentId": package.manifest.id,
                        "contentKind": package.manifest.kind.as_str(),
                        "title": package.manifest.title,
                        "mediaCount": package.manifest.media.len(),
                    })
                })
                .map_err(|error| vec![field_error("contentId", error)])
        }
        _ => Ok(arguments.clone()),
    }
}
