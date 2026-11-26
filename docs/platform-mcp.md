# 平台 MCP 拆分与接口映射

> 状态：已实现平台 profile 与只读工具面；所有平台自动发布仍为关闭，需逐平台完成账号级接口与政策验证后才能升级能力。

## 1. 为什么按平台拆

M7 的 MCP 最初把所有业务工具放在一个 `mcp.rs` 中。平台字段差异很大，例如：

- 抖音作品使用 `item_id`，视频创建使用 `video_id` / `cover_tsp`，图文使用 `image_list`；
- 小红书笔记使用 `note_id`，发布页字段是标题、正文、图片/视频、话题、位置、可见范围；
- Bilibili 稿件使用 `bvid` / `aid`，统计在 `stat.*`；
- YouTube 使用 `videoId`、`snippet.*`、`status.privacyStatus` 和 Data API 指标；
- 微信公众号使用 `msg_id`、`media_id`、`thumb_media_id`、`draft/add` 与 `freepublish/submit`。

把平台字段继续塞进一个通用工具会让 Schema 失去意义，也容易把未验证能力伪装成通用能力。现在每个平台拥有独立的 `*_mcp` 模块，通用 Core 工具只负责本地内容、账号、发布任务和审计。

## 2. 模块布局

```text
src-tauri/src/mcp/
├── mod.rs                 # DTO、模块注册与公开 re-export
├── host.rs                # MCP Host 生命周期、stdio JSON-RPC、崩溃重启
├── schema.rs              # JSON Schema 构造与字段级校验
├── catalog.rs             # Core 工具目录 + 平台工具目录聚合
├── security.rs            # 脱敏、摘要、安全证据
├── audit.rs               # 调用审计与目标摘要
├── authorization.rs       # 一次性确认令牌
├── account_tools.rs       # 通用账号/能力工具
├── content_tools.rs       # 内容列表、详情、创建/更新/删除
├── analytics_tools.rs     # 外部作品与趋势
├── publishing_tools.rs    # 发布任务创建、预览、逐目标执行
├── assistant.rs           # Assistant 历史、计划、上下文与生成
├── server.rs              # 本地 stdio Server
├── outcome.rs             # 工具执行结果结构
└── platform_mcp/
    ├── mod.rs             # PlatformMcpProfile、工具构造器、注册表
    ├── execute.rs         # 平台工具的 Core 只读执行
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

相关大文件也按职责拆分：

- `src-tauri/src/platforms/`：`catalog`、`credentials`、`accounts`、`types` 与测试；
- `src-tauri/src/operations/`：`types`、`publish`、`external_works`、`manual_package`、`analytics`、`dashboard`。

拆分只改变代码边界，不改变数据库、公开 Tauri 命令或 MCP 工具的业务语义；M7/M8 回归测试用于确认这一点。

`lib.rs` 和 `content.rs` 也做了同样处理：

```text
src-tauri/src/
├── lib.rs                  # AppState、Tauri 注册入口、run()
├── lib_tests.rs            # 根模块测试
├── workspace_support.rs    # 文档/Git/工作区辅助逻辑
├── commands/               # 按领域拆分的 Tauri 命令
│   ├── content.rs
│   ├── ai.rs
│   ├── mcp.rs
│   ├── creator_centers.rs
│   ├── ecosystem.rs
│   ├── workspace.rs
│   ├── platforms.rs
│   ├── plugins.rs
│   └── operations.rs
└── content/                # 内容包与平台稿
    ├── mod.rs              # 类型与公开导出
    ├── package.rs          # 内容包、素材、版本
    ├── drafts.rs           # 平台稿
    ├── paths.rs            # 路径、原子写入和通用校验
    └── tests.rs
