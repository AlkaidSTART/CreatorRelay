use super::schema::normalized_timeout;
use super::*;

struct ServerProcess {
    child: Child,
    stdin: ChildStdin,
    responses: Arc<Mutex<mpsc::Receiver<Value>>>,
}

impl ServerProcess {
    fn pid(&self) -> Option<u32> {
        Some(self.child.id())
    }

    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child.try_wait().map_err(|error| error.to_string())
    }
}

struct HostInner {
    executable: PathBuf,
    process: Mutex<Option<ServerProcess>>,
    request_lock: Mutex<()>,
    lifecycle_lock: Mutex<()>,
    config: Mutex<McpHostConfig>,
    restarts: AtomicUsize,
    next_request_id: AtomicU64,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    started_at: Mutex<Option<String>>,
    last_health_at: Mutex<Option<String>>,
    last_error: Mutex<Option<String>>,
    server_name: Mutex<String>,
    protocol_version: Mutex<String>,
}

#[derive(Clone)]
pub struct McpHost {
    inner: Arc<HostInner>,
}

impl McpHost {
    pub fn new(executable: PathBuf) -> Self {
        Self {
            inner: Arc::new(HostInner {
                executable,
                process: Mutex::new(None),
                request_lock: Mutex::new(()),
                lifecycle_lock: Mutex::new(()),
                config: Mutex::new(McpHostConfig::default()),
                restarts: AtomicUsize::new(0),
                next_request_id: AtomicU64::new(1),
                cancels: Mutex::new(HashMap::new()),
                started_at: Mutex::new(None),
                last_health_at: Mutex::new(None),
                last_error: Mutex::new(None),
                server_name: Mutex::new("CreatorRelay 测试 MCP Server".to_string()),
                protocol_version: Mutex::new("2024-11-05".to_string()),
            }),
        }
    }

    pub fn start(&self, config: Option<McpHostConfig>) -> Result<McpHostStatus, String> {
        let _lifecycle = self
            .inner
            .lifecycle_lock
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(config) = config {
            *self
                .inner
                .config
                .lock()
                .map_err(|error| error.to_string())? = config;
        }
        self.inner.restarts.store(0, Ordering::SeqCst);
        self.start_locked()?;
        self.status()
    }

