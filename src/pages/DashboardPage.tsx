import { useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  BarChart3,
  BookOpen,
  CheckCircle2,
  FileText,
  Globe2,
  Plus,
  Send,
} from "../components/Icons";
import { useNavigate } from "react-router";
import { OverviewTrendChart } from "../components/OverviewTrendChart";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { demoOverviewTrends, type TrendRange } from "../data/demoData";
import { useContentStore, useUiStore, useWorkspaceStore } from "../stores";
import { formatRelativeTime } from "../utils/format";
import { getPlatformVisual } from "../utils/platformStyle";

export function DashboardPage() {
  const navigate = useNavigate();
  const [trendRange, setTrendRange] = useState<TrendRange>("7d");

  const documents = useWorkspaceStore((state) => state.documents);
  const selectDocumentSafely = useWorkspaceStore((state) => state.selectDocumentSafely);
  const setIsCreating = useUiStore((state) => state.setIsCreating);

  const platformAccounts = useContentStore((state) => state.platformAccounts);
  const publishedContents = useContentStore((state) => state.publishedContents);
  const publishTasks = useContentStore((state) => state.publishTasks);

  const activeAccounts = platformAccounts.filter((item) => item.status === "ACTIVE").length;
  const issueAccounts = platformAccounts.filter((item) => item.status !== "ACTIVE");
  const pendingTasks = publishTasks.filter(
    (task) => task.status === "NEEDS_MANUAL_ACTION" || task.status === "SCHEDULED",
  );
  const latestDoc = documents[0] ?? null;

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="DASHBOARD"
        title="运营总览"
        description="本地写一稿，接力分发到多平台，并在同一块看板追踪各平台真实数据。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.24)] transition-colors hover:bg-accent-hover"
            onClick={() => setIsCreating(true)}
          >
            <Plus size={13} />
            新建文章
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        {/* French Cream Macaron 3-Step Relay Pipeline Strip */}
        <section className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="flex flex-wrap items-center justify-between gap-3 pb-4">
            <div>
              <h2 className="text-[15px] font-semibold tracking-tight text-ink">
                写一次本地主稿，自动接力分发到 {platformAccounts.length} 个平台
              </h2>
              <p className="mt-0.5 text-xs text-ink-secondary">
                正文始终保存在本地 Markdown，一键拆分为各平台独立适配稿并回收阅读表现。
              </p>
            </div>
            <div className="flex items-center gap-2">
              {latestDoc && (
                <button
                  type="button"
                  onClick={async () => {
                    const ok = await selectDocumentSafely(latestDoc);
                    if (ok) navigate("/studio");
                  }}
                  className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)] transition-colors hover:bg-accent-hover"
                >
                  <BookOpen size={14} />
                  <span className="max-w-[200px] truncate">继续写：{latestDoc.title}</span>
                </button>
              )}
              <button
                type="button"
                onClick={() => navigate("/publishing")}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface-soft px-3 text-xs font-medium text-ink transition-colors hover:bg-subtle"
              >
                <Send size={14} className="text-accent" />
                发布队列 ({pendingTasks.length})
              </button>
            </div>
          </div>

          {/* 3-Step Horizontal Workflow — Coordinated Taro / Rose / Mint Macaron Cards */}
          <div className="grid grid-cols-1 gap-3 border-t border-line pt-4 md:grid-cols-3">
            <button
              type="button"
              onClick={() => navigate("/studio")}
              className="group flex cursor-pointer items-start justify-between gap-3 rounded-xl border border-accent-border bg-accent-soft p-3.5 text-left transition-all hover:border-accent hover:bg-accent-soft"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[11px] font-semibold text-accent-deep">
                  <span>1. 本地写主稿</span>
                </div>
                <strong className="mt-1 block text-[13px] font-semibold text-ink">
                  {documents.length} 篇 Markdown 主稿
                </strong>
                <span className="mt-0.5 block truncate text-[11px] text-ink-secondary">
                  最新：{latestDoc?.title ?? "暂无草稿"}
                </span>
              </div>
              <ArrowRight
                size={13}
                className="mt-0.5 shrink-0 text-accent-deep/60 transition-transform group-hover:translate-x-0.5 group-hover:text-accent-deep"
              />
            </button>

            <button
              type="button"
              onClick={() => navigate("/publishing")}
              className="group flex cursor-pointer items-start justify-between gap-3 rounded-xl border border-rose-border bg-rose-bg p-3.5 text-left transition-all hover:border-rose-border hover:bg-rose-bg"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[11px] font-semibold text-rose-text">
                  <span>2. 多平台接力分发</span>
                </div>
                <strong className="mt-1 block text-[13px] font-semibold text-ink">
                  {pendingTasks.length} 项待发 · {activeAccounts}/{platformAccounts.length} 账号在线
                </strong>
                <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
                  {platformAccounts.slice(0, 5).map((acc) => {
                    return (
                      <span
                        key={acc.id}
                        className="inline-flex items-center gap-1 rounded-md bg-surface/90 px-1.5 py-0.5 text-[10px] font-medium text-ink-secondary ring-1 ring-line"
                      >
                        <PlatformLogo
                          platformId={acc.platformId}
                          platformName={acc.platformName}
                          size={14}
                        />
                        {acc.platformName}
                      </span>
                    );
                  })}
                  {platformAccounts.length > 5 && (
                    <span className="inline-flex items-center rounded-md bg-surface/90 px-1.5 py-0.5 text-[10px] font-medium text-ink-secondary ring-1 ring-line">
                      +{platformAccounts.length - 5}
                    </span>
                  )}
                </div>
              </div>
              <ArrowRight
                size={13}
                className="mt-0.5 shrink-0 text-rose-text/60 transition-transform group-hover:translate-x-0.5 group-hover:text-rose-text"
              />
            </button>

            <button
              type="button"
              onClick={() => navigate("/analytics")}
              className="group flex cursor-pointer items-start justify-between gap-3 rounded-xl border border-mint-border bg-mint-bg p-3.5 text-left transition-all hover:border-mint-border hover:bg-mint-bg"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[11px] font-semibold text-mint-text">
                  <span>3. 追踪全网表现</span>
                </div>
                <strong className="mt-1 block text-[13px] font-semibold text-ink">
                  {publishedContents.length} 篇已同步外部作品
                </strong>
                <span className="mt-0.5 block truncate text-[11px] text-ink-secondary">
                  近 7 天 Bilibili 5,120 播放 · 掘金 2,950 阅读
                </span>
              </div>
              <ArrowRight
                size={13}
                className="mt-0.5 shrink-0 text-mint-text/60 transition-transform group-hover:translate-x-0.5 group-hover:text-mint-text"
              />
            </button>
          </div>
        </section>

        {/* Clickable Metrics Row with Soft Macaron Icon Pills */}
        <section className="grid grid-cols-2 gap-4 xl:grid-cols-4">
          {[
            {
              label: "已接入平台与账号",
              value: platformAccounts.length,
              unit: "个账号",
              sub: `${activeAccounts} 个正常 · ${issueAccounts.length} 个需重连`,
              icon: Globe2,
              iconWrap: "bg-sky-bg text-sky-text",
              path: "/platforms",
            },
            {
              label: "本地主稿库",
              value: documents.length,
              unit: "篇主稿",
              sub: "Markdown 文件实时同步",
              icon: FileText,
              iconWrap: "bg-accent-soft text-accent-deep",
              path: "/studio",
            },
            {
              label: "待处理发布任务",
              value: pendingTasks.length,
              unit: "项待办",
              sub: "支持人工确认与定时排队",
              icon: Send,
              iconWrap: "bg-lemon-bg text-lemon-text",
              path: "/publishing",
            },
            {
              label: "已同步外部作品",
              value: publishedContents.length,
              unit: "篇作品",
              sub: "保留各平台独立统计口径",
              icon: BarChart3,
              iconWrap: "bg-mint-bg text-mint-text",
              path: "/content",
            },
          ].map((card) => {
            const Icon = card.icon;
            return (
              <button
                key={card.label}
                type="button"
                onClick={() => navigate(card.path)}
                className="group flex cursor-pointer flex-col rounded-2xl border border-line bg-surface p-4.5 text-left shadow-[0_2px_10px_rgba(74,58,48,0.03)] transition-all hover:border-line-strong hover:shadow-[0_6px_18px_rgba(74,58,48,0.06)]"
              >
                <div className="flex w-full items-center justify-between text-xs text-ink-secondary">
                  <span>{card.label}</span>
                  <span
                    className={`grid h-7 w-7 place-items-center rounded-lg transition-transform group-hover:scale-105 ${card.iconWrap}`}
                  >
                    <Icon size={14} />
                  </span>
                </div>
                <div className="mt-2.5 flex items-baseline gap-1.5">
                  <strong className="text-2xl font-semibold tracking-tight tabular-nums text-ink">
                    {card.value}
                  </strong>
                  <span className="text-xs text-ink-muted">{card.unit}</span>
                </div>
                <p className="mt-1 text-[11.5px] text-ink-muted">{card.sub}</p>
              </button>
            );
          })}
        </section>

        {/* Traffic Trends */}
        <section className="overflow-hidden rounded-2xl border border-line bg-surface shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="flex flex-wrap items-start justify-between gap-4 border-b border-line px-5 py-4">
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-semibold text-ink">平台流量趋势</h2>
                <span className="rounded-md bg-subtle px-2 py-0.5 text-[10.5px] text-ink-secondary">
                  按各平台原始口径独立呈现
                </span>
              </div>
            </div>
            <div className="flex rounded-xl bg-sidebar p-0.5" aria-label="趋势时间范围">
              {(
                [
                  ["7d", "近 7 天"],
                  ["30d", "近 30 天"],
                ] as const
              ).map(([range, label]) => (
                <button
                  key={range}
                  type="button"
                  aria-pressed={trendRange === range}
                  onClick={() => setTrendRange(range)}
                  className={`h-7 cursor-pointer rounded-lg px-2.5 text-xs transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                    trendRange === range
                      ? "bg-surface font-medium text-ink shadow-[0_1px_3px_rgba(74,58,48,0.08)]"
                      : "text-ink-secondary hover:text-ink"
                  }`}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          <div className="grid grid-cols-1 items-stretch gap-3 px-4 pt-3 pb-4 xl:grid-cols-[minmax(0,1fr)_188px]">
            <OverviewTrendChart dataset={demoOverviewTrends[trendRange]} height={264} />
            <div className="grid grid-cols-2 gap-2 xl:grid-cols-1 xl:content-start">
              {demoOverviewTrends[trendRange].series.map((series) => (
                <div
                  key={series.platformId}
                  className="flex min-w-0 items-start gap-2.5 border-b border-line px-1.5 py-2 last:border-b-0"
                >
                  <PlatformLogo
                    platformId={series.platformId}
                    platformName={series.platformName}
                    size={16}
                    className="mt-0.5"
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="truncate text-xs font-medium text-ink">
                        {series.platformName}
                      </span>
                      <strong className="text-xs font-semibold tabular-nums text-ink">
                        {series.values[series.values.length - 1]?.toLocaleString("zh-CN") ?? "—"}
                      </strong>
                    </div>
                    <div className="mt-0.5 truncate text-[10.5px] text-ink-muted">
                      {series.metricLabel}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Two Column Cards */}
        <section className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
            <div className="mb-3.5 flex items-center justify-between">
              <h2 className="text-sm font-semibold text-ink">待处理提醒</h2>
              <button
                className="cursor-pointer text-xs font-medium text-accent hover:text-accent-deep"
                onClick={() => navigate("/platforms")}
              >
                管理平台
              </button>
            </div>
            <div className="space-y-2">
              {issueAccounts.map((account) => (
                <div
                  key={account.id}
                  className="flex items-center gap-3 rounded-xl border border-lemon-border bg-lemon-bg/80 px-3.5 py-2.5"
                >
                  <AlertTriangle size={14} className="shrink-0 text-lemon-text" />
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-ink">
                      {account.platformName} · {account.alias}
                    </strong>
                    <small className="text-[11px] text-ink-secondary">
                      需要重新授权 · 上次同步 {account.lastSyncedAt}
                    </small>
                  </div>
                  <button
                    className="h-7 shrink-0 cursor-pointer rounded-lg border border-lemon-border bg-surface px-2.5 text-xs font-medium text-lemon-text hover:bg-lemon-bg"
                    onClick={() => navigate("/platforms")}
                  >
                    去重连
                  </button>
                </div>
              ))}
              {pendingTasks.map((task) => {
                return (
                  <div
                    key={task.id}
                    className="flex items-center gap-3 rounded-xl border border-line bg-canvas px-3.5 py-2.5"
                  >
                    <PlatformLogo platformName={task.platform} size={18} />
                    <div className="min-w-0 flex-1">
                      <strong className="block truncate text-xs font-medium text-ink">
                        [{task.platform}] {task.articleTitle}
                      </strong>
                      <small className="block truncate text-[11px] text-ink-muted">
                        {task.note}
                      </small>
                    </div>
                    <button
                      className="h-7 shrink-0 cursor-pointer rounded-lg border border-accent-border bg-surface px-2.5 text-xs font-medium text-accent-deep hover:bg-accent-soft"
                      onClick={() => navigate("/publishing")}
                    >
                      去发布
                    </button>
                  </div>
                );
              })}
              {issueAccounts.length === 0 && pendingTasks.length === 0 && (
                <div className="flex items-center gap-2 rounded-xl border border-mint-border bg-mint-bg px-3.5 py-3 text-xs text-mint-text">
                  <CheckCircle2 size={14} />
                  <span>所有平台账号与发布任务均正常</span>
                </div>
              )}
            </div>
          </article>

          <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
            <div className="mb-3.5 flex items-center justify-between">
              <h2 className="text-sm font-semibold text-ink">最近编辑的主稿</h2>
              <button
                className="cursor-pointer text-xs font-medium text-accent hover:text-accent-deep"
                onClick={() => navigate("/studio")}
              >
                进入写作空间
              </button>
            </div>
            <div className="space-y-2">
              {documents.slice(0, 4).map((doc) => (
                <button
                  key={doc.id}
                  onClick={async () => {
                    const ok = await selectDocumentSafely(doc);
                    if (ok) navigate("/studio");
                  }}
                  className="group flex w-full cursor-pointer items-center gap-3 rounded-xl border border-line bg-canvas px-3.5 py-2.5 text-left transition-colors hover:border-accent-border hover:bg-accent-soft"
                >
                  <FileText size={14} className="shrink-0 text-ink-muted group-hover:text-accent" />
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-ink">
                      {doc.title}
                    </strong>
                    <small className="text-[11px] text-ink-muted">
                      {formatRelativeTime(doc.updatedAt)} · {doc.charCount || doc.wordCount} 字
                    </small>
                  </div>
                  <ArrowRight
                    size={13}
                    className="shrink-0 text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-accent"
                  />
                </button>
              ))}
            </div>
          </article>
        </section>

        {/* Recent Performance Table */}
        <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="mb-3.5 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-ink">近期外部作品表现</h2>
            <button
              className="cursor-pointer text-xs font-medium text-accent hover:text-accent-deep"
              onClick={() => navigate("/content")}
            >
              查看全部作品
            </button>
          </div>
          <div className="divide-y divide-line">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(140px,1.2fr)_minmax(160px,1.3fr)_minmax(120px,1fr)] gap-4 pb-2.5 text-xs font-medium text-ink-muted">
              <span>作品标题</span>
              <span>平台 / 账号</span>
              <span>主指标（按平台口径）</span>
              <span>互动</span>
            </div>
            {publishedContents.map((item) => {
              const visual = getPlatformVisual(item.platform);
              return (
                <div
                  key={item.id}
                  className="grid grid-cols-[minmax(200px,2fr)_minmax(140px,1.2fr)_minmax(160px,1.3fr)_minmax(120px,1fr)] items-center gap-4 py-3"
                >
                  <div className="min-w-0">
                    <strong className="block truncate text-xs font-medium text-ink">
                      {item.title}
                    </strong>
                    <small className="text-[11px] text-ink-muted">{item.publishedAt}</small>
                  </div>
                  <div className="flex items-center gap-2">
                    <span
                      className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium ${visual.badgeClass}`}
                    >
                      <PlatformLogo platformName={item.platform} size={14} />
                      {item.platform}
                    </span>
                    <span className="truncate text-xs text-ink-secondary">{item.accountAlias}</span>
                  </div>
                  <div>
                    <strong className="block text-xs font-semibold tabular-nums text-ink">
                      {item.readsLabel}
                    </strong>
                    <small className="text-[11px] text-ink-muted">{item.readsMetric}</small>
                  </div>
                  <div className="text-xs text-ink-secondary">
                    赞 {item.likes} · 评 {item.comments} · 藏 {item.favorites}
                  </div>
                </div>
              );
            })}
          </div>
        </article>
      </div>
    </main>
  );
}
