# M4 验收报告：Adapter 与插件体系

- **里程碑：** M4 Adapter 与插件体系
- **验证日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（arm64），`/usr/bin/sandbox-exec` 可用
- **代码基线：** `5e7a5192fba8775b284641a85672e6212e03ec39`
- **受检提交：** `5a714647d8c86eac84e2351fc7e83da185fe76ac`
- **受检工作树：** 上述提交 + 2026-11-25 验收前 M4 修订（账号能力隔离、导航可用状态、v2→v3 迁移测试）
- **前置条件：** M1、M3 已通过验收；M4 复用 M3 的账号、Keychain 和隔离会话边界
- **总体结果：** **accepted**
- **验证记录：** [M4-verification-2026-11-25.md](./verification/M4-verification-2026-11-25.md)

## 变更文件

M4 以 `5e7a519` 为基线。基线工作区同时包含 M3 验收文档的未提交改动；本表只列出 M4 所属实现与 M4 验收材料，不把 `progress/M3-*` 和 `progress/verification/M3-*` 计入 M4。

| 类型 | 文件 | 实现内容 |
| --- | --- | --- |
| 新增 | `src-tauri/src/plugins/manifest.rs` | Manifest 数据模型、字段级校验、权限/内容形态/合约版本规则、错误码 |
| 新增 | `src-tauri/src/plugins/publish.rs` | 四形态 `PublishInput`、素材顺序/封面/正文完整性校验、九项自检用例 |
| 新增 | `src-tauri/src/plugins/runtime.rs` | 独立进程 supervision、JSONL 请求/响应、hostCall 权限代理、超时/崩溃处理、macOS 沙箱 |
| 新增 | `src-tauri/src/plugins/registry.rs` | 插件注册表、发现/安装/权限审查/启用/禁用/回退/卸载、能力探测持久化、审计事件 |
| 新增 | `src-tauri/src/plugins/runner.rs` | 官方测试 Adapter runner：describe/probe/validatePublish/publish、故障注入、安全哨兵 |
| 新增 | `src-tauri/src/plugins/mod.rs` | 插件模块公开导出 |
| 新增 | `src-tauri/resources/plugins/test-adapter/manifest.json` | 正常测试 Adapter：图文可用、视频不可用 |
| 新增 | `src-tauri/resources/plugins/fault-adapter/manifest.json` | 崩溃/超时故障隔离测试 Adapter |
| 新增 | `src-tauri/resources/plugins/probe-failure-adapter/manifest.json` | Manifest 声明视频支持但运行时探测失败的 Adapter |
| 新增 | `src-tauri/tests/m4_runtime.rs` | 9 项真实进程/沙箱集成测试，覆盖 AC-02～AC-08、AC-10、AC-11 |
| 新增 | `src/types/plugin.ts` | Adapter、权限、能力、探测、事件和发布载荷前端类型 |
| 新增 | `src/stores/pluginStore.ts` | 前端插件状态、Tauri IPC 调用和浏览器预览降级 |
| 新增 | `src/stores/pluginStore.test.ts` | 生命周期、权限审查、逐形态能力和探测失败的 UI 状态测试 |
| 新增 | `src/components/PluginManager.tsx` | 适配器发现、安装、权限弹窗、能力矩阵、故障注入、重启、安全自检和载荷自检 UI |
| 新增 | `docs/adr/ADR-002-adapter-runtime.md` | 首期 Runtime、隔离、信任、平台差异和不支持场景 ADR |
| 新增 | `docs/adapters.md` | Adapter 合约、权限矩阵、生命周期、协议、能力和载荷实现说明 |
| 新增 | `docs/schemas/adapter-manifest.schema.json` | Manifest JSON Schema |
| 新增 | `progress/M4-acceptance.md` | 本验收报告 |
| 新增 | `progress/verification/M4-verification-2026-11-25.md` | M4 命令、AC 快照、手工证据和风险 |
| 修改 | `src-tauri/src/database.rs` | schema v3：plugins、plugin_grants、plugin_capability_checks、plugin_events 和索引 |
| 修改 | `src-tauri/src/lib.rs` | AppState 接入 PluginRuntime；新增 Manifest/目录/生命周期/探测/故障/卸载/安全自检/publish 校验 Tauri commands |
| 修改 | `src-tauri/src/main.rs` | 识别 `--adapter-runner` 并进入无 GUI 的 Adapter runner |
| 修改 | `src-tauri/tauri.conf.json` | 将官方 Adapter Manifest 作为应用资源分发 |
| 修改 | `src/pages/PlatformsPage.tsx` | 增加“平台账号 / 适配器与插件”视图切换并挂载 PluginManager |
| 修改 | `src/routes/navigation.ts` / `navigation.test.ts` | 平台管理在 M3/M4 验收后标记为“可用”，并补充导航状态回归断言 |
| 修改 | `src/stores/index.ts` / `README.md` | 导出插件 Store，补充 M4 文档入口 |
| 修改 | `progress/M4-adapter-plugin-runtime.md` | 阶段任务总结更新为实现/验证/验收完成 |
| 修改 | `progress/README.md` | 标记 M4 Runtime/隔离决策已由 ADR-002 冻结 |

