import { demoOverviewTrends, demoPlatformAccounts, demoPublishedContents } from "../data/demoData";
import type { OverviewTrendDataset } from "../data/demoData";
import type { ContentKind } from "../types/content";
import type {
  AnalyticsGranularity,
  AnalyticsPoint,
  AnalyticsQuery,
  AnalyticsResult,
  AnalyticsSeries,
  DashboardFilter,
  ExternalWork,
  OperationsDashboard,
  PublishTask,
  PublishTaskStatus,
  SyncRunResult,
} from "../types/operations";
import type { PlatformAccountItem } from "../types/platform";

export interface PublishTaskFilters {
  status?: "ALL" | PublishTaskStatus;
  keyword?: string;
  platformId?: string;
  accountId?: string;
  contentKind?: "ALL" | ContentKind;
  range?: "ALL" | "7d" | "30d" | "90d";
}

export interface ExternalWorkFilters {
  keyword?: string;
  platformId?: string;
  accountId?: string;
  contentKind?: "ALL" | ContentKind | "unknown";
  status?: "ALL" | "published" | "manual" | "synced";
  range?: "ALL" | "7d" | "30d" | "90d";
}

export const PUBLISH_STATUS_META: Record<
  PublishTaskStatus,
  { label: string; tone: string; description: string }
> = {
  CREATED: {
    label: "待确认",
    tone: "bg-sidebar text-ink-secondary",
    description: "已保存最终内容包快照，尚未执行外部副作用",
  },
  VALIDATING: {
    label: "校验中",
    tone: "bg-sky-bg text-sky-text",
    description: "正在校验内容形态、账号和发布载荷",
  },
  PENDING: {
    label: "等待执行",
    tone: "bg-sidebar text-ink-secondary",
    description: "已确认，等待调度器执行",
  },
  SCHEDULED: {
    label: "定时发布",
    tone: "bg-sky-bg text-sky-text",
    description: "已确认，将在指定时间执行一次",
  },
  PUBLISHING: {
    label: "发布中",
    tone: "bg-accent-soft text-accent-deep",
    description: "已提交给适配器，禁止重复提交",
  },
  SUCCESS: {
    label: "发布成功",
    tone: "bg-mint-bg text-mint-text",
    description: "外部作品 ID、URL 和提交快照已归档",
  },
  FAILED: {
    label: "发布失败",
    tone: "bg-coral-bg text-coral-text",
    description: "结果明确失败，可人工检查后重新准备",
  },
  CANCELLED: {
    label: "已取消",
    tone: "bg-sidebar text-ink-muted",
    description: "任务未继续执行外部发布",
  },
  NEEDS_MANUAL_ACTION: {
    label: "需要人工处理",
    tone: "bg-lemon-bg text-lemon-text",
    description: "自动发布不可用、结果不确定或账号失效",
  },
};

export function contentKindText(kind: ContentKind | "unknown"): string {
  switch (kind) {
    case "video":
      return "视频";
    case "imageText":
      return "图文";
    case "imageArticle":
      return "图文文章";
    case "article":
      return "纯文章";
    default:
      return "未知形态";
  }
}

export function workStatus(item: ExternalWork): "published" | "manual" | "synced" {
  if (item.source === "manual") return "manual";
  if (item.source === "sync") return "synced";
  return "published";
}

export function workStatusLabel(item: ExternalWork): string {
  return { published: "已发布", manual: "人工发布", synced: "已同步" }[workStatus(item)];
}

function parseTime(value: string | null | undefined): number | null {
  if (!value) return null;
  const time = new Date(value).getTime();
  return Number.isFinite(time) ? time : null;
}

function rangeStart(range: "ALL" | "7d" | "30d" | "90d"): number | null {
  if (range === "ALL") return null;
  const days = range === "7d" ? 7 : range === "30d" ? 30 : 90;
  return Date.now() - days * 24 * 60 * 60 * 1000;
}

function matchesRange(
  value: string | null | undefined,
  range: "ALL" | "7d" | "30d" | "90d" | undefined,
): boolean {
  const start = rangeStart(range ?? "ALL");
  if (start == null) return true;
  const time = parseTime(value);
  return time != null && time >= start;
}