```

`lib.rs` 从 3450 行降到约 680 行；`content.rs` 从 1610 行拆为最大约 540 行的模块。

## 3. 平台 profile 契约

每个平台模块实现一个 `profile() -> PlatformMcpProfile`：

| 字段                 | 说明                                                    |
| -------------------- | ------------------------------------------------------- |
| `serverName`         | 逻辑 MCP 名称，例如 `douyin_mcp`、`xiaohongshu_mcp`     |
| `platformId`         | Core 平台 ID                                            |
| `authMode`           | OAuth、浏览器 Cookie、公众号 AppID/AppSecret 或人工网页 |
| `admissionStatus`    | 当前为 `evaluation_only`，不能当作发布授权              |
| `autoPublishAllowed` | 当前全部为 `false`                                      |
| `publishDispatch`    | 当前全部为 `manual_only`                                |
| `nativeWorkIdField`  | 平台作品主键字段名                                      |
| `interfaces`         | 官方接口/人工网页、方法、路径、鉴权、验证状态与来源     |
| `tools`              | 平台专属工具名称与输入 Schema                           |
| `fieldMappings`      | 统一字段 ↔ 平台字段及读写方向                           |
| `contentRules`       | 各内容形态的媒体、封面、标题/正文/标签约束              |
| `handoff`            | 人工发布、作品管理与数据页入口                          |

`verification` 明确区分：

- `verified`：当前仓库已有可复现的账号/接口验证；
- `documented_unverified`：官方入口或文档存在，但未完成账号级授权、请求或发布往返；
- `manual_web`：平台没有可用的官方内容发布 API，按人工网页流程交接；
- `not_available_for_content` / `not_available_for_publish`：开放平台存在，但当前不能证明提供对应内容能力。

## 4. 平台工具面

CreatorRelay 的 MCP Host 是 **MCP Server**，把本地内容、账号能力和发布工作流暴露给 MCP Client；它不是 MCP Client，不会因为某个平台有创作者中心网页就自动连接该平台的远程 MCP 服务。当前平台专属工具读取的是 CreatorRelay 的本地数据库、执行本地内容校验并提供人工交接。未完成账号级官方 API 验证的平台，不会被当作实时作品读取或自动发布能力。

平台管理新增“创作者中心与浏览器”视图。8 个用户提供的网址在 SQLite `creator_center_links` 表中维护；`creator_centers_list` 返回这些链接。桌面端还可通过 `creator_browser_status`、`creator_browser_open`、`creator_browser_read_page` MCP 工具操作一个 CreatorRelay 专属 Chrome profile：open 工具必须通过一次性 ExternalAction 确认；仅打开 SQLite 配置的 HTTPS 链接和同平台登录子域，read 工具只返回当前页面可见文本（最多 60,000 字符）。Chrome 使用随机 DevTools 端口绑定 loopback，不访问用户日常浏览器 profile、Cookie 或密码，也不执行表单提交或发布。该能力是受限浏览器中转，不是平台远程 MCP/API；未登录或平台权限不足时必须返回登录页/页面状态，而不能声称作品数据已读取。网页 HTTP 200 也只证明入口响应。

每个平台注册四个只读工具：

| 工具后缀                         | 权限 | 行为                                                             |
| -------------------------------- | ---- | ---------------------------------------------------------------- |
| `<platform>_account_get_info`    | Read | 读取 Core 中该平台的脱敏账号、状态和能力                         |
| `<platform>_work_list`           | Read | 按平台原生作品 ID、形态、时间和关键词查询本地已同步作品          |
| `<platform>_analytics_get_trend` | Read | 读取本地指标趋势，可选按平台指标名过滤                           |
| `<platform>_publish_validate`    | Read | 校验本地内容包的形态/媒体/封面，并返回平台字段映射和人工交接信息 |

例如：

```text
douyin_account_get_info
douyin_work_list               # itemId
douyin_analytics_get_trend     # itemId + play_count/digg_count/...
douyin_publish_validate        # 返回 text/video_id/image_list/cover_tsp 映射

