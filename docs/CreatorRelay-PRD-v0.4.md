# CreatorRelay 产品需求文档（PRD）

**版本：** V0.4\
**日期：** 2026-11-25\
**状态：** 需求与技术基线草案\
**本版关键变更：** Studio 从 Markdown 写作中心扩展为四类内容创作中心；明确视频、图文、图文文章、纯文章的默认优先级、内容包模型、平台能力边界与阶段归属。\
**产品名称：** CreatorRelay\
**产品定位：** Unified Creator Workspace

------------------------------------------------------------------------

## 1. 产品概述

### 1.1 产品定位

CreatorRelay 是一个本地优先、AI
驱动、支持插件扩展的多平台创作者管理与内容生产桌面工作台。

用户可以在一个桌面应用中统一管理多个内容平台的账号、创作内容、发布任务和运营数据，并通过
AI Agent 与 MCP 工具完成跨平台查询、内容适配和运营操作。

### 1.2 核心理念

-   **Local-first：** 核心业务数据、创作内容和素材优先保存在用户本机。
-   **Platform-agnostic：**
    不绑定单一内容平台，通过标准化适配器接入不同平台。
-   **AI-native：** AI
    不只是聊天入口，而是可以在用户授权下调用工具、执行工作流的生产力能力。
-   **Extensible：** 支持官方适配器和第三方插件，逐步形成平台插件生态。

### 1.3 核心用户

首期面向个人创作者、技术博主和多平台内容运营者；长期扩展至内容工作室和小型团队。

### 1.4 要解决的问题

1.  多个平台需要分别登录，账号和会话难以统一管理。
2.  各平台作品分散，缺乏统一检索和管理入口。
3.  视频、图文、图文文章和纯文章各有不同的编辑与发布结构，却缺少统一的内容工作台。
4.  发布任务、发布结果和失败原因分散，缺乏统一跟踪。
5.  各平台数据统计口径不同，难以统一观察内容表现。
6.  AI 工具与平台账号、内容资产和运营数据之间缺乏可靠连接。

### 1.5 产品目标

  目标       说明
  ---------- --------------------------------------------
  账号统一   多平台、多账号集中管理，支持会话隔离
  内容统一   统一管理主稿、平台稿和已发布作品
  创作提效   提供视频、图文、图文文章、纯文章四类内容编辑和 AI 辅助创作
  分发统一   统一管理多平台适配稿和发布任务
  数据聚合   统一查看账号、作品及历史指标
  AI 执行    通过 MCP 和内部工具执行跨模块任务
  可扩展     通过插件机制增加平台和业务能力

### 1.6 非目标

首期不包含：

-   完整云端 SaaS 服务。
-   团队实时协同编辑。
-   专业级视频剪辑和图像处理；基础视频导入、封面、元数据和发布打包不属于专业剪辑。
-   绕过验证码、登录验证、平台风控或访问限制。
-   保证所有平台的全部数据读取和自动发布能力。
-   未经用户确认的自动发布、删除或公开资料修改。

------------------------------------------------------------------------

## 2. 已确认的产品与技术决策

  维度               已确认方案
  ------------------ ---------------------------------------------
  桌面技术方案       Tauri 2 + React + TypeScript + Rust
  内容存储           内容包 + 文章类 Markdown 正文优先
  元数据与业务数据   SQLite
  AI 接入            BYOK（用户自带 API Key）
  AI 模型            支持多个模型服务商及 OpenAI Compatible 接口
  平台接入           Platform Adapter + MCP
  插件生态           官方适配器 + 第三方适配器
  浏览器形态         内置 WebView + 外部浏览器混合模式
  凭据存储           优先使用操作系统安全凭据存储
  内容形态           视频、图文、图文文章、纯文章
  用户范围           首期个人创作者，长期支持工作室和团队

以下事项仍待确认，不视为已冻结决策：

-   第三方插件运行时的首期语言与隔离机制。
-   Git 版本管理与文件快照的组合方式。
-   自动备份的默认策略和频率。
-   最终的浏览器会话实现及各操作系统兼容性。

------------------------------------------------------------------------

## 3. 产品信息架构

一级导航建议包括：

  模块         职责                                  优先级
  ------------ ------------------------------------- --------
  Dashboard    全平台运营总览、待办和异常提醒        P0
  Studio       四类内容创作、编辑、AI 适配          P0
  Publishing   发布任务、发布计划和发布结果          P0/P1
  Content      跨平台作品聚合和检索                  P0
  Analytics    账号及作品数据分析                    P0
  Assistant    AI 对话、工具调用和任务执行           P1
  Platforms    平台接入、账号和会话管理              P0
  Settings     AI、MCP、插件、存储、安全和备份设置   P0

### 3.1 平台、账号与会话的关系

建议采用三级模型：

``` text
Platform
  └── Accounts
        └── Platform Sessions
```

-   一个 Platform 可以拥有多个 Account。
-   一个 Account 归属于一个 Platform。
-   Account 的浏览器会话、授权凭据和登录状态需要独立管理。
-   平台能力由 Platform Adapter 声明，不应假设所有平台都支持相同功能。

------------------------------------------------------------------------

## 4. 平台支持范围

首期产品规划涉及以下 10 个平台：

  平台         主要内容形态   备注
  ------------ -------------- ------------------------------
  Bilibili     视频、专栏     作品及创作者数据
  抖音         短视频、图文   以实际授权能力为准
  快手         短视频         以实际授权能力为准
  小红书       图文、视频     以实际授权能力为准
  YouTube      视频、频道     海外平台
  微信公众号   图文、视频     本文中的"微信"暂按公众号理解
  知乎         问答、文章     文章及回答
  微博         社交动态       动态及互动数据
  掘金         技术文章       适合验证 Markdown 内容链路
  CSDN         技术博客       适合验证 Markdown 内容链路

