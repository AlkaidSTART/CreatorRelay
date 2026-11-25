import type { Document, WorkspaceInfo } from "../stores/appStore";

export const sampleContent = `---
id: article-wasm-edge
title: 在边缘运行 WebAssembly：一次轻量化实践
created_at: 2026-11-24T10:30:00+08:00
updated_at: 2026-11-25T09:41:00+08:00
tags:
  - WebAssembly
  - 边缘计算
status: draft
---

# 在边缘运行 WebAssembly：一次轻量化实践

当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘，而是如何在有限的资源里，运行足够**安全、轻量且可预测**的代码。

最近我把一个数据转换任务迁移到了 WebAssembly 运行时。它并没有让系统架构变得更复杂，反而让部署边界清晰了不少。

## 为什么选择 WebAssembly

对于运行用户提交或团队共享的扩展逻辑，传统容器通常不是最合适的隔离单位。WebAssembly 提供了更细颗粒度的执行边界：

- 启动开销小，可以按请求创建实例
- 默认不具备宿主机访问能力
- 使用 WASI 显式开放文件和网络权限
- 同一份模块可以运行在不同语言的宿主中

\`\`\`rust
let engine = Engine::default();
let module = Module::from_file(&engine, "transform.wasm")?;
let mut store = Store::new(&engine, context);
let instance = linker.instantiate(&mut store, &module)?;
\`\`\`

这段代码只是最小入口。生产环境中，资源限制、超时、日志采样和能力白名单同样重要。

## 设计时的几个取舍

我们先将输入输出定义成稳定的 JSON Schema，再把运行时当成一个可替换的执行器。这样可以让本地调试、单元测试和边缘部署保持一致。

> 沙箱边界应该由宿主明确定义，而不是依赖插件作者自觉。

另一个值得注意的细节是**可观测性**。每次调用至少要记录模块摘要、执行时长和资源消耗，而不是把用户数据完整写入日志。

## 最后

这次改造没有追求“把所有东西都放进 Wasm”，而是只迁移了一个输入输出边界明确的任务。对于大多数项目来说，从这个小切口开始，往往比一次性重写更稳妥。
`;

export const demoWorkspace: WorkspaceInfo = {
  path: "~/Documents/CreatorRelay",
  name: "CreatorRelay",
};

export const demoDocuments: Document[] = [
  {
    id: "article-wasm-edge",
    title: "在边缘运行 WebAssembly：一次轻量化实践",
    path: "articles/wasm-edge/article.md",
    updatedAt: new Date(Date.now() - 38 * 60 * 1000).toISOString(),
    preview: "当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘……",
    wordCount: 684,
    charCount: 1290,
    tags: ["WebAssembly", "边缘计算"],
  },
  {
    id: "article-rust-cli",
    title: "用 Rust 构建一个可靠的命令行工具",
    path: "articles/rust-cli/article.md",
    updatedAt: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    preview: "一个好用的 CLI，不只是参数解析。它还需要在错误发生时给出清晰的下一步……",
    wordCount: 1024,
    charCount: 2018,
    tags: ["Rust", "工程实践"],
  },
  {
    id: "article-design-tokens",
    title: "设计 Token：从视觉约束到工程语言",
    path: "articles/design-tokens/article.md",
    updatedAt: new Date(Date.now() - 3 * 24 * 60 * 60 * 1000).toISOString(),
    preview: "设计 Token 不应该只是颜色变量。它更像是设计决策和代码实现之间的一份契约……",
    wordCount: 832,
    charCount: 1680,
    tags: ["设计系统", "前端"],
  },
  {
    id: "article-observability",
    title: "小团队如何建立有效的可观测性",
    path: "articles/observability/article.md",
    updatedAt: new Date(Date.now() - 7 * 24 * 60 * 60 * 1000).toISOString(),
    preview: "监控系统不应该只在故障之后才被想起。真正有效的信号，往往来自很小的约定……",
    wordCount: 562,
    charCount: 1054,
    tags: ["可观测性", "团队效率"],
  },
];

export type PublishTaskStatus =
  "PENDING" | "SCHEDULED" | "PUBLISHING" | "SUCCESS" | "FAILED" | "NEEDS_MANUAL_ACTION";

export interface PublishTaskItem {
  id: string;
  articleTitle: string;
  draftType: "Platform Draft" | "Master Draft";
  platform: string;
  platformMark: string;
  accountAlias: string;
  status: PublishTaskStatus;
  scheduledAt?: string;
  updatedAt: string;
  note: string;
}

