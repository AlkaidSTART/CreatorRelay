# M8 验证记录 — 2026-11-26

> 本文件记录验证活动与结果，不是验收报告，也不能单独作为 M8 已验收的依据。

- **阶段：** M8 生态扩展
- **验证日期：** 2026-11-26
- **验证环境：** macOS；Google Chrome；CreatorRelay 隔离 Chrome profile；Tauri/Rust MCP Host/Core 调用链
- **验证 HEAD：** `7a4d4d2`
- **总体状态：** 浏览器桥接与真实网页导航 `pass`；哔哩哔哩登录后真实账号可见数据读取与截图交叉核验 `pass`；结构化读取、游标分页、增量变更、Adapter 同步游标和发布门禁自动化测试 `pass`；隔离浏览器上传/填表/提交真实平台动作 `not verified`；其余 7 个平台登录后读取、各平台真实发布 `not verified`；M8 仍 `blocked / not accepted`

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
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` | pass：80 unit、3 browser fixture、9 M4、5 M6、8 M7、4 M8 browser bridge、10 M8 ecosystem，合计 119 项 |
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

## 登录后真实账号读取与截图核验（哔哩哔哩）

本次使用桌面版 CreatorRelay 的实际 MCP Host/Core 授权调用链，而不是裸 stdio MCP Server：

1. 执行 `pnpm tauri build --debug`，构建并启动当前代码对应的 Debug 应用包。
2. 在 CreatorRelay“平台账号 → 创作者中心与浏览器”中，对哔哩哔哩调用 `creator_browser_open`。预览确认后，隔离 Chrome 打开 `https://passport.bilibili.com/login`。
3. 用户在隔离 Chrome 中自行完成真实账号登录，未向 Agent 提供密码、验证码或 Cookie。
4. 对哔哩哔哩调用 `creator_browser_read_page`，经一次性确认后读取当前页可见文本。工具返回页面状态 `页面已加载`，URL 为 `https://member.bilibili.com/platform/home`。
5. 从隔离 profile 的 `DevToolsActivePort` 获取 loopback CDP 端口，只读捕获当前视口截图，并与 MCP 返回文本逐项核对。

| 核对字段 | MCP `creator_browser_read_page` 返回 | 隔离浏览器截图 |
|---|---|---|
| 页面 | 创作中心 / `member.bilibili.com/platform/home` | 哔哩哔哩创作中心首页 |
| 账号创作时长 | 成为 UP 主的第 1294 天 | 成为 UP 主的第 1294 天 |
| 统计截至 | 2026-11-24（每日 12 点更新） | 统计截至 2026-11-24（每日 12 点更新） |
| 播放量 / 评论 / 弹幕 | 687 / 7 / 1 | 687 / 7 / 1 |
| 点赞 / 分享 / 收藏 / 投币 | 34 / 5 / 9 / 5 | 34 / 5 / 9 / 5 |
| 粉丝总数 / 今日变化 | 8 / +0 | 8 / +0 |

结论：`creator_browser_read_page` 返回的可见文本与隔离 Chrome 中真实账号页面截图一致，证明真实登录页读取通路可用。该通用读取工具不代替结构化平台 API；本次随后新增了平台结构化工具链，详见下一节。

截图证据：

- CreatorRelay MCP 读取结果：本地验证证据 `progress/verification/assets/M8-2026-11-26/creatorrelay-mcp-read-result.jpg`，SHA-256 `992ab4e5ac34f1ef314e99496a2ff6f16ee9170bec992293994c6e46f603982a`。
- 隔离 Chrome 哔哩哔哩真实账号页面：本地验证证据 `progress/verification/assets/M8-2026-11-26/bilibili-real-account.png`，2400×1474，SHA-256 `4c12a69d513e3d7f9b6dfef71a297b6c2b67b0ec31acb70f59df17a415bfb217`。
- 按用户要求，`progress/verification/assets/` 与 `docs/plans/` 已加入 `.gitignore`；上述截图和计划文档保留在本地，不进入 Git。

## 平台结构化工具与发布门禁验证

本轮在基线 `7a4d4d2` 的工作树上新增以下能力：

