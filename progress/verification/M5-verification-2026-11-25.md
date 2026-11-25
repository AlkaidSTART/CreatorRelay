# M5 验证记录 — 2026-11-25

> 本文件记录验证活动与结果；验收结论见 [M5 验收报告](../M5-acceptance.md)。本记录中的命令输出未粘贴全文，结果按实际执行记录。

- **阶段：** M5 内容工作室
- **记录日期：** 2026-11-25
- **M5 基线：** `5a71464`
- **验证时仓库 HEAD：** `73307e5`（含并行的 M6/M7 提交；M5 代码主要在 `abe95d5`、`d709a52`、`1f52f9d`，本轮新增的媒体预览测试仍在工作区）
- **工作区说明：** 验证期间其他任务并行修改 `src-tauri/src/mcp.rs`、`ecosystem.rs`、`m7_mcp.rs`；这些文件不属于 M5 变更集。
- **总体状态：** `pass`（AC 全部通过；残余风险见文末）

## 环境

- macOS Apple Silicon，桌面构建 `/Users/allure/Desktop/CreatorRelay/src-tauri/target/debug/bundle/macos/creatorrelay.app`
- 浏览器预览：Chromium in-app browser，临时视口 1440×900
- 工作区（桌面手工验证）：`/Users/allure/Desktop/发文`

## 自动化验证

| 命令 | 结果 |
| --- | --- |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib content::tests` | `10 passed; 0 failed` |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib ai::tests` | `7 passed; 0 failed` |
| `cargo test --manifest-path src-tauri/Cargo.toml` | `65` lib tests、`9` M4、`4` M6、`7` M7 integration 全部通过（M6/M7 仅确认并行改动未破坏 M5 构建） |
| `CREATORRELAY_TEST_KEYCHAIN=1 cargo test --manifest-path src-tauri/Cargo.toml --lib ai::tests::system_keyring_provider_smoke_is_explicitly_opt_in -- --nocapture` | `1 passed`；真实系统钥匙串写入/读取/删除成功，SQLite 不含测试 Key |
| `pnpm typecheck` | 通过（最终共享工作区） |
| `pnpm test` | 最终共享工作区 `11` 个测试文件、`44` 个测试全部通过（含并行新增的非 M5 测试） |
| `pnpm lint` | 通过，无 warning |
| `pnpm build` | 通过；仅 Vite chunk 体积提示 |
| `pnpm tauri build --debug` | 通过；生成 `.app` 与 `.dmg` |
| `CREATORRELAY_TEST_KEYCHAIN=1 cargo test --manifest-path src-tauri/Cargo.toml --lib platforms::tests::system_keyring_smoke_is_explicitly_opt_in -- --nocapture` | `1 passed` |

### 关键自动化用例

- `content::tests::creates_all_four_kinds_and_survives_restart`
- `content::tests::video_assets_and_metadata_persist_without_markdown_body`
- `content::tests::imported_media_bytes_can_be_read_for_preview`
- `content::tests::image_order_cover_and_captions_persist`
- `content::tests::article_body_markdown_is_authoritative_and_base_hash_blocks_overwrite`
- `content::tests::disk_write_failure_returns_error_without_replacing_content`
- `content::tests::image_text_drafts_keep_media_order_and_cover_independently`
- `content::tests::video_drafts_copy_video_and_cover_metadata`
- `content::tests::drafts_are_independent_and_track_source_changes`
- `content::tests::derived_content_is_a_new_package_with_source_reference`
- `ai::tests::provider_connection_success_and_default_switch_are_persisted`
- `ai::tests::test_connection_distinguishes_auth_failure_and_updates_status`
- `ai::tests::ai_provider_secret_never_enters_sqlite_and_success_audit_is_metadata_only`
- `ai::tests::provider_key_is_absent_from_persisted_sqlite_bytes`
- `ai::tests::prompt_contains_only_explicit_context_and_media_metadata`
- `ai::tests::rejects_non_local_plain_http_endpoints`
- `src/utils/contentChecks.test.ts`：能力过滤、硬性错误/警告/未检查、未知规则不通过
- `src/stores/studioStore.test.ts`：保存失败后编辑内容与 hash 保留

## 隔离工作树复核

为避免并行 M8 未提交改动影响 M5 变更集，使用 `git worktree add --detach /tmp/creatorrelay-m5-verify 73307e5` 创建隔离工作树，并复制本轮新增的 `src-tauri/src/content.rs` 媒体预览测试。结果：

| 命令（工作目录 `/tmp/creatorrelay-m5-verify`） | 结果 |
| --- | --- |
| `pnpm test` | `10` files / `43` tests passed |
| `pnpm lint` | pass |
| `pnpm build` | pass（仅 chunk 体积提示） |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib content::tests` | `10 passed; 0 failed` |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib ai::tests` | `7 passed; 0 failed` |

共享主工作区随后完成了并行 M8 改动；最终根工作区 `pnpm typecheck`、`pnpm build`、`pnpm test` 均已通过，隔离工作树仍保留对 M5 变更集本身的独立证明。

## 手工 / 平台验证

### 浏览器交互（1440×900）

