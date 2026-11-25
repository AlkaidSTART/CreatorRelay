# M5 内容工作室验收报告

- **阶段：** M5 内容工作室
- **验证日期：** 2026-11-25
- **M5 基线：** `5a71464`
- **验证时仓库 HEAD：** `73307e5`（M5 代码已进入 `abe95d5`、`d709a52`、`1f52f9d`；验收时工作区另有并行的 M6/M7 改动，未计入 M5 变更集）
- **总体结果：** **accepted（通过）**
- **验证记录：** [M5 验证记录](./verification/M5-verification-2026-11-25.md)
- **实现说明：** [内容工作室实现说明](../docs/content-studio.md)

## 变更文件

> 仅列出属于 M5 的实现、文档和测试；M6/M7 的并行改动（如 `operations.rs`、`mcp.rs`、`ecosystem.rs`）不属于本阶段。

| 文件 | 变更 | 作用 |
| --- | --- | --- |
| `src-tauri/src/content.rs` | 新增 | 四类内容包、`content.json`/`body.md` 权威边界、媒体导入与预览读取、平台稿快照、版本差异、保留/重新生成、跨形态派生及单元测试 |
| `src-tauri/src/ai.rs` | 新增 | BYOK Provider、OpenAI Compatible 调用、默认模型、连接测试、请求构造、调用审计、钥匙串凭据与敏感值测试 |
| `src-tauri/src/database.rs` | 修改 | schema v4：`ai_providers` 与 `ai_call_audit`；迁移与回滚测试 |
| `src-tauri/src/lib.rs` | 修改 | M5 内容包/平台稿/AI Tauri commands；工作区创建 `contents/` 与 `platform-drafts/` |
| `src-tauri/Cargo.toml` | 修改 | 增加 `url` 依赖与 `ureq` JSON feature |
| `src-tauri/Cargo.lock` | 修改 | 依赖锁定 |
| `src/types/content.ts` | 新增 | 内容形态、内容包、平台稿和编辑输入类型 |
| `src/types/ai.ts` | 新增 | BYOK Provider、生成请求与形态化 AI 操作定义 |
| `src/utils/contentTargets.ts` | 新增 | 平台账号与 Adapter 能力过滤 |
| `src/utils/contentChecks.ts` | 新增 | 硬性错误、警告、未检查三级的平台规则检查 |
| `src/utils/contentChecks.test.ts` | 新增 | 能力过滤、检查分级、未知规则与通过/失败测试 |
| `src/stores/studioStore.ts` | 新增 | 四类编辑器状态、自动保存、保存失败保护、媒体顺序/封面、平台稿独立性、版本差异处理 |
| `src/stores/studioStore.test.ts` | 新增 | 保存失败后保留编辑内容和 hash 的回归测试 |
| `src/stores/aiStore.ts` | 新增 | Provider 配置、测试连接、默认模型与生成调用状态 |
| `src/stores/uiStore.ts` | 修改 | 新建内容时持久化所选 `ContentKind` |
| `src/stores/index.ts` | 修改 | 导出内容工作室与 AI store |
| `src/components/studio/MediaAsset.tsx` | 新增 | 内容包/平台稿素材字节读取与图片/视频预览 |
| `src/components/studio/MarkdownPreview.tsx` | 新增 | `asset:` 内联媒体 Markdown 预览 |
| `src/components/studio/ContentEditor.tsx` | 新增 | 视频、图文、图文文章、纯文章四类形态专属编辑与媒体操作 |
| `src/components/studio/PlatformDraftPanel.tsx` | 新增 | 目标能力展示、多平台稿生成、平台稿独立编辑与版本冲突操作 |
| `src/components/studio/AiAssistantPanel.tsx` | 新增 | 形态化 AI 操作、输入范围确认、载荷预览、结果应用/丢弃/撤销 |
| `src/components/studio/ContentChecksPanel.tsx` | 新增 | 主稿/平台稿检查结果展示与通过/失败状态 |
| `src/components/AiProviderSettings.tsx` | 新增 | 供应商、兼容端点、模型、连接测试和默认模型设置 |
| `src/components/Icons.tsx` | 修改 | 新增内容工作室所需图标 |
| `src/pages/ContentStudioPage.tsx` | 新增 | 三栏内容工作室、四类新建入口、搜索/筛选、保存状态、派生入口 |
| `src/pages/SettingsPage.tsx` | 修改 | 用真实 BYOK 配置替换“规划中”占位 |
| `src/layouts/AppLayout.tsx` | 修改 | 新建主稿时选择内容形态并调用内容包创建 |
| `src/App.tsx` | 修改 | `/studio` 指向内容工作室，保留 `/studio/classic` 经典 Markdown 工作区 |
| `src/routes/navigation.ts` | 修改 | 写作空间标记为可用并更新描述 |
| `src/routes/navigation.test.ts` | 修改 | 同步写作空间可用状态断言 |
| `docs/content-studio.md` | 新增 | 内容包、编辑器、平台稿、检查矩阵、AI 隐私和限制说明 |
| `docs/local-workspace-format.md` | 修改 | 增加 `contents/` 与 `platform-drafts/` 权威来源说明 |
| `README.md` | 修改 | 增加 M5 实现说明入口 |
| `progress/verification/M5-verification-2026-11-25.md` | 重写 | M5 验证记录、AC 快照、手工/自动化证据与残余风险 |
| `progress/M5-content-studio.md` | 修改 | 阶段任务总结、验收状态和报告链接 |