**重要说明：**
产品规划支持某个平台，不代表该平台已经提供或允许所有所需接口。每个平台进入开发前，必须单独验证官方
API、授权方式、登录机制、自动化政策和可采集数据范围。对于不支持自动发布的平台，应提供人工发布辅助流程，而不是尝试绕过平台限制。

### 4.1 建议的接入顺序

接入顺序按用户最常用的内容形态排列，同时仍需逐平台验证官方能力：

1.  Bilibili、YouTube：先验证视频内容包、封面、媒体元数据和发布链路。
2.  小红书、抖音、快手：验证轻图文、短视频及多素材顺序模型。
3.  微信公众号、知乎：验证图文文章、长文排版和媒体嵌入差异。
4.  掘金、CSDN：验证纯文章、图文文章、格式适配和发布链路。
5.  微博：验证轻量图文、视频和社交动态模型。

该顺序是工程建议，不代表平台接口可用性已经得到验证。

### 4.2 四类内容形态

CreatorRelay 的 Studio 不以 Markdown 作为所有内容的唯一载体。新建内容时须先选择内容形态；形态决定编辑器、校验规则、平台兼容性和发布的最终内容包。

| 默认顺序 | ContentKind     | 产品名称 | 用户意图                     | 权威内容                                     | MVP 边界                                             |
| -------- | --------------- | -------- | ---------------------------- | -------------------------------------------- | ---------------------------------------------------- |
| 1        | `video`         | 视频     | 主要发布一个视频作品         | 视频文件、封面、标题、简介及可选字幕/章节    | 支持导入、预览、元数据、封面和发布打包；不做专业剪辑 |
| 2        | `image_text`    | 图文     | 主要发布一组有顺序的图片     | 有序图片集、封面、标题、短文案及逐图说明     | 支持排序、封面、逐图文字和发布；不做专业图像处理     |
| 3        | `rich_article`  | 图文文章 | 发布长文字中穿插图片的文章   | Markdown 正文、内联媒体位置、图片说明和封面  | 支持图文混排、预览、平台适配和发布                   |
| 4        | `plain_article` | 纯文章   | 发布只有文字的长文           | Markdown 正文                                | 支持 Markdown 编辑、预览、适配和发布                 |

必须区分以下两类内容，不能合并为一个“图文”类型：

-   **图文（`image_text`）是媒体优先内容。** 图片的顺序和封面是核心，文字通常较短。
-   **图文文章（`rich_article`）是文字优先内容。** 正文结构由 Markdown 决定，图片通过内联引用或位置锚点参与排版。

通用规则：

1.  `kind` 在创建时确定，不能在原内容上静默切换。跨形态转换必须生成新的派生内容，并保留来源 ID 和版本。
2.  平台是否支持某形态必须以 Adapter 的逐形态能力声明和运行验证为准，不能根据平台名称推断。
3.  目标平台不支持当前形态时，该平台/账号不得出现在可直接发布的选择中；可提供明确的“不支持”状态或其他形态的派生建议。
4.  视频和图文不要求存在 Markdown 正文；图文文章和纯文章的正文以 Markdown 为权威来源。
5.  AI 操作、内容检查和发布预览都必须感知 `kind`，不能把文章编辑流程套用到视频或图集。

------------------------------------------------------------------------

## 5. 核心功能需求

## 5.1 Dashboard 总览

**优先级：P0**

### 功能需求

-   展示已接入平台数量和已绑定账号数量。
-   展示最近发布作品数量和待处理发布任务。
-   展示账号登录状态和连接异常。
-   展示近期数据同步任务及失败原因。
-   展示账号粉丝趋势和作品表现趋势。
-   展示最近作品及表现排行。
-   支持按平台、账号和时间范围筛选。

### 数据口径

跨平台粉丝数、播放量、阅读量等指标可能存在定义差异。聚合展示时必须标注指标来源和统计口径。不同平台的指标不能在未经定义的情况下直接相加并解释为统一业务指标。

## 5.2 Studio 内容创作中心

**优先级：P0**

Studio 是 CreatorRelay 的核心模块，不是单一的写作编辑器。它须覆盖视频、图文、图文文章和纯文章四类内容，从创建、编辑、AI 辅助、平台适配到发布准备形成统一工作流。

### 默认新建入口

新建内容的默认顺序如下，界面文案不得只提供“新建文章”：

| 顺序 | 入口     | 适用场景                               | 默认编辑视图                         |
| ---- | -------- | -------------------------------------- | ------------------------------------ |
| 1    | 新建视频 | 发布单个视频作品，文字用于说明和分发   | 视频预览、封面、标题/简介、字幕和章节 |
| 2    | 新建图文 | 发布一组有顺序的图片和短文案           | 图集排序、封面选择、逐图说明、短文案 |
| 3    | 新建图文文章 | 发布正文中穿插图片的长文章         | Markdown 编辑、内联图片、预览        |
| 4    | 新建纯文章 | 发布只有文字的长文章                   | Markdown 编辑和预览                  |

所有入口进入同一个内容库；用户可按形态、平台、账号、标签、状态和时间筛选。

### 页面结构

建议采用三栏式工作区，并按内容形态替换中栏编辑器：

-   左栏：内容列表、文件夹、标签、素材库和内容形态筛选。
-   中栏：形态专属编辑区。视频为媒体与元数据编辑，图文为图集编排，图文文章和纯文章为 Markdown 编辑/预览。
-   右栏：AI 助手、形态化内容检查和平台适配。

### 通用创作能力

-   自动保存、保存状态提示、保存失败保护和内容版本历史。
-   标题、摘要、标签、封面、素材引用和状态管理。
-   全文搜索和标签/形态筛选。
-   从本地导入内容，以及按形态导出可继续编辑的内容包。
-   AI 预览、应用、丢弃和撤销，不自动覆盖用户内容。
-   内容检查：必填字段、素材完整性、格式/时长/尺寸/数量和正文规则。
-   从主内容生成一个或多个独立平台稿，并从平台稿创建发布任务。
-   快捷键和 Slash Commands 适用于 Markdown 文章；视频和图文提供对应的键盘操作。

