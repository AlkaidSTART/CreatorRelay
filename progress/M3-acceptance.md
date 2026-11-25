# M3 验收报告：平台账号体系

- **里程碑：** M3 平台账号体系
- **验证日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（25G83），arm64
- **代码基线：** `93a978c6013974c17d519d5c9a127ec84b1b4f2a`
- **受检版本：** `5e7a5192fba8775b284641a85672e6212e03ec39`
- **M1 前置：** 已通过，见 [M1 验收报告](./M1-acceptance.md)
- **总体结果：** **accepted**
- **验证记录：** [M3-verification-2026-11-25.md](./verification/M3-verification-2026-11-25.md)

## 变更文件

M3 以 `93a978c` 为基线。Shared 文件中同时存在 M1 基础设施改动时，本表只说明 M3 所属部分；M1 专属日志、诊断、占位路由和验收文档不计入 M3 实现所有权。

| 类型 | 文件 | 实现内容 |
| --- | --- | --- |
| 新增 | `src-tauri/src/platforms.rs` | 平台目录、账号/会话状态机、Keychain、身份校验、profile 隔离、删除和审计 |
| 新增 | `src/types/platform.ts` | 账号、会话、认证挑战、删除和诊断相关前端类型 |
| 新增 | `src/stores/platformStore.ts` | 前端平台账号状态、Tauri IPC 调用和浏览器预览降级 |
| 新增 | `src/stores/platformStore.test.ts` | 双账号隔离、状态真实性和删除范围前端测试 |
| 新增 | `docs/platform-accounts.md` | 账号/凭据/会话生命周期、平台能力清单和删除语义 |
| 新增 | `progress/M3-platform-accounts.md` | M3 范围、AC、阶段总结和当前状态 |
| 新增 | `progress/verification/M3-verification-2026-11-25.md` | M3 命令、AC 快照、手工平台证据和风险 |
| 新增 | `progress/M3-acceptance.md` | 本验收报告 |
| 修改 | `src-tauri/Cargo.toml` / `src-tauri/Cargo.lock` | 引入系统 Keyring、HTTP 身份校验依赖及锁定 |
| 修改 | `src-tauri/src/lib.rs` | M3 账号、登录、状态检测、依赖门、删除和审计 Tauri Commands |
| 修改 | `src-tauri/src/database.rs` | 在统一迁移器中登记 M3 schema v2 表、索引和外键约束 |
| 修改 | `src/pages/PlatformsPage.tsx` | 平台目录、多账号新增/编辑/禁用/默认、授权、检测和删除预览 UI |
| 修改 | `src/pages/ContentPage.tsx` / `DashboardPage.tsx` / `StudioPage.tsx` | 从真实平台账号 Store 读取账号；仅 ACTIVE 且启用的账号可进入发布准备 |
| 修改 | `src/layouts/AppLayout.tsx` | 启动时加载真实平台账号；平台入口保留开发中状态 |
| 修改 | `src/stores/contentStore.ts` | 移除虚假的账号重连和账号状态演示逻辑，保留发布任务职责 |
| 修改 | `src/stores/appStore.ts` / `appStore.test.ts` / `stores/index.ts` | 注册平台 Store 和测试入口 |
| 修改 | `src/data/demoData.ts` / `src/components/Icons.tsx` | 平台账号类型复用、图标和浏览器预览数据边界 |
| 修改 | `.gitignore` / `README.md` | 允许平台账号实现与验证文档纳入版本控制并增加说明链接 |

无删除文件。

## 缺陷修复

| 缺陷 | 症状 / 根因 | 修复 | 回归证据 |
| --- | --- | --- | --- |
| 重新连接直接伪造 ACTIVE | 原前端 demo action 不调用平台，直接把状态改为 ACTIVE | 登录改为 `AUTHENTICATING → 平台身份校验 → ACTIVE`；失败进入 ERROR/NEEDS_REAUTH，取消恢复未授权 | `m3_ac_02_identity_verification_controls_active_state`、`m3_ac_08_profile_presence_does_not_imply_active` |
| 凭据没有真实安全边界 | 演示账号只有前端状态，没有凭据引用、Keychain 或真实会话存储 | 后端生成独立 credential reference，敏感值写系统 Keychain，SQLite 只保存引用和非敏感元数据 | `m3_ac_04_sqlite_and_audit_logs_do_not_contain_secret`、Keychain smoke test |
| 多账号可能共享会话 | 账号仅是 UI 数据，没有独立浏览器会话容器 | 每账号生成独立 `profile-<uuid>`，权限 `0700`，credential reference 独立 | `m3_ac_05_accounts_have_isolated_profiles_and_credentials`；桌面双账号 profile 不同 |
| 删除账号可能连带历史作品 | 缺少明确的凭据/会话/历史分离模型 | `platform_works.account_id` 使用 `ON DELETE SET NULL`，删除前生成三项确认预览 | `m3_ac_07_delete_cleans_session_but_retains_history` |
| 浏览器预览 ID 冲突 | 两个账号在相同毫秒创建时使用 `Date.now()` 产生重复 ID | 改为 `crypto.randomUUID()` | `platformStore.test.ts` 双账号隔离测试 |
| 禁用/重新启用状态不一致 | 禁用期间保留凭据，但重新启用后不能直接视为有效 | 禁用进入 DISABLED；重新启用进入 NEEDS_REAUTH，必须重新检测 | 账号状态机测试和桌面检测流程 |