无删除文件。

## 缺陷修复

| 缺陷 | 症状 / 根因 | 修复 | 回归证据 |
| --- | --- | --- | --- |
| 沙箱 profile 解析失败 | Adapter 启动即退出，`sandbox-exec: unexpected sbpl-filter argument`；file-write 规则把已完整的 `(subpath ...)` 又嵌套了一层 | 修正沙箱模板，并补上 root metadata/file-read 规则 | `m4_ac_03_05_lifecycle_trace_registers_identity_capabilities_and_events`、`m4_ac_08_...` |
| 沙箱启动缺少 runner 路径 | `sandbox-exec` 把 `--adapter-runner` 当作可执行文件名，报 `execvp ... failed` | CoreTestRunner 在沙箱命令中显式加入当前应用可执行路径 | `m4_ac_03_05...` |
| hostCall 探测死锁超时 | Adapter 在 probe 中发送 hostCall 后等待 stdin，但外层循环一直持有 stdin 锁，导致 3 秒超时 | runner 改为按行临时加锁，hostCall 可以读取 Core 的 hostResult | `m4_ac_02_04_permission_denials_are_visible_to_adapter_probe` |
| 探测失败仍显示声明能力 | Manifest 声明视频 supported，运行 probe 失败后 UI/Store 仍可能显示 supported | 失败时持久化四种形态为 `unavailable`，有效能力覆盖声明值 | `m4_ac_04_runtime_probe_failure_overrides_manifest_supported_claim` |
| 合约检查使用了错误来源 | enable 只检查 Manifest 内的合约号，未检查注册表中的受检合约版本 | enable 同时校验注册表记录和 Manifest 合约版本，不兼容版本先被拒绝 | `m4_ac_07_version_rollback_preserves_intersected_grants_and_rejects_contract_v2` |
| 启用失败后 UI 未刷新 | Tauri `enable_plugin` 返回 Err 时前端直接抛出，未重新读取 ERROR/不可用状态 | `pluginStore.enable/restart` 在失败后强制刷新注册表再抛出 | `src/stores/pluginStore.test.ts` 的 `never displays Manifest-supported video as available...` |
| 能力状态跨账号混用 | 同一插件多次按不同账号探测时，能力查询只按插件 ID 读取，可能把另一个账号的状态用于当前发布预检 | 能力查询按 `account_ref` 过滤；UI 优先显示当前探测账号结果 | `m4_ac_10_capability_status_is_scoped_to_the_probed_account` |
| 回退后权限处理不明确 | 升级/回退可能沿用已移除权限或自动恢复旧权限 | 版本变化只保留新旧 Manifest 交集权限，新权限置为未决定，已移除权限删除 | `m4_ac_07...` |

## 验收标准

| ID | 验收场景 | 结果 | 证据 |
| --- | --- | --- | --- |
| M4-AC-01 | Manifest 校验 | pass | 合法 Manifest 被解析；缺字段、错误类型、未知字段、非法 SemVer、非法能力、合约 v2 均被拒绝并带字段路径。测试：`m4_ac_01_*`、`m4_ac_07_rejects_unsupported_contract_and_invalid_semver` |
| M4-AC-02 | 权限审查 | pass | 首次启用必须完成全部权限决定；权限弹窗展示来源、版本、权限、能力和风险；未授权 `account.credentials`/`network`/`workspace.read` 的主机调用返回 `PERMISSION_DENIED` 或 `FORBIDDEN_RAW_CREDENTIAL`。测试：`m4_ac_02_*`；手工 UI 检查 |
| M4-AC-03 | 适配器注册 | pass | 官方测试 Adapter 通过公开 Manifest 和 JSONL 合约注册；Core 列出 id、名称、版本、合约和逐形态能力；公共账号模型未加入平台特例。测试：`m4_ac_03_05_*` |
| M4-AC-04 | 能力可信 | pass | 测试 Adapter 声明视频 supported 但 probe 返回失败，状态为 ERROR，四种 effective 状态为 unavailable；UI 显示“运行错误/不可用”。测试：`m4_ac_04_*`；手工 UI 检查 |
| M4-AC-05 | 生命周期 | pass | 发现→安装→权限审查→授权→启用、禁用、卸载全部完成；`plugin_events` 记录 INSTALLED/PERMISSIONS_REVIEWED/ENABLED/DISABLED/UNINSTALLED。测试：`m4_ac_03_05_*` |
| M4-AC-06 | 故障隔离 | pass | 故障 Adapter 崩溃/超时后健康 Adapter 仍可探测；错误可见且可重启。测试：`m4_ac_06_*`；手工 UI 崩溃→重启检查 |
| M4-AC-07 | 版本兼容 | pass | 合约 v2 无法安装或启用；升级后只保留新旧权限交集；回退到 v1 后合约和授权状态符合迁移策略，已移除权限不自动恢复。测试：`m4_ac_07_*` |
| M4-AC-08 | 安全边界 | pass | macOS 沙箱拒绝 SQLite、工作区和凭据哨兵；`database.query` 始终 FORBIDDEN；未授权工作区/网络被主机 API 拒绝；原始凭据只返回不透明代理。测试：`m4_ac_08_*`、`m4_ac_02_04_*` |
| M4-AC-09 | 运行时决策 | pass | [ADR-002](../docs/adr/ADR-002-adapter-runtime.md) 明确首期 Runtime、权限隔离、信任模型、平台差异和不支持场景，并明确独立进程不是安全沙箱；实现与 ADR 一致 |
| M4-AC-10 | 分形态能力 | pass | 测试账号视频 unsupported、图文 supported、图文文章 restricted、纯文章 supported；能力结果按探测账号隔离，视频发布被 Core 拒绝，图文与纯文章预检通过；UI 显示逐形态状态。测试：`m4_ac_10_11_*`、`m4_ac_10_capability_status_is_scoped_to_the_probed_account`；手工 UI 检查 |
| M4-AC-11 | 载荷完整性 | pass | 视频必须有视频素材和封面；图文必须保留图片顺序和封面；图文文章必须有正文；纯文章必须有正文；缺必需载荷在 Adapter/平台调用前拒绝。测试：`m4_ac_11_*`、`m4_ac_10_11_*`；九项 payload self-check |