### 形态专属编辑能力

| ContentKind     | 必备编辑能力                                                                         | 关键校验                                                         |
| --------------- | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| `video`         | 导入/替换视频、封面、标题、简介、标签、可选字幕/章节、视频预览和来源信息           | 文件可读、时长/尺寸/格式可识别、封面存在、标题和平台必填项完整   |
| `image_text`    | 图片批量导入、拖拽排序、设封面、删除、替换、逐图说明、标题和短文案编辑、最终卡片预览 | 图片可读、顺序稳定、至少一张图、封面属于图集、数量/尺寸符合平台 |
| `rich_article`  | Markdown 正文、内联图片/视频、图片说明、标题、摘要、标签、封面和分栏预览             | 正文结构、媒体引用可解析、图片顺序和说明完整、平台长度要求       |
| `plain_article` | Markdown 正文、标题、摘要、标签和阅读预览                                            | 正文非空、Markdown 可渲染、平台长度和格式要求                    |

视频与图文的编辑器不得用空白 Markdown 正文冒充内容主体；文章类形态不得把正文只保存在 SQLite。

### AI 创作能力

AI 操作须按内容形态提供不同能力和输入范围：

-   视频：标题、简介、标签、章节标题和基于用户提供文本的摘要。
-   图文：标题、短文案、标签、封面建议、逐图说明和图片顺序建议。
-   图文文章：标题、摘要、结构优化、段落润色、图片位置/图片说明建议。
-   纯文章：标题、摘要、结构优化、段落润色、语气转换和内容检查。
-   所有形态：用户选择发送范围；发送前展示目标模型服务商和实际内容/素材元数据；未确认不得上传。

不在本阶段承诺 AI 视频生成、自动剪辑、自动换图或专业图像处理。

### 多平台适配能力

-   选择目标平台和账号；候选目标须支持当前 `ContentKind`。
-   基于主内容生成平台稿，保留形态、素材顺序、封面、正文和平台特有字段。
-   对比主内容与平台稿；图文比较图片顺序/说明，视频比较媒体和元数据，文章比较正文结构。
-   独立编辑平台稿；修改一个平台稿不改变主内容或其他平台稿。
-   检查平台格式、长度、数量、尺寸、时长和素材要求；未验证规则不得写成平台事实。
-   保存适配结果并从平台稿创建发布任务。
-   目标平台不支持当前形态时，明确显示原因；如用户选择其他形态，必须创建新的派生内容，不能静默改变原内容。

### 内容数据模型

V0.4 起以 **Content Draft** 统一表示本地主内容。历史 `Master Draft` 可迁移为 Content Draft，但不再默认等同于 Markdown 文章。

内容至少分为三层：

1.  **Content Draft：** 本地主内容，含 `kind`、元数据和形态专属内容。
2.  **Platform Draft：** 为特定平台生成的适配内容，保留来源 Content Draft ID、生成版本和形态。
3.  **Published Content：** 已经存在于外部平台的作品记录。

```ts
type ContentKind = "video" | "image_text" | "rich_article" | "plain_article";

interface MediaRef {
  assetId: string;
  order: number;
  role: "video" | "cover" | "image" | "attachment";
  alt?: string;
  caption?: string;
}

interface ContentDraft {
  id: string;
  kind: ContentKind;
  title: string;
  summary?: string;
  tags: string[];
  coverAssetId?: string;
  mediaRefs: MediaRef[];
  bodyMarkdownRef?: string; // rich_article / plain_article
  status: "draft" | "ready" | "archived";
}
```

`bodyMarkdownRef` 指向正文文件，不是 SQLite 中的正文副本。视频和图文可以不设置该字段。

### 主内容与平台稿关联

-   每个平台稿必须关联一个 Content Draft，并继承其 `ContentKind`。
-   平台稿需要记录生成时的主内容版本或内容哈希，包括正文和素材顺序。
-   主内容更新后，不得自动覆盖用户已经手动修改的平台稿。
-   打开旧平台稿时提示主内容已变化，并允许用户比较、保留或重新适配。
-   平台稿可包含平台特有字段，但不得丢失原始素材引用、顺序和来源信息。

## 5.3 本地内容工作区与文件组织

**优先级：P0**

工作区采用“内容包 + 媒体资产”的本地优先策略，而不是把所有内容都塞进 Markdown 文件：

-   每个内容包包含统一清单，记录 ID、`kind`、标题、标签、状态、封面和素材顺序。
-   `rich_article` 与 `plain_article` 使用 Markdown 文件作为正文权威来源。
-   `video` 与 `image_text` 的权威内容由内容清单、媒体引用和媒体文件共同构成，不要求存在 Markdown 正文。
-   SQLite 保存索引、业务元数据、搜索字段和关联关系，不保存会与文件冲突的正文权威副本。
-   媒体文件保存在内容包或工作区素材目录中；所有引用使用可迁移的相对路径或稳定资源 ID。
-   文件外部发生修改时，应用需要检测并更新索引。
-   文件丢失或索引损坏时，应提供重建索引能力，且不得删除仍可读取的用户文件。

### 建议目录结构

``` text
CreatorRelay/
├── workspace/
│   ├── contents/
│   │   ├── video-001/
│   │   │   ├── content.json
│   │   │   ├── media/
│   │   │   │   ├── video.mp4
│   │   │   │   └── cover.jpg
│   │   │   └── platforms/
│   │   │       └── bilibili.json
│   │   ├── image-text-001/
│   │   │   ├── content.json
│   │   │   ├── media/
│   │   │   │   ├── 01-cover.jpg
│   │   │   │   └── 02-detail.jpg
│   │   │   └── platforms/
│   │   │       └── xiaohongshu.json
│   │   ├── rich-article-001/
│   │   │   ├── content.json
│   │   │   ├── body.md
│   │   │   ├── media/
│   │   │   │   ├── cover.png
│   │   │   │   └── architecture.png
│   │   │   └── platforms/
│   │   │       └── wechat.md
│   │   └── plain-article-001/
│   │       ├── content.json
│   │       ├── body.md
│   │       └── platforms/
│   │           └── juejin.md
│   ├── assets/
│   ├── templates/
│   └── exports/
├── data/
│   └── creator-relay.db
├── profiles/
│   └── browser-sessions/
└── logs/
```

