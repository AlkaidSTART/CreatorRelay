# M7 验证记录 — 2026-11-25

> 本文件只记录验证结果，不是验收报告，也不能单独作为 M7 已验收的依据。

- **阶段：** M7 MCP 与 Agent
- **记录日期：** 2026-11-25
- **代码基线：** `ac99dda`（M7 实现前的最后提交；未包含 `src-tauri/src/mcp.rs`）
- **验证工作区：** `48c07a7` 加上 M7 验证/文档未提交改动；工作区同时含 M5/M6/M8 的并行改动，已从 M7 变更归因中排除
- **总体状态：** `pass`

## 自动化验证

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| M7 集成测试 | `cargo test --manifest-path src-tauri/Cargo.toml --test m7_mcp` | 通过，8/8 |
| Rust 全量测试 | `cargo test --manifest-path src-tauri/Cargo.toml` | 通过；73 个 lib 单测、9 个 M4、5 个 M6、8 个 M7、5 个 M8，总计 100/100 |
| TypeScript | `pnpm typecheck` | 通过 |
| 前端单测 | `pnpm test` | 通过，44/44（11 个测试文件） |
| 生产构建 | `pnpm build` | 通过；Vite 生产包生成成功 |
| 前端 lint | `pnpm lint` | 通过 |
| Rust 格式 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 通过 |
| 仓库 Prettier | `pnpm format:check` | 失败；仅 `src/components/StudioWorkspaceEmpty.tsx` 与 `src/pages/StudioPage.css` 两个既有非 M7 文件未格式化 |

M7 集成测试文件：`src-tauri/tests/m7_mcp.rs`。

## AC 快照

| AC | 结果 | 验证证据 |
| --- | --- | --- |
| M7-AC-01 MCP 生命周期 | pass | `m7_host_lifecycle_discovery_schema_and_crash_restart_are_observable`：启动后为 `healthy`，`ping`/工具发现成功；强杀 PID 后 `health_check` 自动重启且 `restartCount=1`；显式停止后 `pid=None`。开发模式进程树中亦观察到 `target/debug/creatorrelay` 与其 `--mcp-test-server` 子进程。 |
| M7-AC-02 工具发现与校验 | pass | 同一测试通过 `tools/list` 发现 `account_get_info` 与版本化 Schema；未知字段调用在执行前返回 `INVALID_ARGUMENT` 和字段路径 `unexpected`；合法 `mcp_test_echo` 成功。 |
| M7-AC-03 业务边界 | pass | `m7_core_boundary_and_security_evidence_do_not_expose_database_or_credentials`：账号经 Core 服务返回，输出不含 `profile_id`/`credential_ref`；安全证据确认独立进程、stdio、未传入数据库路径/凭据引用、无直接 SQLite/Keychain API。 |
| M7-AC-04 助手调用可见 | pass | Assistant UI 实时展示服务商、模型、工具名、权限、目标、脱敏参数、状态与错误；`m7_timeout_cancel_and_audit_do_not_repeat_or_leak_secrets` 断言取消后的历史消息保留 provider/model/tool 状态；`m7_assistant_history_privacy_and_model_payload_exclude_unselected_body` 断言重开历史可恢复消息。 |
| M7-AC-05 写操作授权 | pass | `m7_write_confirmation_is_one_time_and_rejection_has_no_side_effect`：无令牌创建草稿被拒绝且内容包数量不变；确认令牌成功后只能使用一次，重放返回 `AUTHORIZATION_USED`。 |
| M7-AC-06 外部操作确认 | pass | `m7_publish_preview_and_authorization_preserve_task_on_rejection`：预览包含平台、账号、内容形态、正文与媒体；拒绝执行后任务状态、更新时间和尝试次数完全不变。 |
| M7-AC-07 多目标发布安全 | pass | `m7_batch_publish_previews_every_target_and_reports_independent_partial_result`：批量预览包含两个账号/两种形态/各自平台稿；执行结果为 `partial`（1 成功、1 待人工核对），成功任务保持 `SUCCESS`，失败任务未被成功目标回滚。 |
| M7-AC-08 超时与取消 | pass | `m7_timeout_cancel_and_audit_do_not_repeat_or_leak_secrets`：挂起工具 250 ms 后返回 `timedOut/TOOL_TIMEOUT`；另一调用取消后返回 `cancelled/TOOL_CANCELLED`；随后 MCP 回显工具仍可正常使用。 |
| M7-AC-09 隐私控制 | pass | `m7_assistant_history_privacy_and_model_payload_exclude_unselected_body`：未选择正文时模型 HTTP 请求体不含 `SENSITIVE_BODY`；`api_key=model-secret-value` 被脱敏；API Key 仅出现在认证头，不出现在请求体、SQLite 或审计摘要。 |
| M7-AC-10 MCP 调用审计 | pass | 上述集成测试断言审计含 `timedOut`/`cancelled` 状态；`McpCallResult` 持久化调用时间、工具、形态、目标/参数摘要、状态和错误；摘要不保存正文原文或敏感值。 |
| M7-AC-11 四类内容工具 | pass | `m7_four_content_kinds_create_update_and_restore_with_field_level_rejection`：通过工具创建并更新 `video`、`imageText`、`imageArticle`、`article`，返回 ID/形态正确；重新读取内容包可恢复。 |
| M7-AC-12 形态载荷校验 | pass | 同测试：视频缺少视频、图文空顺序、文章缺正文均在写入前被拒绝，并返回字段级路径 `media` 或 `body`；内容包总数不变。 |

## 手工 / 平台证据

1. 启动 `pnpm dev:web --host 127.0.0.1 --port 4173`，打开 `http://127.0.0.1:4173/#/assistant`。
2. Accessibility 快照确认页面显示 `MCP Host 健康`、协议 `2024-11-05`、工具目录与“外部发布始终每次确认”策略。
3. 点击“查询所有已绑定账号的登录状态”后出现 `account_get_info` 提案，显示“只读”、目标与参数摘要；点击“预览并执行”出现授权预览，包含目标、内容形态、脱敏参数、预期副作用和最终内容区域。
4. 点击“确认并执行”后，对话出现工具完成消息；Web 演示模式明确标注未产生真实副作用。
5. “模型与隐私”页显示服务商/模型、内容来源、标题/摘要/正文/素材元数据复选框；正文默认未勾选，发送按钮仅在显示实际上下文后出现。
6. 桌面试跑期间通过 `pgrep -fl 'target/debug/creatorrelay|--mcp-test-server'` 观察到主进程与本地 MCP Server 子进程；原生窗口的无障碍连接超时，因此原生窗口视觉检查未独立完成。

## 未覆盖与风险

- 原生 Tauri 窗口本次未能附加成功；AC-04 的 UI 证据来自同一 React 页面在 Vite 中的手工 smoke test，Tauri 命令链路由 Rust 集成测试覆盖。
- `pnpm format:check` 仍受两个未修改的既有文件影响；这不改变 M7 功能 AC，但在仓库统一格式化前不能宣称全仓格式检查通过。
- 测试平台适配器只用于发布/超时/部分失败场景；真实平台开放接口能力仍需在 M8 和各平台实际接入时逐项验证。
- 本记录的 `pass` 不代表 PRD 全量 MVP 验收；M2–M6/M8 的独立验收状态仍以其各自文档为准。

## 结论

M7 的 12 项验收标准均取得可复现证据，未发现阻断验收的已知缺陷。可以创建 M7 验收报告。