## 验收标准

| ID | 验收场景 | 结果 | 证据 |
| --- | --- | --- | --- |
| M3-AC-01 | 多账号管理 | pass | 同平台两个账号创建成功，别名、默认标记和 profile 独立；编辑主账号不影响副账号 |
| M3-AC-02 | 登录与身份校验 | pass | 成功 fixture 返回真实结构 UID/昵称后进入 ACTIVE；失败进入 ERROR 且 UID 为空；不会创建虚假 ACTIVE |
| M3-AC-03 | 登录状态检测 | pass | 过期会话进入 NEEDS_REAUTH；`require_active_account` 拒绝依赖任务；桌面无凭据检测显示需重新授权 |
| M3-AC-04 | 凭据安全 | pass | SQLite 文件字节和审计日志无测试 Cookie；真实 macOS Keychain round-trip 通过；运行时验收目录扫描无 secret |
| M3-AC-05 | 会话隔离 | pass | 两个账号 profile_id/credential_ref 均不同；A 写入后 B 读取为空；目录权限 `0700`、元数据 `0600` |
| M3-AC-06 | 浏览器回退 | pass | Bilibili 授权弹窗展示系统浏览器登录、Cookie 获取、后端校验和 Keychain 说明；内置 WebView 明确未开放 |
| M3-AC-07 | 账号删除 | pass | 删除预览展示 Keychain、会话容器和历史作品保留范围；自动化验证清理与历史保留 |
| M3-AC-08 | 状态真实 | pass | profile 存在但平台复检失败时进入 ERROR；未授权账号检测后只能进入 NEEDS_REAUTH，不显示 ACTIVE |

## 测试、构建与手工检查

| 检查 | 命令 / 场景 | 结果 |
| --- | --- | --- |
| M3 领域测试 | `cargo test m3_ac_` | pass，8 项 |
| 平台模块测试 | `cargo test platforms::tests` | pass，12 项 |
| Keychain | `CREATORRELAY_TEST_KEYCHAIN=1 cargo test system_keyring_smoke_is_explicitly_opt_in` | pass，真实 macOS Keychain 写入/读取/删除 |
| 前端 | `pnpm typecheck && pnpm lint && pnpm test && pnpm build` | pass，30 项测试；仅有 Vite chunk 体积警告 |
| Rust | `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test` | pass，38 项测试 |
| Tauri 构建 | `pnpm tauri build --debug --bundles app --no-sign` | pass，`src-tauri/target/debug/bundle/macos/creatorrelay.app` |
| M3 格式 | `pnpm exec prettier --check <M3 修改文件>` | pass |
| 全仓库格式 | `pnpm format:check` | fail；仅 3 个 M3 范围外既有文件，见风险 |
| 线上接口形状 | Bilibili `nav` 无 Cookie 请求 | `code=-101 / isLogin=false`，与 NEEDS_REAUTH 分支一致 |
| 双账号 UI | 隔离目录添加主/副账号，编辑主账号别名和默认状态 | pass，副账号不变 |
| 检测与授权 | 检测主账号并打开授权弹窗、取消 | pass，显示 NEEDS_REAUTH 和系统浏览器/Cookie/Keychain 步骤 |
| 删除预览 | 打开主账号删除确认，检查三项范围后取消 | pass，未执行删除 |
| 数据库 | `PRAGMA integrity_check` / `foreign_key_check` | `ok` / 无输出 |
| 隔离目录 | 两个 profile 目录和 metadata 权限 | `drwx------` / `-rw-------` |

## 剩余风险、排除项与后续事项

- 未使用真实用户 Bilibili Cookie 完成线上成功登录。成功分支由本地 HTTP fixture 验证，失败/过期分支由线上接口验证；这是当前最主要的残余风险。
- M3 自动身份校验只覆盖 Bilibili。其他平台按目录明确标记人工流程或未开放，不宣称自动登录。
- 内置 WebView 账号会话尚未启用；验证路径为系统浏览器人工登录 + Cookie 校验。
- 仅在 macOS 实测 Keychain；Windows Credential Manager 和 Linux Secret Service 尚未实机验证。
- 全仓库 `pnpm format:check` 仍因 3 个 M3 范围外且未修改的文件失败：`StudioWorkspaceEmpty.tsx`、`AssistantPage.tsx`、`StudioPage.css`。
- 真实平台登录成功、跨平台 Keychain 和 WebView 会话应在后续 Adapter/平台兼容性阶段补验。

## 结论

M3 的 8 项验收标准全部通过，多账号管理、身份校验、过期检测、系统凭据、会话隔离、浏览器回退、删除语义和状态真实性具备可复现证据。**M3 验收通过，可作为 M4 的账号、凭据与会话前置条件。**