具体文件拆分可由 TDD 冻结，但必须满足“清单负责结构与元数据、Markdown 负责文章正文、媒体文件负责媒体内容”的单一权威原则。

### 内容清单示例

``` json
{
  "schemaVersion": 1,
  "id": "content_image_001",
  "kind": "image_text",
  "title": "一次真实的桌面工作流",
  "status": "draft",
  "coverAssetId": "asset_01",
  "media": [
    {
      "assetId": "asset_01",
      "role": "image",
      "order": 1,
      "alt": "工作区总览"
    },
    {
      "assetId": "asset_02",
      "role": "image",
      "order": 2,
      "alt": "发布任务详情"
    }
  ]
}
```

### 兼容与迁移

-   现有 Markdown 文件可迁移为 `plain_article` 或 `rich_article`；迁移前须保留未知 Frontmatter 字段。
-   迁移完成后，统一清单成为元数据、形态和媒体顺序的权威来源；正文仍由 Markdown 文件权威保存。
-   不允许同一字段同时由 Frontmatter 和统一清单双向写入而没有明确优先级。
-   旧数据中的图片/视频链接应迁移为可校验的素材引用；无法解析时保留原文本并提示用户修复，不得静默删除。

## 5.4 Publishing 发布中心

**优先级：P0/P1**

### 功能需求

-   从任意 Content Draft 或 Platform Draft 创建发布任务，保留其 `ContentKind`。
-   只展示支持当前内容形态的目标平台和具体账号。
-   发布前按形态预览最终内容：视频检查文件和封面，图文检查图片顺序/封面/文案，文章检查渲染后的正文和媒体位置。
-   支持立即发布。
-   对平台支持的情况提供定时发布。
-   展示任务状态、执行时间、内容形态和结果。
-   失败后允许用户检查原因并重试。
-   对不支持自动发布的平台提供人工发布辅助；视频和图文的辅助流程须能导出完整发布包，而不只是复制文字。

### 发布输入

Core 向 Adapter 提供经过校验的 `PublishInput`，至少包含：

```ts
interface PublishInput {
  kind: ContentKind;
  title: string;
  summary?: string;
  bodyMarkdown?: string;
  coverAssetId?: string;
  mediaRefs: MediaRef[];
  tags: string[];
  platformOptions?: Record<string, unknown>;
}
```

普通文章可以不设置媒体；视频必须包含视频文件和封面（如平台要求）；图文必须保留图片顺序和封面；图文文章必须保留正文中的媒体位置。

### 发布任务状态

``` text
CREATED
   ↓
VALIDATING
   ↓
PENDING
   ├── SCHEDULED
   └── PUBLISHING
          ├── SUCCESS
          ├── FAILED
          ├── CANCELLED
          └── NEEDS_MANUAL_ACTION
```

### 业务规则

-   多平台发布是多个独立任务，不是一个分布式事务。
-   一个平台失败不能影响其他平台任务。
-   发布前必须展示目标平台、目标账号、内容形态和最终内容包。
-   平台不支持当前内容形态时不得伪装为可发布；应说明限制并给出人工路径或经用户确认的跨形态派生建议。
-   不允许对结果不确定的发布请求直接无限重试。
-   请求超时后，应先查询平台作品或要求人工核对，避免重复发布。
-   需要保存外部平台内容 ID 和原始链接（若可获取），并保留平台原始作品类型。

## 5.5 Content 内容中心

**优先级：P0**

统一展示已同步的外部平台作品。

### 筛选维度

-   平台。
-   账号。
-   内容形态：视频、图文、图文文章、纯文章。
-   平台原始内容类型。
-   发布时间。
-   发布状态。
-   关键词。

### 作品详情

-   标题、摘要和封面。
-   统一内容形态 `ContentKind` 与平台原始内容类型。
-   视频可展示媒体信息；图文可展示图片顺序；文章可展示正文摘要。
-   来源平台。
-   所属账号。
-   外部平台作品 ID。
-   原始链接。
-   发布时间。
-   阅读、播放、点赞、评论、收藏和分享等指标（以平台实际提供为准）。
-   历史指标变化。

## 5.6 Analytics 数据分析

**优先级：P0**

### 分析维度

1.  账号维度：粉丝变化、账号表现。
2.  内容维度：作品互动、阅读和播放表现。
3.  平台维度：平台表现对比。
4.  时间维度：日、周、月趋势。

### 指标标准化

建议维护统一指标字典，同时保留平台原始指标：

``` ts
type MetricType =
  | 'followers'
  | 'views'
  | 'likes'
  | 'comments'
  | 'shares'
  | 'favorites'
  | 'reads';

interface MetricSnapshot {
  accountId: string;
  contentId?: string;
  metricType: MetricType;
  value: number;
  capturedAt: string;
  sourceMetricName?: string;
  unit?: string;
  definition?: string;
}
```

不同平台的指标定义、单位、采集时间和统计范围必须保留，避免错误比较。

## 5.7 Assistant AI 助手

**优先级：P1**

AI 助手通过模型服务商和工具注册中心，调用 CreatorRelay 内部工具及平台
MCP 工具。

### 典型场景

-   查询多个平台最近七天的数据。
-   找出表现最好的作品。
-   生成标题和摘要。
-   按内容形态创建或编辑视频、图文、图文文章和纯文章。
-   将主内容适配为不同平台版本。
-   查询账号登录和发布状态。
-   创建发布任务。
-   对比不同平台的内容表现。

