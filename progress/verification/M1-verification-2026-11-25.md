# M1 验证记录 — 2026-11-25

> 本文件只记录验证结果，不是验收报告，也不能单独作为 M1 已验收的依据。

- **阶段：** M1 桌面基础设施
- **记录日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（25G83），arm64
- **代码基线：** `9d6710bc11a939a1672b0910a536891a9012a5db`
- **受检实现：** 当前工作区 M1 改动（基线之后未提交）；用于隔离验证的数据目录为 `/tmp/creatorrelay-m1-acceptance-20261125`
- **变更范围：** 版本化迁移、启动/IPC 日志、日志脱敏、运行时诊断、模块开放状态、M1 测试与文档
- **总体状态：** `pass`

## AC 快照

| 验收标准 | 结果 | 证据 |
| --- | --- | --- |
| M1-AC-01 首次启动 | pass | 使用全新数据目录启动 macOS `.app`，主窗口和 8 个导航入口可见；启动日志记录 `schema_from=0 schema_to=2 applied=[1, 2]`，SQLite `schema_migrations` 存在 v1、v2 记录 |
| M1-AC-02 模块导航 | pass | 导航显示“可用 / 开发中 / 规划中”；运营总览、发布中心、内容中心、数据分析、AI 助手显示明确占位页，不渲染演示指标；写作空间和平台管理标为开发中；系统设置可用 |
| M1-AC-03 IPC 边界 | pass | 桌面 UI 通过 `create_document`、`save_document`、`read_document` 完成代表性写入/读取；不存在工作区路径经 `set_workspace_path` 返回“工作区路径不存在或不可访问”；前端源码扫描无 SQLite/rusqlite/数据库文件访问路径 |
| M1-AC-04 数据库升级 | pass | `database::tests::migrates_v1_fixture_without_losing_data`、`failed_migration_rolls_back_and_preserves_database`、`migration_is_idempotent` 全部通过；失败冲突表场景回滚到 v1 且原有设置/文档/revision 保留 |
| M1-AC-05 重启持久化 | pass | 手工创建并保存“M1 IPC 重启持久化”，关闭应用后使用同一数据目录重启，文章列表、正文、索引均恢复；自动化 `workspace_and_index_survive_database_reopen` 通过；重启为 `schema_from=2 schema_to=2 applied=[]` |
| M1-AC-06 日志脱敏 | pass | `logging::tests::redacts_json_headers_and_key_values_without_losing_context`、`log_file_contains_stage_and_context_but_not_secret` 通过；手工脱敏探针写入固定模拟 password/cookie/token，日志仅含 `[REDACTED]`，扫描无模拟值 |
| M1-AC-07 可诊断性 | pass | 手工触发非法 IPC 与数据库诊断错误，顶部错误栏展示可识别信息，应用继续运行；日志包含 `ipc.workspace.set`、`diagnostics.ipc_payload`、`diagnostics.database` 阶段；`diagnostic_probes_log_context_and_leave_database_usable` 通过 |

## 自动化检查

| 命令 | 结果 |
| --- | --- |
| `pnpm typecheck` | pass |
| `pnpm lint` | pass |
| `pnpm test` | pass，6 个测试文件、30 个测试 |
| `pnpm build` | pass；仅保留 Vite 单 chunk 大于 500 kB 的现有体积警告 |
| `pnpm exec prettier --check <M1 修改文件>` | pass |
| `pnpm format:check`（全仓库） | fail；仅报告 M1 范围外且未修改的 `src/components/StudioWorkspaceEmpty.tsx`、`src/pages/AssistantPage.tsx`、`src/pages/StudioPage.css` |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass，38 个测试 |
| `pnpm tauri build --debug --bundles app --no-sign` | pass，`src-tauri/target/debug/bundle/macos/creatorrelay.app` |
| `pnpm tauri build --bundles app --no-sign` | pass（release），`src-tauri/target/release/bundle/macos/creatorrelay.app` |

## 手工与平台证据

### 首次启动与导航

使用隔离数据目录启动：

```bash
CREATORRELAY_DATA_DIR=/tmp/creatorrelay-m1-acceptance-20261125 \
  src-tauri/target/debug/bundle/macos/creatorrelay.app/Contents/MacOS/creatorrelay
```

观察到：

- 主窗口标题为 `CreatorRelay — 写作空间`，导航包含 8 个模块。
- 运营总览显示“M6 · 规划中 / 尚未开放”，不再渲染演示业务数据。
- 设置页运行时诊断显示 `v2 / v2 · ok`、真实数据库路径和日志路径。

### 工作区、IPC 与重启

1. 在系统文件夹选择器中选择 `/tmp/creatorrelay-m1-workspace-20261125`。
2. 通过“新建文章”创建“M1 IPC 重启持久化”，写入正文并 `⌘S` 保存。
3. 状态显示“已保存到工作区”。
4. 关闭应用，使用同一数据目录重启。
5. 写作空间恢复同一文章、正文和 `已保存` 状态。

重启后文件哈希：

```text
65bc37d866725a88c1dd1f9afcfbf2bd92952b20e2edc75fe860e45edd3424e4
```

SQLite：

```text
documents = 1
PRAGMA integrity_check = ok
workspace_path = /private/tmp/creatorrelay-m1-workspace-20261125
```

### 错误与日志

非法 IPC 错误文案：

```text
工作区路径不存在或不可访问
```

日志关键内容：

```text
stage=startup.ready schema_from=0 schema_to=2 applied=[1, 2]
stage=ipc.workspace.set error=工作区路径不存在或不可访问
stage=diagnostics.ipc_payload request={"password":"[REDACTED]","cookie":"[REDACTED]","token":"[REDACTED]","context":"workspace-open"}
stage=diagnostics.database error=no such table: __creatorrelay_missing_diagnostic_table__
```

模拟敏感值扫描：

```bash
rg -n 'm1-diagnostic-password|m1-diagnostic-cookie|m1-diagnostic-token' \
  /tmp/creatorrelay-m1-acceptance-20261125
# 无匹配
```

日志文件权限：

```text
-rw------- /tmp/creatorrelay-m1-acceptance-20261125/logs/creatorrelay.log
```

最终构建复测中，进入写作空间并等待 6 秒后，新的会话日志没有重复的 `ipc.document.read` 成功记录；读取仅在失败时记录。

## 未覆盖项与风险

- 仅实测 macOS arm64；Windows 和 Linux 的安装、Keychain/Credential Store、文件选择和窗口行为仍未验证，属于后续平台兼容性风险。
- Release `.app` 已构建，但本次未签名、未公证，也未做正式安装包分发检查。
- 全仓库 Prettier 仍有 3 个 M1 范围外文件未格式化；M1 修改文件已通过检查。
- 前端仍保留后续阶段用于用户选择文件导入/导出的 Tauri fs 插件调用；本阶段已确认前端没有直接 SQLite 访问路径，也未把该功能计入 M1 已交付能力。
- M2/M3 页面作为开发中入口保留，尚未通过各自阶段验收。