- 10 个平台 profile 均自动挂载 `work_get`、`work_changes`、`work_sync`、`media_stage`、`publish_submit`；保留既有 `account_get_info`、`work_list`、`analytics_get_trend`、`publish_validate`。
- `work_list` 返回不透明 `cursor`、`pageSize`、`totalCount`、`hasNextPage` 和 `nextCursor`；已验证连续两页无重复。
- `work_get` 返回统一字段、`rawFields`、指标快照和 typed `fieldModel`。
- `work_changes` 使用 `updatedAt + workId` 游标；已验证跨同时间戳记录连续分页不丢项。
- `work_sync` 仅接受显式 `adapterId`，把调用方 `cursor` 传给 Adapter，将 Adapter `nextCursor` 持久化到 `sync_runs.next_cursor`；SQLite schema 升至 v9。
- `media_stage` 只允许内容包内素材，隔离浏览器上传并填写标题/正文/标签，不点击保存或发布。
- `publish_submit` 只接受同一 `contentId` 的暂存页和唯一精确匹配按钮；`publish` 模式缺少 `confirmFinalPublish=true` 时在启动浏览器前拒绝。

自动化证据：

| 测试 | 结果 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem` | pass：10 项；覆盖扩展工具发现、字段模型、分页、增量变更、Adapter 同步游标、缺少浏览器桥接时拒绝暂存/提交 |
| `cargo test --manifest-path src-tauri/Cargo.toml --test m8_browser_bridge platform_final_publish_requires_explicit_confirmation_before_browser_action -- --nocapture` | pass：缺少最终发布确认时拒绝，Chrome 保持 stopped |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass |
| `pnpm test -- --run` | pass：12 files / 48 tests |
| `pnpm typecheck` | pass |
| `pnpm lint` | pass；既有 warning：`src/routes/navigation.ts:239` |
| `pnpm build` | pass；既有主 chunk > 500 kB warning |
| 启动新 Debug 应用包后手工检查“平台账号 → 创作者中心与浏览器”与“AI 助手 → MCP Host” | pass：创作者中心页不再显示假的“服务未就绪/只读关闭”；MCP Host 显示 `107` 个工具，界面可见 `bilibili_work_get`、`bilibili_work_changes` 等新增工具 |

本轮没有对任何真实账号执行 `media_stage` 或 `publish_submit`，因为上传素材和最终发布会产生外部平台副作用。相关代码路径目前只有单元/集成门禁与桥接拒绝测试；真实平台页面选择、上传控件、按钮唯一性和发布结果仍需在用户逐次确认下做账号级实测。

## 安全与边界检查

- 未获授权 token 调用 `creator_browser_open` 返回 `AUTHORIZATION_REQUIRED`，且隔离 Chrome 保持 stopped。
- 裸 stdio MCP Server 不持有桌面 Core/browser bridge；即使直接调用浏览器工具也会显式拒绝，返回 `BROWSER_BRIDGE_UNAVAILABLE`，不会启动 Chrome。
- 打开与读取均经过 MCP 预览与一次性确认；读取预览会明确提示页面可见文本可能包含账号个人信息、作品和数据指标，并会返回给 MCP 调用方。
- 页面文本作为不可信外部内容处理，工具结果会明确提示 AI 不遵循页面内嵌指令。

## 未验证项与剩余风险

- 仅哔哩哔哩完成了登录后真实账号页面可见数据读取与截图核验；其余 7 个平台尚未完成登录后账号级验证。
- 平台表单填写、媒体上传和提交动作已实现为隔离浏览器分阶段工具，但没有对真实账号做上传/发布实测；不能声称各平台真实发布已通过。
- 正式 Tauri 桌面窗口内已完成哔哩哔哩的打开、登录、读取按钮点击与结果展示手工验证；其他平台和重启持久化尚未完成同等验证。
- “平台账号 → 创作者中心与浏览器”入口已改为初始化 MCP store；此前未初始化时出现的“MCP 服务未就绪 / 只读权限已关闭”假警告已修复，并在新构建 Debug 应用包中完成视觉复核。
- 第三方平台页面结构、登录风控和政策变化可能导致可选择器、页面状态或访问稳定性退化；需要在账号级场景继续验证。