export const demoPublishTasks: PublishTaskItem[] = [
  {
    id: "task-001",
    articleTitle: "在边缘运行 WebAssembly：一次轻量化实践",
    draftType: "Platform Draft",
    platform: "掘金",
    platformMark: "稀",
    accountAlias: "技术主号",
    status: "NEEDS_MANUAL_ACTION",
    updatedAt: "10 分钟前",
    note: "已生成掘金适配稿，等待人工确认封面与专栏标签后发布",
  },
  {
    id: "task-002",
    articleTitle: "用 Rust 构建一个可靠的命令行工具",
    draftType: "Platform Draft",
    platform: "CSDN",
    platformMark: "C",
    accountAlias: "Rust 专栏号",
    status: "SCHEDULED",
    scheduledAt: "今天 20:00",
    updatedAt: "1 小时前",
    note: "定时任务已校验通过，将在指定窗口触发发布",
  },
  {
    id: "task-003",
    articleTitle: "设计 Token：从视觉约束到工程语言",
    draftType: "Platform Draft",
    platform: "知乎",
    platformMark: "知",
    accountAlias: "林工笔记",
    status: "SUCCESS",
    updatedAt: "昨天 18:20",
    note: "已同步外部作品 ID 与原始链接",
  },
  {
    id: "task-004",
    articleTitle: "小团队如何建立有效的可观测性",
    draftType: "Platform Draft",
    platform: "微信公众号",
    platformMark: "微",
    accountAlias: "工程手记",
    status: "FAILED",
    updatedAt: "2 天前",
    note: "会话状态已过期，请先重新验证账号后再核对发布状态",
  },
];

export interface PublishedContentItem {
  id: string;
  externalId: string;
  title: string;
  platform: string;
  accountAlias: string;
  contentType: "技术文章" | "视频" | "图文" | "专栏";
  publishedAt: string;
  status: "已发布" | "待同步";
  readsLabel: string;
  readsMetric: string;
  likes: number;
  comments: number;
  favorites: number;
}

export const demoPublishedContents: PublishedContentItem[] = [
  {
    id: "pub-101",
    externalId: "jj_743829104",
    title: "设计 Token：从视觉约束到工程语言",
    platform: "掘金",
    accountAlias: "技术主号",
    contentType: "技术文章",
    publishedAt: "2026-11-22",
    status: "已发布",
    readsLabel: "8,420",
    readsMetric: "文章阅读（掘金口径）",
    likes: 312,
    comments: 46,
    favorites: 289,
  },
  {
    id: "pub-102",
    externalId: "zh_68291044",
    title: "设计 Token：从视觉约束到工程语言",
    platform: "知乎",
    accountAlias: "林工笔记",
    contentType: "专栏",
    publishedAt: "2026-11-24",
    status: "已发布",
    readsLabel: "5,190",
    readsMetric: "专栏浏览（知乎口径）",
    likes: 184,
    comments: 29,
    favorites: 215,
  },
  {
    id: "pub-103",
    externalId: "bv_1xx411c7mD",
    title: "10 分钟讲清 Rust CLI 错误处理与退出码",
    platform: "Bilibili",
    accountAlias: "林工视频号",
    contentType: "视频",
    publishedAt: "2026-11-19",
    status: "已发布",
    readsLabel: "14,280",
    readsMetric: "视频播放（Bilibili 口径）",
    likes: 942,
    comments: 118,
    favorites: 635,
  },
  {
    id: "pub-104",
    externalId: "csdn_14302981",
    title: "小团队如何建立有效的可观测性",
    platform: "CSDN",
    accountAlias: "Rust 专栏号",
    contentType: "技术文章",
    publishedAt: "2026-11-16",
    status: "已发布",
    readsLabel: "6,810",
    readsMetric: "博客阅读（CSDN 口径）",
    likes: 147,
    comments: 19,
    favorites: 174,
  },
];

export type AccountStatus = "ACTIVE" | "EXPIRED" | "NEEDS_REAUTH" | "UNAUTHORIZED";

export interface PlatformAccountItem {
  id: string;
  platformId: string;
  platformName: string;
  contentFormats: string;
  authMode: "browser" | "oauth" | "manual";
  alias: string;
  uid: string;
  status: AccountStatus;
  lastSyncedAt: string;
  capabilities: {
    contentRead: "supported" | "restricted" | "experimental";
    contentPublish: "supported" | "restricted" | "unsupported";
    analytics: "supported" | "restricted" | "experimental";
  };
}

