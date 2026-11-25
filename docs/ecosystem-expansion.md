# M8 生态扩展实现说明

> 状态：M8 平台准入、账号级能力矩阵、兼容性检查与第三方插件市场门禁实现说明。
> 原则：官方文档可达不等于 API 可用；Manifest/Adapter 自报能力不等于真实平台能力；没有账号级运行证据时不得显示 `supported`。

## 1. 数据流

```text
platform-admissions.json
  ├─ 官方来源、验证日期、账号类型、未支持事项
  ├─ 平台 × ContentKind 的 read/publish 评估
  └─ autoPublishAllowed / publishDispatch
            │
            ▼
Rust ecosystem service
  ├─ 校验平台 ID、来源 URL、四形态完整性
  ├─ 结合账号 enabled/status 生成账号级有效能力
  ➔ 发布策略：未通过准入时强制 manual_only
            │
            ▼
React 生态治理页 / MCP 工具 / Publish Preview
```

平台登记是静态、可审计的发布产物。登记更新必须同时更新验证日期、来源和逐形态结果；运行时账号状态只能在登记结果之上降级，不能把未验证能力升级成 `supported`。

## 2. 能力状态

| 状态 | 含义 |
| --- | --- |
| `supported` | 有当前账号级运行证据，并通过平台准入和发布策略 |
| `restricted` | 平台允许但受账号类型、审核或额外授权限制 |
| `experimental` | 官方入口/文档存在，但当前版本没有完整运行证据 |
| `unsupported` | 平台不提供该内容形态或没有可验证的原生能力 |
| `unavailable` | 账号失效、运行时探测失败或接口暂不可用 |

账号不是 `ACTIVE`、已被禁用或平台没有准入记录时，所有读取/发布能力在账号视图中降级为 `unavailable` 并显示原因。`experimental`、`restricted` 不会自动升级为 `supported`。

## 3. 平台准入记录

机器可读登记表：[`platform-admissions.json`](../src-tauri/resources/ecosystem/platform-admissions.json)。

Schema：[`platform-admissions.schema.json`](./schemas/platform-admissions.schema.json)。

当前 M8 对 PRD 中的十个平台完成文档/门户评估，但**没有真实账号级发布验证**：

- 所有平台的 `admissionStatus` 为 `evaluation_only`；
- 所有平台的 `autoPublishAllowed` 为 `false`，`publishDispatch` 为 `manual_only`；
- 四个内容形态全部有独立 `read` / `publish` 结果和原因；
- OAuth/应用审核、账号权限、媒体上传、审核状态和指标读取未验证的能力保持 `experimental` 或 `unsupported`；
- 官方来源检查记录 HTTP 状态；登录受限或自动化检查异常时明确记录，不推断为可用。

因此 M8 新增能力的安全默认值是**人工发布可用、自动发布关闭**。后续新增官方适配器时，只有真实账号运行证据、平台政策复核和失败隔离测试同时通过，才能修改登记并把单个形态提升为 `supported`。

## 4. 账号级能力与自动发布门禁

- `list_account_capability_snapshots` 将平台准入结果与 `platform_accounts.status` 合并。
- `publish_policy` 要求平台已准入、账号 `ACTIVE`、`autoPublishAllowed=true`、发布方式为 `verified_auto`，且该形态有 `runtime_verified` 证据。
- 任一条件失败时，发布预览只能选择人工发布包；不会静默改用未验证 Adapter。
- 人工发布包保留内容形态、正文/媒体顺序、封面、原始文件名和目标账号信息。

## 5. 统一平台数据模型

`ExternalWork` 保留：

- 统一 `contentKind` 和平台原始 `originalContentType`；
- 平台 `externalId` 和来源 (`publish`、`sync`、`manual_check`)；
- 平台原始字段 `rawFields`（JSON），未知字段不会被覆盖或丢弃；
- 指标名称、定义、单位、采样时间和来源；
- 账号删除后保留历史作品与指标，但允许账号引用为空。

Adapter 同步输出必须把平台特有字段放入对象型 `rawFields`；Core 只做映射和展示，不把未验证字段自动升级为统一指标。

## 6. 兼容性与故障隔离

- 所有内嵌 Manifest 必须通过现有 v1 合约校验、ID 唯一性、四形态覆盖和共享发布载荷回归检查。
- 每个 Adapter 独立进程、独立超时；单个适配器崩溃/超时不会停止其他 Adapter。
- 升级/回退前必须禁用；新旧权限取交集，新权限需要重新审查。
- 数据结构升级使用 SQLite 事务；内容包和 Markdown 文件是独立权威来源，迁移失败不得删除或重写用户文件。

## 7. 第三方插件市场

市场策略：[`marketplace-policy.json`](../src-tauri/resources/ecosystem/marketplace-policy.json)。
决策文档：[ADR-003](./adr/ADR-003-plugin-marketplace.md)。

当前决定为 `closed`，仅允许随应用分发的官方适配器。开放第三方安装前必须同时满足：

1. 威胁评估完成；
2. 安装来源和签名校验链可用；
3. 所有支持平台具备强隔离；
4. 更新与回退可验证；
5. 权限可撤销且 Core 能强制执行；
6. 远程应急下架/吊销可执行。

未满足时，安装入口继续保持仅官方分发，不做“受控试点”营销表述。

## 8. 验证入口

- Rust：`cargo test --manifest-path src-tauri/Cargo.toml ecosystem::`
- 集成：`cargo test --manifest-path src-tauri/Cargo.toml --test m8_ecosystem`
- 前端：`pnpm test && pnpm typecheck && pnpm build`
- 格式：`pnpm exec prettier --check <M8 changed frontend files>` 与 `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