### 内部工具示例

-   `studio_search_contents`
-   `studio_create_content`
-   `studio_update_content`
-   `studio_adapt_content`
-   `publishing_create_task`
-   `analytics_compare_platforms`

AI
工具执行需要显示模型、工具名称、目标账号、参数摘要、执行结果和失败原因。内容创建/更新工具必须显式接收 `kind`，并按对应 Schema 校验正文、媒体顺序和必填元数据。

## 5.8 Platforms 平台管理

**优先级：P0**

### 平台接入流程

``` text
选择平台
   ↓
创建平台账号
   ↓
选择登录方式
   ├── 内置浏览器
   ├── 外部浏览器
   └── 官方 OAuth（如支持）
   ↓
完成登录和身份验证
   ↓
验证账号信息
   ↓
保存账号配置及凭据引用
   ↓
初始化平台适配器
   ↓
检测平台能力
   ↓
接入成功
```

### 账号管理功能

-   一个平台注册多个账号。
-   账号别名。
-   头像、昵称和平台 UID。
-   登录状态检测。
-   手动重新登录。
-   启用和禁用账号。
-   删除账号及关联会话。
-   默认操作账号设置。
-   查看最近同步时间和错误状态。

### 账号状态

``` text
UNAUTHORIZED
    ↓
AUTHENTICATING
    ↓
ACTIVE
    ├── EXPIRED
    ├── NEEDS_REAUTH
    ├── ERROR
    └── DISABLED
```

登录状态应以实际检测结果为准。不能仅凭本地 Cookie
文件存在就认定账号仍然有效。

## 5.9 Settings 系统设置

**优先级：P0**

-   工作区路径。
-   数据库和素材存储路径。
-   数据备份与恢复。
-   AI 模型供应商配置。
-   MCP Server 管理。
-   插件管理和权限。
-   日志查看。
-   应用更新。
-   安全设置。
-   中英文语言预留。

------------------------------------------------------------------------

## 6. 技术架构

### 6.1 总体架构

采用 Rust Core + React UI + Platform Plugins + MCP Runtime 的分层架构。

``` text
┌─────────────────────────────────────────────┐
│              Presentation Layer             │
│          React + TypeScript                 │
│ Dashboard / Studio / Publishing / Analytics │
└──────────────────────┬──────────────────────┘
                       │ Tauri IPC
┌──────────────────────▼──────────────────────┐
│              Application Core               │
│                 Rust                        │
│ Account / Content / Publishing / Analytics  │
│ Plugin / Task / Credential / Workspace      │
└─────────────┬───────────────┬───────────────┘
              │               │
      ┌───────▼──────┐  ┌─────▼────────────┐
      │ Data Layer   │  │ Security Layer  │
      │ SQLite       │  │ OS Keychain     │
      │ Markdown     │  │ Credential Ref  │
      │ Local Assets │  │ Session Access  │
      └──────────────┘  └──────────────────┘
              │
      ┌───────▼─────────────────────────────┐
      │ AI Runtime / MCP Host / Plugin Mgr │
      └──────────────────┬──────────────────┘
                         │
      ┌──────────────────▼──────────────────┐
      │ Platform Adapter / MCP Servers      │
      │ Bilibili / Douyin / Juejin / CSDN…  │
      └─────────────────────────────────────┘
```

### 6.2 技术栈建议

  层级           技术建议                  职责
  -------------- ------------------------- ---------------------
  Desktop        Tauri 2                   桌面应用容器
  Frontend       React + TypeScript        页面和交互
  UI State       Zustand                   客户端状态
  Server State   TanStack Query            异步数据和缓存
  IPC            Tauri Commands / Events   UI 与 Rust 通信
  Core           Rust                      核心业务逻辑
  Database       SQLite                    业务元数据
  DB Access      SQLx 或 rusqlite          数据访问
  File           Rust 文件系统能力         Markdown 和素材管理
  Security       OS Keychain               API Key、Token 等
  AI             Provider Adapter          模型供应商接入
  MCP            MCP Client / Host         工具发现与调用
  Plugins        独立进程 Runtime          平台扩展

### 6.3 分层约束

-   React 不直接访问 SQLite。
-   React 不直接访问任意本地文件路径。
-   React 不直接读取凭据。
-   Tauri Commands 只负责 IPC 入口和参数校验，不承载全部业务逻辑。
-   业务逻辑放在独立 Rust Service。
-   平台适配器不能绕过统一账号、凭据和任务管理服务。
-   MCP Server 不直接操作 SQLite，应通过受控 Core API 访问业务能力。

### 6.4 建议项目目录

``` text
creator-relay/
├── apps/
│   └── desktop/
│       ├── src/
│       │   ├── components/
│       │   ├── pages/
│       │   ├── layouts/
│       │   ├── hooks/
│       │   ├── stores/
│       │   ├── services/
│       │   └── types/
│       └── src-tauri/
│           ├── src/
│           │   ├── commands/
│           │   ├── services/
│           │   ├── database/
│           │   ├── filesystem/
│           │   ├── security/
│           │   ├── browser/
│           │   ├── plugins/
│           │   └── main.rs
│           └── Cargo.toml
├── packages/
│   ├── shared-types/
│   ├── platform-contracts/
│   ├── mcp-contracts/
│   ├── ai-sdk/
│   └── ui/
├── plugins/
│   ├── official/
│   └── community/
├── crates/
│   ├── core/
│   ├── database/
│   ├── storage/
│   ├── credential/
│   ├── plugin-manager/
│   ├── mcp-host/
│   └── task-engine/
├── docs/
├── package.json
├── pnpm-workspace.yaml
└── Cargo.toml
```

以上为逻辑组织建议，MVP 阶段可以按实际复杂度合并部分模块。

------------------------------------------------------------------------

## 7. Platform Adapter 与 MCP 规范

