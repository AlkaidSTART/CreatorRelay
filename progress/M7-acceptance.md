# M7 验收报告 — MCP 与 Agent

- **里程碑：** M7 MCP 与 Agent
- **验收日期：** 2026-11-25
- **代码基线：** `ac99dda`（M7 实现前最后提交，尚未包含 `src-tauri/src/mcp.rs`）
- **验收头版本：** `48c07a7`，并包含验收时工作区中的 M7 文档/验证记录改动
- **总体结果：** **accepted（通过）**
- **验证记录：** [M7-verification-2026-11-25.md](./verification/M7-verification-2026-11-25.md)

> 验收范围仅为 M7。工作区同时存在 M5/M6/M8 的并行改动，本报告未将其实施状态或测试计入 M7 AC。M7 已通过的平台发布证据来自官方测试适配器，不代表真实平台接口已通过平台方验证。

## 变更文件

| 文件 | 类型 | M7 实现/变更 |
| --- | --- | --- |
| `src-tauri/src/mcp.rs` | 新增/修改 | 本地 stdio MCP Host、独立测试 Server、健康检查/崩溃重启、工具目录、JSON Schema 校验、授权令牌、Core 工具桥、超时取消、脱敏审计、Assistant 会话与模型上下文。 |
| `src-tauri/tests/m7_mcp.rs` | 新增/修改 | 8 个 M7 集成测试，覆盖 M7-AC-01 至 M7-AC-12。 |
| `src-tauri/src/lib.rs` | 修改 | 管理 `McpHost`；注册 MCP/Assistant Tauri 命令；应用启动时按策略启动测试 Server。 |
| `src-tauri/src/main.rs` | 修改 | 增加 `--mcp-test-server` 独立进程入口。 |
| `src-tauri/src/database.rs` | 修改 | 增加 Assistant 会话/消息、工具审计、授权令牌、MCP 设置与迁移。 |
| `src-tauri/src/content.rs` | 修改 | 增加只接受内容 ID 的安全草稿删除能力。 |
| `docs/mcp-agent.md` | 新增 | MCP Host 生命周期、工具目录、权限矩阵、人员确认、内容载荷、隐私和审计说明。 |
| `src/types/mcp.ts` | 新增 | MCP Host、工具、授权、审计、Assistant 与模型上下文类型。 |
| `src/stores/mcpStore.ts` | 新增 | Assistant/MCP UI 状态、Tauri 调用、预览确认、取消、设置、上下文披露与审计刷新。 |
| `src/stores/mcpStore.test.ts` | 新增 | Assistant 提案、预览和演示执行回归测试。 |
| `src/pages/AssistantPage.tsx` | 修改/替换 | 实际 Assistant 页面：对话历史、模型标识、工具过程、确认弹窗、MCP 状态/工具/权限/审计面板与隐私范围控制。 |
| `src/App.tsx` | 修改 | 将 `/assistant` 从占位页切换为实际 Assistant 页面。 |
| `src/routes/navigation.ts` | 修改 | AI 助手路由标记为可用。 |
| `src/stores/index.ts` | 修改 | 导出 MCP store。 |
| `.gitignore` | 修改 | 允许跟踪 `docs/mcp-agent.md`。 |
| `progress/M7-mcp-agent.md` | 修改 | 更新阶段任务总结、验证链接与验收状态。 |
| `progress/verification/M7-verification-2026-11-25.md` | 修改 | 记录最终 AC 快照、命令结果、手工证据和剩余风险。 |
| `progress/M7-acceptance.md` | 新增 | 本验收报告。 |

## Bug 修复

| Bug | 症状/根因 | 修复 | 回归证据 |
| --- | --- | --- | --- |
| 批量发布把待人工核对待为成功 | 发布循环对任何 `Ok(PublishTask)` 都增加成功数，导致超时/人工核对目标被错误计为已发布。 | 仅把 `SUCCESS` 或已确认的 `SCHEDULED` 计为成功；其余目标计入未完成并保留各自错误。 | `m7_batch_publish_previews_every_target_and_reports_independent_partial_result` 断言 1 成功 + 1 未完成、整体为 `partial`。 |
| Assistant 模型标识可能陈旧 | 模型请求切换 provider 后，消息仍使用线程创建时的旧 provider/model。 | 模型调用前更新线程 provider/model，再写入 Assistant 消息。 | Assistant 隐私/请求测试及取消历史测试断言实际 provider/model。 |
| 重开对话丢失工具提案参数 | 工具调用视图最初只保存摘要，重开历史后无法按原计划执行。 | 提案消息持久化经过校验的 `arguments`，执行结果消息不复制敏感正文。 | `m7_assistant_history_privacy_and_model_payload_exclude_unselected_body` 重开历史并断言提案参数仍为 `{}`。 |
| 内容工具未强制形态必需封面 | 视频/图文/图文文章可能在缺少封面时创建“可发布草稿”。 | 工具业务校验要求视频和图文类内容提供图片封面，并继续校验视频文件、顺序和正文。 | 四类内容工具测试与字段级拒绝测试。 |

