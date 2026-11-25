import { useEffect, useMemo, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, RefreshCw, Search, X } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { useOperationsStore, usePlatformStore, usePluginStore, useUiStore } from "../stores";
import type { ContentKind } from "../types/content";
import {
  contentKindText,
  filterExternalWorks,
  formatMetricNumber,
  primaryMetric,
  workStatusLabel,
} from "../utils/operations";

const KIND_OPTIONS: Array<{ value: "ALL" | ContentKind | "unknown"; label: string }> = [
  { value: "ALL", label: "全部形态" },
  { value: "video", label: "视频" },
  { value: "imageText", label: "图文" },
  { value: "imageArticle", label: "图文文章" },
  { value: "article", label: "纯文章" },
  { value: "unknown", label: "未知形态" },
];

function formatDate(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleDateString("zh-CN");
}

export function ContentPage() {
  const showToast = useUiStore((state) => state.showToast);
  const accounts = usePlatformStore((state) => state.accounts);
  const initializeAccounts = usePlatformStore((state) => state.initialize);
  const plugins = usePluginStore((state) => state.plugins);
  const initializePlugins = usePluginStore((state) => state.initialize);
  const works = useOperationsStore((state) => state.works);
  const initializeOperations = useOperationsStore((state) => state.initialize);
  const loadWorks = useOperationsStore((state) => state.loadWorks);
  const syncWorks = useOperationsStore((state) => state.syncWorks);
  const selectedWork = useOperationsStore((state) => state.selectedWork);
  const selectWork = useOperationsStore((state) => state.selectWork);
  const [keyword, setKeyword] = useState("");
  const [platformId, setPlatformId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [contentKind, setContentKind] = useState<"ALL" | ContentKind | "unknown">("ALL");
  const [status, setStatus] = useState<"ALL" | "published" | "manual" | "synced">("ALL");
  const [range, setRange] = useState<"ALL" | "7d" | "30d" | "90d">("ALL");
  const [syncAccountId, setSyncAccountId] = useState("");
  const [syncAdapterId, setSyncAdapterId] = useState("");
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    void initializeAccounts();
    void initializePlugins();
    void initializeOperations();
  }, [initializeAccounts, initializeOperations, initializePlugins]);

  useEffect(() => {
    if (!syncAccountId && accounts[0]) setSyncAccountId(accounts[0].id);
  }, [accounts, syncAccountId]);

  const enabledAdapters = useMemo(
    () =>
      plugins.filter(
        (plugin) =>
          plugin.status === "ENABLED" &&
          plugin.capabilities.some(
            (capability) => capability.effectiveRead === "supported",
          ),
      ),
    [plugins],
  );

  useEffect(() => {
    if (!syncAdapterId && enabledAdapters[0]) setSyncAdapterId(enabledAdapters[0].id);
  }, [enabledAdapters, syncAdapterId]);

  const filtered = useMemo(
    () =>
      filterExternalWorks(works, {
        keyword,
        platformId: platformId || undefined,
        accountId: accountId || undefined,
        contentKind,
        status,
        range,
      }),
    [accountId, contentKind, keyword, platformId, range, status, works],
  );

  const visibleAccounts = accounts.filter(
    (account) => !platformId || account.platformId === platformId,
  );

  async function handleSync() {
    const account = accounts.find((item) => item.id === syncAccountId);
    const adapter = enabledAdapters.find((item) => item.id === syncAdapterId);
    if (!account || !adapter) {
      showToast("请选择 ACTIVE 平台账号和已启用且可读取作品的适配器");
      return;
    }
    setSyncing(true);
    try {
      const result = await syncWorks({
        accountId: account.id,
        platformId: account.platformId,
        platformName: account.platformName,
        accountAlias: account.alias,
        adapterId: adapter.id,
        asOf: null,
      });
      if (result) {
        showToast(result.accountInvalid ? "账号已失效，请重新授权后恢复任务" : result.message);
        await initializeOperations(true);
      }
    } finally {
      setSyncing(false);
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="CONTENT"
        title="内容中心"
        description="统一查看发布和同步的外部作品；保留平台原始类型与指标口径，不把不同形态强行合并。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
            onClick={() => void loadWorks()}
          >
            <RefreshCw size={12} /> 刷新
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-4 overflow-y-auto px-7 py-6">
        <section className="rounded-2xl border border-line bg-surface p-4 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-end">
            <div className="min-w-0 flex-1">
              <h2 className="text-xs font-semibold text-ink">平台作品同步</h2>
              <p className="mt-0.5 text-[11px] text-ink-muted">
                同一作品按平台外部 ID 去重；同一作品同一指标每个 UTC 日只保留最新样本。
              </p>
            </div>
            <select
              value={syncAccountId}
              onChange={(event) => setSyncAccountId(event.target.value)}
              className="h-8 min-w-44 rounded-lg border border-line bg-canvas px-2 text-xs text-ink"
            >
              <option value="">选择平台账号</option>
              {accounts.map((account) => (
                <option key={account.id} value={account.id}>
                  {account.platformName} · {account.alias} · {account.status}
                </option>
              ))}
            </select>
            <select
              value={syncAdapterId}
              onChange={(event) => setSyncAdapterId(event.target.value)}
              className="h-8 min-w-48 rounded-lg border border-line bg-canvas px-2 text-xs text-ink"
            >
              <option value="">选择读取适配器</option>
              {enabledAdapters.map((adapter) => (
                <option key={adapter.id} value={adapter.id}>
                  {adapter.name}
                </option>
              ))}
            </select>
            <button
              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-50"
              disabled={syncing || !syncAccountId || !syncAdapterId}
              onClick={() => void handleSync()}
            >
              <RefreshCw size={12} className={syncing ? "animate-spin" : ""} />
              {syncing ? "同步中" : "同步作品与指标"}
            </button>
          </div>
        </section>

        <div className="flex flex-wrap items-center gap-2">
          <div className="flex h-8.5 w-full max-w-sm items-center gap-2 rounded-xl border border-line bg-surface px-3 text-ink-muted focus-within:border-accent">
            <Search size={13} className="shrink-0" />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号、原始类型或外部 ID 检索"
              aria-label="检索跨平台作品"
              className="min-w-0 flex-1 bg-transparent text-xs text-ink placeholder:text-ink-muted focus:outline-none"
            />
          </div>
          <select
            value={platformId}
            onChange={(event) => {
              setPlatformId(event.target.value);
              setAccountId("");
            }}
            className="h-8 rounded-lg border border-line bg-surface px-2 text-xs text-ink-secondary"
          >
            <option value="">全部平台</option>
            {Array.from(new Map(accounts.map((account) => [account.platformId, account.platformName]))).map(
              ([id, name]) => (
                <option key={id} value={id}>
                  {name}
                </option>
              ),
            )}
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
            {KIND_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <select
            value={status}
            onChange={(event) => setStatus(event.target.value as typeof status)}
            className="h-8 rounded-lg border border-line bg-surface px-2 text-xs text-ink-secondary"
          >
            <option value="ALL">全部状态</option>
            <option value="published">已发布</option>
            <option value="manual">人工发布</option>
            <option value="synced">已同步</option>
          </select>
          <select
            value={range}
            onChange={(event) => setRange(event.target.value as typeof range)}
            className="h-8 rounded-lg border border-line bg-surface px-2 text-xs text-ink-secondary"
          >
            <option value="ALL">全部时间</option>
            <option value="7d">近 7 天</option>
            <option value="30d">近 30 天</option>
            <option value="90d">近 90 天</option>
          </select>
        </div>

        <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="divide-y divide-line">
            <div className="grid grid-cols-[minmax(220px,2fr)_minmax(160px,1.2fr)_minmax(180px,1.4fr)_minmax(80px,0.6fr)] gap-4 pb-2.5 text-xs font-medium text-ink-muted">
              <span>作品 / 外部 ID</span>
              <span>平台 / 账号 / 形态</span>
              <span>主指标（原文口径）</span>
              <span>详情</span>
            </div>
            {filtered.map((item) => {
              const metric = primaryMetric(item);
              return (
                <div
                  key={item.id}
                  className="grid grid-cols-[minmax(220px,2fr)_minmax(160px,1.2fr)_minmax(180px,1.4fr)_minmax(80px,0.6fr)] items-center gap-4 py-3.5"
                >
                  <div className="min-w-0">
                    <strong className="block truncate text-xs font-medium text-ink">{item.title}</strong>
                    <small className="text-[11px] text-ink-muted">
                      {item.externalId} · {formatDate(item.publishedAt)} · {workStatusLabel(item)}
                    </small>
                  </div>
                  <div className="space-y-1">
                    <div className="flex flex-wrap gap-1.5">
                      <span className="inline-flex items-center gap-1 rounded-md bg-sidebar px-2 py-0.5 text-[11px] text-ink-secondary">
                        <PlatformLogo platformName={item.platformName} size={13} />
                        {item.platformName}
                      </span>
                      <span className="rounded-md bg-sidebar px-2 py-0.5 text-[11px] text-ink-secondary">
                        {contentKindText(item.contentKind)}
                      </span>
                      {item.contentKind === "unknown" && (
                        <span className="rounded-md bg-lemon-bg px-2 py-0.5 text-[11px] text-lemon-text">
                          原始：{item.originalContentType}
                        </span>
                      )}
                    </div>
                    <small className="block text-[11px] text-ink-muted">{item.accountAlias}</small>
                  </div>
                  <div>
                    {metric ? (
                      <>
                        <strong className="block text-xs font-semibold tabular-nums text-ink">
                          {formatMetricNumber(metric.value)} {metric.unit}
                        </strong>
                        <small className="block text-[11px] text-ink-muted">
                          {metric.metricName} · 采集 {formatDate(metric.capturedAt)}
                        </small>
                      </>
                    ) : (
                      <small className="text-[11px] text-ink-muted">尚无指标采样</small>
                    )}
                  </div>
                  <div>
                    <button
                      className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg border border-line bg-surface-soft px-2.5 text-xs font-medium text-ink hover:border-accent-border hover:bg-accent-soft hover:text-accent-deep"
                      onClick={() => selectWork(item)}
                    >
                      <ExternalLink size={11} /> 详情
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
          {!filtered.length && (
            <div className="py-12 text-center text-xs text-ink-muted">
              当前筛选条件下没有作品。可先同步账号作品，或调整平台、形态与时间筛选。
            </div>
          )}
        </article>
      </div>

      {selectedWork && (
        <div className="fixed inset-0 z-50 flex justify-end bg-ink/30 backdrop-blur-sm">
          <aside className="h-full w-full max-w-xl overflow-y-auto border-l border-line bg-surface p-6 shadow-2xl">
            <div className="flex items-start justify-between gap-4">
              <div>
                <p className="text-[11px] font-semibold tracking-[0.16em] text-accent">作品详情</p>
                <h2 className="mt-1 text-lg font-semibold text-ink">{selectedWork.title}</h2>
                <p className="mt-1 text-xs text-ink-secondary">
                  {selectedWork.platformName} · {selectedWork.accountAlias}
                </p>
              </div>
              <button className="cursor-pointer text-ink-muted hover:text-ink" onClick={() => selectWork(null)}>
                <X size={18} />
              </button>
            </div>
            <div className="mt-5 grid grid-cols-2 gap-2 text-xs">
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">外部作品 ID</span>
                <strong className="mt-1 block break-all font-mono text-xs text-ink">{selectedWork.externalId}</strong>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">来源</span>
                <strong className="mt-1 block text-ink">{selectedWork.source}</strong>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">映射形态</span>
                <strong className="mt-1 block text-ink">{contentKindText(selectedWork.contentKind)}</strong>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">平台原始类型</span>
                <strong className="mt-1 block text-ink">{selectedWork.originalContentType}</strong>
              </div>
            </div>
            {selectedWork.url && (
              <button
                className="mt-3 inline-flex h-8 w-full cursor-pointer items-center justify-center gap-1.5 rounded-xl border border-accent-border bg-accent-soft text-xs font-medium text-accent-deep"
                onClick={() => void openUrl(selectedWork.url!)}
              >
                <ExternalLink size={12} /> 打开外部作品
              </button>
            )}
            <h3 className="mt-6 text-xs font-semibold text-ink">平台原始指标快照</h3>
            <div className="mt-2 space-y-2">
              {selectedWork.metrics.map((metric) => (
                <div key={metric.id} className="rounded-xl border border-line bg-canvas p-3">
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <strong className="text-xs text-ink">{metric.metricName}</strong>
                      <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
                        {metric.metricDefinition || "平台未提供定义"}
                      </p>
                    </div>
                    <span className="shrink-0 text-sm font-semibold tabular-nums text-ink">
                      {formatMetricNumber(metric.value)} {metric.unit}
                    </span>
                  </div>
                  <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[10px] text-ink-muted">
                    <span>采集 {new Date(metric.capturedAt).toLocaleString("zh-CN")}</span>
                    <span>来源 {metric.source}</span>
                    <span>{metric.comparable ? "有统一比较口径" : "不可跨平台聚合"}</span>
                  </div>
                </div>
              ))}
              {!selectedWork.metrics.length && (
                <p className="rounded-xl border border-line bg-canvas p-3 text-xs text-ink-muted">
                  此作品尚无指标采样；不会显示伪造的 0 值。
                </p>
              )}
            </div>
          </aside>
        </div>
      )}
    </main>
  );
}
