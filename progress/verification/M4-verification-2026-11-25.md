# M4 验证记录 — 2026-11-25

> 本文件只记录验证活动与结果，不是验收报告；M4 的验收结论见 [M4 验收报告](../M4-acceptance.md)。

- **阶段：** M4 Adapter 与插件体系
- **记录日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（arm64），`/usr/bin/sandbox-exec` 可用
- **代码基线：** `5e7a5192fba8775b284641a85672e6212e03ec39`
- **受检提交：** `5a714647d8c86eac84e2351fc7e83da185fe76ac`
- **受检工作树：** 上述提交 + 2026-11-25 验收前 M4 修订（账号能力隔离、导航可用状态、v2→v3 迁移测试）
- **总体状态：** `passed`

## 命令与结果

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| Rust 格式 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass |
| Rust Clippy | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| Rust 全量测试 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass，48 个单元测试 + 9 个 M4 运行时集成测试 |
| M4 运行时集成测试 | `cargo test --manifest-path src-tauri/Cargo.toml --test m4_runtime` | pass，9/9 |
| 数据库迁移 | `cargo test --manifest-path src-tauri/Cargo.toml database::tests` | pass，5/5；含 v2 fixture → v3，保留既有平台账号数据 |
| Manifest/能力/载荷单元测试 | `cargo test --manifest-path src-tauri/Cargo.toml plugins::` | pass，9/9 |
| 前端类型检查 | `pnpm typecheck` | pass |
| 前端 Lint | `pnpm lint` | pass |
| 前端测试 | `pnpm test` | pass，7 个测试文件、33 项测试 |
| 前端生产构建 | `pnpm build` | pass；仅有既有 chunk > 500 kB 警告 |
| Tauri Debug 打包 | `pnpm tauri build --debug --bundles app --no-sign` | pass，生成 `src-tauri/target/debug/bundle/macos/creatorrelay.app`（47.25 MiB） |
| Bundle 资源 | `find .../Contents/Resources/resources/plugins -name manifest.json` | pass，三个官方 Manifest 均进入 Bundle |
| Bundle Adapter runner | `printf ... | .../Contents/MacOS/creatorrelay --adapter-runner --profile healthy` | pass，`describe` 返回合约 v1 并正常 `shutdown` |
| JSON 文档 | `python3 -m json.tool <schema/manifests>` | pass |
| M4 前端格式 | `pnpm exec prettier --check src/components/PluginManager.tsx src/pages/PlatformsPage.tsx src/stores/index.ts src/stores/pluginStore.ts src/stores/pluginStore.test.ts src/types/plugin.ts` | pass |
| 全仓库格式 | `pnpm format:check` | fail；仍为 3 个 M4 范围外既有文件：`StudioWorkspaceEmpty.tsx`、`AssistantPage.tsx`、`StudioPage.css` |

## M4 AC 快照

| 验收标准 | 结果 | 证据 |
| --- | --- | --- |
| M4-AC-01 Manifest 校验 | pass | `m4_ac_01_accepts_valid_manifest`、`m4_ac_01_rejects_missing_unknown_and_illegal_fields_with_field_paths`；字段级 `field/code/message`；未知字段、非法类型、非法版本、非法能力均拒绝 |
| M4-AC-02 权限审查 | pass | `m4_ac_02_permissions_block_enable_and_core_proxy_never_returns_raw_access`、`m4_ac_02_04_permission_denials_are_visible_to_adapter_probe`；手工 UI 权限弹窗逐一勾选并可拒绝；Core 拒绝未授权网络/工作区/原始凭据 |
| M4-AC-03 适配器注册 | pass | `m4_ac_03_05_lifecycle_trace_registers_identity_capabilities_and_events`；官方测试 Adapter 通过公开 Manifest/JSONL 合约注册，Core 列出身份、版本、合约和逐形态能力 |
| M4-AC-04 能力可信 | pass | `m4_ac_04_runtime_probe_failure_overrides_manifest_supported_claim`；声明视频 supported 但运行时探测失败时状态为 ERROR、四种形态 effective 均为 unavailable；手工 UI 复现 |
| M4-AC-05 生命周期 | pass | `m4_ac_03_05_...` 覆盖发现→安装→权限审查→授权→启用→禁用→卸载；`plugin_events` 含 INSTALLED/PERMISSIONS_REVIEWED/ENABLED/DISABLED/UNINSTALLED |
| M4-AC-06 故障隔离 | pass | `m4_ac_06_crash_and_timeout_isolate_faulty_adapter_from_healthy_adapter`；故障 Adapter 崩溃/超时后健康 Adapter 仍可探测；手工 UI 显示错误并可重启 |
| M4-AC-07 版本兼容 | pass | `m4_ac_07_version_rollback_preserves_intersected_grants_and_rejects_contract_v2`、`m4_ac_07_rejects_unsupported_contract_and_invalid_semver`；合约 v2 不可安装/启用；回退保留授权交集，新权限需重新审查 |
| M4-AC-08 安全边界 | pass | `m4_ac_08_security_self_test_denies_database_workspace_and_credential_canaries`；macOS 沙箱拒绝 SQLite、工作区文件和凭据哨兵；主机 API 拒绝 `database.query`、`credentials.readRaw`，未授权 `workspace.read`/`network.request` |
| M4-AC-09 运行时决策 | pass | [ADR-002](../../docs/adr/ADR-002-adapter-runtime.md) 明确首期 Runtime、权限隔离、信任模型、平台差异和不支持场景，并明确独立进程不是安全沙箱 |
| M4-AC-10 分形态能力 | pass | `m4_ac_10_11_capability_matrix_gates_each_content_kind_and_payload_completeness`、`m4_ac_10_capability_status_is_scoped_to_the_probed_account`；测试账号视频 unsupported、图文 supported、图文文章 restricted、纯文章 supported；账号能力按探测账号隔离存储；视频发布被拒绝，图文/文章不受影响；手工 UI 矩阵展示 |
| M4-AC-11 载荷完整性 | pass | `m4_ac_11_video_requires_file_and_cover_before_publish`、`m4_ac_11_image_order_and_cover_are_preserved`、`m4_ac_11_article_requires_body`、`m4_ac_10_11_...`；缺视频/封面/正文在调用 Adapter/平台前拒绝 |