### 7.1 设计原则

每个平台实现一套标准化 Platform Adapter。Adapter 负责平台差异，Core
负责账号、凭据、任务、数据和权限管理，MCP 负责将能力暴露给 AI Agent
或其他 MCP Client。

不应强迫每个平台实现所有能力。应使用能力接口组合，允许平台只实现自己实际支持的能力。

### 7.2 能力分类

-   Account：账号信息和登录状态。
-   Data：账号数据和作品数据读取。
-   Content：作品列表和详情读取。
-   Publish：内容发布、草稿和定时发布。
-   Analytics：作品及账号指标。
-   AI：平台相关的内容处理工具或上下文能力。

### 7.3 Adapter 接口概念

``` ts
interface PlatformAdapter {
  getManifest(): PlatformManifest;

  getAccountInfo(
    accountId: string
  ): Promise<AccountInfo>;

  checkLoginStatus(
    accountId: string
  ): Promise<LoginStatus>;
}

interface AccountDataCapability {
  getAccountMetrics(
    accountId: string
  ): Promise<AccountMetrics>;
}

interface ContentReadCapability {
  getContentList(
    accountId: string,
    options?: ContentQuery
  ): Promise<Content[]>;

  getContentDetail(
    accountId: string,
    contentId: string
  ): Promise<ContentDetail>;
}

interface ContentPublishCapability {
  publishContent(
    input: PublishInput
  ): Promise<PublishResult>;
}
```

实际项目中还需定义错误类型、分页、速率限制、取消、超时、幂等和数据来源信息。

### 7.4 MCP 工具命名

建议统一使用平台无关的工具名称，并在参数中显式指定平台和账号：

  工具                    用途
  ----------------------- --------------
  `account_get_info`      获取账号信息
  `account_get_metrics`   获取账号指标
  `content_list`          获取作品列表
  `content_get_detail`    获取作品详情
  `content_publish`       发布内容
  `content_update`        更新内容
  `analytics_get_trend`   获取数据趋势
  `analytics_compare`     跨平台对比

平台特有参数通过经过 Schema 校验的扩展字段提供。

### 7.5 MCP 运行方式

建议首期采用本地独立进程和 stdio 通信：

-   桌面端负责 MCP Server 的启动、停止和生命周期管理。
-   每个平台插件可提供一个 MCP Server。
-   支持健康检查、崩溃重启和超时控制。
-   记录工具调用日志。
-   工具调用通过权限检查。
-   MCP Server 不直接持有数据库连接和完整凭据访问权限。

### 7.6 平台能力声明

``` ts
type CapabilityStatus =
  | 'supported'
  | 'unsupported'
  | 'restricted'
  | 'experimental'
  | 'unavailable';

interface PlatformManifest {
  id: string;
  name: string;
  version: string;

  capabilities: {
    accountInfo: CapabilityStatus;
    accountMetrics: CapabilityStatus;
    contentRead: CapabilityStatus;
    contentPublish: CapabilityStatus;
    contentSchedule: CapabilityStatus;
    contentAnalytics: CapabilityStatus;
  };

  authentication: {
    type: 'oauth' | 'browser' | 'manual';
  };
}
```

能力状态必须基于实际实现和运行检测，不得仅依据插件声明就展示为可用。

------------------------------------------------------------------------

## 8. AI 与 Agent 架构

### 8.1 BYOK 模型接入

建议支持：

-   OpenAI。
-   Anthropic。
-   Google Gemini。
-   DeepSeek。
-   通义千问。
-   OpenAI Compatible 自定义端点。
-   后续可扩展本地模型端点。

### 8.2 模型配置字段

-   Provider 名称。
-   API Base URL。
-   API Key 凭据引用。
-   模型名称。
-   模型能力。
-   上下文长度（如可获取）。
-   默认模型标记。
-   连接测试状态。

API Key 存放在系统安全凭据存储中，SQLite 只保存配置和凭据引用。

### 8.3 AI 架构分层

``` text
AI Layer
├── Model Provider
│   ├── OpenAI
│   ├── Anthropic
│   ├── DeepSeek
│   └── Custom Endpoint
├── Agent Runtime
│   ├── Conversation
│   ├── Planning
│   ├── Tool Calling
│   └── Execution
└── Tool Registry
    ├── Platform MCP
    ├── Content Tools
    ├── Analytics Tools
    └── System Tools
```

### 8.4 工具权限等级

  类型              示例                       权限策略
  ----------------- -------------------------- ------------------
  Read              查询账号、作品和数据       用户配置后可允许
  Write             创建草稿、修改本地内容     需要明确授权策略
  External Action   发布、删除、修改公开资料   每次执行前确认

批量发布必须在执行前展示完整目标账号、平台稿和发布计划。

### 8.5 数据隐私

BYOK
不等于数据完全留在本地。使用云端模型时，发送给模型服务商的数据可能包含文章正文、Prompt
和上下文。

必须：

-   明确告知发送的数据范围。
-   允许用户选择发送内容。
-   提供敏感信息保护策略。
-   支持配置本地模型端点。
-   不将 API Key 写入日志。
-   不在未授权时将整个工作区发送给模型。

------------------------------------------------------------------------

## 9. 插件生态

### 9.1 插件类型

1.  **Platform Plugin：** 接入外部内容平台。
2.  **AI Plugin：** 提供 AI 工具、工作流和 Prompt 模板。
3.  **Workflow / Utility Plugin：** 提供内容处理和自动化能力。

首期优先建设 Platform Plugin 规范。

### 9.2 Manifest 示例

``` json
{
  "id": "com.creatorrelay.juejin",
  "name": "Juejin",
  "version": "1.0.0",
  "description": "Juejin platform integration",
  "type": "platform",
  "runtime": "node",
  "entry": "dist/index.js",
  "mcp": {
    "enabled": true,
    "transport": "stdio"
  },
  "capabilities": [
    "account.read",
    "content.read",
    "content.publish",
    "analytics.read"
  ],
  "permissions": [
    "network",
    "account.credentials"
  ]
}
```