export function filterPublishTasks(
  tasks: PublishTask[],
  filters: PublishTaskFilters,
): PublishTask[] {
  const keyword = filters.keyword?.trim().toLocaleLowerCase() ?? "";
  return tasks.filter((task) => {
    if (filters.status && filters.status !== "ALL" && task.status !== filters.status) return false;
    if (filters.platformId && task.platformId !== filters.platformId) return false;
    if (filters.accountId && task.accountId !== filters.accountId) return false;
    if (
      filters.contentKind &&
      filters.contentKind !== "ALL" &&
      task.contentKind !== filters.contentKind
    ) {
      return false;
    }
    if (!matchesRange(task.createdAt, filters.range)) return false;
    if (!keyword) return true;
    return `${task.title} ${task.platformName} ${task.accountAlias} ${task.externalId ?? ""}`
      .toLocaleLowerCase()
      .includes(keyword);
  });
}

export function filterExternalWorks(
  works: ExternalWork[],
  filters: ExternalWorkFilters,
): ExternalWork[] {
  const keyword = filters.keyword?.trim().toLocaleLowerCase() ?? "";
  return works.filter((work) => {
    if (filters.platformId && work.platformId !== filters.platformId) return false;
    if (filters.accountId && work.accountId !== filters.accountId) return false;
    if (
      filters.contentKind &&
      filters.contentKind !== "ALL" &&
      work.contentKind !== filters.contentKind
    ) {
      return false;
    }
    if (filters.status && filters.status !== "ALL" && workStatus(work) !== filters.status) {
      return false;
    }
    if (!matchesRange(work.publishedAt, filters.range)) return false;
    if (!keyword) return true;
    return `${work.title} ${work.platformName} ${work.accountAlias} ${work.externalId} ${work.originalContentType}`
      .toLocaleLowerCase()
      .includes(keyword);
  });
}

const TRAFFIC_METRIC_PATTERN = /播放|阅读|浏览|曝光|查看|观看|流量|访问/;

export function isTrafficMetric(metricName: string): boolean {
  return TRAFFIC_METRIC_PATTERN.test(metricName);
}

export function primaryMetric(work: ExternalWork) {
  return (
    work.metrics.find((metric) => isTrafficMetric(metric.metricName)) ?? work.metrics[0] ?? null
  );
}

export function formatMetricNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    maximumFractionDigits: Number.isInteger(value) ? 0 : 1,
  }).format(value);
}

export function computeDashboardView(input: {
  works: ExternalWork[];
  tasks: PublishTask[];
  accounts: PlatformAccountItem[];
  syncRuns: SyncRunResult[];
  filter: DashboardFilter;
}): OperationsDashboard {
  const range = input.filter.range ?? "7d";
  const taskFilters: PublishTaskFilters = {
    platformId: input.filter.platformId ?? undefined,
    accountId: input.filter.accountId ?? undefined,
    contentKind: input.filter.contentKind ?? "ALL",
    range,
  };
  const workFilters: ExternalWorkFilters = {
    platformId: input.filter.platformId ?? undefined,
    accountId: input.filter.accountId ?? undefined,
    contentKind: input.filter.contentKind ?? "ALL",
    range,
  };
  const works = filterExternalWorks(input.works, workFilters);
  const tasks = filterPublishTasks(input.tasks, taskFilters);
  const accounts = input.accounts.filter((account) => {
    if (input.filter.platformId && account.platformId !== input.filter.platformId) return false;
    if (input.filter.accountId && account.id !== input.filter.accountId) return false;
    return true;
  });
  const pendingTasks = tasks.filter((task) =>
    ["CREATED", "VALIDATING", "PENDING", "SCHEDULED", "NEEDS_MANUAL_ACTION"].includes(task.status),
  );
  const recentPublishes = tasks.filter((task) => task.status === "SUCCESS");
  const syncRuns = input.syncRuns.filter((run) => {
    if (input.filter.platformId && run.platformId !== input.filter.platformId) return false;
    if (input.filter.accountId && run.accountId !== input.filter.accountId) return false;
    return matchesRange(run.completedAt, range);
  });
  const days = range === "7d" ? 7 : range === "30d" ? 30 : 90;
  const now = new Date();
  return {
    platformCount: new Set(accounts.map((account) => account.platformId)).size,
    accountCount: accounts.length,
    recentPublishCount: recentPublishes.length,
    pendingTaskCount: pendingTasks.length,
    syncErrorCount: syncRuns.filter((run) => run.status !== "SUCCESS").length,
    recentWorks: works.slice(0, 10),
    pendingTasks: pendingTasks.slice(0, 10),
    recentPublishes: recentPublishes.slice(0, 10),
    syncRuns,
    filter: {
      range,
      platformId: input.filter.platformId ?? null,
      accountId: input.filter.accountId ?? null,
      contentKind: input.filter.contentKind ?? null,
      from: new Date(now.getTime() - (days - 1) * 24 * 60 * 60 * 1000).toISOString(),
      to: now.toISOString(),
    },
  };
}

