# M8 验证记录 — 2026-11-25

> 本文件只记录验证活动与结果，不是验收报告，也不能单独作为 M8 已验收的依据。

- **阶段：** M8 生态扩展
- **记录日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（arm64）；Tauri 2；Chrome Playwright 浏览器预览
- **M8 实施起点：** `e50c9e1`（M6/M7/M8 并发实施前的功能基线；验收只计入 M8 相关文件与行为）
- **验证 HEAD：** `869f2a8`（M8 平台准入、运营模型、平台 MCP profile 与最终回归验证修订）
- **总体状态：** M8-AC-01 至 M8-AC-09 自动化证据 `pass`；阶段验收 `blocked`，因为前置 M6 仍未 accepted（M6 受 M2 前置门阻塞）

## 实现范围

- 新增平台准入登记、准入 Schema、官方来源/验证日期/账号类型/未支持事项和四类内容形态结果。
- 新增平台及账号级能力矩阵；账号非 ACTIVE、禁用、平台未准入时自动降级并返回原因。
- 新增发布准入门禁：未通过平台准入时强制人工发布，Adapter 自报 `supported` 不能覆盖。
- 扩展统一作品模型：保留平台原始内容类型、原始字段 JSON、指标定义/单位/来源。
- 新增 SQLite v6 → v7 迁移，并通过四类内容包、Markdown、账号、任务和历史指标恢复测试。
- 新增插件升级新权限重新审查与拒绝后 hostCall 强制拒绝验证。
- 新增第三方插件市场策略、六项门禁和 ADR-003；市场保持关闭。
- 新增 MCP `platform_capabilities_get` 只读工具和 10 个平台专属 `*_mcp` profile；profile 保留原生字段、接口来源、内容规则与人工交接信息。
- 新增“平台管理 → 生态治理”UI，展示准入、来源、账号降级、兼容性和市场门禁。

## 命令与结果

