# MCP Host、工具权限与 Assistant 运行说明

本文档描述 M7 的本地 MCP Host、工具注册、授权、隐私和审计实现。代码入口为
`src-tauri/src/mcp/`（模块化目录），平台工具 profile 位于
`src-tauri/src/mcp/platform_mcp/`；桌面 UI 入口为 `src/pages/AssistantPage.tsx`。

## 1. 运行结构

```text
React Assistant UI
    │ Tauri commands
    ▼
Rust MCP Host ── stdio JSON-RPC ── 独立测试 MCP Server
    │                                      │
    │ JSON Schema 校验 + 权限/授权          │ tools/list
    │                                      │ tools/call → forwardToHost
    ▼                                      ▼
Core Service（内容、账号、发布、分析、AI）
    │
    ├─ 内容包与 Markdown 文件
    ├─ SQLite（仅索引、对话、任务与审计）
    └─ 系统 Keychain（凭据引用）
```

测试 MCP Server 以当前应用可执行文件的 `--mcp-test-server` 参数启动。Host 通过 stdin/stdout
发送 JSON-RPC；Server 不接收数据库路径、工作区路径、凭据引用或 API Key。业务工具的
`tools/call` 只返回 `structuredContent.forwardToHost=true`，由 Host 校验工具名后调用 Core。

## 2. 生命周期

- 应用启动时按已保存设置尝试启动本地测试 Server；失败会写入脱敏启动日志，状态仍可在 UI 查看。
- `get_mcp_host_status` 返回 `healthy`/`stopped`、PID、协议版本、最近健康检查和重启次数。
- `start_mcp_server` 启动或重启；`stop_mcp_server` 停止。
- `mcp_health_check` 发送 `ping`。检测到异常退出且开启自动重启时，在 `maxRestarts` 内重新启动。
- Host 请求超时会终止当前 Server 连接并返回可见的超时状态；不会自动重放工具调用。

## 3. 工具目录

| 工具                      | 权限            | 用途                          | 关键约束                                         |
| ------------------------- | --------------- | ----------------------------- | ------------------------------------------------ |
| `account_get_info`        | Read            | 读取脱敏账号状态与能力        | 不返回 `profile_id`、`credential_ref` 或原始凭据 |
| `content_list`            | Read            | 检索四类本地内容包            | 不读取任意文件路径                               |
| `content_get_detail`      | Read            | 读取内容详情                  | 正文是否包含由模型上下文范围决定                 |
| `external_content_list`   | Read            | 查询已同步作品与原始指标      | 保留平台原始指标口径                             |
| `analytics_get_trend`     | Read            | 读取趋势                      | 仅返回有真实采样点的数据，不补零                 |
| `studio_create_content`   | Write           | 创建视频/图文/图文文章/纯文章 | 媒体仅接受 `sourceContentId + assetId`           |
| `studio_update_content`   | Write           | 更新内容草稿                  | 形态不可被工具改变；支持 `baseHash` 冲突保护     |
| `publishing_create_task`  | Write           | 创建发布任务                  | 每个平台/账号为独立任务，不自动发布              |
| `publishing_publish_task` | External Action | 执行发布任务                  | 每次强制预览与一次性确认，逐目标返回结果         |
| `content_delete_draft`    | Write           | 删除本地内容包                | 只接受内容 ID，不接受路径                        |
| `mcp_test_echo`           | Read            | 协议与发现自检                | 仅用于测试                                       |
| `mcp_test_hang`           | Read            | 超时/取消自检                 | 不产生外部副作用                                 |
| `creator_browser_status` | Read            | 查看隔离 Chrome 状态          | 不公开固定 DevTools 端口                         |
| `creator_browser_open`   | External Action | 打开配置的创作者中心          | 用户确认；HTTPS / 平台域名限制；仅导航            |
| `creator_browser_read_page` | Read         | 读取打开页的可见文本          | 当前标签与平台绑定、限长；网页文本为不可信数据     |

所有 Schema 带 `schemaVersion: "1.0"`。Host 自行校验 `type`、`required`、`enum`、字符串长度、
数字范围、数组长度、`additionalProperties` 和嵌套对象；非法参数在 Core 调用前即返回字段路径。

## 3.1 平台专属 MCP

通用的内容、账号、发布和数据分析工具继续由 Core 提供；平台字段差异收进独立 profile：

```text
src-tauri/src/mcp/platform_mcp/
├── douyin_mcp.rs
├── xiaohongshu_mcp.rs
├── bilibili_mcp.rs
├── youtube_mcp.rs
├── wechat_mcp.rs
├── weibo_mcp.rs
├── kuaishou_mcp.rs
├── juejin_mcp.rs
├── csdn_mcp.rs
└── zhihu_mcp.rs
```

每个平台注册 `<platform>_account_get_info`、`<platform>_work_list`、
`<platform>_analytics_get_trend`、`<platform>_publish_validate` 四个只读工具。工具 Schema
使用平台原生字段，例如抖音 `itemId` / `video_id` / `cover_tsp`，小红书 `noteId` /
`content` / `images`，YouTube `videoId` / `snippet.title`。