Manifest 是能力和权限申请声明，不代表插件自动获得对应权限。

### 9.3 插件安全要求

-   官方插件可以随应用打包。
-   第三方插件独立进程运行。
-   安装时校验来源、版本和签名（如有）。
-   插件权限由用户明确授权。
-   默认禁止任意访问整个本地工作区。
-   凭据通过受控 Credential Service 按账号授权。
-   插件网络访问应受权限策略约束。
-   插件崩溃不能影响 Core。
-   支持禁用、卸载和版本回退。

**安全说明：** 独立 Node.js
进程不等于安全沙箱。若要支持不受信任的第三方代码，需要进一步采用操作系统权限隔离或其他强隔离机制。首期不应直接开放任意代码插件市场。

### 9.4 插件生命周期

``` text
DISCOVERED
    ↓
INSTALLED
    ↓
PERMISSION_REVIEW
    ↓
AUTHORIZED
    ↓
ENABLED
    ├── DISABLED
    ├── ERROR
    └── UNINSTALLED
```

------------------------------------------------------------------------

## 10. 本地数据与安全设计

### 10.1 数据职责

  存储               内容
  ------------------ ----------------------------------
  Markdown           主稿、平台稿和正文
  SQLite             平台、账号、索引、任务、指标快照
  Local Assets       图片、视频、封面和附件
  OS Keychain        密码、Token、API Key 等敏感凭据
  Browser Profiles   独立浏览器会话数据

### 10.2 凭据安全

-   不在 SQLite 中保存明文密码。
-   Cookie、Token 和 API Key 同样属于敏感凭据。
-   SQLite 只保存凭据引用和非敏感元数据。
-   凭据访问必须经过统一服务。
-   删除账号时明确处理对应凭据和会话。
-   日志不得包含密码、完整 Cookie 或 Token。
-   备份文件中的敏感数据应加密。

### 10.3 浏览器会话隔离

建议按平台和账号隔离会话：

``` text
browser-profiles/
├── bilibili/
│   ├── account-001/
│   └── account-002/
├── xiaohongshu/
│   └── account-003/
└── douyin/
    ├── account-004/
    └── account-005/
```

多个账号不能共用同一个 Cookie 容器。部分平台可能不支持内嵌 WebView
登录，因此必须保留外部浏览器回退路径。

浏览器会话目录本身也包含敏感信息，应纳入访问控制、备份加密和删除策略。

------------------------------------------------------------------------

## 11. 核心数据库模型

### 11.1 核心实体

-   `workspaces`
-   `platforms`
-   `accounts`
-   `platform_sessions`
-   `master_drafts`
-   `platform_drafts`
-   `contents`
-   `publish_tasks`
-   `sync_tasks`
-   `account_metrics`
-   `content_metrics`
-   `plugins`
-   `plugin_permissions`
-   `mcp_servers`
-   `ai_providers`
-   `ai_models`
-   `ai_conversations`
-   `ai_messages`
-   `tool_invocations`
-   `media_assets`
-   `article_versions`
-   `operation_logs`

### 11.2 主要关系

``` text
WORKSPACES
  ├── MASTER_DRAFTS
  │      └── PLATFORM_DRAFTS
  ├── ACCOUNTS
  │      ├── ACCOUNT_METRICS
  │      ├── CONTENTS
  │      │      └── CONTENT_METRICS
  │      └── PUBLISH_TASKS
  └── MEDIA_ASSETS

PLATFORMS
  └── ACCOUNTS

PLUGINS
  ├── PLUGIN_PERMISSIONS
  ├── MCP_SERVERS
  └── PLATFORMS

AI_PROVIDERS
  └── AI_MODELS
```

### 11.3 关键数据规则

-   主稿正文不只存 SQLite，正文权威来源是 Markdown 文件。
-   平台稿单独保存并关联主稿。
-   外部平台作品与本地稿件通过平台 ID、账号 ID 和关联记录连接。
-   指标快照保留采集时间和来源。
-   发布任务记录每个平台、每个账号的独立状态。
-   账号删除与历史作品数据删除应分开处理，避免意外丢失运营历史。
-   数据库使用迁移机制，不能依赖启动时随意修改表结构。

------------------------------------------------------------------------

## 12. 关键业务流程

### 12.1 多平台发布

``` text
创建 Master Draft
      ↓
编辑文章
      ↓
选择目标平台和账号
      ↓
AI 平台适配
      ↓
生成 Platform Draft
      ↓
用户检查和修改
      ↓
确认发布
      ↓
创建独立发布任务
      ↓
调用 Platform Adapter
      ↓
保存每个平台结果
      ↓
同步作品和数据
```

### 12.2 数据同步

``` text
定时或手动触发同步
      ↓
获取已绑定账号
      ↓
检查登录状态
      ↓
有效？ ── 否 ──> 标记需要重新授权
      │
      是
      ↓
调用平台 Data Adapter
      ↓
获取原始数据
      ↓
标准化并保留原始口径
      ↓
保存数据快照
      ↓
更新 Dashboard 和 Analytics
```

### 12.3 异常处理

-   平台登录失效：暂停该账号相关任务并提示重新登录。
-   插件崩溃：记录错误并允许重启，不影响其他插件。
-   MCP 超时：终止或取消当前调用，记录状态。
-   发布结果不确定：先查询外部平台或请求人工核对。
-   文件被外部修改：检测差异并更新索引，不覆盖用户文件。
-   数据库索引损坏：允许从工作区文件重建可恢复的数据。

------------------------------------------------------------------------

