use chrono::Utc;
use regex::Regex;
use serde_json::json;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::LazyLock,
};

const MAX_LOG_MESSAGE_CHARS: usize = 4_000;

struct RedactionRule {
    pattern: Regex,
    replacement: &'static str,
}

static REDACTION_RULES: LazyLock<Vec<RedactionRule>> = LazyLock::new(|| {
    vec![
        RedactionRule {
            pattern: Regex::new(
                r#"(?i)(\"(?:password|passwd|pwd|cookie|token|access_token|refresh_token|api[_-]?key|secret|authorization|sessdata)\"\s*:\s*\")[^\"]*(\")"#,
            )
            .expect("valid JSON redaction regex"),
            replacement: "$1[REDACTED]$2",
        },
        RedactionRule {
            pattern: Regex::new(r"(?im)^(\s*(?:cookie|authorization|x-api-key)\s*:\s*).*$")
                .expect("valid header redaction regex"),
            replacement: "$1[REDACTED]",
        },
        RedactionRule {
            pattern: Regex::new(
                r"(?i)\b(password|passwd|pwd|cookie|token|access_token|refresh_token|api[_-]?key|secret|authorization|sessdata)=([^&\s]+)",
            )
            .expect("valid key-value redaction regex"),
            replacement: "$1=[REDACTED]",
        },
        RedactionRule {
            pattern: Regex::new(r"(?i)\b(Bearer\s+)[A-Za-z0-9._~+/=-]+")
                .expect("valid bearer redaction regex"),
            replacement: "$1[REDACTED]",
        },
    ]
});

pub fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join("logs").join("creatorrelay.log")
}

pub fn initialize(data_dir: &Path) -> Result<PathBuf, String> {
    let path = log_path(data_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| "无法创建日志目录".to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(parent)
                .map_err(|_| "无法读取日志目录权限".to_string())?
                .permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(parent, permissions)
                .map_err(|_| "无法设置日志目录权限".to_string())?;
        }
    }
    if !path.exists() {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|_| "无法创建应用日志文件".to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&path)
            .map_err(|_| "无法读取日志文件权限".to_string())?
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&path, permissions).map_err(|_| "无法设置日志文件权限".to_string())?;
    }
    Ok(path)
}

pub fn redact_sensitive(input: &str) -> String {
    let mut output = input.to_string();
    for rule in REDACTION_RULES.iter() {
        output = rule
            .pattern
            .replace_all(&output, |captures: &regex::Captures<'_>| {
                let mut expanded = String::new();
                captures.expand(rule.replacement, &mut expanded);
                expanded
            })
            .into_owned();
    }
    output
}

fn sanitize_message(message: &str) -> String {
    let redacted = redact_sensitive(message).replace(['\r', '\n'], " | ");
    redacted.chars().take(MAX_LOG_MESSAGE_CHARS).collect()
}

pub fn append_log(log_path: &Path, level: &str, stage: &str, message: &str) -> Result<(), String> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|_| "无法创建日志目录".to_string())?;
    }
    let line = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "level": level,
        "stage": stage,
        "message": sanitize_message(message),
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|_| "无法打开应用日志文件".to_string())?;
    writeln!(file, "{line}").map_err(|_| "无法写入应用日志文件".to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = file
            .metadata()
            .map_err(|_| "无法读取应用日志文件权限".to_string())?
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(log_path, permissions)
            .map_err(|_| "无法设置应用日志文件权限".to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_json_headers_and_key_values_without_losing_context() {
        let input = concat!(
            "request failed context=workspace-open ",
            "{\"password\":\"p@ss-value\",\"cookie\":\"SESSDATA=cookie-value\",",
            "\"apiKey\":\"key-value\",\"message\":\"cannot open workspace\"} ",
            "Cookie: SESSDATA=header-cookie\n",
            "Authorization: Bearer bearer-value\n",
            "token=query-token api_key=api-key-value"
        );
        let output = redact_sensitive(input);
        for secret in [
            "p@ss-value",
            "cookie-value",
            "key-value",
            "header-cookie",
            "bearer-value",
            "query-token",
            "api-key-value",
        ] {
            assert!(
                !output.contains(secret),
                "secret leaked: {secret}\n{output}"
            );
        }
        assert!(output.contains("context=workspace-open"));
        assert!(output.contains("cannot open workspace"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn log_file_contains_stage_and_context_but_not_secret() {
        let temp = tempfile::tempdir().unwrap();
        let path = initialize(temp.path()).unwrap();
        append_log(
            &path,
            "error",
            "ipc.workspace.open",
            "失败 request={\"password\":\"file-secret\",\"cookie\":\"SESSDATA=file-cookie\"} context=workspace-open",
        )
        .unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        assert!(contents.contains("ipc.workspace.open"));
        assert!(contents.contains("workspace-open"));
        assert!(contents.contains("[REDACTED]"));
        assert!(!contents.contains("file-secret"));
        assert!(!contents.contains("file-cookie"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o077, 0);
        }
    }
}