export const demoPlatformAccounts: PlatformAccountItem[] = [
  {
    id: "acc-juejin-1",
    platformId: "juejin",
    platformName: "掘金",
    contentFormats: "技术文章",
    authMode: "browser",
    alias: "技术主号",
    uid: "uid_948201",
    status: "ACTIVE",
    lastSyncedAt: "15 分钟前",
    capabilities: {
      contentRead: "supported",
      contentPublish: "supported",
      analytics: "supported",
    },
  },
  {
    id: "acc-csdn-1",
    platformId: "csdn",
    platformName: "CSDN",
    contentFormats: "技术博客",
    authMode: "browser",
    alias: "Rust 专栏号",
    uid: "lin_rust_dev",
    status: "ACTIVE",
    lastSyncedAt: "42 分钟前",
    capabilities: {
      contentRead: "supported",
      contentPublish: "supported",
      analytics: "supported",
    },
  },
  {
    id: "acc-bilibili-1",
    platformId: "bilibili",
    platformName: "Bilibili",
    contentFormats: "视频、专栏",
    authMode: "oauth",
    alias: "林工视频号",
    uid: "mid_3210948",
    status: "ACTIVE",
    lastSyncedAt: "2 小时前",
    capabilities: {
      contentRead: "supported",
      contentPublish: "restricted",
      analytics: "supported",
    },
  },
  {
    id: "acc-zhihu-1",
    platformId: "zhihu",
    platformName: "知乎",
    contentFormats: "问答、文章",
    authMode: "browser",
    alias: "林工笔记",
    uid: "zhihu_lin_note",
    status: "ACTIVE",
    lastSyncedAt: "昨天",
    capabilities: {
      contentRead: "supported",
      contentPublish: "restricted",
      analytics: "experimental",
    },
  },
  {
    id: "acc-wechat-1",
    platformId: "wechat",
    platformName: "微信公众号",
    contentFormats: "图文、视频",
    authMode: "manual",
    alias: "工程手记",
    uid: "gh_8820192a",
    status: "EXPIRED",
    lastSyncedAt: "3 天前",
    capabilities: {
      contentRead: "restricted",
      contentPublish: "unsupported",
      analytics: "restricted",
    },
  },
  {
    id: "acc-youtube-1",
    platformId: "youtube",
    platformName: "YouTube",
    contentFormats: "视频、频道",
    authMode: "oauth",
    alias: "CreatorRelay Lab",
    uid: "UC_demo_001",
    status: "NEEDS_REAUTH",
    lastSyncedAt: "5 天前",
    capabilities: {
      contentRead: "supported",
      contentPublish: "supported",
      analytics: "supported",
    },
  },
];

export const plannedPlatforms = [
  { name: "小红书", formats: "图文、视频", note: "按实际授权能力接入，支持人工发布辅助" },
  { name: "抖音", formats: "短视频、图文", note: "按实际授权能力接入" },
  { name: "快手", formats: "短视频", note: "按实际授权能力接入" },
  { name: "微博", formats: "社交动态", note: "轻量动态与互动指标" },
];

export type TrendRange = "7d" | "30d";

export interface PlatformTrendSeries {
  platformId: string;
  platformName: string;
  accountAlias: string;
  metricLabel: string;
  color: string;
  values: number[];
}

export interface OverviewTrendDataset {
  dates: string[];
  series: PlatformTrendSeries[];
}

export const demoOverviewTrends: Record<TrendRange, OverviewTrendDataset> = {
  "7d": {
    dates: ["11-19", "11-20", "11-21", "11-22", "11-23", "11-24", "11-25"],
    series: [
      {
        platformId: "juejin",
        platformName: "掘金",
        accountAlias: "技术主号",
        metricLabel: "文章阅读（掘金口径）",
        color: "#1E80FF",
        values: [1120, 1480, 1360, 2240, 2680, 2410, 2950],
      },
      {
        platformId: "bilibili",
        platformName: "Bilibili",
        accountAlias: "林工视频号",
        metricLabel: "视频播放（Bilibili 口径）",
        color: "#FB7299",
        values: [2840, 3420, 3190, 3860, 4290, 4610, 5120],
      },
      {
        platformId: "csdn",
        platformName: "CSDN",
        accountAlias: "Rust 专栏号",
        metricLabel: "博客阅读（CSDN 口径）",
        color: "#E05A2B",
        values: [890, 1020, 1180, 1290, 1410, 1340, 1580],
      },
      {
        platformId: "zhihu",
        platformName: "知乎",
        accountAlias: "林工笔记",
        metricLabel: "专栏浏览（知乎口径）",
        color: "#2F7D5B",
        values: [640, 720, 810, 960, 1120, 1380, 1490],
      },
    ],
  },
  "30d": {
    dates: [
      "10-27",
      "10-30",
      "11-02",
      "11-05",
      "11-08",
      "11-11",
      "11-14",
      "11-17",
      "11-20",
      "11-23",
      "11-25",
    ],
    series: [
      {
        platformId: "juejin",
        platformName: "掘金",
        accountAlias: "技术主号",
        metricLabel: "文章阅读（掘金口径）",
        color: "#1E80FF",
        values: [1420, 1680, 1950, 1810, 2190, 2480, 2320, 2610, 2890, 3120, 3450],
      },
      {
        platformId: "bilibili",
        platformName: "Bilibili",
        accountAlias: "林工视频号",
        metricLabel: "视频播放（Bilibili 口径）",
        color: "#FB7299",
        values: [2150, 2480, 2760, 3120, 2980, 3540, 3890, 4120, 4380, 4760, 5120],
      },
      {
        platformId: "csdn",
        platformName: "CSDN",
        accountAlias: "Rust 专栏号",
        metricLabel: "博客阅读（CSDN 口径）",
        color: "#E05A2B",
        values: [780, 860, 940, 1080, 1150, 1210, 1190, 1340, 1420, 1490, 1580],
      },
      {
        platformId: "zhihu",
        platformName: "知乎",
        accountAlias: "林工笔记",
        metricLabel: "专栏浏览（知乎口径）",
        color: "#2F7D5B",
        values: [520, 610, 690, 740, 820, 890, 970, 1050, 1180, 1340, 1490],
      },
    ],
  },
};
