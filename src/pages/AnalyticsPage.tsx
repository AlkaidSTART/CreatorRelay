import { useEffect, useMemo, useState } from "react";
import { BarChart3, Info } from "../components/Icons";
import { OverviewTrendChart } from "../components/OverviewTrendChart";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { useOperationsStore, usePlatformStore } from "../stores";
import type { AnalyticsGranularity, AnalyticsRange } from "../types/operations";
import type { ContentKind } from "../types/content";
import {
  analyticsToTrendDataset,
  contentKindText,
  formatMetricNumber,
  isTrafficMetric,
} from "../utils/operations";

const rangeOptions: Array<{ id: AnalyticsRange; label: string }> = [
  { id: "7d", label: "近 7 天" },
  { id: "30d", label: "近 30 天" },
  { id: "90d", label: "近 90 天" },
];

const granularityOptions: Array<{ id: AnalyticsGranularity; label: string }> = [
  { id: "day", label: "按日" },
  { id: "week", label: "按周" },
  { id: "month", label: "按月" },
];

const kindOptions: Array<{ value: "ALL" | ContentKind; label: string }> = [
  { value: "ALL", label: "全部形态" },
  { value: "video", label: "视频" },
  { value: "imageText", label: "图文" },
  { value: "imageArticle", label: "图文文章" },
  { value: "article", label: "纯文章" },
];