## 13. 非功能性需求

  领域         需求
  ------------ --------------------------------------
  本地优先     核心内容和业务数据默认存储本机
  数据安全     敏感凭据使用系统安全存储
  文件安全     文件操作限制在用户授权工作区
  插件安全     第三方插件独立运行并实施权限控制
  AI 隐私      调用云端模型前明确数据发送范围
  操作安全     发布、删除等外部操作需要确认
  稳定性       插件崩溃不影响桌面 Core
  数据一致性   文件与数据库索引可恢复
  备份恢复     支持工作区和业务数据备份
  可观测性     MCP、插件、同步和发布任务均有日志
  可维护性     数据库迁移、插件版本和接口版本可管理

------------------------------------------------------------------------

## 14. MVP 范围与验收标准

### 14.1 MVP 必须实现

-   Tauri 2 桌面应用和基础导航。
-   Rust Core 与 React IPC。
-   SQLite 数据库及迁移机制。
-   Markdown 工作区和素材管理。
-   本地账号管理和系统凭据管理。
-   内置及外部浏览器访问。
-   Platform Adapter 注册机制。
-   至少两个经过实际验证的平台适配器。
-   Markdown 内容工作室。
-   BYOK 和多模型配置。
-   主稿到平台稿的 AI 适配。
-   发布任务和结果管理。
-   统一作品列表。
-   基础数据分析。
-   MCP Host 和平台工具调用。

### 14.2 MVP 暂缓

-   完整第三方插件市场。
-   团队实时协作。
-   云端同步。
-   专业视频剪辑。
-   复杂多 Agent 编排。
-   所有平台的全部自动发布能力。

### 14.3 端到端验收场景

用户能够：

1.  在 CreatorRelay 中添加多个平台账号。
2.  重启应用后恢复仍然有效的登录会话。
3.  创建并编辑一篇本地 Markdown 主稿。
4.  选择两个平台生成独立平台稿。
5.  手动调整平台稿。
6.  对支持的目标平台创建发布任务；不支持自动发布的平台进入人工发布流程。
7.  查看每个平台独立的发布结果。
8.  同步已发布作品和可用指标。
9.  通过 AI 助手调用 MCP 工具查询账号和作品数据。

------------------------------------------------------------------------

## 15. 开发里程碑

  阶段   核心任务       交付物
  ------ -------------- ------------------------------
  M1     桌面基础设施   Tauri、Rust Core、SQLite
  M2     本地工作区     Markdown、素材、搜索
  M3     账号体系       平台注册、登录、凭据
  M4     插件体系       Manifest、Adapter Runtime
  M5     内容工作室     编辑器、AI 创作、平台适配
  M6     发布与同步     发布任务、作品同步、指标快照
  M7     MCP 与 Agent   工具调用、跨模块工作流
  M8     生态扩展       更多平台和第三方插件

每个里程碑应包含功能实现、单元测试、集成测试、数据迁移测试、异常恢复测试和开发文档。

------------------------------------------------------------------------

## 16. 待确认的架构决策

以下选项尚未冻结，需在正式 TDD 和插件规范中确定。

### 16.1 第三方插件运行时

-   Node.js + TypeScript 独立进程。
-   Rust 插件。
-   多语言 Runtime。

建议：首期优先 Node.js + TypeScript，后续扩展 Rust 等 Runtime。

### 16.2 Markdown 版本管理

-   内置 Git。
-   自行实现文件快照。
-   Git + 自动快照。

建议：Git
负责可追溯版本历史，自动快照负责编辑过程中的意外恢复；但需控制实现复杂度。

### 16.3 本地备份

-   手动导出。
-   自动定期备份。
-   手动 + 自动备份。

建议：支持手动和自动备份，并提供备份加密、恢复预览和版本保留策略。

### 16.4 浏览器会话

已确认采用混合模式，但具体实现仍需工程验证：

-   优先使用 Tauri WebView。
-   平台不兼容时回退系统浏览器。
-   为平台和账号隔离会话。
-   不将浏览器会话文件视为普通无敏感数据。

------------------------------------------------------------------------

## 17. 产品长期竞争力

CreatorRelay 的长期价值不在于 MCP Server 的数量，而在于建立统一的
Creator Platform Runtime。

核心能力包括：

1.  **统一内容模型：** 让不同平台的作品能够统一管理、检索和分析。
2.  **平台适配体系：** 降低新增平台的开发和维护成本。
3.  **本地 AI 执行环境：** 让 AI
    能够在用户授权下查询、创作、发布和分析。
4.  **插件生态：** 让第三方开发者可以扩展平台和工作流能力。

建议将 CreatorRelay 的核心资产抽象为：

  核心资产       说明
  -------------- --------------------------------
  Accounts       创作者账号和授权关系
  Contents       主稿、平台稿和外部作品
  Analytics      运营指标和历史快照
  Capabilities   平台适配器、MCP 工具和 AI 能力

这些资产由 CreatorRelay Core 统一管理，UI、MCP 和 Agent
通过受控接口访问。

------------------------------------------------------------------------

## 18. 后续开发文档拆分

正式进入工程实施前，建议将本文档拆分为三个交付物：

### PRD.md

-   产品定位。
-   用户场景。
-   页面结构。
-   功能需求。
-   业务流程。
-   状态机。
-   验收标准。

### TDD.md

-   技术架构。
-   模块边界。
-   Rust Core 设计。
-   React 与 Tauri IPC。
-   SQLite ER 图和迁移。
-   文件系统和凭据管理。
-   AI Runtime 与 MCP Host。
-   测试和异常恢复。

### PLUGIN_SPEC.md

-   Platform Adapter 接口。
-   Manifest Schema。
-   MCP 工具规范。
-   插件生命周期。
-   权限模型。
-   Runtime 协议。
-   插件测试和兼容性要求。

------------------------------------------------------------------------

**文档说明：** 本文档是 CreatorRelay V0.3
产品需求与技术架构基线草案。已明确的产品定位和技术选型作为当前基线；平台接口可用性、第三方插件隔离、浏览器会话和部分架构细节仍需在工程验证阶段确认。文档不将未经验证的平台能力描述为已实现能力。
