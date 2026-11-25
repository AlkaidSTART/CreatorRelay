# M1 验收报告：桌面基础设施

- **里程碑：** M1 桌面基础设施
- **验证日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（25G83），arm64
- **代码基线：** `9d6710bc11a939a1672b0910a536891a9012a5db`
- **受检版本：** 基线之后的 M1 工作区改动（尚未提交）；隔离验收数据位于 `/tmp/creatorrelay-m1-acceptance-20261125`
- **总体结果：** **accepted**
- **验证记录：** [M1-verification-2026-11-25.md](./verification/M1-verification-2026-11-25.md)

## 变更文件

本 Milestone 以 `9d6710b` 为基线，只统计 M1 基础设施改动。工作区中已有的 M3 文档、平台登录 URL、平台 Store 测试等未纳入本清单；共享文件若同时含既有改动，下表只说明 M1 所属部分。

| 类型 | 文件 | 实现内容 |
| --- | --- | --- |
| 新增 | `src-tauri/src/database.rs` | schema v1/v2 版本化迁移、事务回滚、v1 fixture、幂等测试 |
| 新增 | `src-tauri/src/logging.rs` | JSON Lines 日志、目录/文件权限、敏感字段脱敏和测试 |
| 新增 | `src-tauri/src/diagnostics.rs` | 运行时诊断快照、IPC/数据库安全探针和故障恢复测试 |
| 新增 | `src/components/ModulePlaceholder.tsx` | 未开放模块统一占位页，阻断演示业务数据 |
| 新增 | `docs/m1-desktop-foundation.md` | 启动、目录、迁移、IPC、日志、诊断和构建说明 |
| 新增 | `progress/M1-acceptance.md` | 本验收报告 |
| 修改 | `src-tauri/src/lib.rs` | 接入迁移器、启动日志、诊断 Commands、IPC 错误日志、路径错误稳定化和重启持久化测试 |
| 修改 | `src-tauri/src/platforms.rs` | 平台 schema 创建交由 M1 迁移器统一管理；平台测试改用统一迁移入口 |
| 修改 | `src/App.tsx` | 未验收模块路由切换到占位页，仅保留真实已落地入口 |
| 修改 | `src/layouts/AppLayout.tsx` | 导航显示可用/开发中/规划中状态，开发中模块展示阶段提示 |
| 修改 | `src/pages/SettingsPage.tsx` | M1 运行时诊断、schema/完整性/路径展示和安全故障探针 |
| 修改 | `src/routes/navigation.ts` | 路由元数据增加 availability 与 milestone |
| 修改 | `src/routes/navigation.test.ts` | 模块开放状态回归测试 |
| 修改 | `.gitignore` | 允许 M1 实现文档和 progress 验收文件纳入版本控制 |
| 修改 | `README.md` | 增加 M1 实现说明链接 |
| 修改 | `progress/M1-desktop-foundation.md` | 更新阶段总结、证据和验收状态 |
| 修改 | `progress/verification/M1-verification-2026-11-25.md` | 写入 M1 AC、命令、手工证据和残余风险 |

无删除文件。

## 缺陷修复

| 缺陷 | 症状 / 根因 | 修复 | 回归证据 |
| --- | --- | --- | --- |
| 数据库启动路径没有真正版本化 | 启动时直接 `CREATE TABLE IF NOT EXISTS` 并插入 v1，后续平台 schema 逻辑独立写版本，无法证明旧库升级和失败回滚 | 新增 `database.rs`，按 `schema_migrations` 逐版本在事务中执行和记录；应用启动统一调用迁移器 | `migrates_v1_fixture_without_losing_data`、`failed_migration_rolls_back_and_preserves_database`、`migration_is_idempotent` |
| 未交付模块显示演示业务数据 | 运营总览、发布、内容、分析和助手页面直接使用 demo 数据，容易被误认为已交付 | 路由标记 planned/development/available；planned 路由统一渲染占位页；导航显示状态 | `navigation.test.ts`；桌面手工检查占位页无演示指标 |
| 缺少启动、IPC 和迁移日志 | 出错时只有界面错误，无法从日志定位阶段，也没有脱敏基础设施 | 新增结构化日志、固定阶段名、启动迁移日志和 IPC 关键命令日志 | 日志测试；验收数据目录中的 `startup.ready`、`ipc.workspace.set` 等证据 |
| 读取轮询造成日志膨胀 | 写作空间每 2 秒调用读取命令，成功读取也写日志 | 读取成功不记录，仅失败记录错误阶段 | 最终构建进入写作空间等待 6 秒，新会话无重复 `ipc.document.read` |
| 设置页显示过期数据库路径 | 固定文案 `data/creator-relay.db` 与实际应用数据目录不一致 | 改为运行时诊断返回的真实数据库路径 | 桌面设置页显示隔离验收目录下的真实 `creatorrelay.sqlite3` |