平台 profile 同时记录官方接口/人工网页来源、鉴权方式、验证状态、字段映射、内容规则和人工交接入口。
在账号级授权、发布与回滚证据完成前，`autoPublishAllowed` 保持 `false`，`publishDispatch` 保持
`manual_only`；平台工具不会读取 Cookie、Token、AppSecret 或调用网页内部接口。

详细设计、平台矩阵和新增平台步骤见 [平台 MCP 拆分与接口映射](./platform-mcp.md)。

桌面端的创作者中心桥接在 CreatorRelay 管理的 MCP Host/Core 调用链中启动专用 Chrome profile。它只是在隔离浏览器内打开已配置的网页并读取可见文本，并非平台远程 MCP/API。用户需在新开的专用 Chrome 窗口手动登录；会话与日常 Chrome 分开。裸 stdio MCP Server 子进程不持有桌面 Core/browser bridge，直接调用浏览器工具会返回 `BROWSER_BRIDGE_UNAVAILABLE`；请通过 CreatorRelay 桌面 MCP Assistant/授权流程调用。不要因此宣称已读取账号作品或获得发布能力。

## 4. 权限与确认

| 权限            | 默认策略             | 实现                                              |
| --------------- | -------------------- | ------------------------------------------------- |
| Read            | 用户可启用/停用      | `mcp_settings.read_enabled`                       |
| Write           | `confirm` 或 `allow` | 默认每次预览后发放 10 分钟、一次性授权令牌        |
| External Action | 始终每次确认         | 即使 Write 设为 `allow`，外部发布仍强制预览与确认 |

授权令牌绑定工具名、权限等级和完整参数摘要哈希，只能使用一次；参数或目标在预览后变化时，
执行会被拒绝。预览包含最终内容、目标账号、内容形态、媒体顺序与封面、预期副作用。

外部发布在一次批量确认后按任务逐项执行。每个目标独立记录 `SUCCESS`、失败或
`NEEDS_MANUAL_ACTION`，不会因为一个目标失败回滚其他目标，也不会自动重复提交。

## 5. 内容载荷规则

- `video`：至少一个视频；媒体列表不能混入图片；必须提供图片封面；不使用 Markdown 正文。
- `imageText`：媒体列表不能为空且只能包含图片；必须明确封面；短文案写入 `summary` 和逐图说明。
- `imageArticle`：正文不能为空；至少一张图片；图片顺序连续且封面明确。
- `article`：正文不能为空；媒体可选。

素材只能通过 Core 资产引用创建或更新：

```json
{
  "sourceContentId": "content-uuid",
  "assetId": "asset-uuid",
  "order": 1,
  "caption": "可选的逐图说明"
}
```

工具不接受 `path`、`filePath`、`credentialRef`、`cookie`、`token` 或 `apiKey` 参数。

## 6. 隐私

云端模型请求前，UI 展示服务商、模型、内容 ID、实际发送载荷与包含/排除字段。默认：

- 标题、摘要、标签、素材元数据可选。
- 正文默认不发送。
- 不发送整个工作区、未选素材、工作区路径或凭据引用。
- 对常见 `api_key=...`、`password`、`cookie`、`authorization`、`Bearer` 等模式做文本脱敏。
- API Key 只从系统凭据库读取，并放在模型服务商的认证头中，不进入 Prompt、工具输出、SQLite 或日志。

## 7. 调用审计

`mcp_tool_audit` 记录：

- 调用时间、调用 ID、线程 ID、Server ID、工具名与权限等级。
- 目标摘要、脱敏参数摘要、内容形态。
- 结果状态、错误码、脱敏错误信息、完成时间。

参数摘要不会保存正文原文（只记录字符数）、媒体文件字节或敏感值。对话在
`assistant_threads` / `assistant_messages` 中持久化，重开应用后可恢复历史与工具结果。

## 8. 主要 Tauri 命令

- 生命周期：`get_mcp_host_status`、`start_mcp_server`、`stop_mcp_server`、`mcp_health_check`
- 平台目录：`list_platform_mcp_servers`
- 工具：`mcp_list_tools`、`mcp_preview_tool_call`、`mcp_call_tool`、`mcp_cancel_tool`
- 设置与审计：`get_mcp_settings`、`save_mcp_settings`、`list_mcp_audit`、`mcp_security_self_test`
- Assistant：`list_assistant_threads`、`create_assistant_thread`、`get_assistant_thread`、
  `assistant_plan_message`、`assistant_context_preview`、`assistant_generate_reply`

## 9. 验证

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test m7_mcp
pnpm typecheck
pnpm test
pnpm build
```

M7 集成测试覆盖：Server 启动/停止/崩溃重启、工具发现与 Schema 拒绝、Core 边界与凭据脱敏、
四类内容工具创建/更新与恢复、缺少形态载荷的字段级拒绝、写授权一次性与拒绝无副作用、
批量发布逐目标部分失败、超时/取消后其他工具可用、Assistant 历史、正文不发送与凭据不进入
模型请求正文/SQLite/日志。
