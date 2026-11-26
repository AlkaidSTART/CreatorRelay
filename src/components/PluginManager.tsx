import { useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Plug,
  RefreshCw,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Trash2,
  X,
} from "./Icons";
import { usePluginStore, useUiStore } from "../stores";
import type {
  AdapterCatalogEntry,
  CapabilityLevel,
  PluginEvent,
  PluginPermissionDecision,
  PluginProbeResult,
  PluginPublishValidationResult,
  PluginStatus,
  PluginSummary,
  PublishInput,
  PublishPayloadCheckResult,
  SecuritySelfTestResult,
} from "../types/plugin";

const statusCopy: Record<PluginStatus, { label: string; className: string; description: string }> =
  {
    REVIEW_REQUIRED: {
      label: "待审查",
      className: "border-amber-200 bg-amber-50 text-amber-700",
      description: "需要逐项确认来源、版本、权限和能力后才能启用",
    },
    READY: {
      label: "可启用",
      className: "border-emerald-200 bg-emerald-50 text-emerald-700",
      description: "权限审查已完成，可以启动独立进程",
    },
    ENABLED: {
      label: "已启用",
      className: "border-emerald-200 bg-emerald-50 text-emerald-700",
      description: "独立进程正在运行，能力状态来自最近一次探测",
    },
    DISABLED: {
      label: "已禁用",
      className: "border-line bg-subtle text-ink-muted",
      description: "插件进程已停止，配置与审计记录保留",
    },
    ERROR: {
      label: "运行错误",
      className: "border-rose-200 bg-rose-50 text-rose-700",
      description: "探测、崩溃或超时导致插件不可用；可查看错误并重启",
    },
  };

const levelCopy: Record<
  CapabilityLevel,
  { label: string; className: string; description: string }
> = {
  supported: {
    label: "可用",
    className: "border-emerald-200 bg-emerald-50 text-emerald-700",
    description: "运行时探测通过",
  },
  unsupported: {
    label: "不支持",
    className: "border-line bg-subtle text-ink-muted",
    description: "该账号不支持此形态",
  },
  restricted: {
    label: "受限",
    className: "border-amber-200 bg-amber-50 text-amber-700",
    description: "需要额外授权或人工审核，不能视为完全可用",
  },
  experimental: {
    label: "实验",
    className: "border-sky-200 bg-sky-50 text-sky-700",
    description: "实验能力，结果可能不稳定",
  },
  unavailable: {
    label: "不可用",
    className: "border-rose-200 bg-rose-50 text-rose-700",
    description: "运行时检查失败，不采用 Manifest 声明",
  },
};

const permissionRiskCopy = {
  medium: { label: "中风险", className: "border-amber-200 bg-amber-50 text-amber-700" },
  high: { label: "高风险", className: "border-rose-200 bg-rose-50 text-rose-700" },
} as const;