xiaohongshu_work_list          # noteId
xiaohongshu_publish_validate   # 返回 title/content/topics/images/cover 映射
```

这些工具不会：

- 读取 Cookie、Browser profile、access_token、AppSecret 或 Keychain；
- 调用平台网页内部接口；
- 自动发布、自动登录或绕过审核；
- 把未验证能力标记为 `supported`。

真正的发布仍走通用 `publishing_create_task` / `publishing_publish_task`，并受 M6/M7 的预览、授权、平台准入和逐目标结果约束。

## 5. 平台接口矩阵

| server            | 主要接口来源                                                      | 鉴权                        | 原生作品 ID | 发布方式                    | 当前验证                            |
| ----------------- | ----------------------------------------------------------------- | --------------------------- | ----------- | --------------------------- | ----------------------------------- |
| `douyin_mcp`      | 抖音开放平台 OAuth、video/image upload、create、list、data        | OAuth + 应用审核            | `itemId`    | 人工网页 / 待开放平台验收   | 接口入口可达；账号与发布未验证      |
| `xiaohongshu_mcp` | 创作中心发布页、笔记管理、创作者数据；开放平台仅商家/电商入口     | 人工登录会话                | `noteId`    | 人工网页                    | 无已核实的个人笔记发布 API          |
| `bilibili_mcp`    | `x/web-interface/nav` 身份；投稿/专栏人工页面                     | 浏览器 Cookie（身份已验证） | `bvid`      | 人工网页                    | 身份已验证；发布未验证              |
| `youtube_mcp`     | Data API `videos.list` / `videos.insert`、Analytics API           | OAuth 2.0                   | `videoId`   | 人工 Studio / 待 OAuth 验收 | 文档存在；OAuth 未配置              |
| `wechat_mcp`      | `stable_token`、素材、`draft/add`、`freepublish/submit`、datacube | AppID/AppSecret + IP 白名单 | `msgId`     | 人工后台 / 待公众号验收     | 文档存在；账号权限未验证            |
| `weibo_mcp`       | `statuses/share`、媒体上传、`statuses/show`                       | OAuth access_token          | `statusId`  | 人工主站 / 待 OAuth 验收    | 分享接口字段与 140 字限制有官方文档 |
| `kuaishou_mcp`    | 开放平台 OAuth、photo 上传/发布                                   | OAuth + 应用审核            | `photoId`   | 人工创作者中心              | 入口存在；接口未账号级验证          |
| `juejin_mcp`      | 文章编辑器、文章管理、创作者数据                                  | 人工登录会话                | `articleId` | 人工网页                    | 无已核实的官方文章发布 API          |
| `csdn_mcp`        | 创作中心编辑器、文章管理                                          | 人工登录会话                | `articleId` | 人工网页                    | 无已核实的官方文章发布 API          |
| `zhihu_mcp`       | 专栏编辑器、创作者中心；开放平台入口                              | 人工登录会话                | `articleId` | 人工网页                    | 无已核实的官方专栏发布 API          |

抖音接口检查（2026-11-25）确认以下开放平台路径返回平台 JSON 错误而不是 HTML 404；无有效 `access_token` 时返回 `access_token无效`：

- `https://open.douyin.com/api/douyin/v1/video/upload_video/`
- `https://open.douyin.com/api/douyin/v1/video/upload_image/`
- `https://open.douyin.com/api/douyin/v1/video/create_video/`
- `https://open.douyin.com/api/douyin/v1/video/create_image_text/`
- `https://open.douyin.com/api/douyin/v1/video/video_list/`
- `https://open.douyin.com/api/douyin/v1/video/video_data/`

这只能证明路径存在，不能证明应用资格、账号权限或发布成功。

## 6. 字段映射示例

### 抖音

| 统一字段         | 抖音字段     | 方向  | 说明                     |
| ---------------- | ------------ | ----- | ------------------------ |
| `media.video`    | `video_id`   | write | 先上传，再创建作品       |
| `media.image`    | `image_list` | write | 图文图片列表             |
| `title` / `body` | `text`       | write | 视频文案/图文标题语义    |
| `cover`          | `cover_tsp`  | write | 封面时间点，不传本地路径 |
| `externalId`     | `item_id`    | read  | 作品回读主键             |
| `metrics.play`   | `play_count` | read  | 播放数                   |
| `metrics.like`   | `digg_count` | read  | 点赞数                   |

### 小红书

| 统一字段           | 小红书字段        | 方向  | 说明               |
| ------------------ | ----------------- | ----- | ------------------ |
| `title`            | `title`           | write | 笔记标题           |
| `body` / `summary` | `content`         | write | 笔记正文           |
| `tags`             | `topics`          | write | 话题选择           |
| `media.image`      | `images`          | write | 图片顺序即展示顺序 |
| `media.video`      | `video`           | write | 视频笔记           |
| `cover`            | `cover`           | write | 首图或视频封面     |
| `externalId`       | `note_id`         | read  | 笔记回读主键       |
| `metrics.like`     | `liked_count`     | read  | 点赞数             |
| `metrics.collect`  | `collected_count` | read  | 收藏数             |

## 7. 新增平台步骤

1. 在 `src-tauri/src/mcp/platform_mcp/` 新建 `<platform>_mcp.rs`。
2. 在 `mod.rs` 注册模块，并把 profile 加入 `platform_mcp_catalog()`。
3. 只声明有来源或实际验证的 interface；人工网页明确写 `manual_web`。
4. 为四个平台工具提供平台原生字段 Schema，不要复用其他平台的字段名。
5. 添加字段映射与内容规则；未验证的长度/规格只能作为 warning，不能伪造硬限制。
6. 在 `src-tauri/tests/m8_ecosystem.rs` 增加 profile、Schema 和只读执行测试。
7. 更新 `src-tauri/resources/ecosystem/platform-admissions.json` 的来源、日期、逐形态结果和 `autoPublishAllowed`；没有账号级证据时保持关闭。
8. 只有真实账号、接口、发布与回滚验证全部通过后，才可把某个形态提升为 `supported` 并开放自动发布。

## 8. 验证命令

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --test m7_mcp
cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
pnpm test
pnpm typecheck
pnpm lint
pnpm build
```

当前验证结果见 [M8 验证记录](../progress/verification/M8-verification-2026-11-25.md)。