| 检查             | 命令                                                                                                                                                                                                                                                  | 结果                                                                                         |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Rust 格式        | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`                                                                                                                                                                                     | pass                                                                                         |
| Rust Clippy      | `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`                                                                                                                                                                      | pass                                                                                         |
| Rust 全量测试    | `cargo test --manifest-path src-tauri/Cargo.toml --all-targets`                                                                                                                                                                                       | pass：103 项（73 个 lib + M4 9 + M6 5 + M7 8 + M8 8）                                        |
| M8 集成测试      | `cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem`                                                                                                                                                                                 | pass，8/8                                                                                    |
| 数据库迁移       | `cargo test --manifest-path src-tauri/Cargo.toml --lib database::tests`                                                                                                                                                                               | pass；含 v6 四类内容/账号/任务/指标 → v7，以及失败迁移回滚                                   |
| Adapter 回归     | `cargo test --manifest-path src-tauri/Cargo.toml --test m4_runtime`                                                                                                                                                                                   | pass，9/9；含崩溃/超时隔离和版本回退                                                         |
| M6/M7 回归       | `cargo test --manifest-path src-tauri/Cargo.toml --test m6_operations && cargo test --manifest-path src-tauri/Cargo.toml --test m7_mcp`                                                                                                               | pass，5/5 + 8/8                                                                              |
| 前端测试         | `pnpm test`                                                                                                                                                                                                                                           | pass，11 个测试文件、44 项测试                                                               |
| 前端类型检查     | `pnpm typecheck`                                                                                                                                                                                                                                      | pass                                                                                         |
| 前端 Lint        | `pnpm lint`                                                                                                                                                                                                                                           | pass                                                                                         |
| 前端构建         | `pnpm build`                                                                                                                                                                                                                                          | pass；仅既有 chunk > 500 kB 警告                                                             |
| M8 前端格式      | `pnpm exec prettier --check src/components/EcosystemPanel.tsx src/stores/ecosystemStore.ts src/types/ecosystem.ts src/pages/PlatformsPage.tsx src/stores/index.ts src/types/operations.ts src/stores/operationsStore.ts src/utils/operations.test.ts` | pass                                                                                         |
| 全仓库格式       | `pnpm format:check`                                                                                                                                                                                                                                   | fail；M8 范围外既有文件 `StudioWorkspaceEmpty.tsx`、`StudioPage.css` 未格式化                |
| Tauri Debug 打包 | `pnpm tauri build --debug --bundles app --no-sign`                                                                                                                                                                                                    | pass；`src-tauri/target/debug/bundle/macos/creatorrelay.app`（53.02 MiB）                    |
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
- 截图：本地证据 `progress/verification/assets/M8-ecosystem-panel-2026-11-25.png`；按仓库策略截图目录不进入 Git。
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
| M8-AC-04 平台数据差异 | pass | `m8_platform_sync_preserves_raw_fields_content_mapping_and_metric_provenance`；四类作品映射、原始内容类型、rawFields、指标定义/单位/source 均保留；平台 profile 保留原生 ID 和字段映射                   |
| M8-AC-05 版本升级     | pass | `m8_upgrade_from_v6_preserves_content_accounts_tasks_and_metrics`；四类内容包、Markdown、账号、任务和历史指标恢复；`failed_migration_rolls_back_and_preserves_database`                                  |
| M8-AC-06 权限治理     | pass | `m8_plugin_upgrade_requires_review_of_new_permission_and_denial_is_effective`；新权限待审查，用户拒绝后 `network.request` 返回 `PERMISSION_DENIED`                                                       |
| M8-AC-07 发布安全     | pass | `m8_unverified_platform_forces_manual_publish_even_when_adapter_claims_support`；Adapter 声明支持仍只进入人工发布；UI 显示自动发布关闭                                                                   |
| M8-AC-08 插件市场决策 | pass | `m8_marketplace_policy_stays_closed_until_all_gates_pass`；ADR-003 明确六项门禁，当前市场 closed、第三方安装和受控试点均禁止                                                                             |
| M8-AC-09 内容形态覆盖 | pass | 准入 Schema 要求每个平台恰好四种形态；10 × 4 个读取/发布结果均为显式状态；平台专属 MCP profile 的只读发布校验和人工交接测试通过；未完成真实账号验证的形态不在 UI/MCP 显示 supported                      |

## 未覆盖项与剩余风险

- 未使用 10 个平台的真实账号完成 OAuth/应用审核、读取、媒体上传、发布和指标往返验证。因此所有平台自动发布保持关闭，能力显式标记为 experimental/restricted/unsupported；这是本阶段的安全结果，不是 `supported` 证据。
- Windows/Linux 仍未提供强隔离；第三方插件市场保持关闭，ADR-003 记录开放前门禁。
- 官方来源可达性只证明公开入口可达，不证明接口权限、发布政策或账号能力；登记文件已明确区分。
- 系统文件对话框、真实 Keychain 和真实平台账号的桌面手工步骤未在本记录中重复执行；相关既有 M3/M4/M6/M7 自动化回归通过。
- M6 的 AC 自动化均通过，但其里程碑仍因 M2 前置门和真实平台补验未 accepted。M8 依赖 M6 accepted 的阶段前置条件未满足，因此本记录不创建 M8 acceptance report。

## 结论

M8-AC-01 至 M8-AC-09 的实现与自动化证据已通过，UI 和打包检查通过。M8 阶段保持 **implementation complete / acceptance blocked**：只有前置 M6 完成验收后，才可创建 `progress/M8-acceptance.md` 并标记 M8 accepted。

## 2026-11-25 平台 MCP profile 验证补充

- **范围：** 10 个平台 `*_mcp` profile、平台原生字段 Schema、Core 只读执行、发布载荷校验、`lib.rs`/`content.rs` 模块化与 M8 回归测试。
- **基线：** `48c07a7`；**验证 HEAD：** `95e129b` + 本次文档修订工作树。
- **实现结果：** 新增 `douyin_mcp`、`xiaohongshu_mcp`、`bilibili_mcp`、`youtube_mcp`、`wechat_mcp`、`weibo_mcp`、`kuaishou_mcp`、`juejin_mcp`、`csdn_mcp`、`zhihu_mcp`；每个 profile 含接口来源、验证状态、原生作品 ID、字段映射、内容规则和人工交接入口。所有 profile 的 `autoPublishAllowed=false`、`publishDispatch=manual_only`。
- **命令与结果：**

```text
cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem
# pass, 8/8；新增 2 项平台 profile/发布校验测试
cargo test --manifest-path src-tauri/Cargo.toml --all-targets
# pass, 106 tests
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
# pass
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
# pass
pnpm test
# pass, 47/47
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

