# 平台账号、凭据与会话实现说明

> 状态：M3 当前实现说明。平台认证能力以实际接入和验证结果为准，未开放能力不会显示为可用。

## 1. 数据模型

M3 使用 `Platform → Accounts → Platform Sessions` 三级边界：

```text
platform_catalog（Rust 静态目录，不入 SQLite）
        │
        └── platform_accounts（账号别名、已验证 UID/昵称、状态、启用/默认设置）
                    │
                    └── platform_sessions（每账号独立 profile_id、认证方式、挑战和会话状态）
```

相关 SQLite 表：

| 表 | 内容 | 不保存 |
| --- | --- | --- |
| `platform_accounts` | 平台 ID、别名、已验证 UID/昵称、状态、`profile_id`、`credential_ref` | Cookie、Token、密码 |
| `platform_sessions` | 会话状态、认证方式、一次性 `challenge_id`、最近验证时间 | 会话明文 |
| `platform_works` | 外部作品与账号的关联；账号删除时使用 `ON DELETE SET NULL` 保留作品 | 凭据 |
| `account_audit_logs` | 脱敏操作动作、平台、别名和固定结果说明 | Cookie、Token、密码、请求原文 |

数据库 schema 通过 `schema_migrations` 升级到版本 2。账号表、会话表、作品关联表和审计表在同一迁移事务内创建，重复初始化不会重复写入版本。

## 2. 状态机

账号状态：

```text
UNAUTHORIZED ──开始认证──> AUTHENTICATING ──身份验证成功──> ACTIVE
       ▲                         │
       │                         ├──会话失效/身份失败──> NEEDS_REAUTH
       │                         ├──网络或校验异常────> ERROR
       │                         └──用户取消──────────> UNAUTHORIZED / NEEDS_REAUTH
       │
       └──重新启用────────────────────────────────────── DISABLED

ACTIVE ──检测到会话失效──> NEEDS_REAUTH
ACTIVE/其他状态 ──检测异常──> ERROR
```

规则：

- `ACTIVE` 只能由平台身份校验成功产生。
- 仅存在本地 profile 目录、本地 Cookie 文件或用户手工填写的内容，不足以进入 `ACTIVE`。
- 会话检测成功后更新 `last_checked_at`、UID 和昵称；检测失败后更新错误状态并阻止依赖该账号的后续任务。
- 禁用账号保留凭据，但阻止依赖任务；重新启用后进入 `NEEDS_REAUTH`，需要重新检测。

## 3. 凭据生命周期

1. 用户为账号创建独立 `credential_ref`，例如 `account:<uuid>:session`。
2. 用户通过系统浏览器完成平台登录，后端接收用户粘贴的会话 Cookie。
3. 后端先调用平台身份接口，只有返回真实 UID 与昵称时才接受该会话。
4. 校验通过后，敏感值写入操作系统安全凭据存储：
   - macOS：Keychain；
   - Windows：Credential Manager；
   - Linux：Secret Service。
5. SQLite 只保存 `credential_ref` 和会话状态；审计日志只保存固定动作说明。
6. 删除账号时按确认范围删除 Keychain 凭据和账号会话容器；历史作品记录保留并解除账号关联。

凭据存储不可用时，连接流程返回明确错误，不降级为 SQLite 或普通文件明文存储。

## 4. 浏览器与账号隔离

- 每个账号创建独立 `profile-<uuid>` 容器，目录权限为 `0700`，元数据文件权限为 `0600`。
- 元数据只包含账号 ID、平台 ID、创建时间和 `sensitiveDataStoredIn: os-keychain`。
- 会话 Cookie 使用账号独立的 Keychain reference 保存，不写入 profile 容器。
- 浏览器回退流程优先使用系统浏览器，平台页面由用户自行完成登录；CreatorRelay 不读取系统浏览器主配置目录。
- 同一平台两个账号分别拥有不同 `profile_id` 与 `credential_ref`。一个账号的登出、重登、禁用或删除不会改变另一个账号的凭据引用。

## 5. 平台认证能力清单

M3 只声明已经验证的能力；平台特有 OAuth、发布和数据分析能力在后续 Adapter 阶段独立验证。

| 平台 | 分类 | M3 身份校验方式 | 说明 |
| --- | --- | --- | --- |
| Bilibili | 视频 | 系统浏览器 Cookie + Bilibili `nav` 接口校验 | 返回 `mid`、`uname` 且 `isLogin=true` 才进入 `ACTIVE` |
| 掘金 | 技术社区 | 系统浏览器人工流程 | 官方身份接口尚未验证，不生成 `ACTIVE` 会话 |
| CSDN | 技术社区 | 系统浏览器人工流程 | 同上 |
| 知乎 | 深度图文 | 系统浏览器人工流程 | 同上 |
| 微信公众号 | 深度图文 | 系统浏览器人工流程 | 自动发布能力未声明 |
| YouTube | 视频 | 人工流程；OAuth 未开放 | 尚未配置官方 OAuth 客户端与回调 |
| 抖音 | 视频 | 系统浏览器人工流程 | 官方身份接口尚未验证 |
| 快手 | 视频 | 系统浏览器人工流程 | 官方身份接口尚未验证 |
| 小红书 | 社交与种草 | 系统浏览器人工流程 | 官方身份接口尚未验证 |
| 微博 | 社交与种草 | 系统浏览器人工流程 | 官方身份接口尚未验证 |

内置 WebView 登录在当前构建中标记为未开放。平台不兼容或身份接口未验证时，产品提供明确的人工步骤，不伪装自动登录。

## 6. 删除语义

删除前由后端生成预览，包含：

- 将删除的系统 Keychain 凭据；
- 将删除的独立会话容器路径；
- 将保留的历史作品记录数量和保留说明。

用户必须分别确认凭据删除、会话删除和历史保留范围。确认后：

1. 删除当前账号的 Keychain 凭据；
2. 删除当前账号的会话容器；
3. 删除账号/会话记录；
4. 通过外键把 `platform_works.account_id` 置空，保留运营历史；
5. 写入不含敏感值的 `ACCOUNT_DELETED` 审计记录。

本地主稿、Markdown 正文、素材和其他账号不会被连带删除。

## 7. 验证覆盖

自动化测试覆盖：

- 同平台多账号新增、编辑、禁用与默认账号隔离；
- 身份校验成功才进入 `ACTIVE`，失败不产生虚假 UID/昵称；
- 会话过期后进入 `NEEDS_REAUTH` 并阻止依赖任务；
- SQLite 文件和审计日志不包含测试 Cookie；
- 两个账号 profile/凭据引用隔离，目录权限为 `0700`；
- 系统浏览器回退步骤和登录 URL；
- 删除清理凭据/会话且保留历史作品；
- 仅有 profile 文件时检测失败不会回到 `ACTIVE`；
- Bilibili 身份/过期 fixture、真实 HTTP fixture 和 macOS Keychain 显式 smoke test；
- schema v2 迁移版本与幂等性。

验证命令、结果和未覆盖项见 `progress/verification/M3-verification-2026-11-25.md`。

## 8. 已知限制

- 当前自动身份校验只覆盖 Bilibili；其他平台需要后续 Adapter 按官方授权和接口独立验证。
- 内置 WebView 账号会话尚未启用；当前可验证路径为系统浏览器人工登录。
- 真实 Bilibili 成功登录依赖用户自己的有效 Cookie，自动化验收使用同结构 HTTP fixture 验证成功分支，并使用线上接口验证未登录响应。
- 平台接口或风控策略变化时，账号可能进入 `ERROR` 或 `NEEDS_REAUTH`，需要人工重试。