## Bug 修复

| Bug / 根因 | 症状 | 修复 | 回归证据 |
| --- | --- | --- | --- |
| 平台稿复制素材时写回源内容包目录，平台稿目录没有素材快照 | 图文平台稿保存时报“素材文件不存在”，且平台稿并非真正独立 | `copy_asset` 拆分源/目标目录；派生、生成平台稿、重新生成都写入各自目标目录 | `image_text_drafts_keep_media_order_and_cover_independently`、`video_drafts_copy_video_and_cover_metadata` |
| 浏览器演示模式下平台稿 `sourceChanged` 不会在主稿保存后重新计算 | 主稿变化后打开平台稿不提示版本差异 | 读取/保存平台稿时按当前 `contentVersion` 重新计算 `sourceChanged` | 浏览器手工验证：修改主稿后打开抖音稿出现“源内容已变化” |
| 有效标题被记录为“警告”，导致检查结果语义混淆 | 标题合法的内容仍显示警告 | 标题在范围内不再生成 warning；只有越界/缺失为硬性错误 | `contentChecks.test.ts` 完整图文用例 `errorCount === 0` |
| AI 连接成功测试中第二个 fixture server 从未收到请求，测试线程永久等待 | `ai::tests` 偶发挂起 | 移除未使用的第二个 fixture server，只 join 实际接收请求的 server | `ai::tests` 7 tests 全部通过 |

## 验收标准

| ID | 验收场景 | 结果 | 证据 |
| --- | --- | --- | --- |
| M5-AC-01 | 四类新建入口 | pass | 新建面板按视频/图文/图文文章/纯文章显示；真实 Tauri 创建后退出重启恢复；`creates_all_four_kinds_and_survives_restart` |
| M5-AC-02 | 视频编辑 | pass | 视频/封面元数据与简介持久化、无空 Markdown；`video_assets_and_metadata_persist_without_markdown_body`、`imported_media_bytes_can_be_read_for_preview` |
| M5-AC-03 | 图文编辑 | pass | 3 图导入、排序、封面、逐图说明；`image_order_cover_and_captions_persist`；浏览器刷新后恢复 |
| M5-AC-04 | 文章编辑 | pass | Markdown 权威与内联素材引用；`article_body_markdown_is_authoritative_and_base_hash_blocks_overwrite`；浏览器图文文章插入 `asset:` 引用 |
| M5-AC-05 | 自动保存失败 | pass | 只读目录写入失败、前端故障演练、重试恢复；`disk_write_failure_returns_error_without_replacing_content`、`studioStore.test.ts` |
| M5-AC-06 | 模型配置 | pass | Provider 保存/默认切换/连接成功与失败；真实系统钥匙串烟测；`provider_connection_success_and_default_switch_are_persisted` |
| M5-AC-07 | 隐私告知 | pass | 浏览器确认弹窗展示内容形态、Provider、模型、范围和实际载荷；确认前无结果；`prompt_contains_only_explicit_context_and_media_metadata` |
| M5-AC-08 | 形态化 AI 辅助 | pass | 视频标题/简介、图文短文案、文章标题/摘要操作按形态提供；浏览器生成/应用/撤销；后端成功调用 fixture |
| M5-AC-09 | 平台选择过滤 | pass | `contentTargets` 测试：测试 Adapter 视频 unsupported、图文 supported；UI 显示能力原因并禁用不支持项 |
| M5-AC-10 | 多平台适配 | pass | 同一主稿生成两份独立平台稿；图文顺序/封面、视频元数据快照；修改一份不影响另一份和主稿 |
| M5-AC-11 | 版本差异保护 | pass | 主稿变化后提示源版本差异；手动修改保留；可保留当前稿或重新生成；`drafts_are_independent_and_track_source_changes` |
| M5-AC-12 | 内容检查 | pass | 硬性错误/警告/未检查分开；未知规则不显示通过；`contentChecks.test.ts` 和浏览器检查面板 |
| M5-AC-13 | 凭据与日志 | pass | SQLite 文件字节不含测试 Key；错误消息不含 Key；日志脱敏；真实钥匙串写入/读取/删除烟测 |

