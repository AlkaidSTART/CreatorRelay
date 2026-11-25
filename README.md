# Tauri + React + Typescript

This template should help get you started developing with Tauri, React and Typescript in Vite.

## 功能文档

- [生态扩展、平台准入与插件市场治理（M8）](./docs/ecosystem-expansion.md)
- [M8 插件市场决策 ADR](./docs/adr/ADR-003-plugin-marketplace.md)
- [发布、同步与运营数据实现说明（M6）](./docs/m6-publishing-operations.md)
- [内容工作室实现说明（M5）](./docs/content-studio.md)
- [平台账号、凭据与会话实现说明](./docs/platform-accounts.md)
- [Adapter 与插件体系实现说明](./docs/adapters.md)
- [M4 Runtime 决策 ADR](./docs/adr/ADR-002-adapter-runtime.md)
- [M1 桌面基础设施实现说明](./docs/m1-desktop-foundation.md)

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## 自动发版

工作流位于 `.github/workflows/release.yml`。每次推送或合并到 `main` 分支（也可在 Actions 页面手动触发）都会：

- 在 macOS 上构建 Universal `.dmg`（Apple Silicon + Intel）
- 在 Windows 上构建 NSIS `.exe`
- 校验构建产物并生成 `SHA256SUMS.txt`
- 创建或更新对应的 GitHub Release，并自动生成变更说明

构建版本由 `package.json` 的基础版本和 GitHub Actions 运行序号组成，例如基础版本为 `0.1.0`、运行序号为 `12` 时，构建版本为 `0.1.12`。版本只会在 CI 构建过程中写入，不会回写仓库。

如需启用 macOS 签名和公证，请在仓库 Secrets 中配置：

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- 可选：`KEYCHAIN_PASSWORD`

Windows 安装包当前未配置代码签名；如未签名，SmartScreen 可能显示警告。

如果 Release 步骤出现权限错误，请在仓库 `Settings → Actions → General → Workflow permissions` 中选择 “Read and write permissions”。
