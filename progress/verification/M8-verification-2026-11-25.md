# M8 验证记录 — 2026-11-25

> 本文件只记录验证活动与结果，不是验收报告，也不能单独作为 M8 已验收的依据。

- **阶段：** M8 生态扩展
- **记录日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（arm64）；Tauri 2；Chrome Playwright 浏览器预览
- **M8 实施起点：** `e50c9e1`（M6/M7/M8 并发实施前的功能基线；验收只计入 M8 相关文件与行为）
- **验证 HEAD：** `e871d11`（M8 实现、MCP 模块化后的最终验证修订）
- **总体状态：** M8-AC-01 至 M8-AC-09 自动化证据 `pass`；阶段验收 `blocked`，因为前置 M6 仍未 accepted（M6 受 M2 前置门阻塞）

## 实现范围

- 新增平台准入登记、准入 Schema、官方来源/验证日期/账号类型/未支持事项和四类内容形态结果。
- 新增平台及账号级能力矩阵；账号非 ACTIVE、禁用、平台未准入时自动降级并返回原因。
- 新增发布准入门禁：未通过平台准入时强制人工发布，Adapter 自报 `supported` 不能覆盖。
- 扩展统一作品模型：保留平台原始内容类型、原始字段 JSON、指标定义/单位/来源。
- 新增 SQLite v6 → v7 迁移，并通过四类内容包、Markdown、账号、任务和历史指标恢复测试。
- 新增插件升级新权限重新审查与拒绝后 hostCall 强制拒绝验证。
- 新增第三方插件市场策略、六项门禁和 ADR-003；市场保持关闭。
- 新增 MCP `platform_capabilities_get` 只读工具。
- 新增“平台管理 → 生态治理”UI，展示准入、来源、账号降级、兼容性和市场门禁。

## 命令与结果

| 检查             | 命令                                                                                                                                                                                                                                                  | 结果                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Rust 格式        | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`                                                                                                                                                                                     | pass                                                                                         |
| Rust Clippy      | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`                                                                                                                                                                      | pass                                                                                         |
| Rust 全量测试    | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`                                                                                                                                                                                       | pass：73 个 lib 测试 + M4 9 + M6 5 + M7 8 + M8 6                                             |
| M8 集成测试      | `cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem`                                                                                                                                                                                 | pass，6/6                                                                                    |
| 数据库迁移       | `cargo test --manifest-path src-tauri/Cargo.toml --lib database::tests`                                                                                                                                                                               | pass；含 v6 四类内容/账号/任务/指标 → v7，以及失败迁移回滚                                   |
| Adapter 回归     | `cargo test --manifest-path src-tauri/Cargo.toml --test m4_runtime`                                                                                                                                                                                   | pass，9/9；含崩溃/超时隔离和版本回退                                                         |
| M6/M7 回归       | `cargo test --manifest-path src-tauri/Cargo.toml --test m6_operations && cargo test --manifest-path src-tauri/Cargo.toml --test m7_mcp`                                                                                                               | pass，5/5 + 8/8                                                                              |
| 前端测试         | `pnpm test`                                                                                                                                                                                                                                           | pass，11 个测试文件、44 项测试                                                               |
| 前端类型检查     | `pnpm typecheck`                                                                                                                                                                                                                                      | pass                                                                                         |
| 前端 Lint        | `pnpm lint`                                                                                                                                                                                                                                           | pass                                                                                         |
| 前端构建         | `pnpm build`                                                                                                                                                                                                                                          | pass；仅既有 chunk > 500 kB 警告                                                             |
| M8 前端格式      | `pnpm exec prettier --check src/components/EcosystemPanel.tsx src/stores/ecosystemStore.ts src/types/ecosystem.ts src/pages/PlatformsPage.tsx src/stores/index.ts src/types/operations.ts src/stores/operationsStore.ts src/utils/operations.test.ts` | pass                                                                                         |
| 全仓库格式       | `pnpm format:check`                                                                                                                                                                                                                                   | fail；M8 范围外既有文件 `StudioWorkspaceEmpty.tsx`、`StudioPage.css` 未格式化                |
| Tauri Debug 打包 | `pnpm tauri build --debug --bundles app --no-sign`                                                                                                                                                                                                    | pass；`src-tauri/target/debug/bundle/macos/creatorrelay.app`（52.50 MiB）                    |
| JSON 文件        | `python3 -m json.tool <admissions/marketplace/schema files>`                                                                                                                                                                                          | pass                                                                                         |
| 官方来源可达性   | 对准入登记中的 13 个官方入口执行 `curl -L -sS --max-time 15 -o /dev/null -w '%{http_code}'`                                                                                                                                                           | 12 个返回 200；`https://open.zhihu.com/` 返回 422，登记中明确记录为入口异常且不声明 API 可用 |

