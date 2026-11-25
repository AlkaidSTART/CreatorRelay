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
    <main className="flex h-full min-w-0 flex-col bg-[#FAFAFB]">
      <PageHeader
        eyebrow="DASHBOARD"
        title="运营总览"
        description="本地写一稿，接力分发到多平台，并在同一块看板追踪各平台真实数据。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-[#2563EB] px-3 text-xs font-medium text-white shadow-[0_1px_2px_rgba(37,99,235,0.25)] transition-colors hover:bg-[#1D4ED8]"
            onClick={() => setIsCreating(true)}
          >
            <Plus size={13} />
            新建文章
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        {/* Clean Native-Style 3-Second Relay Pipeline Strip (No cheesy gradient hero!) */}
        <section className="rounded-xl border border-black/[0.07] bg-white p-5 shadow-[0_1px_2px_rgba(0,0,0,0.03)]">
          <div className="flex flex-wrap items-center justify-between gap-3 pb-4">
            <div>
              <h2 className="text-[15px] font-semibold tracking-tight text-[#1D1D1F]">
                写一次本地主稿，自动接力分发到 {platformAccounts.length} 个平台
              </h2>
              <p className="mt-0.5 text-xs text-[#6E6E73]">
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
                  className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-[#2563EB] px-3 text-xs font-medium text-white transition-colors hover:bg-[#1D4ED8]"
                >
                  <BookOpen size={12} />
                  <span className="max-w-[200px] truncate">继续写：{latestDoc.title}</span>
                </button>
              )}
              <button
                type="button"
                onClick={() => navigate("/publishing")}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-black/[0.1] bg-white px-3 text-xs font-medium text-[#1D1D1F] transition-colors hover:bg-[#F5F5F7]"
              >
                <Send size={12} className="text-[#6E6E73]" />
                发布队列 ({pendingTasks.length})
              </button>
            </div>
          </div>

          {/* 3-Step Horizontal Workflow */}
          <div className="grid grid-cols-1 gap-3 border-t border-black/[0.06] pt-4 md:grid-cols-3">
            <button
              type="button"
              onClick={() => navigate("/studio")}
              className="group flex cursor-pointer items-start justify-between gap-3 rounded-lg border border-black/[0.06] bg-[#FAFAFB] p-3.5 text-left transition-all hover:border-[#2563EB]/35 hover:bg-white"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[11px] font-medium text-[#2563EB]">
                  <span>1. 本地写主稿</span>
                </div>
                <strong className="mt-1 block text-[13px] font-semibold text-[#1D1D1F]">
                  {documents.length} 篇 Markdown 主稿
                </strong>
                <span className="mt-0.5 block truncate text-[11px] text-[#6E6E73]">
                  最新：{latestDoc?.title ?? "暂无草稿"}
                </span>
              </div>
              <ArrowRight
                size={13}
                className="mt-0.5 shrink-0 text-[#C7C7CC] transition-transform group-hover:translate-x-0.5 group-hover:text-[#2563EB]"
              />
            </button>

            <button
              type="button"
              onClick={() => navigate("/publishing")}
              className="group flex cursor-pointer items-start justify-between gap-3 rounded-lg border border-black/[0.06] bg-[#FAFAFB] p-3.5 text-left transition-all hover:border-[#2563EB]/35 hover:bg-white"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[11px] font-medium text-[#2563EB]">
                  <span>2. 多平台接力分发</span>
                </div>
                <strong className="mt-1 block text-[13px] font-semibold text-[#1D1D1F]">
                  {pendingTasks.length} 项待发 · {activeAccounts}/{platformAccounts.length} 账号在线
                </strong>
                <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
                  {platformAccounts.slice(0, 5).map((acc) => {
                    const visual = getPlatformVisual(acc.platformName);
                    return (
                      <span
                        key={acc.id}
                        className="inline-flex items-center gap-1 rounded bg-white px-1.5 py-0.5 text-[10px] font-medium text-[#58585D] ring-1 ring-black/[0.06]"
                      >
                        <span
                          className="h-1.5 w-1.5 rounded-full"
                          style={{ backgroundColor: visual.color }}
                        />
                        {acc.platformName}
                      </span>
                    );
                  })}
                </div>
              </div>
              <ArrowRight
                size={13}
                className="mt-0.5 shrink-0 text-[#C7C7CC] transition-transform group-hover:translate-x-0.5 group-hover:text-[#2563EB]"
              />
            </button>

            <button
              type="button"
              onClick={() => navigate("/analytics")}
              className="group flex cursor-pointer items-start justify-between gap-3 rounded-lg border border-black/[0.06] bg-[#FAFAFB] p-3.5 text-left transition-all hover:border-[#2563EB]/35 hover:bg-white"
            >
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 text-[11px] font-medium text-[#2563EB]">
                  <span>3. 追踪全网表现</span>
                </div>
                <strong className="mt-1 block text-[13px] font-semibold text-[#1D1D1F]">
                  {publishedContents.length} 篇已同步外部作品
                </strong>
                <span className="mt-0.5 block truncate text-[11px] text-[#6E6E73]">
                  近 7 天 Bilibili 5,120 播放 · 掘金 2,950 阅读
                </span>
              </div>
              <ArrowRight
                size={13}
                className="mt-0.5 shrink-0 text-[#C7C7CC] transition-transform group-hover:translate-x-0.5 group-hover:text-[#2563EB]"
              />
            </button>
          </div>
        </section>

        {/* Clean Clickable Metrics Row */}
        <section className="grid grid-cols-2 gap-4 xl:grid-cols-4">
          {[
            {
              label: "已接入平台与账号",
              value: platformAccounts.length,
              unit: "个账号",
              sub: `${activeAccounts} 个正常 · ${issueAccounts.length} 个需重连`,
              icon: Globe2,
              path: "/platforms",
            },
            {
              label: "本地主稿库",
              value: documents.length,
              unit: "篇主稿",
              sub: "Markdown 文件实时同步",
              icon: FileText,
              path: "/studio",
            },
            {
              label: "待处理发布任务",
              value: pendingTasks.length,
              unit: "项待办",
              sub: "支持人工确认与定时排队",
              icon: Send,
              path: "/publishing",
            },
            {
              label: "已同步外部作品",
              value: publishedContents.length,
              unit: "篇作品",
              sub: "保留各平台独立统计口径",
              icon: BarChart3,
              path: "/content",
            },
          ].map((card) => {
            const Icon = card.icon;
            return (
              <button
                key={card.label}
                type="button"
                onClick={() => navigate(card.path)}
                className="group flex cursor-pointer flex-col rounded-xl border border-black/[0.07] bg-white p-4.5 text-left transition-all hover:border-black/[0.15] hover:shadow-[0_2px_10px_rgba(0,0,0,0.04)]"
              >
                <div className="flex w-full items-center justify-between text-xs text-[#6E6E73]">
                  <span>{card.label}</span>
                  <Icon size={14} className="text-[#AEAEB2] transition-colors group-hover:text-[#2563EB]" />
                </div>
                <div className="mt-2.5 flex items-baseline gap-1.5">
                  <strong className="text-2xl font-semibold tracking-tight tabular-nums text-[#1D1D1F]">
                    {card.value}
                  </strong>
                  <span className="text-xs text-[#8E8E93]">{card.unit}</span>
                </div>
                <p className="mt-1 text-[11.5px] text-[#8E8E93]">{card.sub}</p>
              </button>
            );
          })}
        </section>

        {/* Traffic Trends */}
        <section className="overflow-hidden rounded-xl border border-black/[0.07] bg-white">
          <div className="flex flex-wrap items-start justify-between gap-4 border-b border-black/[0.06] px-5 py-4">
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-semibold text-[#1D1D1F]">平台流量趋势</h2>
                <span className="rounded bg-[#F5F5F7] px-2 py-0.5 text-[10.5px] text-[#6E6E73]">
                  按各平台原始口径独立呈现
                </span>
              </div>
            </div>
            <div
              className="flex rounded-lg bg-[#F5F5F7] p-0.5"
              aria-label="趋势时间范围"
            >
              {([
                ["7d", "近 7 天"],
                ["30d", "近 30 天"],
              ] as const).map(([range, label]) => (
                <button
                  key={range}
                  type="button"
                  aria-pressed={trendRange === range}
                  onClick={() => setTrendRange(range)}
                  className={`h-7 cursor-pointer rounded-md px-2.5 text-xs transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#2563EB]/40 ${
                    trendRange === range
                      ? "bg-white font-medium text-[#1D1D1F] shadow-2xs"
                      : "text-[#6E6E73] hover:text-[#1D1D1F]"
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
                  className="flex min-w-0 items-start gap-2.5 border-b border-black/[0.05] px-1.5 py-2 last:border-b-0"
                >
                  <span
                    className="mt-1.5 h-2 w-2 shrink-0 rounded-full"
                    style={{ backgroundColor: series.color }}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline justify-between gap-2">
                      <span className="truncate text-xs font-medium text-[#1D1D1F]">
                        {series.platformName}
                      </span>
                      <strong className="text-xs font-semibold tabular-nums text-[#1D1D1F]">
                        {series.values[series.values.length - 1]?.toLocaleString("zh-CN") ?? "—"}
                      </strong>
                    </div>
                    <div className="mt-0.5 truncate text-[10.5px] text-[#8E8E93]">
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
          <article className="rounded-xl border border-black/[0.07] bg-white p-5">
            <div className="mb-3.5 flex items-center justify-between">
              <h2 className="text-sm font-semibold text-[#1D1D1F]">待处理提醒</h2>
              <button
                className="cursor-pointer text-xs font-medium text-[#2563EB] hover:text-[#1D4ED8]"
                onClick={() => navigate("/platforms")}
              >
                管理平台
              </button>
            </div>
            <div className="space-y-2">
              {issueAccounts.map((account) => (
                <div
                  key={account.id}
                  className="flex items-center gap-3 rounded-lg border border-amber-200/70 bg-amber-50/40 px-3.5 py-2.5"
                >
                  <AlertTriangle size={14} className="shrink-0 text-amber-600" />
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-[#1D1D1F]">
                      {account.platformName} · {account.alias}
                    </strong>
                    <small className="text-[11px] text-[#6E6E73]">
                      需要重新授权 · 上次同步 {account.lastSyncedAt}
                    </small>
                  </div>
                  <button
                    className="h-7 shrink-0 cursor-pointer rounded-md border border-black/[0.1] bg-white px-2.5 text-xs font-medium text-[#1D1D1F] hover:bg-[#F5F5F7]"
                    onClick={() => navigate("/platforms")}
                  >
                    去重连
                  </button>
                </div>
              ))}
              {pendingTasks.map((task) => {
                const visual = getPlatformVisual(task.platform);
                return (
                  <div
                    key={task.id}
                    className="flex items-center gap-3 rounded-lg border border-black/[0.06] bg-[#FAFAFB] px-3.5 py-2.5"
                  >
                    <span
                      className="h-2 w-2 shrink-0 rounded-full"
                      style={{ backgroundColor: visual.color }}
                    />
                    <div className="min-w-0 flex-1">
                      <strong className="block truncate text-xs font-medium text-[#1D1D1F]">
                        [{task.platform}] {task.articleTitle}
                      </strong>
                      <small className="block truncate text-[11px] text-[#8E8E93]">{task.note}</small>
                    </div>
                    <button
                      className="h-7 shrink-0 cursor-pointer rounded-md border border-black/[0.1] bg-white px-2.5 text-xs font-medium text-[#2563EB] hover:bg-[#EFF6FF]"
                      onClick={() => navigate("/publishing")}
                    >
                      去发布
                    </button>
                  </div>
                );
              })}
              {issueAccounts.length === 0 && pendingTasks.length === 0 && (
                <div className="flex items-center gap-2 rounded-lg bg-emerald-50/60 px-3.5 py-3 text-xs text-emerald-700">
                  <CheckCircle2 size={14} />
                  <span>所有平台账号与发布任务均正常</span>
                </div>
              )}
            </div>
          </article>

          <article className="rounded-xl border border-black/[0.07] bg-white p-5">
            <div className="mb-3.5 flex items-center justify-between">
              <h2 className="text-sm font-semibold text-[#1D1D1F]">最近编辑的主稿</h2>
              <button
                className="cursor-pointer text-xs font-medium text-[#2563EB] hover:text-[#1D4ED8]"
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
                  className="group flex w-full cursor-pointer items-center gap-3 rounded-lg border border-black/[0.05] bg-[#FAFAFB] px-3.5 py-2.5 text-left transition-colors hover:border-black/[0.12] hover:bg-white"
                >
                  <FileText size={14} className="shrink-0 text-[#8E8E93] group-hover:text-[#2563EB]" />
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-[#1D1D1F]">
                      {doc.title}
                    </strong>
                    <small className="text-[11px] text-[#8E8E93]">
                      {formatRelativeTime(doc.updatedAt)} · {doc.charCount || doc.wordCount} 字
                    </small>
                  </div>
                  <ArrowRight
                    size={13}
                    className="shrink-0 text-[#C7C7CC] transition-transform group-hover:translate-x-0.5 group-hover:text-[#2563EB]"
                  />
                </button>
              ))}
            </div>
          </article>
        </section>

        {/* Recent Performance Table */}
        <article className="rounded-xl border border-black/[0.07] bg-white p-5">
          <div className="mb-3.5 flex items-center justify-between">
            <h2 className="text-sm font-semibold text-[#1D1D1F]">近期外部作品表现</h2>
            <button
              className="cursor-pointer text-xs font-medium text-[#2563EB] hover:text-[#1D4ED8]"
              onClick={() => navigate("/content")}
            >
              查看全部作品
            </button>
          </div>
          <div className="divide-y divide-black/[0.05]">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(140px,1.2fr)_minmax(160px,1.3fr)_minmax(120px,1fr)] gap-4 pb-2.5 text-xs font-medium text-[#8E8E93]">
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
                    <strong className="block truncate text-xs font-medium text-[#1D1D1F]">
                      {item.title}
                    </strong>
                    <small className="text-[11px] text-[#8E8E93]">{item.publishedAt}</small>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="inline-flex items-center gap-1.5 rounded-md border border-black/[0.08] bg-[#FAFAFB] px-2 py-0.5 text-[11px] font-medium text-[#1D1D1F]">
                      <span
                        className="h-1.5 w-1.5 rounded-full"
                        style={{ backgroundColor: visual.color }}
                      />
                      {item.platform}
                    </span>
                    <span className="truncate text-xs text-[#6E6E73]">{item.accountAlias}</span>
                  </div>
                  <div>
                    <strong className="block text-xs font-semibold tabular-nums text-[#1D1D1F]">
                      {item.readsLabel}
                    </strong>
                    <small className="text-[11px] text-[#8E8E93]">{item.readsMetric}</small>
                  </div>
                  <div className="text-xs text-[#6E6E73]">
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
