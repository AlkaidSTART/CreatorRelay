use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Component, Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use uuid::Uuid;

use super::manifest::{AdapterManifest, RuntimeKind};

const MAX_STDERR_BYTES: usize = 8 * 1024;
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IsolationSummary {
    pub mode: String,
    pub strong: bool,
    pub network: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostAccessObservation {
    pub method: String,
    pub allowed: bool,
    pub code: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxProbeObservation {
    pub target: String,
    pub allowed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecuritySelfTestResult {
    pub plugin_id: String,
    pub sandboxed: bool,
    pub database_denied: bool,
    pub workspace_denied: bool,
    pub credential_denied: bool,
    pub all_denied: bool,
    pub observations: Vec<SandboxProbeObservation>,
}

#[derive(Clone)]
struct HostPolicy {
    grants: HashSet<String>,
    workspace: Option<PathBuf>,
    account_ref: Option<String>,
}

impl HostPolicy {
    fn empty() -> Self {
        Self {
            grants: HashSet::new(),
            workspace: None,
            account_ref: None,
        }
    }

    fn handle(&self, method: &str, params: &Value) -> Value {
        let denied = |code: &str, message: &str| {
            json!({
              "ok": false,
              "error": { "code": code, "message": message }
            })
        };

        match method {
            "credentials.get" => {
                if !self.grants.contains("account.credentials") {
                    return denied(
                        "PERMISSION_DENIED",
                        "用户尚未授予 account.credentials；插件不能读取账号会话或凭据",
                    );
                }
                json!({
                  "ok": true,
                  "result": {
                    "proxy": "credential-service",
                    "accountRef": self.account_ref,
                    "rawAccess": false,
                    "sessionScope": "publish-only"
                  }
                })
            }
            "workspace.read" => {
                if !self.grants.contains("workspace.read") {
                    return denied(
                        "PERMISSION_DENIED",
                        "用户尚未授予 workspace.read；插件不能读取工作区素材",
                    );
                }
                let Some(path) = params.get("path").and_then(Value::as_str) else {
                    return denied("INVALID_ARGUMENT", "workspace.read 缺少 path");
                };
                let Some(root) = &self.workspace else {
                    return denied("WORKSPACE_UNAVAILABLE", "当前没有活动工作区");
                };
                if !path_is_within_workspace(root, Path::new(path)) {
                    return denied("PATH_OUTSIDE_WORKSPACE", "请求路径不在当前授权工作区内");
                }
                let asset_hash = format!("{:x}", Sha256::digest(path.as_bytes()));
                json!({
                  "ok": true,
                  "result": {
                    "proxy": "asset-service",
                    "assetRef": format!("asset:{asset_hash}"),
                    "rawPath": false
                  }
                })
            }
            "network.request" => {
                if !self.grants.contains("network") {
                    return denied(
                        "PERMISSION_DENIED",
                        "用户尚未授予 network；插件不能通过 Core 发起网络请求",
                    );
                }
                json!({
                  "ok": true,
                  "result": {
                    "proxy": "network-proxy",
                    "rawAccess": false,
                    "status": "not_implemented_in_m4",
                    "message": "M4 仅验证网络权限门，发布网络代理在 M6 实现"
                  }
                })
            }
            "database.query" | "database.open" | "sqlite.open" => denied(
                "FORBIDDEN",
                "插件不能访问 Core SQLite；所有业务数据必须通过公开的 Core 服务",
            ),
            "keychain.read" | "credentials.readRaw" => denied(
                "FORBIDDEN_RAW_CREDENTIAL",
                "插件不能读取 Keychain 或原始 Cookie/Token",
            ),
            _ => denied("METHOD_NOT_FOUND", "Core 主机 API 不存在该方法"),
        }
    }
}

fn path_is_within_workspace(root: &Path, requested: &Path) -> bool {
    let normalized_requested = normalize_lexically(requested);
    let normalized_root = normalize_lexically(root);
    normalized_requested.starts_with(&normalized_root)
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut output = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                output.pop();
            }
            other => output.push(other.as_os_str()),
        }
    }
    output
}

pub struct PluginRuntime {
    root: PathBuf,
    data_root: PathBuf,
    database_path: PathBuf,
    runner_executable: PathBuf,
    sandbox_available: bool,
    processes: Mutex<HashMap<String, Arc<Mutex<AdapterProcess>>>>,
}