### 官方来源检查输出

```text
200 https://open.bilibili.com/
200 https://member.bilibili.com/platform/home
200 https://developers.google.com/youtube/v3/docs
200 https://developers.weixin.qq.com/doc/
200 https://mp.weixin.qq.com/
200 https://developer.open-douyin.com/
200 https://open.kuaishou.com/
200 https://open.xiaohongshu.com/
200 https://open.weibo.com/wiki/API
200 https://open.juejin.cn/
200 https://www.csdn.net/
200 https://www.zhihu.com/
422 https://open.zhihu.com/
```

## 手工/UI 证据

- 浏览器预览地址：`http://localhost:1420/#/platforms`
- 操作：打开“平台管理” → 点击“生态治理”。
- 页面可见：平台 × 内容形态准入矩阵、自动发布关闭、账号级降级原因、兼容性回归、市场关闭和官方来源入口。
- 浏览器控制台/页面错误：0。
- 截图：[M8 生态治理界面](./assets/M8-ecosystem-panel-2026-11-25.png)
- Playwright 断言结果：

```json
{
  "heading": true,
  "market": true,
  "present": {
    "Bilibili": true,
    "自动发布关闭": true,
    "账号级能力与降级原因": true,
    "兼容性回归": true,
    "市场关闭": true,
    "账号会话已过期": true
  },
  "errors": []
}
```

## M8 AC 快照

| 验收标准              | 结果 | 证据                                                                                                                                                                                                     |
| --------------------- | ---- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M8-AC-01 平台准入     | pass | 10 个平台均有官方来源、HTTP 检查结果、验证日期、账号类型、未支持事项和四形态结果；`m8_admission_registry_is_traceable_and_covers_every_content_kind`；未验证能力保持 experimental/restricted/unsupported |
| M8-AC-02 兼容性       | pass | `m8_embedded_adapter_contract_and_payload_regression_passes_together`；6 个内嵌 Adapter 通过身份唯一、v1 合约、四形态和共享载荷检查；M4 崩溃/超时隔离 9/9                                                |
| M8-AC-03 能力呈现     | pass | `m8_expired_account_degrades_platform_and_account_capabilities`；`m8_mcp_returns_platform_and_account_capability_matrix`；UI 账号级能力与降级原因截图                                                    |
| M8-AC-04 平台数据差异 | pass | `m8_platform_sync_preserves_raw_fields_content_mapping_and_metric_provenance`；四类作品映射、原始内容类型、rawFields、指标定义/单位/source 均保留                                                        |
| M8-AC-05 版本升级     | pass | `m8_upgrade_from_v6_preserves_content_accounts_tasks_and_metrics`；四类内容包、Markdown、账号、任务和历史指标恢复；`failed_migration_rolls_back_and_preserves_database`                                  |
| M8-AC-06 权限治理     | pass | `m8_plugin_upgrade_requires_review_of_new_permission_and_denial_is_effective`；新权限待审查，用户拒绝后 `network.request` 返回 `PERMISSION_DENIED`                                                       |
| M8-AC-07 发布安全     | pass | `m8_unverified_platform_forces_manual_publish_even_when_adapter_claims_support`；Adapter 声明支持仍只进入人工发布；UI 显示自动发布关闭                                                                   |
| M8-AC-08 插件市场决策 | pass | `m8_marketplace_policy_stays_closed_until_all_gates_pass`；ADR-003 明确六项门禁，当前市场 closed、第三方安装和受控试点均禁止                                                                             |
| M8-AC-09 内容形态覆盖 | pass | 准入 Schema 要求每个平台恰好四种形态；10 × 4 个读取/发布结果均为显式状态；未完成真实账号验证的形态不在 UI/MCP 显示 supported                                                                             |

