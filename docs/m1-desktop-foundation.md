# M1 桌面基础设施实现说明

> 状态：M1 实现与验证基线。仅 M1 范围内已开放的能力标记为可用，后续阶段模块不展示演示业务数据。

## 1. 启动与目录

CreatorRelay 使用 Tauri 2 + React/TypeScript + Rust Core。正式 macOS 构建输出为：

```text
src-tauri/target/debug/bundle/macos/creatorrelay.app
```

应用数据目录按 Tauri 标识 `com.allure.creatorrelay` 解析。调试构建支持 `CREATORRELAY_DATA_DIR` 环境变量覆盖数据目录，用于隔离首次启动、迁移和重启验收，不影响正式构建。

应用数据目录内：

```text
<data-dir>/
├── creatorrelay.sqlite3
├── logs/creatorrelay.log
├── article-history/
└── platform-sessions/
```

启动顺序：

1. 创建应用数据目录。
2. 打开 SQLite 连接。
3. 执行版本化迁移。
4. 写入 `startup.ready` 或 `startup.migration` 日志。
5. 注册 Tauri Commands 并显示主窗口。

## 2. SQLite 迁移

当前 schema 版本由 `schema_migrations.version` 记录，目标版本为 v2：

- v1：`app_settings`、`documents`、`revisions`。
- v2：M3 平台账号、平台会话、作品关联和脱敏审计表。

每个版本在一个 SQLite 事务内执行：

1. 执行该版本 DDL。
2. 写入 `schema_migrations` 版本记录。
3. 提交事务。

任何语句失败都会回滚整个版本，保留迁移前版本和数据。应用若无法完成迁移，会写入脱敏启动日志并拒绝继续启动，不留下半迁移数据库。

验证覆盖：

- 全新数据库从 v0 → v2。
- v1 fixture 升级到 v2 后设置、文档和 revision 数据完整。
- 重复迁移幂等。
- 人为制造 v2 冲突表时回滚，版本仍为 v1，原有数据保留。

## 3. IPC 边界

- 前端只通过 `@tauri-apps/api/core` 的 `invoke` 访问 Rust Commands。
- Rust Core 独占 SQLite 连接、工作区文件解析、Git 历史、系统凭据和平台会话路径。
- 文件路径必须是工作区内相对 Markdown 路径；绝对路径、`..` 和符号链接逃逸会被拒绝。
- 对代表性读写命令记录阶段与结果，不记录文档正文或敏感请求体。
- 非法路径和数据库诊断错误返回可识别错误，前端顶部错误栏展示，应用继续运行。

## 4. 日志与脱敏

日志为 JSON Lines，文件权限 `0600`，目录权限 `0700`。记录字段：

```json
{"timestamp":"...","level":"info","stage":"startup.ready","message":"..."}
```

固定日志阶段包括：

- `startup.ready` / `startup.migration`
- `ipc.workspace.set`
- `ipc.document.read`
- `ipc.document.create`
- `ipc.document.save`
- `diagnostics.ipc_payload`
- `diagnostics.database`

脱敏规则覆盖 JSON 字段、HTTP Header、键值查询参数和 Bearer Token，包括 password、cookie、token、secret、authorization、API Key 和 SESSDATA。日志保留错误阶段与上下文，不保留凭据值。

## 5. 运行时诊断

系统设置页提供 M1 运行时诊断：

- 应用版本、运行时长。
- 当前/目标 schema 版本、`PRAGMA integrity_check`。
- 应用数据目录、数据库路径、脱敏日志路径和工作区路径。
- 三个安全探针：
  - 非法路径 IPC：调用 `set_workspace_path` 的不存在路径，验证稳定错误和日志。
  - 日志脱敏：使用固定模拟 password/cookie/token 触发失败请求，验证日志只含 `[REDACTED]`。
  - 数据库错误：查询不存在的诊断表，验证错误反馈、日志阶段和后续可用性。

诊断探针不会读取、写入或发送真实凭据。

## 6. 模块开放状态

M1 验证当时：

| 模块 | 状态 | 说明 |
| --- | --- | --- |
| 运营总览 | 规划中 | M6，占位页，不展示演示指标 |
| 写作空间 | 开发中 | 使用真实本地工作区，尚未完成 M2/M5 验收 |
| 发布中心 | 规划中 | M6，占位页 |
| 内容中心 | 规划中 | M6，占位页 |
| 数据分析 | 规划中 | M6，占位页 |
| AI 助手 | 规划中 | M7，占位页 |
| 平台管理 | 开发中 | 使用真实 Rust 账号服务，尚未完成 M3/M4 验收 |
| 系统设置 | 可用 | M1 主题、工作区、索引重建和运行时诊断 |

导航栏直接显示“可用 / 开发中 / 规划中”，未开放模块不会渲染演示业务数据。

## 7. 构建与检查

```bash
pnpm install
pnpm typecheck
pnpm lint
pnpm test
pnpm build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cd ..
pnpm tauri build --debug --bundles app --no-sign
```

首次启动隔离验收：

```bash
CREATORRELAY_DATA_DIR=/tmp/creatorrelay-m1-acceptance \
  src-tauri/target/debug/bundle/macos/creatorrelay.app/Contents/MacOS/creatorrelay
```
