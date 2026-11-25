# M3 验证记录 — 2026-11-25

> 本文件只记录验证结果，不是验收报告，也不能单独作为 M3 已验收的依据。

- **阶段：** M3 平台账号体系
- **记录日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（25G83），arm64
- **代码基线：** `93a978c6013974c17d519d5c9a127ec84b1b4f2a`
- **受检实现：** `5e7a5192fba8775b284641a85672e6212e03ec39`（M3 实现主体为 `9d6710b`，含后续 M1 迁移整合、测试与文档）
- **变更范围：** M3 平台目录、账号 CRUD、认证状态机、系统凭据、隔离会话、状态检测、删除语义、前端平台管理页及相关测试/文档
- **总体状态：** `pass`（M3 全部 AC 已有可复现证据；M1 前置验收已通过）

## AC 快照

| 验收标准 | 结果 | 证据 |
| --- | --- | --- |
| M3-AC-01 多账号管理 | pass | `cargo test m3_ac_01_multiple_accounts_are_independent`；桌面手工检查添加两个 Bilibili 账号，分别显示别名/UID 状态，编辑主账号别名和默认设置后副账号保持不变 |
| M3-AC-02 登录与身份校验 | pass | `cargo test m3_ac_02_identity_verification_controls_active_state`；`real_http_verifier_uses_fixture_server`；校验成功保存 UID `mid-1001`/昵称并进入 `ACTIVE`，失败进入 `ERROR` 且 UID 为空 |
| M3-AC-03 登录状态检测 | pass | `cargo test m3_ac_03_expired_session_blocks_dependent_tasks`；会话改为过期后进入 `NEEDS_REAUTH`，`require_active_account` 拒绝依赖任务；桌面手工检测无凭据账号后进入“需重新授权” |
| M3-AC-04 凭据安全 | pass | `cargo test m3_ac_04_sqlite_and_audit_logs_do_not_contain_secret`；SQLite 文件字节和审计日志均未出现测试 Cookie；`CREATORRELAY_TEST_KEYCHAIN=1` smoke test 通过真实 macOS Keychain 写入、读取、删除 |
| M3-AC-05 会话隔离 | pass | `cargo test m3_ac_05_accounts_have_isolated_profiles_and_credentials`；两个账号 `profile_id`/`credential_ref` 不同，A 会话写入后 B 读取为空；手工检查实际目录两层均为 `drwx------`，`profile.json` 为 `-rw-------` |
| M3-AC-06 浏览器回退 | pass | `cargo test m3_ac_06_browser_fallback_exposes_manual_steps`；桌面手工打开 Bilibili 授权弹窗，显示系统浏览器登录页、Cookie 获取和校验三步，内置 WebView 明确标记未开放 |
| M3-AC-07 账号删除 | pass | `cargo test m3_ac_07_delete_cleans_session_but_retains_history`；手工打开删除预览，逐项显示 Keychain 凭据、会话目录和历史作品保留范围；自动化验证删除后凭据/会话容器清理、账号 B 不受影响、作品记录保留且 `account_id` 置空 |
| M3-AC-08 状态真实 | pass | `cargo test m3_ac_08_profile_presence_does_not_imply_active`；profile 目录存在但平台复检失败后账号进入 `ERROR`，不回到 `ACTIVE`；手工检测未授权账号只进入 `NEEDS_REAUTH`，未伪造登录 |

## 自动化检查

