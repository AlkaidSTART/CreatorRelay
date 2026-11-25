use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

pub(super) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn content_root(workspace: &Path) -> PathBuf {
    workspace.join("contents")
}

pub(super) fn draft_root(workspace: &Path) -> PathBuf {
    workspace.join("platform-drafts")
}

pub(super) fn validate_component(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(format!("{label}格式无效"));
    }
    Ok(())
}

pub(super) fn package_directory(workspace: &Path, package_id: &str) -> Result<PathBuf, String> {
    validate_component(package_id, "内容 ID")?;
    let root = content_root(workspace);
    let candidate = root.join(package_id);
    if root.exists() {
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
        if candidate.exists() {
            let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
            if !canonical.starts_with(&canonical_root) {
                return Err("内容路径超出工作区范围".to_string());
            }
        }
    }
    Ok(candidate)
}

pub(super) fn draft_directory(workspace: &Path, draft_id: &str) -> Result<PathBuf, String> {
    validate_component(draft_id, "平台稿 ID")?;
    let root = draft_root(workspace);
    let candidate = root.join(draft_id);
    if root.exists() {
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
        if candidate.exists() {
            let canonical = candidate.canonicalize().map_err(|e| e.to_string())?;
            if !canonical.starts_with(&canonical_root) {
                return Err("平台稿路径超出工作区范围".to_string());
            }
        }
    }
    Ok(candidate)
}

pub(super) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("目标文件没有父目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let mut temp = NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.error.to_string())?;
    Ok(())
}

pub(super) fn normalized_title(value: &str) -> Result<String, String> {
    let title = value.trim();
    if title.is_empty() {
        return Err("标题不能为空".to_string());
    }
    if title.chars().count() > MAX_CONTENT_TITLE_CHARS {
        return Err(format!("标题不能超过 {MAX_CONTENT_TITLE_CHARS} 个字符"));
    }
    if title.chars().any(char::is_control) {
        return Err("标题包含非法控制字符".to_string());
    }
    Ok(title.to_string())
}

pub(super) fn normalized_tags(tags: Vec<String>) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 40 {
            return Err("单个标签不能超过 40 个字符".to_string());
        }
        if !output.iter().any(|existing: &String| existing == tag) {
            output.push(tag.to_string());
        }
    }
    if output.len() > MAX_TAGS {
        return Err(format!("标签不能超过 {MAX_TAGS} 个"));
    }
    Ok(output)
}