const contentKindOptions = [
  { value: "fixture-image-text", label: "测试账号 · 图文可用" },
  { value: "fixture-article", label: "测试账号 · 纯文章（模拟）" },
];

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function formatDate(value?: string | null) {
  if (!value) return "尚未探测";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function Badge({ children, className }: { children: React.ReactNode; className: string }) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-lg border px-2 py-0.5 text-[11px] font-medium ${className}`}
    >
      {children}
    </span>
  );
}

function CapabilityRow({
  capability,
  compact = false,
}: {
  capability: PluginSummary["capabilities"][number];
  compact?: boolean;
}) {
  const publish = levelCopy[capability.effectivePublish];
  const read = levelCopy[capability.effectiveRead];
  return (
    <div
      className={`grid items-center gap-2 rounded-xl border border-line bg-surface px-3 ${
        compact ? "py-2" : "py-2.5"
      }`}
      style={{ gridTemplateColumns: "minmax(72px,1fr) minmax(74px,0.8fr) minmax(84px,0.8fr)" }}
    >
      <div className="flex items-center gap-2">
        <span className="text-xs font-medium text-ink">{capability.label}</span>
        {capability.source === "probe" ? (
          <span className="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] text-accent-deep">
            探测
          </span>
        ) : (
          <span className="rounded bg-subtle px-1.5 py-0.5 text-[10px] text-ink-muted">声明</span>
        )}
      </div>
      <Badge className={read.className}>读 · {read.label}</Badge>
      <Badge className={publish.className}>发 · {publish.label}</Badge>
      {capability.reason && (
        <p className="col-span-3 text-[11px] leading-relaxed text-ink-muted">{capability.reason}</p>
      )}
    </div>
  );
}

function ModalShell({
  children,
  onClose,
  label,
  width = "680px",
}: {
  children: React.ReactNode;
  onClose: () => void;
  label: string;
  width?: string;
}) {
  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/30 p-4 backdrop-blur-[2px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      role="presentation"
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-label={label}
        className="relative max-h-[min(860px,calc(100vh-32px))] w-full overflow-y-auto rounded-2xl border border-line bg-surface-soft p-6 shadow-xl"
        style={{ maxWidth: width }}
      >
        {children}
        <button
          type="button"
          onClick={onClose}
          aria-label="关闭"
          className="absolute top-4 right-4 grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-muted transition-colors hover:bg-sidebar hover:text-ink"
        >
          <X size={14} />
        </button>
      </section>
    </div>
  );
}

function CatalogCard({
  entry,
  busy,
  onInstall,
}: {
  entry: AdapterCatalogEntry;
  busy: boolean;
  onInstall: () => void;
}) {
  return (
    <article className="flex flex-col gap-3 rounded-2xl border border-line bg-surface p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-3">
          <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent-deep">
            <Plug size={16} />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-1.5">
              <h3 className="text-sm font-semibold text-ink">{entry.name}</h3>
              <Badge className="border-accent/30 bg-accent-soft text-accent-deep">
                官方 · v{entry.version}
              </Badge>
              <Badge className="border-line bg-subtle text-ink-muted">
                合约 v{entry.adapterContractVersion}
              </Badge>
            </div>
            <p className="mt-1 line-clamp-2 text-xs leading-relaxed text-ink-secondary">
              {entry.description}
            </p>
          </div>
        </div>
        <button
          type="button"
          disabled={!entry.installable || busy}
          onClick={onInstall}
          className="inline-flex h-8 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Plug size={12} />
          {entry.installed ? "已安装" : busy ? "安装中" : "安装"}
        </button>
      </div>
      <div className="flex flex-wrap gap-1.5">
        <Badge className="border-line bg-subtle text-ink-muted">
          {entry.source === "BUNDLED" ? "随应用分发" : "本地来源"}
        </Badge>
        <Badge
          className={
            entry.isolation.strong
              ? "border-emerald-200 bg-emerald-50 text-emerald-700"
              : "border-amber-200 bg-amber-50 text-amber-700"
          }
        >
          {entry.isolation.strong ? "强隔离" : "独立进程"}
        </Badge>
        {entry.permissions.map((permission) => (
          <Badge key={permission.id} className="border-line bg-subtle text-ink-muted">
            {permission.label}
          </Badge>
        ))}
      </div>
      {entry.blockedReason && <p className="text-[11px] text-ink-muted">{entry.blockedReason}</p>}
    </article>
  );
}

function PluginCard({
  plugin,
  accountRef,
  busy,
  probe,
  security,
  events,
  onReview,
  onEnable,
  onDisable,
  onRestart,
  onProbe,
  onSecurityTest,
  onUninstall,
  onRollback,
  onFault,
  onShowEvents,
}: {
  plugin: PluginSummary;
  accountRef: string;
  busy: boolean;
  probe?: PluginProbeResult;
  security?: SecuritySelfTestResult;
  events?: PluginEvent[];
  onReview: () => void;
  onEnable: () => void;
  onDisable: () => void;
  onRestart: () => void;
  onProbe: () => void;
  onSecurityTest: () => void;
  onUninstall: () => void;
  onRollback: () => void;
  onFault: (fault: "crash" | "timeout") => void;
  onShowEvents: () => void;
}) {
  const status = statusCopy[plugin.status];
  const displayCapabilities = probe?.capabilities ?? plugin.capabilities;
  const displayAccount = probe?.accountRef ?? accountRef;
  const reviewDone = plugin.permissions.every(
    (permission) => typeof permission.granted === "boolean",
  );
  const requiredGranted = plugin.permissions
    .filter((permission) => permission.required)
    .every((permission) => permission.granted === true);

  return (
    <article className="flex flex-col gap-4 rounded-2xl border border-line bg-surface p-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-line bg-white text-accent-deep">
            <Plug size={16} />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-1.5">
              <h3 className="text-sm font-semibold text-ink">{plugin.name}</h3>
              <Badge className={status.className}>{status.label}</Badge>
              <Badge className="border-line bg-subtle text-ink-muted">v{plugin.version}</Badge>
              <Badge className="border-line bg-subtle text-ink-muted">
                合约 v{plugin.adapterContractVersion}
              </Badge>
              {plugin.sandboxed ? (
                <Badge className="border-emerald-200 bg-emerald-50 text-emerald-700">
                  <ShieldCheck size={10} /> 沙箱
                </Badge>
              ) : (
                <Badge className="border-amber-200 bg-amber-50 text-amber-700">
                  <ShieldAlert size={10} /> 进程隔离
                </Badge>
              )}
            </div>
            <p className="mt-1 text-xs leading-relaxed text-ink-secondary">{status.description}</p>
            <p className="mt-1 text-[11px] text-ink-muted">
              来源 {plugin.source === "BUNDLED" ? "官方分发" : "本地"} · 信任{" "}
              {plugin.trustLevel === "OFFICIAL" ? "官方" : "未受信任"} · 进程{" "}
              {plugin.runtimeState === "running" ? "运行中" : "已停止"} · 最近探测{" "}
              {formatDate(plugin.lastProbeAt)}
            </p>
          </div>
        </div>
        <div className="flex flex-wrap justify-end gap-1.5">
          <button
            type="button"
            onClick={onReview}
            disabled={busy || plugin.enabled}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-2.5 text-[11px] font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:cursor-not-allowed disabled:opacity-50"
          >
            <Shield size={11} />
            {reviewDone ? "查看权限" : "权限审查"}
          </button>
          {plugin.status === "ENABLED" ? (
            <button
              type="button"
              onClick={onDisable}
              disabled={busy}
              className="h-8 cursor-pointer rounded-xl border border-line bg-surface px-2.5 text-[11px] font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:opacity-50"
            >
              禁用
            </button>
          ) : (
            <button
              type="button"
              onClick={onEnable}
              disabled={busy || !reviewDone || !requiredGranted}
              className="h-8 cursor-pointer rounded-xl bg-accent px-2.5 text-[11px] font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-50"
            >
              启用
            </button>
          )}
          <button
            type="button"
            onClick={onRestart}
            disabled={busy || !plugin.enabled}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-2.5 text-[11px] font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:cursor-not-allowed disabled:opacity-50"
          >
            <RefreshCw size={11} />
            重启
          </button>
          {plugin.previousVersion && (
            <button
              type="button"
              onClick={onRollback}
              disabled={busy || plugin.enabled}
              className="h-8 cursor-pointer rounded-xl border border-line bg-surface px-2.5 text-[11px] font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:opacity-50"
            >
              回退 v{plugin.previousVersion}
            </button>
          )}
          <button
            type="button"
            onClick={onUninstall}
            disabled={busy}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-2.5 text-[11px] font-medium text-rose-600 transition-colors hover:bg-rose-50 disabled:opacity-50"
          >
            <Trash2 size={11} />
            卸载
          </button>
        </div>
      </div>

      {plugin.lastError && (
        <div className="flex items-start gap-2 rounded-xl border border-rose-200 bg-rose-50 px-3 py-2.5 text-xs text-rose-700">
          <AlertTriangle size={13} className="mt-0.5 shrink-0" />
          <div className="min-w-0 flex-1">
            <p className="font-medium">运行错误</p>
            <p className="mt-0.5 break-words leading-relaxed">{plugin.lastError}</p>
          </div>
        </div>
      )}

      <section className="space-y-2" aria-label={`${plugin.name} 能力矩阵`}>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div>
            <h4 className="text-xs font-semibold text-ink">逐形态能力状态</h4>
            <p className="text-[11px] text-ink-muted">
              账号 {displayAccount} · Core 以实际探测覆盖 Manifest 声明，不显示笼统的“支持发布”
            </p>
          </div>
          <button
            type="button"
            onClick={onProbe}
            disabled={busy}
            className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-surface px-2 text-[11px] font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:opacity-50"
          >
            <RefreshCw size={10} />
            运行探测
          </button>
        </div>
        <div className="grid gap-1.5">
          {displayCapabilities.map((capability) => (
            <CapabilityRow key={capability.kind} capability={capability} compact />
          ))}
        </div>
      </section>

      {probe && (
        <div className="rounded-xl border border-line bg-subtle px-3 py-2.5 text-[11px] text-ink-secondary">
          <div className="flex flex-wrap items-center gap-2">
            <Badge
              className={
                probe.ok
                  ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                  : "border-rose-200 bg-rose-50 text-rose-700"
              }
            >
              {probe.ok ? "探测通过" : "探测失败"}
            </Badge>
            <span>账号 {probe.accountRef}</span>
            <span>隔离 {probe.sandboxed ? "沙箱" : "进程"}</span>
            <span>时间 {formatDate(probe.checkedAt)}</span>
          </div>
          {probe.error && <p className="mt-1 text-rose-700">{probe.error}</p>}
          {probe.hostAccess.length > 0 && (
            <div className="mt-2 grid gap-1 sm:grid-cols-2">
              {probe.hostAccess.map((item) => (
                <div
                  key={item.method}
                  className="flex items-center justify-between gap-2 rounded-lg bg-surface px-2 py-1"
                >
                  <code className="truncate text-[10px] text-ink-muted">{item.method}</code>
                  <span className={item.allowed ? "text-emerald-700" : "text-rose-700"}>
                    {item.allowed ? "允许" : (item.code ?? "拒绝")}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {security && (
        <div className="rounded-xl border border-emerald-200 bg-emerald-50/70 px-3 py-2.5 text-[11px] text-emerald-800">
          <div className="flex flex-wrap items-center gap-2">
            <ShieldCheck size={12} />
            <span className="font-medium">安全边界自检</span>
            <span>{security.sandboxed ? "沙箱已启用" : "沙箱未启用"}</span>
          </div>
          <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1">
            <span>SQLite {security.databaseDenied ? "已拒绝" : "未拦截"}</span>
            <span>工作区 {security.workspaceDenied ? "已拒绝" : "未拦截"}</span>
            <span>凭据 {security.credentialDenied ? "已拒绝" : "未拦截"}</span>
            <span className={security.allDenied ? "font-medium" : "font-medium text-rose-700"}>
              {security.allDenied ? "全部通过" : "存在未拦截项"}
            </span>
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2 border-t border-line pt-3">
        <button
          type="button"
          onClick={onSecurityTest}
          disabled={busy}
          className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-surface px-2 text-[11px] font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:opacity-50"
        >
          <Shield size={10} />
          安全边界自检
        </button>
        <button
          type="button"
          onClick={() => onFault("crash")}
          disabled={busy || plugin.status !== "ENABLED"}
          className="h-7 cursor-pointer rounded-lg border border-line bg-surface px-2 text-[11px] font-medium text-ink-muted transition-colors hover:bg-subtle disabled:opacity-50"
        >
          模拟崩溃
        </button>
        <button
          type="button"
          onClick={() => onFault("timeout")}
          disabled={busy || plugin.status !== "ENABLED"}
          className="h-7 cursor-pointer rounded-lg border border-line bg-surface px-2 text-[11px] font-medium text-ink-muted transition-colors hover:bg-subtle disabled:opacity-50"
        >
          模拟超时
        </button>
        <button
          type="button"
          onClick={onShowEvents}
          className="h-7 cursor-pointer rounded-lg border border-line bg-surface px-2 text-[11px] font-medium text-ink-muted transition-colors hover:bg-subtle"
        >
          审计事件
        </button>
      </div>

      {events && (
        <div className="rounded-xl border border-line bg-subtle px-3 py-2.5">
          <p className="mb-1 text-[11px] font-medium text-ink-secondary">最近插件事件</p>
          {events.length === 0 ? (
            <p className="text-[11px] text-ink-muted">暂无事件</p>
          ) : (
            <div className="max-h-40 space-y-1 overflow-y-auto">
              {events.map((event) => (
                <div key={event.id} className="flex items-start gap-2 text-[11px]">
                  <Clock3 size={10} className="mt-0.5 shrink-0 text-ink-muted" />
                  <span className="w-32 shrink-0 text-ink-muted">
                    {formatDate(event.createdAt)}
                  </span>
                  <span className="font-medium text-ink-secondary">{event.action}</span>
                  <span className="min-w-0 break-words text-ink-muted">{event.detail}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </article>
  );
}

export function PluginManager() {
  const showToast = useUiStore((state) => state.showToast);
  const catalog = usePluginStore((state) => state.catalog);
  const plugins = usePluginStore((state) => state.plugins);
  const loading = usePluginStore((state) => state.loading);
  const error = usePluginStore((state) => state.error);
  const initialize = usePluginStore((state) => state.initialize);
  const install = usePluginStore((state) => state.install);
  const savePermissions = usePluginStore((state) => state.savePermissions);
  const enable = usePluginStore((state) => state.enable);
  const disable = usePluginStore((state) => state.disable);
  const restart = usePluginStore((state) => state.restart);
  const rollback = usePluginStore((state) => state.rollback);
  const uninstall = usePluginStore((state) => state.uninstall);
  const probe = usePluginStore((state) => state.probe);
  const simulateFault = usePluginStore((state) => state.simulateFault);
  const runSecuritySelfTest = usePluginStore((state) => state.runSecuritySelfTest);
  const listEvents = usePluginStore((state) => state.listEvents);
  const runPayloadSelfCheck = usePluginStore((state) => state.runPayloadSelfCheck);

  const [accountRef, setAccountRef] = useState(contentKindOptions[0].value);
  const [busyId, setBusyId] = useState("");
  const [reviewPlugin, setReviewPlugin] = useState<PluginSummary | null>(null);
  const [draftGrants, setDraftGrants] = useState<Record<string, boolean>>({});
  const [uninstallPlugin, setUninstallPlugin] = useState<PluginSummary | null>(null);
  const [deletePluginData, setDeletePluginData] = useState(false);
  const [probeResults, setProbeResults] = useState<Record<string, PluginProbeResult>>({});
  const [securityResults, setSecurityResults] = useState<Record<string, SecuritySelfTestResult>>(
    {},
  );
  const [eventResults, setEventResults] = useState<Record<string, PluginEvent[]>>({});
  const [payloadChecks, setPayloadChecks] = useState<PublishPayloadCheckResult[]>([]);
  const [payloadBusy, setPayloadBusy] = useState(false);
  const [lastPublishCheck, setLastPublishCheck] = useState<PluginPublishValidationResult | null>(
    null,
  );
  const [publishBusy, setPublishBusy] = useState(false);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  const availableCatalog = useMemo(() => catalog.filter((entry) => !entry.installed), [catalog]);
  const enabledPlugin = plugins.find((plugin) => plugin.status === "ENABLED");

  async function withBusy<T>(id: string, action: () => Promise<T>): Promise<T | undefined> {
    setBusyId(id);
    try {
      return await action();
    } catch (cause) {
      showToast(errorMessage(cause));
      return undefined;
    } finally {
      setBusyId("");
    }
  }

  async function handleInstall(entry: AdapterCatalogEntry) {
    const installed = await withBusy(`install-${entry.id}`, () => install(entry.id));
    if (!installed) return;
    showToast(`${installed.name} 已安装，请先完成权限审查`);
    openReview(installed);
  }

  function openReview(plugin: PluginSummary) {
    setReviewPlugin(plugin);
    setDraftGrants(
      Object.fromEntries(
        plugin.permissions.map((permission) => [permission.id, permission.granted ?? false]),
      ),
    );
  }

  async function handleSaveReview() {
    if (!reviewPlugin) return;
    const decisions: PluginPermissionDecision[] = reviewPlugin.permissions.map((permission) => ({
      id: permission.id,
      granted: draftGrants[permission.id] === true,
    }));
    const updated = await withBusy(`permissions-${reviewPlugin.id}`, () =>
      savePermissions(reviewPlugin.id, decisions),
    );
    if (!updated) return;
    setReviewPlugin(null);
    showToast(
      updated.status === "READY"
        ? "权限审查完成，可以启用"
        : "权限决定已保存；必需权限未全部授予时不能启用",
    );
  }

  async function handleEnable(plugin: PluginSummary) {
    const updated = await withBusy(`enable-${plugin.id}`, () => enable(plugin.id));
    if (updated) showToast(`${plugin.name} 已启用`);
    else await initialize(true);
  }

  async function handleDisable(plugin: PluginSummary) {
    const updated = await withBusy(`disable-${plugin.id}`, () => disable(plugin.id));
    if (updated) showToast(`${plugin.name} 已禁用`);
  }

  async function handleRestart(plugin: PluginSummary) {
    const updated = await withBusy(`restart-${plugin.id}`, () => restart(plugin.id));
    if (updated) showToast(`${plugin.name} 已重启`);
  }

  async function handleProbe(plugin: PluginSummary) {
    const result = await withBusy(`probe-${plugin.id}`, () => probe(plugin.id, accountRef));
    if (!result) return;
    setProbeResults((current) => ({ ...current, [plugin.id]: result }));
    showToast(result.ok ? `${plugin.name} 能力探测通过` : `${plugin.name} 探测失败，已标记不可用`);
  }

  async function handleSecurityTest(plugin: PluginSummary) {
    const result = await withBusy(`security-${plugin.id}`, () => runSecuritySelfTest(plugin.id));
    if (!result) return;
    setSecurityResults((current) => ({ ...current, [plugin.id]: result }));
    showToast(result.allDenied ? "安全边界自检通过" : "安全边界自检存在未拦截项");
  }

  async function handleFault(plugin: PluginSummary, fault: "crash" | "timeout") {
    const result = await withBusy(`fault-${plugin.id}`, () => simulateFault(plugin.id, fault));
    if (!result) return;
    setProbeResults((current) => {
      const next = { ...current };
      delete next[plugin.id];
      return next;
    });
    showToast(`${plugin.name} 故障已捕获：${result.error}`);
  }

  async function handleShowEvents(plugin: PluginSummary) {
    if (eventResults[plugin.id]) {
      setEventResults((current) => {
        const next = { ...current };
        delete next[plugin.id];
        return next;
      });
      return;
    }
    const events = await withBusy(`events-${plugin.id}`, () => listEvents(plugin.id));
    if (events) setEventResults((current) => ({ ...current, [plugin.id]: events }));
  }

  async function handleUninstall() {
    if (!uninstallPlugin) return;
    const result = await withBusy(`uninstall-${uninstallPlugin.id}`, () =>
      uninstall(uninstallPlugin.id, deletePluginData),
    );
    if (!result) return;
    setUninstallPlugin(null);
    setDeletePluginData(false);
    showToast(result.message);
  }

  async function handleRollback(plugin: PluginSummary) {
    const updated = await withBusy(`rollback-${plugin.id}`, () => rollback(plugin.id));
    if (updated) showToast(`${plugin.name} 已回退到 v${updated.version}`);
  }

  async function handlePayloadSelfCheck() {
    setPayloadBusy(true);
    try {
      setPayloadChecks(await runPayloadSelfCheck());
    } catch (cause) {
      showToast(errorMessage(cause));
    } finally {
      setPayloadBusy(false);
    }
  }

  async function handlePublishProbe() {
    const plugin = enabledPlugin ?? plugins[0];
    if (!plugin) {
      showToast("请先安装并启用一个适配器");
      return;
    }
    const input: PublishInput = {
      contentKind: "imageText",
      title: "M4 图文载荷预检",
      body: "验证图片顺序与封面",
      media: [
        {
          assetId: "asset-image-1",
          kind: "image",
          mimeType: "image/png",
          byteSize: 128,
          order: 1,
        },
        {
          assetId: "asset-image-2",
          kind: "image",
          mimeType: "image/png",
          byteSize: 256,
          order: 2,
        },
      ],
      cover: {
        assetId: "asset-image-cover",
        kind: "image",
        mimeType: "image/png",
        byteSize: 64,
        order: 1,
      },
    };
    setPublishBusy(true);
    try {
      const result = await usePluginStore
        .getState()
        .validatePublishInput(plugin.id, accountRef, input);
      setLastPublishCheck(result);
      showToast(result.ok ? "图文载荷预检通过" : "图文载荷预检被拒绝");
    } catch (cause) {
      showToast(errorMessage(cause));
    } finally {
      setPublishBusy(false);
    }
  }

  return (
    <div className="space-y-6">
      <section className="rounded-2xl border border-amber-200 bg-amber-50/80 p-4">
        <div className="flex items-start gap-3">
          <ShieldAlert size={17} className="mt-0.5 shrink-0 text-amber-700" />
          <div>
            <h2 className="text-sm font-semibold text-amber-900">独立进程不是强安全沙箱</h2>
            <p className="mt-1 text-xs leading-relaxed text-amber-800/90">
              首期 Runtime 只允许官方适配器。macOS
              使用系统沙箱限制文件与网络访问；其他平台仅有进程隔离，
              非官方第三方代码在强隔离方案通过前不得安装或启用。所有账号、工作区和网络能力必须经
              Core 主机 API 授权，插件不会获得原始凭据或工作区路径。
            </p>
          </div>
        </div>
      </section>

      <section className="rounded-2xl border border-line bg-surface p-5">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-ink">能力探测与载荷完整性</h2>
            <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
              选择测试账号后探测逐形态能力；视频必须包含视频与封面，图文必须保留图片顺序与封面，
              纯文章必须包含正文。缺少必需载荷时会在调用平台前被拒绝。
            </p>
          </div>
          <label className="flex items-center gap-2 text-xs text-ink-secondary">
            探测账号
            <select
              value={accountRef}
              onChange={(event) => setAccountRef(event.target.value)}
              className="h-8 cursor-pointer rounded-lg border border-line bg-surface px-2 text-xs text-ink outline-none focus:border-accent"
            >
              {contentKindOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="mt-3 flex flex-wrap gap-2">
          <button
            type="button"
            onClick={handlePayloadSelfCheck}
            disabled={payloadBusy}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:opacity-50"
          >
            <CheckCircle2 size={12} />
            {payloadBusy ? "校验中" : "运行四形态载荷自检"}
          </button>
          <button
            type="button"
            onClick={handlePublishProbe}
            disabled={publishBusy}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle disabled:opacity-50"
          >
            <Shield size={12} />
            {publishBusy ? "预检中" : "预检图文载荷"}
          </button>
        </div>
        {payloadChecks.length > 0 && (
          <div className="mt-3 grid gap-1.5 sm:grid-cols-2 lg:grid-cols-3">
            {payloadChecks.map((check) => (
              <div
                key={check.id}
                className="flex items-center justify-between gap-2 rounded-xl border border-line bg-subtle px-2.5 py-1.5 text-[11px]"
              >
                <span className="truncate text-ink-secondary">{check.id}</span>
                <Badge
                  className={
                    check.passed
                      ? "border-emerald-200 bg-emerald-50 text-emerald-700"
                      : "border-rose-200 bg-rose-50 text-rose-700"
                  }
                >
                  {check.passed ? "通过" : "失败"}
                </Badge>
              </div>
            ))}
          </div>
        )}
        {lastPublishCheck && (
          <div className="mt-3 rounded-xl border border-line bg-subtle px-3 py-2 text-[11px] text-ink-secondary">
            <span className="font-medium">最近预检：{lastPublishCheck.ok ? "通过" : "拒绝"}</span>
            {lastPublishCheck.payload.issues.length > 0 && (
              <span className="ml-2 text-rose-700">
                {lastPublishCheck.payload.issues.map((issue) => issue.message).join("；")}
              </span>
            )}
            {lastPublishCheck.adapterMessage && (
              <span className="ml-2 text-ink-muted">{lastPublishCheck.adapterMessage}</span>
            )}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex items-baseline justify-between gap-2">
          <h2 className="text-sm font-semibold text-ink">官方适配器发现</h2>
          <span className="text-xs text-ink-muted">{availableCatalog.length} 个可安装</span>
        </div>
        {availableCatalog.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-line-strong bg-surface p-8 text-center text-xs text-ink-muted">
            所有随应用分发的官方适配器均已安装。
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
            {availableCatalog.map((entry) => (
              <CatalogCard
                key={entry.id}
                entry={entry}
                busy={busyId === `install-${entry.id}`}
                onInstall={() => void handleInstall(entry)}
              />
            ))}
          </div>
        )}
      </section>

      <section className="space-y-3">
        <div className="flex items-baseline justify-between gap-2">
          <h2 className="text-sm font-semibold text-ink">已安装适配器</h2>
          <span className="text-xs text-ink-muted">
            {plugins.filter((plugin) => plugin.status === "ENABLED").length} 个运行中 /{" "}
            {plugins.length} 个已安装
          </span>
        </div>
        {loading && plugins.length === 0 ? (
          <div className="rounded-2xl border border-line bg-surface p-8 text-center text-xs text-ink-muted">
            正在读取适配器注册表…
          </div>
        ) : error ? (
          <div className="rounded-2xl border border-rose-200 bg-rose-50 p-4 text-xs text-rose-700">
            {error}
          </div>
        ) : plugins.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-line-strong bg-surface p-10 text-center">
            <div className="mx-auto grid h-11 w-11 place-items-center rounded-2xl bg-accent-soft text-accent-deep">
              <Plug size={19} />
            </div>
            <h3 className="mt-3 text-sm font-semibold text-ink">还没有安装适配器</h3>
            <p className="mx-auto mt-1 max-w-lg text-xs leading-relaxed text-ink-secondary">
              从上方官方分发区安装测试适配器，完成权限审查后即可启动独立进程并验证能力探测。
            </p>
          </div>
        ) : (
          <div className="space-y-4">
            {plugins.map((plugin) => (
              <PluginCard
                key={plugin.id}
                plugin={plugin}
                accountRef={accountRef}
                busy={busyId.endsWith(plugin.id)}
                probe={probeResults[plugin.id]}
                security={securityResults[plugin.id]}
                events={eventResults[plugin.id]}
                onReview={() => openReview(plugin)}
                onEnable={() => void handleEnable(plugin)}
                onDisable={() => void handleDisable(plugin)}
                onRestart={() => void handleRestart(plugin)}
                onProbe={() => void handleProbe(plugin)}
                onSecurityTest={() => void handleSecurityTest(plugin)}
                onUninstall={() => setUninstallPlugin(plugin)}
                onRollback={() => void handleRollback(plugin)}
                onFault={(fault) => void handleFault(plugin, fault)}
                onShowEvents={() => void handleShowEvents(plugin)}
              />
            ))}
          </div>
        )}
      </section>

      {reviewPlugin && (
        <ModalShell label="插件权限审查" onClose={() => setReviewPlugin(null)}>
          <div className="pr-8">
            <div className="flex items-start gap-3">
              <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent-deep">
                <Shield size={16} />
              </div>
              <div>
                <h2 className="text-sm font-semibold text-ink">
                  审查 {reviewPlugin.name} v{reviewPlugin.version}
                </h2>
                <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
                  Manifest
                  是权限申请，不是授权凭证。请确认来源、版本、能力与风险；未授予的账号凭据、
                  网络和工作区操作会被 Core 拒绝。
                </p>
              </div>
            </div>
          </div>
          <div className="mt-4 grid gap-2 rounded-xl border border-line bg-subtle p-3 text-[11px] text-ink-secondary sm:grid-cols-2">
            <span>
              来源：{reviewPlugin.source === "BUNDLED" ? "随应用分发的官方适配器" : "本地来源"}
            </span>
            <span>版本：v{reviewPlugin.version}</span>
            <span>Adapter 合约：v{reviewPlugin.adapterContractVersion}</span>
            <span>隔离：{reviewPlugin.isolation.description}</span>
            <span className="sm:col-span-2 break-all">
              Manifest SHA-256：{reviewPlugin.manifestHash}
            </span>
          </div>
          <div className="mt-4 space-y-2">
            <h3 className="text-xs font-semibold text-ink">申请权限</h3>
            {reviewPlugin.permissions.map((permission) => {
              const risk = permissionRiskCopy[permission.risk] ?? permissionRiskCopy.high;
              return (
                <label
                  key={permission.id}
                  className="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-surface p-3 transition-colors hover:bg-subtle"
                >
                  <input
                    type="checkbox"
                    checked={draftGrants[permission.id] === true}
                    onChange={(event) =>
                      setDraftGrants((current) => ({
                        ...current,
                        [permission.id]: event.target.checked,
                      }))
                    }
                    className="mt-0.5 h-4 w-4 accent-[var(--accent, #7b68a8)]"
                  />
                  <span className="min-w-0 flex-1">
                    <span className="flex flex-wrap items-center gap-1.5">
                      <span className="text-xs font-medium text-ink">{permission.label}</span>
                      <Badge className={risk.className}>{risk.label}</Badge>
                      {permission.required ? (
                        <Badge className="border-rose-200 bg-rose-50 text-rose-700">必需</Badge>
                      ) : (
                        <Badge className="border-line bg-subtle text-ink-muted">可选</Badge>
                      )}
                    </span>
                    <span className="mt-1 block text-[11px] leading-relaxed text-ink-secondary">
                      {permission.description}
                    </span>
                    <span className="mt-1 block text-[11px] text-ink-muted">
                      申请理由：{permission.reason}
                    </span>
                  </span>
                </label>
              );
            })}
          </div>
          <div className="mt-4 space-y-2">
            <h3 className="text-xs font-semibold text-ink">声明能力（启用后以运行时探测为准）</h3>
            <div className="grid gap-1.5 sm:grid-cols-2">
              {reviewPlugin.capabilities.map((capability) => (
                <CapabilityRow key={capability.kind} capability={capability} />
              ))}
            </div>
          </div>
          <div className="mt-5 flex flex-wrap items-center justify-between gap-2 border-t border-line pt-4">
            <p className="text-[11px] text-ink-muted">
              所有权限都需作出授权或拒绝决定；必需权限被拒绝时不能启用。
            </p>
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setReviewPlugin(null)}
                className="h-8 cursor-pointer rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
              >
                取消
              </button>
              <button
                type="button"
                disabled={busyId === `permissions-${reviewPlugin.id}`}
                onClick={() => void handleSaveReview()}
                className="h-8 cursor-pointer rounded-xl bg-accent px-3 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:opacity-50"
              >
                保存权限决定
              </button>
            </div>
          </div>
        </ModalShell>
      )}

      {uninstallPlugin && (
        <ModalShell label="卸载插件" onClose={() => setUninstallPlugin(null)} width="520px">
          <div className="pr-8">
            <h2 className="text-sm font-semibold text-ink">卸载 {uninstallPlugin.name}</h2>
            <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
              卸载会停止插件进程并删除插件文件。审计事件保留；插件数据是否删除由你明确选择。
            </p>
          </div>
          <div className="mt-4 space-y-2 rounded-xl border border-line bg-subtle p-3 text-[11px] text-ink-secondary">
            <p>插件目录：{uninstallPlugin.installPath}</p>
            <p>当前版本：v{uninstallPlugin.version}</p>
            <p>
              已授予权限：{uninstallPlugin.permissions.filter((item) => item.granted).length} 项
            </p>
          </div>
          <label className="mt-4 flex cursor-pointer items-start gap-2 rounded-xl border border-rose-200 bg-rose-50 p-3 text-xs text-rose-700">
            <input
              type="checkbox"
              checked={deletePluginData}
              onChange={(event) => setDeletePluginData(event.target.checked)}
              className="mt-0.5 h-4 w-4"
            />
            <span>
              同时删除该插件的缓存和数据目录。不勾选时保留插件数据，仅卸载代码与授权记录。
            </span>
          </label>
          <div className="mt-4 flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setUninstallPlugin(null)}
              className="h-8 cursor-pointer rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
            >
              取消
            </button>
            <button
              type="button"
              disabled={busyId === `uninstall-${uninstallPlugin.id}`}
              onClick={() => void handleUninstall()}
              className="h-8 cursor-pointer rounded-xl bg-rose-600 px-3 text-xs font-medium text-white hover:bg-rose-700 disabled:opacity-50"
            >
              确认卸载
            </button>
          </div>
        </ModalShell>
      )}
    </div>
  );
}