## 验收标准

| AC | 结果 | 证据 |
| --- | --- | --- |
| M7-AC-01 MCP 生命周期 | pass | 启动、ping、工具发现、强杀后自动重启、显式停止均由 `m7_host_lifecycle_discovery_schema_and_crash_restart_are_observable` 覆盖。 |
| M7-AC-02 工具发现与校验 | pass | 同测试断言只读工具/Schema 发现、合法回显调用、未知字段在执行前返回字段路径。 |
| M7-AC-03 业务边界 | pass | `m7_core_boundary_and_security_evidence_do_not_expose_database_or_credentials` 验证 Core 查询、脱敏输出和进程安全证据。 |
| M7-AC-04 助手调用可见 | pass | Assistant UI 手工 smoke test显示 provider/model、工具、目标、参数、状态；历史持久化与重开由 M7 测试验证。 |
| M7-AC-05 写操作授权 | pass | 无确认无副作用、一次性令牌和拒绝路径由 `m7_write_confirmation_is_one_time_and_rejection_has_no_side_effect` 覆盖。 |
| M7-AC-06 外部操作确认 | pass | 外部预览完整展示目标/正文/媒体；拒绝后任务不变，见 `m7_publish_preview_and_authorization_preserve_task_on_rejection`。 |
| M7-AC-07 多目标发布安全 | pass | 两账号/两形态批量预览与独立部分失败由 `m7_batch_publish_previews_every_target_and_reports_independent_partial_result` 覆盖。 |
| M7-AC-08 超时与取消 | pass | 超时/取消状态、无重复执行、其他工具继续可用，见 `m7_timeout_cancel_and_audit_do_not_repeat_or_leak_secrets`。 |
| M7-AC-09 隐私控制 | pass | 未选正文不进入模型请求体，敏感值脱敏且不进入 SQLite/审计，见 Assistant 隐私测试。 |
| M7-AC-10 MCP 调用审计 | pass | 工具、形态、目标/参数摘要、时间、状态和错误持久化；敏感值缺失由超时/取消测试断言。 |
| M7-AC-11 四类内容工具 | pass | 创建并更新四类草稿、返回 ID/形态、重读恢复，见 `m7_four_content_kinds_create_update_and_restore_with_field_level_rejection`。 |
| M7-AC-12 形态载荷校验 | pass | 缺视频/空顺序/缺正文在写入前返回 `media` 或 `body` 字段错误，且内容包数量不变。 |

## 测试、构建与手工检查

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| M7 集成测试 | `cargo test --manifest-path src-tauri/Cargo.toml --test m7_mcp` | 通过，8/8 |
| Rust 全量测试 | `cargo test --manifest-path src-tauri/Cargo.toml` | 通过；共 100 个测试（73 lib、9 M4、5 M6、8 M7、5 M8） |
| TypeScript | `pnpm typecheck` | 通过 |
| 前端单测 | `pnpm test` | 通过，44/44 |
| 生产构建 | `pnpm build` | 通过；仅有 Vite 大 chunk 警告 |
| 前端 lint | `pnpm lint` | 通过 |
| Rust 格式 | `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 通过 |
| 仓库 Prettier | `pnpm format:check` | 失败；仅两个既有非 M7 文件 `src/components/StudioWorkspaceEmpty.tsx`、`src/pages/StudioPage.css` 未格式化 |
| UI smoke | `pnpm dev:web --host 127.0.0.1 --port 4173`，访问 `/#/assistant` | 通过；助手工具提案、授权预览、执行结果、MCP 状态/权限面板和隐私范围面板均可见 |
| 桌面进程观察 | `pgrep -fl 'target/debug/creatorrelay|--mcp-test-server'` | 观察到主应用进程与 `--mcp-test-server` 子进程；原生窗口无障碍附加超时，未完成原生窗口视觉检查 |

## 剩余风险、排除项与后续

- 原生 Tauri 窗口本次未完成自动视觉附加；共享 React UI 已手工验证，Tauri 命令与生命周期由 Rust 集成测试覆盖。建议后续 CI 或桌面 smoke test矩阵补上原生窗口自动化。
- `pnpm format:check` 的两个失败文件属于既有非 M7 改动，未在本次验收中修改；仓库统一格式化仍需单独处理。
- 实际平台 API、平台审核策略和真实账号发布仍需按 M8 平台准入逐个验证；M7 的多目标证据使用官方测试适配器。
- 当前 M7 仅支持用户发起、逐项授权的工具工作流；复杂多 Agent 自主编排仍按 PRD 排除在 MVP 之外。
- 外部发布在可自动执行时仍依赖 M6 发布任务状态机；M6 的真实平台能力应按其独立验收记录维护。
