import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
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
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { useOperationsStore, usePlatformStore, useUiStore, useWorkspaceStore } from "../stores";
import type { ContentKind } from "../types/content";
import type { AnalyticsRange } from "../types/operations";
import {
  computeDashboardView,
  contentKindText,
  formatMetricNumber,
  primaryMetric,
  PUBLISH_STATUS_META,
  workStatusLabel,
} from "../utils/operations";

const kindOptions: Array<{ value: "ALL" | ContentKind; label: string }> = [
  { value: "ALL", label: "全部形态" },
  { value: "video", label: "视频" },
  { value: "imageText", label: "图文" },
  { value: "imageArticle", label: "图文文章" },
  { value: "article", label: "纯文章" },
];

export function DashboardPage() {
  const navigate = useNavigate();
  const documents = useWorkspaceStore((state) => state.documents);
  const selectDocumentSafely = useWorkspaceStore((state) => state.selectDocumentSafely);
  const setIsCreating = useUiStore((state) => state.setIsCreating);
  const accounts = usePlatformStore((state) => state.accounts);
  const initializeAccounts = usePlatformStore((state) => state.initialize);
  const tasks = useOperationsStore((state) => state.tasks);
  const works = useOperationsStore((state) => state.works);
  const syncRuns = useOperationsStore((state) => state.syncRuns);
  const backendDashboard = useOperationsStore((state) => state.dashboard);
  const initializeOperations = useOperationsStore((state) => state.initialize);
  const loadDashboard = useOperationsStore((state) => state.loadDashboard);
  const [range, setRange] = useState<AnalyticsRange>("7d");
  const [platformId, setPlatformId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [contentKind, setContentKind] = useState<"ALL" | ContentKind>("ALL");

  useEffect(() => {
    void initializeAccounts();
    void initializeOperations();
  }, [initializeAccounts, initializeOperations]);

  useEffect(() => {
    void loadDashboard({
      range,
      platformId: platformId || null,
      accountId: accountId || null,
      contentKind: contentKind === "ALL" ? null : contentKind,
    });
  }, [accountId, contentKind, loadDashboard, platformId, range]);

  const localDashboard = useMemo(
    () =>
      computeDashboardView({
        works,
        tasks,
        accounts,
        syncRuns,
        filter: {
          range,
          platformId: platformId || null,
          accountId: accountId || null,
          contentKind: contentKind === "ALL" ? null : contentKind,
        },
      }),
    [accountId, accounts, contentKind, platformId, range, syncRuns, tasks, works],
  );
  const dashboard = backendDashboard ?? localDashboard;
  const issueAccounts = accounts.filter(
    (account) => account.status !== "ACTIVE" || account.enabled === false,
  );
  const latestDoc = documents[0] ?? null;
  const visibleAccounts = accounts.filter(
    (account) => !platformId || account.platformId === platformId,
  );

  const metrics = [
    {
      label: "平台 / 账号",
      value: `${dashboard.platformCount} / ${dashboard.accountCount}`,
      note: "筛选范围内的平台与账号",
      icon: Globe2,
      accentClass: "text-accent",
    },
    {
      label: "近期发布",
      value: String(dashboard.recentPublishCount),
      note: "成功归档的外部发布",
      icon: Send,
      accentClass: "text-mint-text",
    },
    {
      label: "待处理任务",
      value: String(dashboard.pendingTaskCount),
      note: "确认、定时、人工或暂停任务",
      icon: BookOpen,
      accentClass: "text-lemon-text",
    },
    {
      label: "同步异常",
      value: String(dashboard.syncErrorCount),
      note: "失败、部分失败或账号失效",
      icon: AlertTriangle,
      accentClass: dashboard.syncErrorCount > 0 ? "text-coral-text" : "text-ink-muted",
    },
  ];

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="DASHBOARD"
        title="运营总览"
        description="从账号健康、发布任务、同步异常与真实作品指标出发，快速定位下一步操作。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md bg-accent px-3 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover"
            onClick={() => setIsCreating(true)}
          >
            <Plus size={13} />
            新建内容
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
        {/* Filter Toolbar */}
        <div className="flex flex-wrap items-center gap-2">
          <select
            value={range}
            onChange={(event) => setRange(event.target.value as AnalyticsRange)}
            className="h-8 rounded-md border border-line bg-surface px-2.5 text-xs text-ink-secondary transition-colors hover:border-line-strong focus:border-accent focus:outline-none"
          >
            <option value="7d">近 7 天</option>
            <option value="30d">近 30 天</option>
            <option value="90d">近 90 天</option>
          </select>
          <select
            value={platformId}
            onChange={(event) => {
              setPlatformId(event.target.value);
              setAccountId("");
            }}
            className="h-8 rounded-md border border-line bg-surface px-2.5 text-xs text-ink-secondary transition-colors hover:border-line-strong focus:border-accent focus:outline-none"
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
            className="h-8 rounded-md border border-line bg-surface px-2.5 text-xs text-ink-secondary transition-colors hover:border-line-strong focus:border-accent focus:outline-none"
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
            className="h-8 rounded-md border border-line bg-surface px-2.5 text-xs text-ink-secondary transition-colors hover:border-line-strong focus:border-accent focus:outline-none"
          >
            {kindOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <span className="ml-auto font-mono text-[11px] text-ink-muted">
            {dashboard.filter.from.slice(0, 10)} — {dashboard.filter.to.slice(0, 10)}
          </span>
        </div>

        {/* Architectural Connected KPI Strip */}
        <section className="grid grid-cols-2 divide-y divide-line overflow-hidden rounded-lg border border-line bg-surface lg:grid-cols-4 lg:divide-x lg:divide-y-0">
          {metrics.map((card) => {
            const Icon = card.icon;
            return (
              <article key={card.label} className="px-4 py-3.5">
                <div className="flex items-center justify-between gap-2">
                  <span className="text-[11px] font-medium text-ink-muted">{card.label}</span>
                  <Icon size={13} className={card.accentClass} />
                </div>
                <strong className="mt-1.5 block text-[22px] leading-tight font-semibold tracking-tight tabular-nums text-ink">
                  {card.value}
                </strong>
                <p className="mt-1 truncate text-[11px] text-ink-secondary">{card.note}</p>
              </article>
            );
          })}
        </section>

        {/* Structured Pipeline Panel */}
        <section className="overflow-hidden rounded-lg border border-line bg-surface">
          <div className="flex flex-wrap items-center justify-between gap-3 border-b border-line px-4 py-3">
            <div>
              <h2 className="text-[13px] font-semibold text-ink">创作与分发接力</h2>
              <p className="mt-0.5 text-[11.5px] text-ink-muted">
                主稿保持本地，平台稿独立修改；发布前必须确认最终内容包。
              </p>
            </div>
            <div className="flex items-center gap-2">
              {latestDoc && (
                <button
                  className="inline-flex h-7.5 max-w-64 cursor-pointer items-center gap-1.5 rounded-md border border-line-strong bg-surface px-2.5 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle hover:text-ink"
                  onClick={async () => {
                    const ok = await selectDocumentSafely(latestDoc);
                    if (ok) navigate("/studio");
                  }}
                >
                  <FileText size={12} />
                  <span className="truncate">继续写：{latestDoc.title}</span>
                </button>
              )}
              <button
                className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-md bg-accent px-2.5 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover"
                onClick={() => navigate("/publishing")}
              >
                <Send size={12} /> 发布队列 ({dashboard.pendingTaskCount})
              </button>
            </div>
          </div>

          <div className="grid divide-y divide-line md:grid-cols-3 md:divide-x md:divide-y-0">
            <button
              className="group cursor-pointer px-4 py-3.5 text-left transition-colors hover:bg-subtle/70"
              onClick={() => navigate("/studio")}
            >
              <div className="flex items-center justify-between">
                <span className="font-mono text-[10.5px] font-semibold tracking-wider text-accent">
                  01 / 主稿
                </span>
                <ArrowRight
                  size={12}
                  className="text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-ink-secondary"
                />
              </div>
              <strong className="mt-1.5 block text-[13px] font-medium text-ink">
                在 Studio 准备四类内容
              </strong>
              <p className="mt-0.5 text-[11.5px] leading-relaxed text-ink-secondary">
                视频、图文、图文文章和纯文章分别保留完整媒体与正文。
              </p>
            </button>

            <button
              className="group cursor-pointer px-4 py-3.5 text-left transition-colors hover:bg-subtle/70"
              onClick={() => navigate("/publishing")}
            >
              <div className="flex items-center justify-between">
                <span className="font-mono text-[10.5px] font-semibold tracking-wider text-ink-muted">
                  02 / 确认
                </span>
                <ArrowRight
                  size={12}
                  className="text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-ink-secondary"
                />
              </div>
              <strong className="mt-1.5 block text-[13px] font-medium text-ink">
                预览最终内容包与目标
              </strong>
              <p className="mt-0.5 text-[11.5px] leading-relaxed text-ink-secondary">
                未确认账号、媒体顺序、封面或正文时不会执行外部发布。
              </p>
            </button>

            <button
              className="group cursor-pointer px-4 py-3.5 text-left transition-colors hover:bg-subtle/70"
              onClick={() => navigate("/analytics")}
            >
              <div className="flex items-center justify-between">
                <span className="font-mono text-[10.5px] font-semibold tracking-wider text-ink-muted">
                  03 / 复盘
                </span>
                <ArrowRight
                  size={12}
                  className="text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-ink-secondary"
                />
              </div>
              <strong className="mt-1.5 block text-[13px] font-medium text-ink">
                同步作品并保留原始指标
              </strong>
              <p className="mt-0.5 text-[11.5px] leading-relaxed text-ink-secondary">
                同名指标不跨平台相加；无采样时明确显示未知。
              </p>
            </button>
          </div>
        </section>

        {/* Structured Two-Column Data Panels */}
        <section className="grid gap-4 xl:grid-cols-[1.15fr_0.85fr]">
          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center justify-between border-b border-line px-4 py-2.5">
              <h2 className="text-[13px] font-semibold text-ink">近期外部作品</h2>
              <button
                className="cursor-pointer text-[11.5px] font-medium text-accent transition-colors hover:text-accent-deep"
                onClick={() => navigate("/content")}
              >
                查看全部作品
              </button>
            </div>
            <div className="divide-y divide-line">
              {dashboard.recentWorks.slice(0, 5).map((work) => {
                const metric = primaryMetric(work);
                return (
                  <div
                    key={work.id}
                    className="flex items-center gap-3 px-4 py-2.5 transition-colors hover:bg-subtle/60"
                  >
                    <PlatformLogo platformName={work.platformName} size={18} />
                    <div className="min-w-0 flex-1">
                      <strong className="block truncate text-[12.5px] font-medium text-ink">
                        {work.title}
                      </strong>
                      <small className="block truncate text-[11px] text-ink-muted">
                        {work.platformName} · {work.accountAlias} ·{" "}
                        {contentKindText(work.contentKind)} · {workStatusLabel(work)}
                      </small>
                    </div>
                    <div className="shrink-0 text-right">
                      <strong className="block text-xs font-semibold tabular-nums text-ink">
                        {metric ? `${formatMetricNumber(metric.value)} ${metric.unit}` : "无采样"}
                      </strong>
                      <small className="text-[10.5px] text-ink-muted">
                        {metric?.metricName ?? "等待同步"}
                      </small>
                    </div>
                  </div>
                );
              })}
              {!dashboard.recentWorks.length && (
                <div className="px-4 py-10 text-center text-xs text-ink-muted">
                  当前筛选范围内没有作品；不会展示伪造数据。
                </div>
              )}
            </div>
          </article>

          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center justify-between border-b border-line px-4 py-2.5">
              <h2 className="text-[13px] font-semibold text-ink">待处理与风险</h2>
              <button
                className="cursor-pointer text-[11.5px] font-medium text-accent transition-colors hover:text-accent-deep"
                onClick={() => navigate("/platforms")}
              >
                管理平台
              </button>
            </div>
            <div className="divide-y divide-line">
              {issueAccounts.slice(0, 3).map((account) => (
                <div
                  key={account.id}
                  className="flex items-center gap-3 bg-lemon-bg/50 px-4 py-2.5"
                >
                  <AlertTriangle size={14} className="shrink-0 text-lemon-text" />
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-[12.5px] font-medium text-ink">
                      {account.platformName} · {account.alias}
                    </strong>
                    <small className="block truncate text-[11px] text-ink-secondary">
                      {account.status} · {account.lastError ?? "需要重新授权或恢复"}
                    </small>
                  </div>
                  <button
                    className="h-7 shrink-0 cursor-pointer rounded-md border border-lemon-border bg-surface px-2.5 text-[11.5px] font-medium text-lemon-text transition-colors hover:bg-lemon-bg"
                    onClick={() => navigate("/platforms")}
                  >
                    去处理
                  </button>
                </div>
              ))}
              {dashboard.pendingTasks.slice(0, 4).map((task) => {
                const meta = PUBLISH_STATUS_META[task.status];
                return (
                  <button
                    key={task.id}
                    className="flex w-full cursor-pointer items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-subtle/60"
                    onClick={() => navigate("/publishing")}
                  >
                    <PlatformLogo platformName={task.platformName} size={18} />
                    <div className="min-w-0 flex-1">
                      <strong className="block truncate text-[12.5px] font-medium text-ink">
                        {task.title}
                      </strong>
                      <small className="block truncate text-[11px] text-ink-muted">
                        {task.platformName} · {task.accountAlias} · {meta.label}
                      </small>
                    </div>
                    <ArrowRight size={12} className="shrink-0 text-ink-muted" />
                  </button>
                );
              })}
              {!issueAccounts.length && !dashboard.pendingTasks.length && (
                <div className="flex items-center gap-2 px-4 py-4 text-xs text-mint-text">
                  <CheckCircle2 size={14} /> 当前筛选范围内没有账号或发布风险
                </div>
              )}
            </div>
          </article>
        </section>

        <div className="flex items-center justify-end gap-2 text-[11px] text-ink-muted">
          <BarChart3 size={12} />
          <span>指标趋势与采集时间可在数据分析中逐作品核对。</span>
          <button
            className="cursor-pointer font-medium text-accent hover:text-accent-deep"
            onClick={() => navigate("/analytics")}
          >
            打开分析 →
          </button>
        </div>
      </div>
    </main>
  );
}