## 验收标准

| ID | 验收场景 | 结果 | 证据 |
| --- | --- | --- | --- |
| M1-AC-01 | 首次启动 | pass | 全新隔离目录启动 macOS `.app`，主窗口和 8 个导航入口可见；schema 从 v0 迁移到 v2；SQLite 记录 v1、v2 |
| M1-AC-02 | 模块导航 | pass | 导航区分可用/开发中/规划中；planned 模块显示阶段占位，不渲染 demo 指标；设置页可切换并展示诊断 |
| M1-AC-03 | IPC 边界 | pass | UI 通过 Rust IPC 创建、保存、读取文章；非法工作区路径返回稳定错误；前端数据库访问扫描无结果；后端路径逃逸测试通过 |
| M1-AC-04 | 数据库升级 | pass | v1 fixture 升级到 v2 后设置、文档和 revision 完整；冲突迁移事务回滚，版本保持 v1；重复迁移无副作用 |
| M1-AC-05 | 重启持久化 | pass | 保存文章后关闭并重启，同一数据目录恢复工作区、文档列表、正文和索引；文件哈希与数据库内容哈希一致 |
| M1-AC-06 | 日志脱敏 | pass | JSON、Header、键值和 Bearer Token 脱敏测试通过；手工探针日志仅含 `[REDACTED]`；模拟敏感值扫描无匹配 |
| M1-AC-07 | 可诊断性 | pass | 非法 IPC 和数据库探针均显示顶部错误，日志包含错误阶段，应用保持运行并可继续读取数据库 |

## 测试、构建与手工检查

| 检查 | 命令 / 场景 | 结果 |
| --- | --- | --- |
| TypeScript | `pnpm typecheck` | pass |
| Lint | `pnpm lint` | pass |
| 前端测试 | `pnpm test` | pass，6 个文件、30 个测试 |
| 前端构建 | `pnpm build` | pass；仅有现有 chunk 体积警告 |
| M1 格式 | `pnpm exec prettier --check <M1 修改文件>` | pass |
| Rust 格式 | `cargo fmt --all -- --check` | pass |
| Rust lint | `cargo clippy --all-targets -- -D warnings` | pass |
| Rust 测试 | `cargo test` | pass，38 个测试 |
| Debug App | `pnpm tauri build --debug --bundles app --no-sign` | pass |
| Release App | `pnpm tauri build --bundles app --no-sign` | pass，产物 `src-tauri/target/release/bundle/macos/creatorrelay.app` |
| 首次启动 | `CREATORRELAY_DATA_DIR=/tmp/creatorrelay-m1-acceptance-20261125 ...` | pass，schema v0 → v2，日志与导航正常 |
| IPC 读写 | 新建“M1 IPC 重启持久化”并 `⌘S` | pass，状态“已保存到工作区” |
| 重启持久化 | 关闭应用后同目录重启 | pass，文章、正文、索引恢复 |
| 非法参数 | “测试非法参数 IPC” | pass，显示“工作区路径不存在或不可访问”，应用继续运行 |
| 日志脱敏 | “测试日志脱敏” | pass，日志无模拟 password/cookie/token 明文 |
| 数据库错误 | “测试数据库错误” | pass，显示明确错误，日志记录 `diagnostics.database`，应用未崩溃 |
| 数据库完整性 | `PRAGMA integrity_check` | `ok` |
| 日志权限 | `stat -f '%Sp'` | `-rw-------` |

## 剩余风险、排除项与后续事项

- 本次平台验收仅覆盖 macOS 26.6.2 arm64。Windows/Linux 的安装、窗口行为、系统凭据和日志权限仍需在对应平台验证。
- Release `.app` 已构建但未签名、未公证；正式分发安装包和升级链路不在本次 M1 验收范围。
- 全仓库 `pnpm format:check` 仍报告 3 个 M1 范围外且未修改的文件：`StudioWorkspaceEmpty.tsx`、`AssistantPage.tsx`、`StudioPage.css`。M1 修改文件均通过格式检查。
- 写作空间和平台管理仍为开发中入口，不计入 M1 已交付业务能力；它们分别等待 M2/M5 和 M3/M4 验收。
- 前端仍保留后续阶段用于用户选择文件导入/导出的 fs 插件调用；本阶段已确认前端没有直接 SQLite 访问路径，该功能不纳入 M1 能力声明。
- 后续应补充 CI 中的 macOS 打包检查和至少一个其他桌面平台的启动/迁移/日志脱敏矩阵。

## 结论

M1 的 7 项验收标准全部通过，桌面应用、版本化数据库迁移、受控 IPC、错误反馈、脱敏日志和重启持久化具备可复现证据。**M1 验收通过，可解除 M2/M3 的阶段门。**