const ANALYTICS_COLORS = ["#6B9ED4", "#E38AA6", "#68B692", "#E39472", "#9D89C8", "#5DAAA8"];

const DEMO_90D_DATES = [
  "09-01",
  "09-10",
  "09-20",
  "09-30",
  "10-10",
  "10-20",
  "10-30",
  "11-08",
  "11-17",
  "11-25",
];

const DEMO_90D_SCALE = [0.36, 0.44, 0.51, 0.58, 0.65, 0.72, 0.79, 0.86, 0.93, 1.0];

function isoWeekBucket(date: Date): string {
  const utc = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate()));
  const dayNum = utc.getUTCDay() || 7;
  utc.setUTCDate(utc.getUTCDate() + 4 - dayNum);
  const yearStart = new Date(Date.UTC(utc.getUTCFullYear(), 0, 1));
  const week = Math.ceil(((utc.getTime() - yearStart.getTime()) / 86400_000 + 1) / 7);
  return `${utc.getUTCFullYear()}-W${String(week).padStart(2, "0")}`;
}

function formatDemoBucket(isoDate: string, granularity: AnalyticsGranularity): string {
  const date = new Date(`${isoDate}T12:00:00Z`);
  if (granularity === "day") return isoDate;
  if (granularity === "month") return isoDate.slice(0, 7);
  return isoWeekBucket(date);
}

function demoContentKind(contentType: string): ContentKind {
  if (contentType === "视频") return "video";
  if (contentType === "图文") return "imageText";
  return "article";
}

