use super::*;
use serde::{Deserialize, Serialize};

pub fn export_manual_publish_package(
    conn: &Connection,
    workspace: &Path,
    task_id: &str,
) -> Result<ManualPublishPackage, String> {
    let task = task_record(conn, task_id)?;
    let view = task.to_view()?;
    if !matches!(
        task.status,
        PublishTaskStatus::Created
            | PublishTaskStatus::NeedsManualAction
            | PublishTaskStatus::Failed
    ) {
        return Err("当前任务状态不能导出人工发布包".to_string());
    }
    validate_source_unchanged(workspace, &task)?;
    let safe_id = task
        .id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>();
    let directory = workspace.join("exports").join(format!(
        "manual-{safe_id}-{}",
        Utc::now().format("%Y%m%d%H%M%S")
    ));
    fs::create_dir_all(directory.join("assets"))
        .map_err(|error| format!("无法创建人工发布包目录：{error}"))?;
    let mut assets = Vec::new();
    let cover_id = task.cover()?.map(|cover| cover.id);
    for (index, asset) in task.media()?.into_iter().enumerate() {
        let role = if cover_id.as_deref() == Some(asset.id.as_str()) {
            "cover-and-media"
        } else {
            "media"
        };
        assets.push(copy_manual_asset(
            &directory,
            &task,
            asset,
            role,
            index + 1,
        )?);
    }
    if let Some(cover) = task.cover()?
        && !task.media()?.iter().any(|asset| asset.id == cover.id)
    {
        assets.push(copy_manual_asset(&directory, &task, cover, "cover", 0)?);
    }
    let body_path = directory.join("body.md");
    fs::write(&body_path, task.body.as_bytes())
        .map_err(|error| format!("无法写入人工发布正文：{error}"))?;
    let generated_at = Utc::now().to_rfc3339();
    let manifest = serde_json::json!({
        "schemaVersion": 1,
        "generatedAt": generated_at,
        "taskId": task.id,
        "contentId": task.content_id,
        "contentVersion": task.content_version,
        "contentKind": task.content_kind.as_str(),
        "platformId": task.platform_id,
        "platformName": task.platform_name,
        "accountId": task.account_id,
        "accountAlias": task.account_alias,
        "title": task.title,
        "bodyFile": "body.md",
        "tags": task.tags()?,
        "mediaOrder": assets.iter().filter(|asset| asset.role != "cover").map(|asset| asset.order).collect::<Vec<_>>(),
        "coverAssetId": cover_id,
        "assets": assets,
        "instructions": [
            "打开目标平台并选择对应账号",
            "按 assets 中 order 顺序上传媒体；role 为 cover 的素材设为封面",
            "将 body.md 内容粘贴或导入正文编辑器",
            "发布完成后在 CreatorRelay 中填写外部作品 ID/URL 并标记完成",
            "若结果不确定，请先到平台作品列表核对，避免重复发布"
        ]
    });
    let manifest_path = directory.join("publish-package.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("无法写入人工发布清单：{error}"))?;
    insert_task_event(
        conn,
        &task.id,
        "MANUAL_PACKAGE_EXPORTED",
        &directory.to_string_lossy(),
    )?;
    Ok(ManualPublishPackage {
        task: view,
        directory: directory.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        body_path: body_path.to_string_lossy().to_string(),
        assets,
        generated_at,
    })
}

fn copy_manual_asset(
    directory: &Path,
    task: &TaskRow,
    asset: ContentAsset,
    role: &str,
    index: usize,
) -> Result<ManualPackageAsset, String> {
    let bytes = match task.source_type {
        PublishSourceType::PlatformDraft => content::read_platform_draft_asset(
            &task_source_workspace(directory)?,
            &task.source_id,
            &asset.id,
        ),
        PublishSourceType::Content => content::read_content_asset(
            &task_source_workspace(directory)?,
            &task.content_id,
            &asset.id,
        ),
    };
    let content = bytes?;
    let extension = Path::new(&asset.file)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let prefix = if index == 0 {
        "cover".to_string()
    } else {
        format!("{index:02}")
    };
    let file_name = format!(
        "{prefix}-{role}-{}.{}",
        safe_component(&asset.original_name),
        extension
    );
    let relative = format!("assets/{file_name}");
    fs::write(directory.join(&relative), content.bytes)
        .map_err(|error| format!("无法复制人工发布素材：{error}"))?;
    Ok(ManualPackageAsset {
        asset_id: asset.id,
        kind: match asset.kind {
            content::AssetKind::Video => "video".to_string(),
            content::AssetKind::Image => "image".to_string(),
        },
        role: role.to_string(),
        order: asset.order,
        file: relative,
        original_name: asset.original_name,
        mime_type: content.mime_type,
        byte_size: asset.byte_size,
        caption: asset.caption,
    })
}

fn task_source_workspace(directory: &Path) -> Result<PathBuf, String> {
    directory
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "人工发布包无法定位工作区".to_string())
}

fn safe_component(value: &str) -> String {
    let output = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = output.trim_matches('-');
    if trimmed.is_empty() {
        "asset".to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