export function AnalyticsPage() {
  const accounts = usePlatformStore((state) => state.accounts);
  const initializeAccounts = usePlatformStore((state) => state.initialize);
  const analytics = useOperationsStore((state) => state.analytics);
  const initializeOperations = useOperationsStore((state) => state.initialize);
  const loadAnalytics = useOperationsStore((state) => state.loadAnalytics);
  const [range, setRange] = useState<AnalyticsRange>("30d");
  const [granularity, setGranularity] = useState<AnalyticsGranularity>("day");
  const [metricScope, setMetricScope] = useState<"traffic" | "all">("traffic");
  const [platformId, setPlatformId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [contentKind, setContentKind] = useState<"ALL" | ContentKind>("ALL");

  useEffect(() => {
    void initializeAccounts();
    void initializeOperations();
  }, [initializeAccounts, initializeOperations]);

  useEffect(() => {
    void loadAnalytics({
      range,
      granularity,
      platformId: platformId || null,
      accountId: accountId || null,
      contentKind: contentKind === "ALL" ? null : contentKind,
      workId: null,
    });
  }, [accountId, contentKind, granularity, loadAnalytics, platformId, range]);

  const displayedSeries = useMemo(() => {
    if (!analytics) return [];
    if (metricScope === "all") return analytics.series;
    const trafficOnly = analytics.series.filter((item) => isTrafficMetric(item.metricName));
    return trafficOnly.length > 0 ? trafficOnly : analytics.series;
  }, [analytics, metricScope]);

  const hasNonTrafficMetrics = useMemo(
    () => Boolean(analytics?.series.some((item) => !isTrafficMetric(item.metricName))),
    [analytics],
  );

  const dataset = useMemo(
    () =>
      analytics
        ? analyticsToTrendDataset({ ...analytics, series: displayedSeries })
        : { dates: [], series: [] },
    [analytics, displayedSeries],
  );
  const visibleAccounts = accounts.filter(
    (account) => !platformId || account.platformId === platformId,
  );

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="ANALYTICS"
        title="数据分析"
        description="按账号、内容形态、作品和时间观察原始指标趋势；不同平台口径默认不合并。"
        actions={
          <div className="flex items-center gap-2">
            <div className="flex items-center rounded-xl bg-sidebar p-0.5">
              {granularityOptions.map((item) => (
                <button
                  key={item.id}
                  onClick={() => setGranularity(item.id)}
                  className={`h-7 cursor-pointer rounded-lg px-2.5 text-xs font-medium transition-all ${
                    granularity === item.id
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-secondary hover:text-ink"
                  }`}
                >
                  {item.label}
                </button>
              ))}
            </div>
            <div className="flex items-center rounded-xl bg-sidebar p-0.5">
              {rangeOptions.map((item) => (
                <button
                  key={item.id}
                  onClick={() => setRange(item.id)}
                  className={`h-7 cursor-pointer rounded-lg px-2.5 text-xs font-medium transition-all ${
                    range === item.id
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-secondary hover:text-ink"
                  }`}
                >
                  {item.label}
                </button>
              ))}
            </div>
          </div>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
        <div className="flex flex-wrap items-center gap-2">
          <select
            value={platformId}
            onChange={(event) => {
              setPlatformId(event.target.value);
              setAccountId("");
            }}
            className="h-8 rounded-lg border border-line bg-surface px-2 text-xs text-ink-secondary"
          >
            <option value="">全部平台</option>
            {Array.from(
              new Map(accounts.map((account) => [account.platformId, account.platformName])),
            ).map(([id, name]) => (
              <option key={id} value={id}>
                {name}
              </option>
            ))}
          </select>
          <select
            value={accountId}
            onChange={(event) => setAccountId(event.target.value)}
            className="h-8 rounded-lg border border-line bg-surface px-2 text-xs text-ink-secondary"
          >
            <option value="">全部账号</option>
            {visibleAccounts.map((account) => (
              <option key={account.id} value={account.id}>
                {account.alias}
              </option>
            ))}
          </select>
          <select
            value={contentKind}
            onChange={(event) => setContentKind(event.target.value as typeof contentKind)}
            className="h-8 rounded-lg border border-line bg-surface px-2 text-xs text-ink-secondary"
          >
            {kindOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <span className="ml-auto inline-flex items-center gap-1.5 rounded-lg bg-sidebar px-2.5 py-1 text-[11px] text-ink-secondary">
            <Info size={11} />
            每个序列只代表一个作品的一个平台原始指标
          </span>
        </div>

        <section className="rounded-2xl border border-line bg-surface p-5">
          <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
            <div>
              <h2 className="text-sm font-semibold text-ink">流量趋势折线图</h2>
              <p className="mt-0.5 text-[11px] text-ink-muted">
                按平台原始流量口径（阅读、播放、浏览、曝光等）独立绘制；时间桶内取最新采样，缺失桶留空不补零。
              </p>
            </div>
            <div className="flex items-center gap-2">
              {hasNonTrafficMetrics ? (
                <div className="flex items-center rounded-xl bg-sidebar p-0.5">
                  {(
                    [
                      { id: "traffic", label: "流量指标" },
                      { id: "all", label: "全部指标" },
                    ] as const
                  ).map((item) => (
                    <button
                      key={item.id}
                      onClick={() => setMetricScope(item.id)}
                      className={`h-6 cursor-pointer rounded-lg px-2 text-[11px] font-medium transition-all ${
                        metricScope === item.id
                          ? "bg-surface text-ink shadow-2xs"
                          : "text-ink-secondary hover:text-ink"
                      }`}
                    >
                      {item.label}
                    </button>
                  ))}
                </div>
              ) : null}
              <span className="inline-flex items-center gap-1 text-[11px] text-ink-muted">
                <BarChart3 size={12} /> {dataset.series.length} 条流量折线
              </span>
            </div>
          </div>
          {dataset.series.length ? (
            <OverviewTrendChart dataset={dataset} height={330} ariaLabel="各平台流量趋势折线图" />
          ) : (
            <div className="grid h-64 place-items-center rounded-xl border border-dashed border-line bg-canvas text-center text-xs text-ink-muted">
              所选范围没有至少一个可展示的流量指标序列。
            </div>
          )}
        </section>

        {analytics?.warnings.length ? (
          <section className="rounded-2xl border border-lemon-border bg-lemon-bg p-4 text-xs leading-relaxed text-lemon-text">
            {analytics.warnings.map((warning) => (
              <p key={warning}>· {warning}</p>
            ))}
          </section>
        ) : null}

        <section className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          {displayedSeries.map((series) => {
            const last = series.points[series.points.length - 1];
            const first = series.points[0];
            return (
              <article
                key={series.key}
                className="rounded-2xl border border-line bg-surface p-5"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex min-w-0 items-start gap-3">
                    <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl border border-line bg-white">
                      <PlatformLogo platformName={series.platformName} size={24} />
                    </div>
                    <div className="min-w-0">
                      <p className="text-[11px] text-ink-muted">{series.metricName}</p>
                      <h3 className="mt-1 truncate text-sm font-semibold text-ink">
                        {series.title}
                      </h3>
                      <p className="mt-0.5 truncate text-[11px] text-ink-secondary">
                        {series.platformName} · {series.accountAlias} ·{" "}
                        {contentKindText(series.contentKind)}
                      </p>
                    </div>
                  </div>
                  <span className="shrink-0 rounded-lg bg-sidebar px-2 py-1 text-[10px] text-ink-secondary">
                    {series.points.length} 个采样点
                  </span>
                </div>
                <div className="mt-4 flex items-baseline justify-between gap-3">
                  <strong className="text-2xl font-semibold tabular-nums text-ink">
                    {last ? `${formatMetricNumber(last.value)} ${series.unit}` : "无采样"}
                  </strong>
                  <span className="text-[11px] text-ink-muted">
                    {first?.bucket ?? "—"} → {last?.bucket ?? "—"}
                  </span>
                </div>
                <p className="mt-3 border-t border-line pt-3 text-[11px] leading-relaxed text-ink-muted">
                  {series.metricDefinition || "平台未提供指标定义"} · {series.comparabilityNote}
                </p>
              </article>
            );
          })}
        </section>
      </div>
    </main>
  );
}