1. 新建面板按“视频、图文、图文文章、纯文章”显示；分别创建视频与图文主稿，刷新后列表、形态、标题和编辑视图恢复。
2. 图文编辑：导入 3 张 PNG，填写逐图说明，设置第一张为封面；刷新后 3 个媒体、说明、封面和顺序保持。
3. 视频编辑：导入主视频与封面，填写简介；刷新后视频/封面元数据与简介保持，视频草稿没有生成 Markdown 正文。
4. 图文文章：导入图片、插入 `asset:<id>` 内联引用，说明保存在引用 alt 文本和媒体卡片中。
5. 保存失败：点击“故障演练”后显示“保存失败，未保存内容仍在编辑器中”和“重试保存”，标题/正文未丢失；修复后重试恢复“已保存”。
6. 平台稿：对同一图文生成抖音、小红书两份平台稿；修改并保存抖音稿后，小红书稿和主稿保持不变；修改主稿后打开抖音稿显示“源内容已变化”，可“保留当前平台稿”。
7. 内容检查：未知规则目标只显示“未检查”；可验证目标显示硬性错误、警告和通过/失败状态。
8. AI：打开发送确认弹窗后可见内容形态、供应商、模型、标题/摘要/正文范围、素材元数据和完整请求载荷；确认前不生成；生成后预览、应用、撤销均成功，主内容只在应用后变化。

### 真实 Tauri 桌面（debug bundle）

1. `pnpm tauri build --debug` 后启动 `src-tauri/target/debug/bundle/macos/creatorrelay.app`。
2. 在 `文章 -> 写作空间` 使用“新建内容”创建 `桌面图文验收草稿`，确认 `contents/8842182f-124c-42ac-8cfe-f731337cc0ac/content.json` 不含 `body.md`。
3. 填写短文案 `桌面端自动保存短文案`，按 `⌘S`，界面显示“内容已保存到工作区”；检查 `content.json` 的 `summary` 与 `updatedAt` 已更新。
4. 退出应用并重新启动；进入写作空间后列表仍显示该主稿，标题、短文案、`contentVersion` 与内容包路径恢复。
5. 验证后删除本次手工测试创建的内容包，避免污染用户工作区。

## AC 快照

| AC | 结果 | 证据 |
| --- | --- | --- |
| M5-AC-01 | pass | 新建面板顺序/四形态；浏览器刷新与真实 Tauri 退出重启后内容恢复；`creates_all_four_kinds_and_survives_restart` |
| M5-AC-02 | pass | 视频/封面元数据、简介、无空 Markdown；`video_assets_and_metadata_persist_without_markdown_body`、`imported_media_bytes_can_be_read_for_preview`；浏览器视频编辑手工检查 |
| M5-AC-03 | pass | 3 图导入、排序、封面、逐图说明；`image_order_cover_and_captions_persist`；浏览器刷新后恢复 |
| M5-AC-04 | pass | Markdown 权威与内联素材；`article_body_markdown_is_authoritative_and_base_hash_blocks_overwrite`；浏览器图文文章插入引用 |
| M5-AC-05 | pass | 真实只读目录写入失败用例、前端保存失败用例；浏览器故障演练与重试恢复 |
| M5-AC-06 | pass | 供应商保存/默认切换/连接成功与失败测试；真实钥匙串烟测；Settings 表单通过 typecheck/build |
| M5-AC-07 | pass | AI 确认弹窗展示范围和实际载荷；`prompt_contains_only_explicit_context_and_media_metadata`；手工验证确认前无结果 |
| M5-AC-08 | pass | 浏览器生成标题/应用/撤销；后端请求构造与成功调用测试 |
| M5-AC-09 | pass | `contentTargets` 测试：测试 Adapter 视频 unsupported、图文 supported；浏览器目标列表显示原因 |
| M5-AC-10 | pass | 两份平台稿独立；图文顺序/封面与视频元数据快照测试；浏览器修改一份不影响另一份 |
| M5-AC-11 | pass | `drafts_are_independent_and_track_source_changes`；浏览器源内容变化提示、保留与重新生成路径 |
| M5-AC-12 | pass | `contentChecks.test.ts` 分级和未知规则；浏览器检查面板显示错误/警告/未检查及通过/失败 |
| M5-AC-13 | pass | SQLite 文件字节扫描、错误消息不含 Key、日志脱敏测试、真实钥匙串烟测 |

## 复核备注

- 最终一次全量 `cargo test` 在多个并行任务同时编译/运行的环境下，M4 `m4_ac_04_runtime_probe_failure_overrides_manifest_supported_claim` 因 2 秒 Adapter 启动超时偶发失败（8/9 M4 通过）；隔离工作树中 M5 及 M4 此前完整运行均通过。该超时与 M5 文件无关，保留为并行负载下的测试稳定性风险。
- 快速验收补充修复了并行测试文件的 ES2020 类型兼容问题：`src/stores/mcpStore.test.ts` 的 `Array.at` 改为索引访问，`src/utils/operations.test.ts` 补齐必填 `rawFields`；最终 `pnpm typecheck`、`pnpm build`、`pnpm test`、`pnpm lint` 均通过。

## 剩余风险与未覆盖项

- 真实 Tauri 文件选择器对视频/多图的完整人工导入流程未单独完成；媒体导入、字节读取与持久化由 Rust 用例覆盖，浏览器编辑流程已手工验证。
- 真实第三方模型服务未在验证中调用；使用本地 HTTP fixture 覆盖 OpenAI Compatible 的成功、认证失败、默认切换和请求体行为。
- 浏览器预览的 AI 使用演示模型，不等同于真实 BYOK 调用；桌面端真实 Key 路径由钥匙串烟测和 fixture HTTP 测试覆盖。
- 大型视频 IPC 性能、平台真实上传/转码/时长探测仍不在 M5 范围。