impl PluginRuntime {
    pub fn new(
        root: PathBuf,
        data_root: PathBuf,
        database_path: PathBuf,
        runner_executable: PathBuf,
    ) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|error| format!("无法创建插件目录：{error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::metadata(&root)
                .map_err(|error| format!("无法读取插件目录权限：{error}"))?
                .permissions();
            let mut permissions = permissions;
            permissions.set_mode(0o700);
            fs::set_permissions(&root, permissions)
                .map_err(|error| format!("无法设置插件目录权限：{error}"))?;
        }
        #[cfg(target_os = "macos")]
        let sandbox_available = Path::new("/usr/bin/sandbox-exec").exists();
        #[cfg(not(target_os = "macos"))]
        let sandbox_available = false;

        Ok(Self {
            root,
            data_root,
            database_path,
            runner_executable,
            sandbox_available,
            processes: Mutex::new(HashMap::new()),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn data_directory(&self) -> &Path {
        &self.data_root
    }

    pub fn plugin_directory(&self, plugin_id: &str) -> Result<PathBuf, String> {
        validate_plugin_identifier(plugin_id)?;
        Ok(self.root.join(plugin_id))
    }

    pub fn plugin_data_directory(&self, plugin_id: &str) -> Result<PathBuf, String> {
        validate_plugin_identifier(plugin_id)?;
        Ok(self.data_root.join("plugin-data").join(plugin_id))
    }

    pub fn isolation_summary(&self, manifest: &AdapterManifest) -> IsolationSummary {
        if self.sandbox_available {
            IsolationSummary {
                mode: "macos-sandbox".to_string(),
                strong: true,
                network: "controlled".to_string(),
                description:
                    "macOS sandbox-exec 限制插件文件与网络访问；Core 通过主机 API 代理授权操作"
                        .to_string(),
            }
        } else {
            let _ = manifest;
            IsolationSummary {
                mode: "process-only".to_string(),
                strong: false,
                network: "controlled".to_string(),
                description: "当前平台仅提供独立进程隔离，不是强安全沙箱；只允许官方适配器"
                    .to_string(),
            }
        }
    }

    pub fn is_running(&self, plugin_id: &str) -> bool {
        let mut processes = match self.processes.lock() {
            Ok(processes) => processes,
            Err(_) => return false,
        };
        if let Some(process) = processes.get(plugin_id) {
            let running = process
                .lock()
                .map(|mut process| process.is_alive())
                .unwrap_or(false);
            if !running {
                processes.remove(plugin_id);
            }
            running
        } else {
            false
        }
    }

    pub fn ensure_running(
        &self,
        manifest: &AdapterManifest,
        grants: &HashSet<String>,
    ) -> Result<(), String> {
        if self.is_running(&manifest.id) {
            return Ok(());
        }
        let plugin_dir = self.plugin_directory(&manifest.id)?;
        let data_dir = self.plugin_data_directory(&manifest.id)?;
        fs::create_dir_all(&data_dir).map_err(|error| format!("无法创建插件数据目录：{error}"))?;
        let (process, sandboxed) = self.spawn_process(manifest, &plugin_dir, &data_dir, grants)?;
        let process = Arc::new(Mutex::new(process));
        {
            let mut processes = self
                .processes
                .lock()
                .map_err(|_| "插件进程表不可用".to_string())?;
            if let Some(existing) = processes.get(&manifest.id)
                && existing
                    .lock()
                    .map(|mut existing| existing.is_alive())
                    .unwrap_or(false)
            {
                return Ok(());
            }
            processes.insert(manifest.id.clone(), Arc::clone(&process));
        }

        let describe = {
            let mut process = process.lock().map_err(|_| "插件进程锁不可用".to_string())?;
            process.request(
                "describe",
                json!({}),
                &HostPolicy::empty(),
                Duration::from_millis(manifest.runtime.timeout_ms),
            )
        };
        match describe {
            Ok(value) => {
                let id = value.get("id").and_then(Value::as_str).unwrap_or_default();
                let version = value
                    .get("version")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let contract = value
                    .get("adapterContractVersion")
                    .and_then(Value::as_u64)
                    .unwrap_or_default();
                if id != manifest.id
                    || version != manifest.version
                    || contract != manifest.adapter_contract_version as u64
                {
                    self.stop(&manifest.id);
                    return Err(format!(
                        "Adapter 自述身份与 Manifest 不一致：id={id} version={version} contract={contract} sandboxed={sandboxed}"
                    ));
                }
                Ok(())
            }
            Err(error) => {
                self.stop(&manifest.id);
                Err(error)
            }
        }
    }

    pub fn request(
        &self,
        manifest: &AdapterManifest,
        method: &str,
        params: Value,
        grants: &HashSet<String>,
        workspace: Option<PathBuf>,
        account_ref: Option<String>,
    ) -> Result<Value, String> {
        let process = {
            let processes = self
                .processes
                .lock()
                .map_err(|_| "插件进程表不可用".to_string())?;
            processes
                .get(&manifest.id)
                .cloned()
                .ok_or_else(|| "插件进程未启动".to_string())?
        };
        let policy = HostPolicy {
            grants: grants.clone(),
            workspace,
            account_ref,
        };
        let result = {
            let mut process = process.lock().map_err(|_| "插件进程锁不可用".to_string())?;
            process.request(
                method,
                params,
                &policy,
                Duration::from_millis(manifest.runtime.timeout_ms),
            )
        };
        if result.is_err() {
            self.stop(&manifest.id);
        }
        result
    }

    pub fn stop(&self, plugin_id: &str) {
        let process = self
            .processes
            .lock()
            .ok()
            .and_then(|mut processes| processes.remove(plugin_id));
        if let Some(process) = process
            && let Ok(mut process) = process.lock()
        {
            let _ = process.request(
                "shutdown",
                json!({}),
                &HostPolicy::empty(),
                SHUTDOWN_TIMEOUT,
            );
            process.wait_or_kill(SHUTDOWN_TIMEOUT);
        }
    }

    pub fn security_self_test(
        &self,
        manifest: &AdapterManifest,
        workspace_path: Option<PathBuf>,
        credential_canary: &Path,
    ) -> Result<SecuritySelfTestResult, String> {
        if !self.sandbox_available {
            return Err("当前平台未启用强隔离，不能执行的安全沙箱自检".to_string());
        }
        let plugin_dir = self.plugin_directory(&manifest.id)?;
        let data_dir = self.plugin_data_directory(&manifest.id)?;
        fs::create_dir_all(&data_dir).map_err(|error| format!("无法创建插件数据目录：{error}"))?;
        let (mut process, sandboxed) =
            self.spawn_process(manifest, &plugin_dir, &data_dir, &HashSet::new())?;
        let workspace_canary = workspace_path
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| "/__creatorrelay_missing_workspace__".to_string());
        let request = process.request(
            "securitySelfTest",
            json!({
              "databasePath": self.database_path.to_string_lossy(),
              "workspacePath": workspace_canary,
              "credentialPath": credential_canary.to_string_lossy()
            }),
            &HostPolicy::empty(),
            Duration::from_millis(manifest.runtime.timeout_ms.max(2_000)),
        );
        process.wait_or_kill(SHUTDOWN_TIMEOUT);
        let result = request?;
        let observations = result
            .get("observations")
            .cloned()
            .ok_or_else(|| "Adapter 安全自检响应缺少 observations".to_string())?;
        let observations = serde_json::from_value::<Vec<SandboxProbeObservation>>(observations)
            .map_err(|error| format!("Adapter 安全自检返回格式无效：{error}"))?;
        let database_denied = observations
            .iter()
            .find(|item| item.target == "database")
            .is_some_and(|item| !item.allowed);
        let workspace_denied = observations
            .iter()
            .find(|item| item.target == "workspace")
            .is_some_and(|item| !item.allowed);
        let credential_denied = observations
            .iter()
            .find(|item| item.target == "credential")
            .is_some_and(|item| !item.allowed);
        Ok(SecuritySelfTestResult {
            plugin_id: manifest.id.clone(),
            sandboxed,
            database_denied,
            workspace_denied,
            credential_denied,
            all_denied: database_denied && workspace_denied && credential_denied,
            observations,
        })
    }

    fn spawn_process(
        &self,
        manifest: &AdapterManifest,
        plugin_dir: &Path,
        data_dir: &Path,
        grants: &HashSet<String>,
    ) -> Result<(AdapterProcess, bool), String> {
        let sandboxed = self.sandbox_available;
        let mut command = if sandboxed {
            let profile = self.sandbox_profile(manifest, plugin_dir, data_dir, grants);
            let mut command = Command::new("/usr/bin/sandbox-exec");
            command.arg("-p").arg(profile).arg("--");
            command
        } else {
            Command::new(&self.runner_executable)
        };

        match manifest.runtime.kind {
            RuntimeKind::CoreTestRunner => {
                if !sandboxed {
                    command = Command::new(&self.runner_executable);
                } else {
                    command.arg(&self.runner_executable);
                }
                command.arg("--adapter-runner");
                command.arg("--profile");
                command.arg(manifest.runtime.profile.as_deref().unwrap_or("healthy"));
            }
            RuntimeKind::Executable => {
                let entry = safe_entry_path(plugin_dir, &manifest.entry)?;
                if sandboxed {
                    command.arg(&entry);
                } else {
                    command = Command::new(&entry);
                }
            }
        }

        command
            .current_dir(plugin_dir)
            .env_clear()
            .env("HOME", data_dir)
            .env("TMPDIR", data_dir.join("tmp"))
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("LANG", "en_US.UTF-8")
            .env("CREATORRELAY_ADAPTER_ID", &manifest.id)
            .env("CREATORRELAY_ADAPTER_VERSION", &manifest.version)
            .env("CREATORRELAY_PLUGIN_DIR", plugin_dir)
            .env("CREATORRELAY_PLUGIN_DATA_DIR", data_dir)
            .env(
                "CREATORRELAY_SANDBOX_ACTIVE",
                if sandboxed { "1" } else { "0" },
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| format!("无法启动 Adapter 进程：{error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "无法连接 Adapter stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "无法连接 Adapter stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "无法连接 Adapter stderr".to_string())?;
        let (sender, receiver) = mpsc::channel::<String>();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        let stderr_buffer = Arc::new(Mutex::new(String::new()));
        let stderr_clone = Arc::clone(&stderr_buffer);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut chunk = [0_u8; 1024];
            loop {
                match reader.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(read) => {
                        if let Ok(mut text) = stderr_clone.lock() {
                            text.push_str(&String::from_utf8_lossy(&chunk[..read]));
                            if text.len() > MAX_STDERR_BYTES {
                                let start = text.len() - MAX_STDERR_BYTES;
                                *text = text[start..].to_string();
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok((
            AdapterProcess {
                child,
                stdin,
                receiver,
                stderr: stderr_buffer,
                failed: false,
            },
            sandboxed,
        ))
    }

    fn sandbox_profile(
        &self,
        manifest: &AdapterManifest,
        plugin_dir: &Path,
        data_dir: &Path,
        grants: &HashSet<String>,
    ) -> String {
        let exe_parent = self
            .runner_executable
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("/"));
        // 网络授权只开放 Core 主机代理，不向插件进程开放原始 socket。
        let network = "(deny network*)\n";
        let _ = (manifest, grants);
        format!(
            r#"(version 1)
(deny default)
(allow process*)
(allow signal (target self))
(allow sysctl-read)
(allow mach-lookup)
(allow file-read-metadata (subpath "/"))
(allow file-read*
  (literal "/")
  (subpath "/System")
  (subpath "/usr")
  (subpath "/bin")
  (subpath "/sbin")
  (subpath "/Library/Apple")
  (subpath "/Library/Frameworks")
  (subpath "/private/var/db/dyld")
  (literal "/dev/null")
  (literal "/dev/urandom")
  (literal "/dev/random")
  {exe_parent_literal}
  {plugin_dir_literal}
  {data_dir_literal})
(allow file-write* {data_dir_literal})
{network}"#,
            exe_parent_literal = sandbox_subpath(&exe_parent),
            plugin_dir_literal = sandbox_subpath(plugin_dir),
            data_dir_literal = sandbox_subpath(data_dir),
        )
    }
}

fn validate_plugin_identifier(plugin_id: &str) -> Result<(), String> {
    if plugin_id.is_empty()
        || plugin_id.len() > 128
        || plugin_id.contains('/')
        || plugin_id.contains('\\')
        || plugin_id.contains("..")
        || !plugin_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
    {
        return Err("插件 ID 不合法".to_string());
    }
    Ok(())
}

fn safe_entry_path(plugin_dir: &Path, entry: &str) -> Result<PathBuf, String> {
    let entry_path = Path::new(entry);
    if entry_path.is_absolute()
        || entry_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("插件入口必须是插件目录内的相对路径".to_string());
    }
    let candidate = plugin_dir.join(entry_path);
    let canonical_plugin = plugin_dir
        .canonicalize()
        .map_err(|error| format!("无法解析插件目录：{error}"))?;
    let canonical_entry = candidate
        .canonicalize()
        .map_err(|error| format!("无法解析插件入口：{error}"))?;
    if !canonical_entry.starts_with(&canonical_plugin) {
        return Err("插件入口不能逃逸插件目录".to_string());
    }
    Ok(canonical_entry)
}

fn sandbox_subpath(path: &Path) -> String {
    format!(
        "(subpath \"{}\")",
        path.to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

struct AdapterProcess {
    child: Child,
    stdin: ChildStdin,
    receiver: mpsc::Receiver<String>,
    stderr: Arc<Mutex<String>>,
    failed: bool,
}

impl AdapterProcess {
    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn request(
        &mut self,
        method: &str,
        params: Value,
        policy: &HostPolicy,
        timeout: Duration,
    ) -> Result<Value, String> {
        if self.failed {
            return Err("Adapter 进程已因上一次超时或协议错误关闭".to_string());
        }
        let request_id = Uuid::new_v4().to_string();
        let request = json!({
          "type": "request",
          "id": request_id,
          "method": method,
          "params": params
        });
        let mut serialized = serde_json::to_vec(&request)
            .map_err(|error| format!("无法编码 Adapter 请求：{error}"))?;
        serialized.push(b'\n');
        self.stdin
            .write_all(&serialized)
            .and_then(|_| self.stdin.flush())
            .map_err(|error| format!("无法写入 Adapter stdin：{error}"))?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_else(|| Duration::from_millis(0));
            if remaining.is_zero() {
                self.fail_with_timeout(method);
                return Err(format!(
                    "Adapter 响应超时（{} ms，方法 {method}）",
                    timeout.as_millis()
                ));
            }
            let line = match self.receiver.recv_timeout(remaining) {
                Ok(line) => line,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    self.fail_with_timeout(method);
                    return Err(format!(
                        "Adapter 响应超时（{} ms，方法 {method}）",
                        timeout.as_millis()
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    let _ = self.child.wait();
                    self.failed = true;
                    let stderr = self.stderr_snapshot();
                    return Err(if stderr.is_empty() {
                        "Adapter 进程已退出或崩溃".to_string()
                    } else {
                        format!("Adapter 进程已退出或崩溃：{stderr}")
                    });
                }
            };
            let message: Value = serde_json::from_str(&line)
                .map_err(|error| format!("Adapter 返回非 JSON 协议消息：{error}"))?;
            match message.get("type").and_then(Value::as_str) {
                Some("hostCall") => {
                    let call_id = message
                        .get("callId")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "Adapter hostCall 缺少 callId".to_string())?;
                    let host_method = message
                        .get("method")
                        .and_then(Value::as_str)
                        .ok_or_else(|| "Adapter hostCall 缺少 method".to_string())?;
                    let host_params = message.get("params").cloned().unwrap_or(Value::Null);
                    let outcome = policy.handle(host_method, &host_params);
                    let host_result = json!({
                      "type": "hostResult",
                      "callId": call_id,
                      "ok": outcome.get("ok").and_then(Value::as_bool).unwrap_or(false),
                      "result": outcome.get("result").cloned().unwrap_or(Value::Null),
                      "error": outcome.get("error").cloned().unwrap_or(Value::Null)
                    });
                    let mut encoded = serde_json::to_vec(&host_result)
                        .map_err(|error| format!("无法编码 hostResult：{error}"))?;
                    encoded.push(b'\n');
                    self.stdin
                        .write_all(&encoded)
                        .and_then(|_| self.stdin.flush())
                        .map_err(|error| format!("无法写入 Adapter hostResult：{error}"))?;
                }
                Some("response") => {
                    if message.get("id").and_then(Value::as_str) != Some(request_id.as_str()) {
                        return Err("Adapter 返回了不匹配的请求 ID".to_string());
                    }
                    if message.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        return Ok(message.get("result").cloned().unwrap_or(Value::Null));
                    }
                    let error = message.get("error").cloned().unwrap_or(Value::Null);
                    let code = error
                        .get("code")
                        .and_then(Value::as_str)
                        .unwrap_or("ADAPTER_ERROR");
                    let text = error
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("Adapter 返回失败");
                    return Err(format!("{code}: {text}"));
                }
                Some(other) => {
                    self.failed = true;
                    return Err(format!("Adapter 协议违规：未知消息类型 `{other}`"));
                }
                None => {
                    self.failed = true;
                    return Err("Adapter 协议违规：消息缺少 type".to_string());
                }
            }
        }
    }

    fn fail_with_timeout(&mut self, method: &str) {
        self.failed = true;
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = method;
    }

    fn stderr_snapshot(&self) -> String {
        self.stderr
            .lock()
            .map(|text| text.trim().replace(['\r', '\n'], " | "))
            .unwrap_or_default()
    }

    fn wait_or_kill(&mut self, timeout: Duration) {
        let started = Instant::now();
        loop {
            match self.child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) if started.elapsed() < timeout => {
                    thread::sleep(Duration::from_millis(20));
                }
                _ => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return;
                }
            }
        }
    }
}

impl Drop for PluginRuntime {
    fn drop(&mut self) {
        let processes = self
            .processes
            .lock()
            .map(|mut processes| std::mem::take(&mut *processes))
            .unwrap_or_default();
        for process in processes.values() {
            if let Ok(mut process) = process.lock() {
                let _ = process.child.kill();
                let _ = process.child.wait();
            }
        }
    }
}
