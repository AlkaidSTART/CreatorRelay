# M6 验证记录 — 2026-11-25

> 本文件只记录验证结果，不是验收报告，也不能单独作为 M6 已验收的依据。

- **阶段：** M6 发布、同步与运营数据
- **记录日期：** 2026-11-25
- **验证环境：** macOS 26.6.2（25G83），arm64
- **功能基线：** `d709a52`（M6 实现开始前）
- **M6 实现提交：** `1f52f9d`、`e50c9e1`、`ac99dda`；`53428d0` 中仅计入 M6 发布/同步/页面文件，M7 `mcp.rs` 不计入 M6
- **隔离验证提交：** `73307e5` + 当前 M6 工作区修订（临时 detached worktree，避免并行 M7/M8 未完成改动）
- **前置状态：** M1、M3、M4、M5 已通过；M2 仍为 `not accepted`
- **总体状态：** M6 实现和 M6-AC-01～14 自动化验证通过；里程碑验收因 M2 前置门未通过而保持 `blocked`，未创建验收报告

## 验证范围与隔离说明

共享工作区同时存在 M7/M8 未完成改动。为使 M6 验证不被范围外文件干扰，本次在 `/tmp` 的 detached Git worktree 中基于 `73307e5` 叠加当前 M6 工作区修订后执行检查。验证结果只代表 M6 文件，不把 M7/M8 改动计入 M6。

## 验收标准快照

| 验收标准 | 结果 | 证据 |
| --- | --- | --- |
| M6-AC-01 多目标任务 | pass | `m6_ac_01_03_05_06_13_*`：同一主稿创建两个独立目标；成功目标写入 `SUCCESS`，另一目标进入 `NEEDS_MANUAL_ACTION`，任务记录互不覆盖 |
| M6-AC-02 分形态能力 | pass | `m6_ac_02_04_07_to_12_and_14_*`：同步 Adapter 的图文可自动发布；视频预览返回 `manual`，`auto_publish` 动作为不可用并带运行时原因 |
| M6-AC-03 发布前确认 | pass | 预览返回完整任务快照、10 分钟确认令牌和媒体/封面；伪造令牌被拒绝；取消任务路径可执行 |
| M6-AC-04 人工发布路径 | pass | 人工任务导出 `publish-package.json`、`body.md` 与素材目录；可标记成功或取消；封面和媒体信息保留 |
| M6-AC-05 超时不确定 | pass | 超时 Adapter 使任务进入 `NEEDS_MANUAL_ACTION / RESULT_UNCERTAIN`，尝试次数保持 1；再次确认被拒绝；人工核对后保存结果 |
| M6-AC-06 结果持久化 | pass | 成功任务保存内容版本、目标账号、时间、外部 ID 和结果事件；失败任务保存 `FAILED / VERIFIED_FAILED` 与原因 |
| M6-AC-07 分形态作品同步 | pass | 同步导入四种 `ContentKind`；内容中心支持四形态、平台、账号、时间、来源状态和关键词过滤；详情保留原始类型与外部 ID |
| M6-AC-08 同步幂等 | pass | 同日重复同步不新增作品；同指标同日更新采样；跨日同步增加新样本；作品总数保持不变 |
| M6-AC-09 指标口径 | pass | `视频播放量`、`文章阅读量` 等原始名称/定义/单位/采集时间保留；`canonical_key` 为空且 `comparable=false`，不合并总数 |
| M6-AC-10 趋势与筛选 | pass | 日/周/月聚合均对有至少两个采样点的视频作品产生可追溯序列；空筛选明确提示且不补零 |
| M6-AC-11 Dashboard | pass | Dashboard 卡片和列表按平台、账号、内容形态与 7/30/90 天过滤；空数据不使用伪造值 |
| M6-AC-12 账号失效恢复 | pass | `m6_ac_12_*`：同步返回 `ACCOUNT_INVALID` 后账号进入 `NEEDS_REAUTH`，未完成任务暂停；重新授权后显式恢复为 `CREATED` |
| M6-AC-13 视频发布往返 | pass | 视频 Adapter 接收 `video`、1 个有序视频和封面；成功事件记录 `mediaOrder=[1]` 与封面 ID；任务保存外部 ID |
| M6-AC-14 图文顺序与封面 | pass | 三图任务 Adapter 收到 `mediaOrder=[1, 2, 3]`；人工包 `mediaOrder` 相同，封面素材角色和归属正确 |

