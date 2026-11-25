# ADR-002：首期 Adapter Runtime 采用 Core 监督的独立进程与 JSONL 合约

- **状态：** Accepted（2026-11-25，M4 实现决策）
- **决策范围：** M4 Adapter 与插件体系；M5/M6 的平台发布实现复用
- **上下文：** M4 需要让平台差异不侵入 Core，同时阻止插件直接读取 SQLite、Keychain 原始凭据或任意工作区。PRD 推荐 Node.js + TypeScript，但在首期 Runtime 尚未冻结时，直接引入可执行任意第三方 JavaScript 会增加打包、权限隔离和供应链风险。

## 决策

1. **首期只运行官方适配器。** Adapter 由 CreatorRelay 官方随应用分发，Runtime 为 Rust Core 监督的独立子进程；官方测试 Adapter 以 `creatorrelay --adapter-runner --profile <profile>` 启动。M4 不开放任意第三方代码的安装或启用。
2. **协议为逐行 JSON（JSONL）。** Core 发送 `request`，Adapter 返回 `response`；Adapter 可通过 `hostCall` 请求账号、工作区、网络或平台能力，Core 按用户授权逐项允许或拒绝。协议版本由 Manifest 的 `adapterContractVersion` 固定，首期只支持 `1`。
3. **权限在 Core 执行。** 插件不会收到 SQLite 路径、Keychain 引用、Cookie/Token 原文或任意工作区路径。账号能力通过 `credential-service` 代理返回不透明账号引用；工作区能力通过 `asset-service` 返回受控资产引用；网络能力通过 Core 代理，插件进程本身没有直接 socket 权限。
4. **macOS 使用系统沙箱。** 进程通过 `/usr/bin/sandbox-exec` 启动，默认拒绝文件、网络和进程能力，只允许系统运行时、当前应用可执行目录、插件目录和插件数据目录。SQLite、工作区、Keychain 和任意用户目录不在允许列表内。
5. **其他平台明确降级。** Windows/Linux 首期只提供独立进程隔离，不声称强安全沙箱；该状态下只允许官方适配器，非官方代码禁止安装和启用。后续若引入 AppContainer、bubblewrap 或平台等价机制，再提升隔离等级。
6. **故障隔离。** 每个 Adapter 拥有独立进程、请求超时和健康检查。崩溃、超时和协议违规只影响当前 Adapter；Core 记录审计事件、停止该进程并向用户展示错误，用户可重启。其他 Adapter 和 Core 继续工作。
7. **版本与回退。** 插件版本使用 SemVer，Adapter 合约版本独立校验。升级/回退必须先在禁用状态执行；权限只保留新旧 Manifest 的交集，新权限必须重新审查，已移除权限不会自动恢复。
8. **逐形态能力。** Manifest 声明是申请，不是可信结果。Core 必须通过 `probe` 获得每个 `ContentKind` 的真实状态；声明为 `supported` 但探测失败时显示 `unavailable`，受限/实验能力单独标识。

## 备选方案

- **Node.js + TypeScript 独立进程：** 开发体验好，但需要额外打包 Node 运行时，且当前阶段无法为任意第三方代码提供跨平台强隔离；暂缓。
- **Rust 动态库/同进程插件：** 没有进程故障隔离，单个 panic/死循环可影响 Core；不采用。
- **进程内 WASM：** 隔离潜力较好，但当前平台 API、异步网络和文件素材访问模型尚未确定；留作后续强隔离候选。
- **完整第三方插件市场：** M4 明确不在范围内；先冻结官方适配器合约和权限模型。

## 后果与限制

- M4 可验证真实进程隔离、权限代理、沙箱文件拒绝、能力探测和崩溃/超时恢复，但适配器需要遵循 Rust 官方 runner 的实现方式。
- macOS `sandbox-exec` 已被 Apple 标记为 deprecated；当前可用于首期防误用与验收，但不能作为长期唯一隔离方案。ADR 后续需要重新评估。
- Windows/Linux 没有强隔离；虽然仍执行 Core 主机 API 权限检查和独立进程，但恶意代码可能有与当前用户相同的本地文件访问能力，因此第三方代码保持禁用。
- 网络代理在 M4 只完成权限门与拒绝路径；真实发布网络代理在 M6 实现，之前不要将 `network` 授权视为完整发布能力。
- 工作区资产服务在 M4 固定为不透明 `assetId` 引用和路径校验；真实素材传输、临时 URL 或流式读取在 M5/M6 按发布实现补全。