## 手工与平台证据

### 浏览器预览 UI 检查

在 `http://127.0.0.1:1420/#/platforms` 的“适配器与插件”页完成：

1. 点击安装 `CreatorRelay 测试适配器`，弹出权限审查窗口，显示来源、版本、Adapter 合约、Manifest 哈希、四项权限及高风险/中风险标识和逐形态声明能力。
2. 勾选 `account.credentials` 与 `platform.publish` 后保存，状态进入“可启用”；启用后状态进入“已启用”，运行探测后显示视频“不支持”、图文“可用”、图文文章“受限”、纯文章“可用”，来源标记为“探测”。
3. 探测结果同时显示 `credentials.readRaw = FORBIDDEN_RAW_CREDENTIAL`、`workspace.read = PERMISSION_DENIED`、`network.request = PERMISSION_DENIED`、`database.query = FORBIDDEN`。
4. 点击“模拟崩溃”后状态变为“运行错误”，展示错误文本；点击“重启”后恢复“已启用”。
5. 安装 `CreatorRelay 能力探测失败适配器`，授权必需权限后启用失败，状态为“运行错误”，四种形态均显示“不可用”，与 Manifest 声明的视频“可用”相反。
6. 四形态载荷自检按钮可运行；浏览器预览仅返回演示项，完整九项载荷自检由 Rust 单元/集成测试覆盖。

### macOS 沙箱

`m4_ac_08_security_self_test_denies_database_workspace_and_credential_canaries` 在真实 macOS `sandbox-exec` 下启动官方 Adapter runner，尝试读取：

- Core SQLite 文件：拒绝；
- 工作区内测试哨兵：拒绝；
- 插件数据目录外的凭据哨兵：拒绝。

同一测试还断言 `database.query` 主机 API 返回 `FORBIDDEN`。

## 未覆盖项与剩余风险

- Windows/Linux 未进行实机强隔离验证；当前 Runtime 在这些平台只有独立进程隔离，因此仅官方 Adapter 可安装/启用，非官方代码保持禁用。
- M4 网络代理只验证授权门与直接网络拒绝，真实发布网络请求不在 M4 范围（ADR-002 记录为 M6 事项）。
- macOS `sandbox-exec` 已被 Apple 标记为 deprecated；ADR-002 明确后续需复核替代方案。
- 未记录 Tauri WebView 的自动化视觉快照；UI 交互在 Vite 浏览器预览中完成，桌面后端通过真实进程集成测试和 Tauri Bundle 构建验证。
- 全仓库 `pnpm format:check` 仍因 3 个 M4 范围外既有文件失败；M4 修改的 6 个前端文件全部通过 Prettier。
- 未使用真实第三方平台账号完成 M4 发布；M4 使用官方测试 Adapter 验证合约、权限和能力状态，真实平台发布由 M6 按平台单独验收。

## 结论

M4 的 11 项验收标准全部通过。Manifest 校验、权限审查、官方 Adapter 注册、真实能力探测、生命周期、崩溃/超时隔离、版本回退、macOS 安全边界、ADR、分形态能力和载荷完整性均有自动化或手工证据。**M4 验证通过，可进入验收报告与 M5。**