| 命令 | 结果 |
| --- | --- |
| `pnpm typecheck` | pass |
| `pnpm lint` | pass |
| `pnpm test` | pass，6 个测试文件、30 个测试 |
| `pnpm build` | pass；仅有 Vite 单 chunk 大于 500 kB 的体积警告 |
| `pnpm exec prettier --check`（M3 修改文件清单） | pass |
| `pnpm format:check`（全仓库） | fail；仅报告未由 M3 修改的 `src/components/StudioWorkspaceEmpty.tsx`、`src/pages/AssistantPage.tsx`、`src/pages/StudioPage.css`，属于既有格式债务 |
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test` | pass，38 个测试 |
| `CREATORRELAY_TEST_KEYCHAIN=1 cargo test system_keyring_smoke_is_explicitly_opt_in -- --nocapture` | pass，真实 macOS Keychain round-trip |
| `pnpm tauri build --debug --bundles app --no-sign` | pass，产物 `src-tauri/target/debug/bundle/macos/creatorrelay.app` |

## 手工与平台证据

### 桌面 UI

在 `src-tauri/target/debug/bundle/macos/creatorrelay.app` 中执行：

1. 打开“平台管理”，确认平台目录展示 10 个平台及各认证方式的能力边界。
2. 为 Bilibili 添加“验收主号”和“验收副号”，确认两个账号分别显示独立 `profile` 标识且初始状态均为“未登录”。
3. 编辑“验收主号”别名并设为其平台默认账号，确认“验收副号”别名和状态不变。
4. 检测“验收主号”，确认状态进入“需重新授权”且显示“未找到安全的平台会话凭据”。
5. 点击重新授权，确认系统浏览器回退、Cookie 处理说明和“内置 WebView 未开放”均可见；取消后恢复“未登录”。
6. 打开删除预览，确认三项显式确认范围和历史作品保留说明，随后取消，未执行删除。

手工验收产生的两个测试账号、审计记录和会话目录已在测试后定向清理；清理后 `platform_accounts` 中 `验收*` 账号计数为 0。

### 独立验收复核（2026-11-25）

使用隔离目录 `/tmp/creatorrelay-m3-acceptance-20261125` 重新执行：

- `cargo test m3_ac_`：8 项 M3 AC 领域测试通过。
- `cargo test platforms::tests`：12 项平台测试通过。
- 同一 Bilibili 平台添加“M3 主号”和“M3 副号”，profile 与 credential reference 均不同。
- 主账号编辑别名并设为默认，副账号不变。
- 检测主账号进入“需重新授权”，显示“未找到安全的平台会话凭据”。
- 打开授权弹窗确认系统浏览器与 Keychain 步骤；取消后恢复未登录。
- 打开删除预览，确认 Keychain、会话目录、历史作品保留三项范围；未执行删除。

复核数据库结果：

```text
platform_accounts = 2
platform_sessions = 2
distinct_profile_id = 2
distinct_credential_ref = 2
PRAGMA foreign_key_check = 无输出
PRAGMA integrity_check = ok
platform-sessions/<profile> = drwx------
profile.json = -rw-------
```

### 数据库与文件系统

```text
schema_version = 2
PRAGMA integrity_check = ok
test_accounts_remaining = 0
platform-sessions/<profile>/ = drwx------ 
profile.json = -rw-------
```

SQLite 表包括 `platform_accounts`、`platform_sessions`、`platform_works`、`account_audit_logs`，`PRAGMA foreign_key_check` 无输出。

### 线上接口形状

```bash
curl -sS --max-time 15 \
  -A 'CreatorRelay/0.1 platform-session-check' \
  -H 'Accept: application/json' \
  'https://api.bilibili.com/x/web-interface/nav'
```

未携带 Cookie 时返回：

```json
{"code":-101,"message":"账号未登录","ttl":1,"data":{"isLogin":false}}
```

该结果与后端将未登录会话判定为 `NEEDS_REAUTH` 的解析分支一致。成功身份分支通过本地 HTTP fixture 返回同结构 `mid`/`uname` 完成验证。

## 未验证项与风险

- 未使用真实用户 Bilibili Cookie 完成线上成功登录；成功分支由 HTTP fixture 验证，失败/过期分支由线上接口验证。真实账号成功登录仍属于人工剩余风险。
- 当前自动身份校验仅覆盖 Bilibili；其他平台明确为人工流程或未开放，不宣称自动连接。
- 内置 WebView 账号会话未启用，当前可验证回退路径为系统浏览器。
- 仅实测 macOS Keychain；Windows Credential Manager 和 Linux Secret Service 未在对应平台实机验证。
- 全仓库 `pnpm format:check` 仍因 3 个 M3 范围外文件失败；M3 修改文件已全部通过 Prettier。
- Vite 构建存在现有单 chunk 体积警告，未影响构建产物。

## 阶段门说明

本记录形成时 M1 尚未通过验收，因此当时未授权发布 M3 验收报告。后续状态更新：M1 已于 2026-11-25 通过全部 AC 并发布 [M1 验收报告](../M1-acceptance.md)；M3 已完成独立验收复核，正式验收结论见 [M3 验收报告](../M3-acceptance.md)。
