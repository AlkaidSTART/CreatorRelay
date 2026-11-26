# 创作者中心浏览器桥接验证记录 — 2026-11-25

> 这是实现验证记录，不是 M8 验收报告。真实页面状态只说明浏览器会话中观察到的页面，不代表平台远程 MCP/API 或发布授权。

## 实现边界

- MCP 工具 `creator_browser_status`、`creator_browser_open`、`creator_browser_read_page` 接入 Tauri 的本地 MCP→Core 转发路径。
- Chrome/Chromium 使用 CreatorRelay 专属 profile `creatorrelay-mcp-browser`，目录限制为当前用户；不连接日常 Chrome profile、不读写日常 Cookie。采用随机 DevTools 端口（由该专用 profile 的 `DevToolsActivePort` 获取），不固定开放调试端口。
- Chrome DevTools 随机本机端口只绑定 loopback，并从专用 profile 的 `DevToolsActivePort` 读取。仅通过 SQLite 已配置 HTTPS creator URL 打开，允许目标站点同平台已知登录子域；禁止其他主机与 HTTP。
- 页面读取只返回 `document.body.innerText`、标题与 URL（上限 60,000 字符）；不执行页面提供的脚本、不提取 Cookie/网络响应、不点发布、不填写表单。
- MCP 打开浏览器属于 ExternalAction，需要 CreatorRelay 一次性确认流程；只读页面工具使用 Read 权限。打开预览显示平台与 URL、外部网络连接副作用，用户确认后才启动 Chrome；不提供 raw stdio 绕过 UI 的 Chrome 控制能力。

## 自动化检查

| 命令 | 结果 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --all-targets` | pass：80 unit、3 MCP fixture、9 M4 integration、5 M6 operations、8 M7 MCP、3 M8 browser bridge、8 M8 ecosystem |
| `CREATORRELAY_LIVE_BROWSER_TEST=1 cargo test --manifest-path src-tauri/Cargo.toml --test m8_browser_bridge live_creator_browser_opens_all_eight_platform_urls_through_mcp -- --nocapture --test-threads=1` | pass，8 个真实网站探测结果见下表 |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | pass |
| `pnpm typecheck` | pass |
| `pnpm test` | pass，12 files / 48 tests |
| `pnpm lint` | pass with existing warning `src/routes/navigation.ts:239` (`unicorn(no-array-sort)`) |
| `pnpm build` | pass；既有大 chunk 警告 |

## 真实隔离浏览器结果

测试经 MCP `creator_browser_open`（含 preview 与一次性授权令牌）启动全新临时 profile 的 Chrome，通过 DevTools 打开配置网址，并读取可见 body 文本；测试后关闭进程并删除临时目录。全新 profile 无用户授权 Cookie，所以需登录平台如实报告登录态。

| 平台 | 结果 | 页面观察 |
|---|---|---|
| 抖音 | `creator_page_visible` | creator.douyin.com 页面返回一站式创作服务平台和 AI 工坊可见文本；无登录 Cookie 时未显示账号作品数据 |
| 小红书 | `login_required` | 已重定向到创作服务登录页，有短信验证码入口 |
| 知乎 | `login_required` | 登录/验证码页面 |
| 微信公众号 | `login_required` | 显示“登录超时，请重新登录” |
| 快手 | `creator_page_visible` | MCP 导航后显示创作者服务平台与内容运营/数据分析介绍；账号数据未验证 |
| 哔哩哔哩 | `login_required` | passport.bilibili.com 登录页，HTTPS 域在已知 Bilibili 官方域名 allowlist 内 |
| 稀土掘金 | `creator_page_visible` | 创作者中心、文章/数据入口可见；用户账号的具体数据需登录后确认 |
| CSDN | `page_visible` | 创作中心导航壳可见；账号登录态/作品正文未能确认，不能当作账号数据读取 |

> 浏览器 `pageStatus` 是可见页面提示的粗粒度状态，不是服务端认证证明。快手、掘金、CSDN 可见创作入口不代表用户已登录或数据 API 可访问；本测试只验证 MCP 到隔离 Chrome 的真实导航和 DOM 可见文本通路。

## 验收结论与缺口

- `pass`：MCP `tools/list` 注册、Tauri→Core 浏览器注入、打开工具确认门禁（无授权 token 时不启动 Chrome）、本地专用 Chrome/CDP 运行、平台主机/登录子域校验、8 个 URL 经 MCP 授权调用导航及页面可见文本读取机制。
- `partial`：抖音创作服务页、快手创作者服务介绍、掘金文章/数据入口与 CSDN 创作中心外壳可见。该新隔离 profile 没有用户账号授权，所以页面可见不等于该用户作品/指标已读取。
- `not verified`：登录后的平台作品/指标读取、页面交互自动化、任一平台真实发布。MCP 没有发布工具；即使未来增加，也必须单独执行确认。
- 桌面 Tauri UI 的按钮可见性与点击手工检查未执行；浏览器预览不具备 `creator_browser_*` Tauri commands。首次启动应用后从“平台账号 → 创作者中心与浏览器”打开，CreatorRelay MCP Host 提供同名工具；本地 MCP Client 对 `creator_browser_open` 必须先调用 CreatorRelay preview API 获得一次性令牌再调用；直接跳过确认会返回 `AUTHORIZATION_REQUIRED`，不会启动浏览器。