    fn start_locked(&self) -> Result<(), String> {
        self.stop_locked(false)?;
        let mut child = Command::new(&self.inner.executable)
            .arg("--mcp-test-server")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("无法启动本地 MCP Server：{error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "MCP Server stdin 不可用".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "MCP Server stdout 不可用".to_string())?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else {
                    break;
                };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(value) => {
                        if sender.send(value).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        *self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())? = Some(ServerProcess {
            child,
            stdin,
            responses: Arc::new(Mutex::new(receiver)),
        });
        *self
            .inner
            .started_at
            .lock()
            .map_err(|error| error.to_string())? = Some(Utc::now().to_rfc3339());
        *self
            .inner
            .last_error
            .lock()
            .map_err(|error| error.to_string())? = None;

        let initialized = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "CreatorRelay", "version": "0.1.0" }
            }),
            StdDuration::from_secs(5),
        )?;
        let result = initialized
            .get("result")
            .ok_or_else(|| "MCP initialize 响应缺少 result".to_string())?;
        let server_name = result
            .pointer("/serverInfo/name")
            .and_then(Value::as_str)
            .unwrap_or("CreatorRelay 测试 MCP Server")
            .to_string();
        let protocol_version = result
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or("2024-11-05")
            .to_string();
        *self
            .inner
            .server_name
            .lock()
            .map_err(|error| error.to_string())? = server_name;
        *self
            .inner
            .protocol_version
            .lock()
            .map_err(|error| error.to_string())? = protocol_version;
        self.notify("notifications/initialized", json!({}))?;
        Ok(())
    }

    pub fn stop(&self) -> Result<McpHostStatus, String> {
        let _lifecycle = self
            .inner
            .lifecycle_lock
            .lock()
            .map_err(|error| error.to_string())?;
        self.stop_locked(true)?;
        self.status()
    }

    fn stop_locked(&self, clear_state: bool) -> Result<(), String> {
        let mut guard = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        if let Some(mut process) = guard.take() {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }
        drop(guard);
        if clear_state {
            *self
                .inner
                .started_at
                .lock()
                .map_err(|error| error.to_string())? = None;
            *self
                .inner
                .last_health_at
                .lock()
                .map_err(|error| error.to_string())? = None;
        }
        Ok(())
    }

    fn process_exited(&self) -> Result<bool, String> {
        let mut guard = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        let Some(process) = guard.as_mut() else {
            return Ok(true);
        };
        match process.try_wait()? {
            Some(status) => {
                let message = if status.success() {
                    "MCP Server 已退出".to_string()
                } else {
                    format!("MCP Server 异常退出，退出码={status}")
                };
                *self
                    .inner
                    .last_error
                    .lock()
                    .map_err(|error| error.to_string())? = Some(message);
                guard.take();
                Ok(true)
            }
            None => Ok(false),
        }
    }

    pub fn status(&self) -> Result<McpHostStatus, String> {
        let alive = !self.process_exited()?;
        let process = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        let pid = process.as_ref().and_then(ServerProcess::pid);
        let config = self
            .inner
            .config
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        let last_error = self
            .inner
            .last_error
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        Ok(McpHostStatus {
            state: if alive { "healthy" } else { "stopped" }.to_string(),
            server_id: SERVER_ID.to_string(),
            server_name: self
                .inner
                .server_name
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            protocol_version: self
                .inner
                .protocol_version
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            pid,
            started_at: self
                .inner
                .started_at
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            last_health_at: self
                .inner
                .last_health_at
                .lock()
                .map_err(|error| error.to_string())?
                .clone(),
            last_error,
            restart_count: self.inner.restarts.load(Ordering::SeqCst) as u32,
            config,
        })
    }

    pub fn health_check(&self) -> Result<McpHostStatus, String> {
        if self.process_exited()? {
            let config = self
                .inner
                .config
                .lock()
                .map_err(|error| error.to_string())?
                .clone();
            if config.auto_restart
                && self.inner.restarts.load(Ordering::SeqCst) < config.max_restarts as usize
            {
                let _lifecycle = self
                    .inner
                    .lifecycle_lock
                    .lock()
                    .map_err(|error| error.to_string())?;
                if self.process_exited()? {
                    self.inner.restarts.fetch_add(1, Ordering::SeqCst);
                    self.start_locked()?;
                }
            } else {
                return self.status();
            }
        }
        let response = self.request("ping", json!({}), StdDuration::from_secs(2))?;
        if response.get("result").is_none() {
            return Err("MCP ping 未返回成功结果".to_string());
        }
        *self
            .inner
            .last_health_at
            .lock()
            .map_err(|error| error.to_string())? = Some(Utc::now().to_rfc3339());
        self.status()
    }

    pub fn configure(&self, config: McpHostConfig) -> Result<(), String> {
        *self
            .inner
            .config
            .lock()
            .map_err(|error| error.to_string())? = config;
        Ok(())
    }

    fn write_message(&self, value: &Value) -> Result<(), String> {
        let mut process = self
            .inner
            .process
            .lock()
            .map_err(|error| error.to_string())?;
        let process = process
            .as_mut()
            .ok_or_else(|| "MCP Server 未运行".to_string())?;
        let serialized = serde_json::to_string(value).map_err(|error| error.to_string())?;
        process
            .stdin
            .write_all(serialized.as_bytes())
            .and_then(|_| process.stdin.write_all(b"\n"))
            .and_then(|_| process.stdin.flush())
            .map_err(|error| format!("MCP 消息写入失败：{error}"))
    }

    fn notify(&self, method: &str, params: Value) -> Result<(), String> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn request(&self, method: &str, params: Value, timeout: StdDuration) -> Result<Value, String> {
        let _request = self
            .inner
            .request_lock
            .lock()
            .map_err(|error| error.to_string())?;
        let id = self.inner.next_request_id.fetch_add(1, Ordering::SeqCst);
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;
        let receiver = {
            let process = self
                .inner
                .process
                .lock()
                .map_err(|error| error.to_string())?;
            process
                .as_ref()
                .ok_or_else(|| "MCP Server 未运行".to_string())?
                .responses
                .clone()
        };
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                let _ = self.stop_locked(false);
                return Err(format!("MCP 请求超时：{method}"));
            }
            let value = receiver
                .lock()
                .map_err(|error| error.to_string())?
                .recv_timeout(remaining)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => format!("MCP 请求超时：{method}"),
                    mpsc::RecvTimeoutError::Disconnected => "MCP Server 连接已断开".to_string(),
                })?;
            if value.get("id").and_then(Value::as_u64) == Some(id) {
                return Ok(value);
            }
        }
    }

    pub fn list_tools(&self) -> Result<Vec<McpToolDefinition>, String> {
        let response = self.request("tools/list", json!({}), StdDuration::from_secs(5))?;
        let remote_tools = response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .ok_or_else(|| "MCP tools/list 响应缺少工具列表".to_string())?;
        let local = tool_catalog();
        let by_name = local
            .into_iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect::<HashMap<_, _>>();
        let mut tools = Vec::new();
        for remote in remote_tools {
            let Some(name) = remote.get("name").and_then(Value::as_str) else {
                continue;
            };
            let Some(mut definition) = by_name.get(name).cloned() else {
                continue;
            };
            if let Some(schema) = remote.get("inputSchema") {
                definition.input_schema = schema.clone();
            }
            if let Some(description) = remote.get("description").and_then(Value::as_str) {
                definition.description = description.to_string();
            }
            tools.push(definition);
        }
        Ok(tools)
    }

    pub(super) fn call_server(&self, call: &McpCallRequest) -> Result<Value, String> {
        self.request(
            "tools/call",
            json!({
                "name": call.tool_name,
                "arguments": call.arguments,
            }),
            StdDuration::from_millis(normalized_timeout(call.timeout_ms)),
        )
    }

    pub fn cancel(&self, call_id: &str) -> Result<(), String> {
        let cancels = self
            .inner
            .cancels
            .lock()
            .map_err(|error| error.to_string())?;
        let flag = cancels
            .get(call_id)
            .ok_or_else(|| "未找到正在执行的工具调用".to_string())?;
        flag.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub(super) fn register_cancel(&self, call_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        self.inner
            .cancels
            .lock()
            .expect("cancel registry lock")
            .insert(call_id.to_string(), flag.clone());
        flag
    }

    pub(super) fn finish_cancel(&self, call_id: &str) {
        self.inner
            .cancels
            .lock()
            .expect("cancel registry lock")
            .remove(call_id);
    }
}