## 2026-11-25 创作者中心链接与 MCP 边界验证补充

- **范围：** 用户提供的 8 个创作者中心地址、SQLite v8 持久化与 URL 更新、平台管理 UI、MCP `creator_centers_list` 链接目录工具，以及匿名 HTTPS 可达性检查。
- **基线：** `8fef1965e8a352d3104ad10f98460d20618d39a3`；**HEAD：** 同一提交。实现与本记录均为当前未提交工作树改动。
- **实际 MCP 边界：** 仓库中的 MCP Host 是 CreatorRelay 的本地 MCP Server，不是连接平台远程 MCP Server 的 MCP Client。现有平台专属 MCP profile 对本地缓存做只读查询、对内容做本地校验并生成手工交接；未调用平台实时作品 API，也没有向平台实际发布。新增 `creator_centers_list` 只暴露 SQLite 链接，返回 `platformDataAccess=not_connected`、`automatedPublishing=disabled`。

### 实时网页入口检查

以下请求在本验证环境中执行；均为未登录 HTTPS GET，curl 跟随重定向并丢弃响应正文。`200 text/html` 仅表示网页入口响应，不能证明身份已登录、MCP 读取或作品发布能力。

```sh
python3 - <<'PY'
import subprocess
urls={
'douyin':'https://creator.douyin.com/',
'xiaohongshu':'https://creator.xiaohongshu.com/new/home',
'zhihu':'https://www.zhihu.com/signin?next=%2F',
'wechat':'https://mp.weixin.qq.com/cgi-bin/home?t=home/index&lang=zh_CN',
'kuaishou':'https://cp.kuaishou.com/profile',
'bilibili':'https://member.bilibili.com/platform/home',
'juejin':'https://juejin.cn/creator/home',
'csdn':'https://mp.csdn.net/',
}
for name,url in urls.items():
    r=subprocess.run(['curl','-L','-sS','--max-time','12','-o','/dev/null','-w','%{http_code}\t%{url_effective}\t%{content_type}\\n',url],capture_output=True,text=True)
    print(f'{name}\t{r.stdout.strip() or "NO_HTTP_RESPONSE"}\t{r.stderr.strip()}')
PY
```

| 平台       | 匿名 GET 结果                  | 结论                            |
| ---------- | ------------------------------ | ------------------------------- |
| 抖音       | `200 text/html; charset=utf-8` | 入口可响应；账号/MCP/发布未验证 |
| 小红书     | `200 text/html; charset=UTF-8` | 入口可响应；账号/MCP/发布未验证 |
| 知乎       | `200 text/html; charset=utf-8` | 入口可响应；账号/MCP/发布未验证 |
| 微信公众号 | `200 text/html; charset=UTF-8` | 入口可响应；账号/MCP/发布未验证 |
| 快手       | `200 text/html; charset=UTF-8` | 入口可响应；账号/MCP/发布未验证 |
| 哔哩哔哩   | `200 text/html`                | 入口可响应；账号/MCP/发布未验证 |
| 稀土掘金   | `200 text/html; charset=utf-8` | 入口可响应；账号/MCP/发布未验证 |
| CSDN       | `200 text/html; charset=utf-8` | 入口可响应；账号/MCP/发布未验证 |

### 自动化检查