## 测试、构建与手工检查

| 检查 | 命令 / 场景 | 结果 |
| --- | --- | --- |
| Rust 格式/Clippy | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`；`cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| Rust 全量 | `cargo test --manifest-path src-tauri/Cargo.toml` | pass，48 单元 + 9 M4 集成 = 57 项 |
| 数据库迁移 | `cargo test --manifest-path src-tauri/Cargo.toml database::tests` | pass，5/5；v2 fixture 升级到 v3 后既有平台账号数据保留 |
| M4 单元 | `cargo test --manifest-path src-tauri/Cargo.toml plugins::` | pass，9/9 |
| M4 集成 | `cargo test --manifest-path src-tauri/Cargo.toml --test m4_runtime` | pass，9/9；包含真实子进程、超时、崩溃、macOS 沙箱和账号能力隔离 |
| 前端检查 | `pnpm typecheck && pnpm lint && pnpm test && pnpm build` | pass；33 项前端测试；仅有既有 chunk 体积警告 |
| M4 格式 | `pnpm exec prettier --check <M4 六个前端文件>` | pass |
| Tauri 构建 | `pnpm tauri build --debug --bundles app --no-sign` | pass，`src-tauri/target/debug/bundle/macos/creatorrelay.app` |
| Bundle 资源 | 检查 `Contents/Resources/resources/plugins/**/manifest.json` | pass，三个 Manifest 均在 Bundle 中 |
| Bundle runner | `creatorrelay --adapter-runner --profile healthy` | pass，describe 合约 v1、shutdown 正常 |
| 手工 UI | Vite 预览 `#/platforms` → “适配器与插件” | pass：安装、权限审查、授权、启用、能力探测、崩溃/重启、探测失败不可用状态均可见 |
| macOS 沙箱 | `m4_ac_08_...` | pass：SQLite/工作区/凭据哨兵全部拒绝 |
| 全仓库格式 | `pnpm format:check` | fail；3 个 M4 范围外既有文件（`StudioWorkspaceEmpty.tsx`、`AssistantPage.tsx`、`StudioPage.css`） |

## 剩余风险、排除项与后续事项

- macOS 的 `sandbox-exec` 是首期强隔离机制，但 Apple 已标记 deprecated；后续需在 ADR 中复核替代方案。
- Windows/Linux 当前只有独立进程隔离，没有强安全沙箱；非官方 Adapter 被禁止安装/启用。跨平台强隔离和实机验证应在后续里程碑完成。
- M4 网络代理只完成授权门和拒绝路径，真实发布网络调用在 M6 实现；M4 不宣称真实平台发布能力。
- 工作区资产服务当前返回不透明 `assetId` 并做路径授权；真实素材传输/临时 URL 在 M5/M6 补齐。
- 第三方插件市场、签名分发、自动更新和社区信任体系不在 M4 范围（M8）。
- 未使用真实第三方平台账号完成 M4 发布；M4 使用官方测试 Adapter 验证合约、权限、能力和故障隔离。
- 全仓库 `pnpm format:check` 仍因 3 个 M4 范围外既有文件失败；M4 修改文件全部通过格式检查。

## 结论

M4 的 11 项验收标准全部通过。实现具备可复现的 Manifest 校验、逐项权限审查、官方 Adapter 注册、真实能力探测、生命周期审计、故障隔离、版本回退、macOS 安全边界、Runtime ADR、分形态能力矩阵和发布载荷完整性验证。**M4 验收通过，可作为 M5 内容工作室的 Adapter 合约与权限基础。**
