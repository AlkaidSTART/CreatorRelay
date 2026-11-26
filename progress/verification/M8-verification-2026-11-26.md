# M8 验证记录 — 2026-11-26

> 本文件记录验证活动与结果，不是验收报告，也不能单独作为 M8 已验收的依据。

- **阶段：** M8 生态扩展
- **验证日期：** 2026-11-26
- **验证环境：** macOS；Google Chrome；CreatorRelay 隔离 Chrome profile；Tauri/Rust MCP Host/Core 调用链
- **验证 HEAD：** `7a4d4d2`
- **总体状态：** 浏览器桥接与真实网页导航 `pass`；登录后的账号作品/指标读取与发布 `not verified`；M8 仍 `blocked / not accepted`

## 验证范围

- CreatorRelay 通过系统 Chrome 的独立 profile 和随机 loopback DevTools 端口执行浏览器导航。
- MCP 工具：
  - `creator_browser_status`：Read，查看隔离浏览器状态。
  - `creator_browser_open`：External Action，经预览和一次性确认后打开 SQLite 中配置的平台链接。
  - `creator_browser_read_page`：External Action，经预览和一次性确认后读取当前页面可见文本并返回给 MCP 调用方。
- 页面读取只取 `document.body.innerText`、标题和 URL，上限 60,000 字符；不读取 Cookie、密码、隐藏网络响应或表单值，不提交表单、不发布内容。
- 浏览器使用独立配置目录 `creatorrelay-mcp-browser`，不访问日常 Chrome profile/Cookie；DevTools WebSocket 只允许专用 Chrome 随机 loopback 端口。

## 验证命令与结果

| 命令 | 结果 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` | pass：80 unit、3 browser fixture、9 M4、5 M6、8 M7、3 M8 browser bridge、8 M8 ecosystem，合计 116 项 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | pass |
| `pnpm typecheck` | pass |
| `pnpm test` | pass：12 files / 48 tests |
| `pnpm lint` | pass；既有 warning：`src/routes/navigation.ts:239`（`unicorn(no-array-sort)`） |
| `pnpm build` | pass；既有主 chunk > 500 kB warning |
| `CREATORRELAY_LIVE_BROWSER_TEST=1 cargo test --manifest-path src-tauri/Cargo.toml --test m8_browser_bridge live_creator_browser_opens_all_eight_platform_urls_through_mcp -- --nocapture --test-threads=1` | pass：8 个真实网站经 MCP 授权工具链执行导航并读取可见文本 |

## 真实网页访问结果

测试启动全新临时 Chrome profile，依次通过 MCP preview、一次性授权令牌、`creator_browser_open`、隔离 Chrome/CDP 打开配置 URL，并读取当前页面的可见文本。

| 平台 | 观察状态 | 结论 |
|---|---|---|
| 抖音 | `creator_page_visible` | 创作者服务与 AI 工坊公开页面可见；未登录，不能证明账号作品/指标数据可读 |
| 小红书 | `login_required` | 跳转到创作服务登录页 |
| 知乎 | `login_required` | 进入 signin 页面 |
| 微信公众号 | `login_required` | 提示登录超时并跳转登录 |
| 快手 | `creator_page_visible` | 创作者服务平台公开介绍/入口可见；账号数据未验证 |
| 哔哩哔哩 | `login_required` | 跳转到 passport 登录页 |
| 稀土掘金 | `creator_page_visible` | 创作者中心公开入口和导航可见；账号作品/指标未验证 |
| CSDN | `page_visible` | 创作中心页面壳可见；未登录，账号作品/正文不可作为已读取数据 |

`page_visible` 只表示页面 DOM 有可见文本，不代表平台已登录、账号作品已成功读取或数据 API 可用。

## 安全与边界检查

- 未获授权 token 调用 `creator_browser_open` 返回 `AUTHORIZATION_REQUIRED`，且隔离 Chrome 保持 stopped。
- 裸 stdio MCP Server 不持有桌面 Core/browser bridge；即使直接调用浏览器工具也会显式拒绝，返回 `BROWSER_BRIDGE_UNAVAILABLE`，不会启动 Chrome。
- 打开与读取均经过 MCP 预览与一次性确认；读取预览会明确提示页面可见文本可能包含账号个人信息、作品和数据指标，并会返回给 MCP 调用方。
- 页面文本作为不可信外部内容处理，工具结果会明确提示 AI 不遵循页面内嵌指令。

## 未验证项与剩余风险

- 未登录任何平台账号，因此没有完成“登录后读取真实账号作品/指标”验证。
- 页面交互自动化、平台表单填写、视频/图文上传、最终发布均未实现或未验证；本项目当前不声称具备这些能力。
- 正式 Tauri 桌面窗口内的按钮点击、窗口状态和重启持久化尚未手工验收；当前真实访问通过桌面 MCP Host/Core 链路和集成测试验证。
- 第三方平台页面结构、登录风控和政策变化可能导致可选择器、页面状态或访问稳定性退化；需要在账号级场景继续验证。