补充验证：

- 定时任务确认后进入 `SCHEDULED`，未到期不执行；到期后只执行一次并成功。
- 部分同步失败返回 `PARTIAL`，保留 4 个成功作品并暴露 1 条可读错误。

## 自动化命令与结果

以下命令在隔离 worktree 中执行：

| 检查 | 命令 | 结果 |
| --- | --- | --- |
| Rust 格式 | `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` | pass |
| M6 验收集成测试 | `cargo test --manifest-path src-tauri/Cargo.toml --test m6_operations` | pass，5/5 |
| 数据库迁移 | `cargo test --manifest-path src-tauri/Cargo.toml database::tests` | pass，5/5；fresh/v1/v2→当前 schema，含 schema v5 发布与同步表 |
| 前端 lint | `pnpm lint` | pass，无错误/警告 |
| 前端类型检查 | `pnpm typecheck` | pass |
| 前端单测 | `pnpm test` | pass，43/43 |
| 前端生产构建 | `pnpm build` | pass；仅有既有大 chunk 告警 |
| Tauri 桌面构建 | `pnpm tauri build --debug --bundles app --no-sign` | pass，`src-tauri/target/debug/bundle/macos/creatorrelay.app`（51.78 MiB） |

M6 相关 Rust 代码已在 `-D warnings` 下通过 Clippy；全仓库原样 Clippy 仍受并行 M7 `mcp.rs` 的未使用字段和既有 AI 范围内 lint 影响，未把范围外警告计为 M6 通过证据。

## 手工/UI 证据

在 Tauri debug bundle 中实际打开并检查：

- `#/dashboard`：运营总览、7/30/90 天、平台/账号/形态筛选、四类指标卡、近期作品与风险区均渲染；无数据时显示“不会展示伪造数据”。
- `#/publishing`：状态筛选、运行到期任务、从平台稿发起入口和空状态正常；页面文案明确发布前确认。
- `#/content`：同步账号/适配器选择、关键词、平台、账号、形态、来源状态和时间过滤均可见；无作品时提示先同步。
- `#/analytics`：日/周/月、7/30/90 天、平台/账号/形态过滤均可见；无采样时明确显示“不会用 0 代替缺失数据”。

未配置真实工作区账号，因此 UI 手工检查不替代集成测试中的成功、失败、超时、人工包和同步场景。

## 未覆盖与剩余风险

- **M2 前置未满足：** M2 阶段仍为 `not accepted`，因此 M6 不创建验收报告、不标记 accepted。
- **真实平台验证不足：** 当前通过的是官方测试 Adapter；尚未用至少两个真实平台账号验证真实发布、读取和指标口径。
- **真实平台定时执行：** 已有确认、计划时间和应用内到期执行；系统级后台调度器不在 M6 实现范围。
- **UI 手工场景：** 本次 Tauri UI 检查没有真实平台数据，完整媒体/失败/同步状态由 M6 集成测试覆盖。
- **共享工作区：** M8 并行改动曾使全仓前端 typecheck 暂时失败；隔离 M6 worktree 检查通过，不能把并行阶段状态算作 M6 缺陷或完成证据。

## 结论

M6 的 14 项验收场景及补充定时/部分失败场景通过自动化验证，M6 实现具备发布确认、分形态能力、人工包、超时核对、幂等同步、原始指标趋势和 Dashboard 筛选能力。但 M2 前置尚未验收，真实平台验证也未完成，因此 M6 **保持未验收**；只有满足前置门并完成真实平台补验后才可创建 `progress/M6-acceptance.md`。