| 命令                                                                                                                                                                                                                       | 结果                                                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `pnpm typecheck`                                                                                                                                                                                                           | pass                                                                                                                                                                                                                                 |
| `pnpm test`                                                                                                                                                                                                                | pass，12 files / 47 tests                                                                                                                                                                                                            |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib`                                                                                                                                                                    | pass，76 tests；包括链接初始化、SQLite 更新/HTTPS 校验及 MCP 链接工具边界测试                                                                                                                                                        |
| `pnpm lint`                                                                                                                                                                                                                | pass                                                                                                                                                                                                                                 |
| `pnpm build`                                                                                                                                                                                                               | pass；Vite 输出既有的大 chunk 警告（主 JS chunk 约 1.53 MB）                                                                                                                                                                         |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`                                                                                                                                                                | pass                                                                                                                                                                                                                                 |
| `pnpm exec prettier --check src/components/CreatorCentersPanel.tsx src/components/Icons.tsx src/pages/PlatformsPage.tsx src/stores/creatorCenterStore.ts src/stores/creatorCenterStore.test.ts src/types/creatorCenter.ts` | pass                                                                                                                                                                                                                                 |
| `pnpm format:check`                                                                                                                                                                                                        | fail：仓库中 4 个未修改文件格式不符：`src/components/studio/ContentEditor.tsx`、`src/components/StudioWorkspaceEmpty.tsx`、`src/pages/StudioPage.css`、`src/utils/operations.ts`；本次修改的 UI 文件单独格式检查通过，未改动无关文件 |

### 本次需求验收快照

| 标准                                                          | 结果                                          | 证据 / 缺口                                                                                                                                                                    |
| ------------------------------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 8 个创作者中心链接在前端可见、可编辑、打开，并持久化到 SQLite | pass（自动化）；桌面 UI 手工操作 not verified | 平台管理新增“创作者中心”页；schema v8 建表并预置 8 个地址；Rust CRUD/HTTPS 校验测试通过，前端 store 更新测试及 typecheck/build 通过；本次未启动桌面客户端逐项手工保存/重启验证 |
| 通过 MCP 读取这些平台的已登录实时作品与指标                   | not verified / 当前未接入                     | 无外部 MCP Server 配置或账号授权；匿名 HTTP 200 不是该能力证据。新增 MCP 工具只读取配置链接，既有能力工具仍按当前 profile 报告已验证能力                                       |
| 通过 MCP 实际发布作品                                         | not verified / 自动发布关闭                   | 无平台账号/授权往返测试；没有实际提交。现有 profile 的发布 dispatch 为 `manual_only`，专属工具只校验并交接                                                                     |
| MCP Client 可读取链接目录（只读）                             | pass（代码与单元测试）                        | MCP 工具 `creator_centers_list` 已注册为 read 权限，支持 `platformId` 过滤，并明确返回未连接/自动发布关闭；单测验证 URL 和边界字段                                             |

- **人工/平台检查：** 未登录各平台账号，未读取真实作品/分析数据，未进行实际发布，也未在桌面 Tauri 窗口执行 UI 手工回归。
- **总体状态：** 创作者中心链接 UI、SQLite 和 MCP 链接目录实现检查通过；真实平台 MCP 数据读取与发布仍 `not verified`。该功能增量为 `partial`；M8 保持 `blocked / not accepted`，不创建 acceptance report。

## 当前工作树复核补充（2026-11-25）

验证过程中仓库 HEAD 从起始 `8fef196` 前进到 `e9d0e7f`，期间出现额外的 Rust 源码拆分改动；当前工作树还包含 `src-tauri/src/content/*` 与 `src-tauri/src/lib_tests.rs`、`src-tauri/src/workspace_support.rs` 等非本需求文件变化。为避免覆盖并行工作，本需求没有回滚或修理这些文件。

先前 `cargo test --manifest-path src-tauri/Cargo.toml --lib` 的 76 项通过结果是在 HEAD 前进前取得。针对当前工作树再次执行：

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
# fail：src-tauri/src/lib.rs:2477 的 #[cfg(test)] 后没有 item

cargo test --manifest-path src-tauri/Cargo.toml --all-targets
# fail：额外出现 src-tauri/src/content/* 拆分模块中的未解析函数/私有可见性错误
```

因此当前最终工作树的 Rust 回归为 `blocked`；不能把之前的 76 项通过当作当前 HEAD 的结果。创作者中心代码的 TypeScript 测试、类型检查、lint、前端构建仍通过，但当前 Tauri/Rust 检查未通过。修复上述并行源码拆分后，必须重跑 Rust 全目标测试，再做桌面 UI 手工检查；在此之前 M8 继续 `not accepted / blocked`。