## 未覆盖项与剩余风险

- 未使用 10 个平台的真实账号完成 OAuth/应用审核、读取、媒体上传、发布和指标往返验证。因此所有平台自动发布保持关闭，能力显式标记为 experimental/restricted/unsupported；这是本阶段的安全结果，不是 `supported` 证据。
- Windows/Linux 仍未提供强隔离；第三方插件市场保持关闭，ADR-003 记录开放前门禁。
- 官方来源可达性只证明公开入口可达，不证明接口权限、发布政策或账号能力；登记文件已明确区分。
- 系统文件对话框、真实 Keychain 和真实平台账号的桌面手工步骤未在本记录中重复执行；相关既有 M3/M4/M6/M7 自动化回归通过。
- M6 的 AC 自动化均通过，但其里程碑仍因 M2 前置门和真实平台补验未 accepted。M8 依赖 M6 accepted 的阶段前置条件未满足，因此本记录不创建 M8 acceptance report。

## 结论

M8-AC-01 至 M8-AC-09 的实现与自动化证据已通过，UI 和打包检查通过。M8 阶段保持 **implementation complete / acceptance blocked**：只有前置 M6 完成验收后，才可创建 `progress/M8-acceptance.md` 并标记 M8 accepted。

## 2026-11-25 平台 MCP profile 验证补充

- **范围：** 10 个平台 `*_mcp` profile、平台原生字段 Schema、Core 只读执行、发布载荷校验与 M8 回归测试。
- **基线：** `48c07a7` 及工作树中既有 M7/M8 改动；**验证 HEAD：** `4d95d5f` + 当前工作树文档/测试修订。
- **实现结果：** 新增 `douyin_mcp`、`xiaohongshu_mcp`、`bilibili_mcp`、`youtube_mcp`、`wechat_mcp`、`weibo_mcp`、`kuaishou_mcp`、`juejin_mcp`、`csdn_mcp`、`zhihu_mcp`；每个 profile 含接口来源、验证状态、原生作品 ID、字段映射、内容规则和人工交接入口。所有 profile 的 `autoPublishAllowed=false`、`publishDispatch=manual_only`。
- **命令与结果：**

```text
cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem
# pass, 8/8；新增 2 项平台 profile/发布校验测试
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
# pass, 103 tests
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
# pass
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
# pass
pnpm test
# pass, 44/44
pnpm typecheck
# pass
pnpm lint
# pass
pnpm build
# pass；仅既有 chunk > 500 kB 警告
```

- **接口检查：** 抖音开放平台 `upload_video`、`upload_image`、`create_video`、`create_image_text`、`video_list`、`video_data` 路径返回平台 JSON；无有效 token 时为 `access_token无效`。这只证明路径可达，不证明应用资格或发布成功。
- **未完成：** 10 个平台的账号级 OAuth/应用审核、真实作品回读、媒体上传、发布和指标往返仍未验证；小红书/掘金/CSDN/知乎没有已核实的官方内容发布 API，继续人工网页交接。
- **结论：** 平台 MCP 实现与自动化回归通过，但 M8 阶段仍为 `implementation in progress / acceptance blocked`，不创建 `M8-acceptance.md`。