export function buildDemoAnalytics(query: AnalyticsQuery): AnalyticsResult {
  const baseDataset = query.range === "7d" ? demoOverviewTrends["7d"] : demoOverviewTrends["30d"];
  const rawDates = query.range === "90d" ? DEMO_90D_DATES : baseDataset.dates;
  const isoDates = rawDates.map((mmdd) => `2026-${mmdd}`);

  const workByPlatform = new Map(
    demoPublishedContents.map((item) => {
      const platformId =
        demoPlatformAccounts.find((account) => account.platformName === item.platform)
          ?.platformId ?? item.platform;
      return [platformId, item] as const;
    }),
  );
  const accountByPlatform = new Map(
    demoPlatformAccounts.map((account) => [account.platformId, account] as const),
  );

  const series: AnalyticsSeries[] = [];
  for (const trend of baseDataset.series) {
    const account = accountByPlatform.get(trend.platformId);
    const work = workByPlatform.get(trend.platformId);
    const workId = work?.id ?? `demo-work-${trend.platformId}`;
    const contentKind = work ? demoContentKind(work.contentType) : "article";

    if (query.platformId && trend.platformId !== query.platformId) continue;
    if (query.accountId && account?.id !== query.accountId) continue;
    if (query.contentKind && contentKind !== query.contentKind) continue;
    if (query.workId && workId !== query.workId) continue;

    const latestValue =
      trend.values.reduce<number>((acc, val) => (typeof val === "number" ? val : acc), 0) || 1000;
    const rawValues =
      query.range === "90d"
        ? DEMO_90D_SCALE.map((factor) => Math.round(latestValue * factor))
        : trend.values;

    const bucketed = new Map<string, AnalyticsPoint>();
    for (let index = 0; index < isoDates.length; index += 1) {
      const value = rawValues[index];
      if (value == null) continue;
      const isoDate = isoDates[index];
      const bucket = formatDemoBucket(isoDate, query.granularity);
      bucketed.set(bucket, {
        bucket,
        capturedAt: `${isoDate}T12:00:00Z`,
        value,
      });
    }

    series.push({
      key: `${workId}::${trend.metricLabel}`,
      workId,
      title: work?.title ?? `${trend.platformName}演示作品`,
      platformId: trend.platformId,
      platformName: trend.platformName,
      accountAlias: trend.accountAlias,
      contentKind,
      metricName: trend.metricLabel,
      metricDefinition: "浏览器预览演示流量数据，按平台原始口径独立统计",
      unit: "次",
      points: Array.from(bucketed.values()),
      comparable: false,
      comparabilityNote: "仅展示同一作品、同一平台的原始流量指标，不与其他平台或形态聚合",
    });
  }

  const from = `${isoDates[0]}T00:00:00Z`;
  const to = `${isoDates[isoDates.length - 1]}T23:59:59Z`;
  const warnings = [
    "当前为浏览器预览演示流量数据；在桌面应用同步账号作品后将展示真实平台采样趋势",
  ];
  if (series.length === 0) {
    warnings.push("当前筛选条件下没有匹配的流量序列");
  }

  return {
    range: query.range,
    from,
    to,
    granularity: query.granularity,
    series,
    warnings,
  };
}

export function analyticsToTrendDataset(analytics: AnalyticsResult): OverviewTrendDataset & {
  series: Array<OverviewTrendDataset["series"][number] & { title: string }>;
} {
  const dates = Array.from(
    new Set(analytics.series.flatMap((series) => series.points.map((point) => point.bucket))),
  ).reduce<string[]>((ordered, value) => {
    const index = ordered.findIndex((item) => item > value);
    if (index < 0) ordered.push(value);
    else ordered.splice(index, 0, value);
    return ordered;
  }, []);
  const activeSeries = analytics.series.filter((item) => item.points.length > 0);
  const baseNameCounts = new Map<string, number>();
  for (const item of activeSeries) {
    const baseName = `${item.platformName} · ${item.title}`;
    baseNameCounts.set(baseName, (baseNameCounts.get(baseName) ?? 0) + 1);
  }
  const series = activeSeries.map((item, index) => {
    const byBucket = new Map(item.points.map((point) => [point.bucket, point.value]));
    const baseName = `${item.platformName} · ${item.title}`;
    const platformName =
      (baseNameCounts.get(baseName) ?? 0) > 1 ? `${baseName} · ${item.metricName}` : baseName;
    return {
      platformId: item.platformId,
      platformName,
      accountAlias: item.accountAlias,
      metricLabel: item.metricName,
      color: ANALYTICS_COLORS[index % ANALYTICS_COLORS.length],
      values: dates.map((date) => byBucket.get(date) ?? null),
      title: item.title,
    };
  });
  return { dates, series };
}

export function platformHomeUrl(platformId: string): string {
  const urls: Record<string, string> = {
    juejin: "https://juejin.cn/creator",
    csdn: "https://mp.csdn.net/",
    zhihu: "https://zhuanlan.zhihu.com/write",
    wechat: "https://mp.weixin.qq.com/",
    bilibili: "https://member.bilibili.com/platform/home",
    youtube: "https://studio.youtube.com/",
    douyin: "https://creator.douyin.com/",
    kuaishou: "https://cp.kuaishou.com/",
    xiaohongshu: "https://creator.xiaohongshu.com/",
    weibo: "https://weibo.com/",
  };
  return urls[platformId] ?? "https://example.invalid/";
}

export function taskActionSummary(task: PublishTask): string {
  if (task.paused) return task.pauseReason ?? "任务因账号状态暂停";
  if (task.errorMessage) return task.errorMessage;
  return PUBLISH_STATUS_META[task.status].description;
}