## 测试、构建与手工检查

- `cargo test --manifest-path src-tauri/Cargo.toml --lib content::tests` → 10 passed
- `cargo test --manifest-path src-tauri/Cargo.toml --lib ai::tests` → 7 passed
- `cargo test --manifest-path src-tauri/Cargo.toml` → 65 lib、9 M4、4 M6、7 M7 integration 全部通过
- `CREATORRELAY_TEST_KEYCHAIN=1 cargo test ... ai::tests::system_keyring_provider_smoke_is_explicitly_opt_in` → 1 passed
- `pnpm test` → 11 files / 44 tests passed（最终共享工作区，含并行非 M5 测试）
- `pnpm lint` → pass
- `pnpm build` → pass（仅 chunk 体积提示）
- `pnpm tauri build --debug` → 生成 `.app` 与 `.dmg`
- 隔离工作树 `73307e5`：`pnpm test`、`pnpm lint`、`pnpm build`、`cargo test ... content::tests`、`cargo test ... ai::tests` 全部通过
- 浏览器手工：四类新建、视频/图文/图文文章编辑、刷新恢复、保存失败与重试、双平台稿独立性、版本冲突、检查面板、AI 隐私确认/应用/撤销
- 真实 Tauri 手工：创建图文主稿、保存短文案、退出重启后内容包与编辑状态恢复；测试后清理手工内容包

完整命令、输出摘要和手工步骤见 [M5 验证记录](./verification/M5-verification-2026-11-25.md)。

## 剩余风险与排除项

- 真实 Tauri 文件选择器对视频/多图的完整人工导入流程未单独完成；媒体导入、字节读取和持久化由 Rust 用例覆盖，浏览器编辑流程已手工验证。
- 真实第三方模型服务未调用；OpenAI Compatible 成功、认证失败、默认切换与请求体行为使用本地 HTTP fixture 验证。
- M5 不执行真实平台发布、转码、时长探测和上传；这些属于 M6/平台真实接口验证范围。
- 超大视频通过 IPC 传输的性能仍需后续优化。
- 并行 M6/M7/M8 改动不属于本报告验收范围。M5 在隔离工作树 `73307e5` 中通过全部构建与测试；最终共享工作区 `pnpm typecheck`、`pnpm build`、`pnpm test`、`pnpm lint` 也全部通过。
- 最终一次全量 Rust 测试在多个并行任务同时运行时出现 M4 2 秒 Adapter 启动超时（非 M5 测试）；M5 定向测试与隔离工作树均为绿色，该事件作为测试稳定性风险保留。
- M2 里程碑在本报告生成时仍未完成独立验收；M5 的全部 AC 已独立通过，但未把 M2 的索引重建、导入/导出、内容清单版本恢复等 AC 声称为通过。M2 阶段门仍需单独关闭。
