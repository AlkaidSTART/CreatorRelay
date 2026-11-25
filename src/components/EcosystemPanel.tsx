import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useMemo, useState } from "react";
import { useEcosystemStore, useUiStore } from "../stores";
import type {
  AccountCapabilitySnapshot,
  ContentKindCapability,
  MarketplaceGate,
  PlatformAdmission,
} from "../types/ecosystem";
import type { CapabilityLevel } from "../types/plugin";
import {
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  Globe2,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
} from "./Icons";
import { PlatformLogo } from "./PlatformLogo";

const levelCopy: Record<CapabilityLevel, { label: string; className: string; dot: string }> = {
  supported: {
    label: "已验证",
    className: "border-emerald-200 bg-emerald-50 text-emerald-700",
    dot: "bg-emerald-500",
  },
  restricted: {
    label: "受限",
    className: "border-amber-200 bg-amber-50 text-amber-700",
    dot: "bg-amber-500",
  },
  experimental: {
    label: "实验",
    className: "border-sky-200 bg-sky-50 text-sky-700",
    dot: "bg-sky-500",
  },
  unsupported: {
    label: "不支持",
    className: "border-line bg-subtle text-ink-muted",
    dot: "bg-ink-muted",
  },
  unavailable: {
    label: "不可用",
    className: "border-rose-200 bg-rose-50 text-rose-700",
    dot: "bg-rose-500",
  },
};

const evidenceCopy: Record<string, string> = {
  runtime_verified: "运行验证",
  official_docs_only: "官方文档",
  official_scope: "官方范围",
  not_verified: "未运行验证",
};

function LevelBadge({ level }: { level: CapabilityLevel }) {
  const copy = levelCopy[level];
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10px] font-medium ${copy.className}`}
    >
      <span className={`h-1.5 w-1.5 rounded-full ${copy.dot}`} />
      {copy.label}
    </span>
  );
}

function CapabilityMatrix({ items }: { items: ContentKindCapability[] }) {
  return (
    <div className="overflow-hidden rounded-xl border border-line">
      <div className="grid grid-cols-[minmax(64px,1fr)_84px_84px] bg-sidebar px-2.5 py-1.5 text-[10px] font-medium text-ink-muted">
        <span>内容形态</span>
        <span>读取</span>
        <span>发布</span>
      </div>
      {items.map((item) => (
        <div
          key={item.kind}
          className="grid grid-cols-[minmax(64px,1fr)_84px_84px] items-center border-t border-line px-2.5 py-2 first:border-t-0"
        >
          <div>
            <strong className="block text-[11px] font-medium text-ink">{item.label}</strong>
            <span className="text-[9.5px] text-ink-muted">
              {evidenceCopy[item.read.evidence]} / {evidenceCopy[item.publish.evidence]}
            </span>
          </div>
          <LevelBadge level={item.read.level} />
          <LevelBadge level={item.publish.level} />
        </div>
      ))}
    </div>
  );
}

function SourceLinks({ admission }: { admission: PlatformAdmission }) {
  const showToast = useUiStore((state) => state.showToast);
  async function open(sourceUrl: string) {
    try {
      if (isTauri()) await openUrl(sourceUrl);
      else window.open(sourceUrl, "_blank", "noopener,noreferrer");
    } catch (error) {
      showToast(`无法打开官方来源：${error instanceof Error ? error.message : String(error)}`);
    }
  }
  return (
    <div className="flex flex-wrap gap-1.5">
      {admission.officialSources.map((source) => (
        <button
          key={`${admission.platformId}-${source.url}`}
          type="button"
          onClick={() => void open(source.url)}
          className="inline-flex max-w-full cursor-pointer items-center gap-1 rounded-md border border-line bg-canvas px-2 py-1 text-[10px] text-ink-secondary hover:bg-subtle"
          title={`${source.result} · ${source.checkedAt}`}
        >
          <ExternalLink size={9} className="shrink-0" />
          <span className="truncate">{source.title}</span>
          <span className="shrink-0 text-ink-muted">{source.httpStatus ?? "未验证"}</span>
        </button>
      ))}
    </div>
  );
}

function AdmissionCard({ admission }: { admission: PlatformAdmission }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <article className="rounded-2xl border border-line bg-surface p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-center gap-2.5">
          <PlatformLogo
            platformId={admission.platformId}
            platformName={admission.platformName}
            size={26}
          />
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-1.5">
              <h3 className="truncate text-xs font-semibold text-ink">{admission.platformName}</h3>
              <span className="rounded-md border border-amber-200 bg-amber-50 px-1.5 py-0.5 text-[9.5px] font-medium text-amber-700">
                准入评估
              </span>
            </div>
            <p className="mt-0.5 text-[10px] text-ink-muted">
              合约 v{admission.adapterContractVersion} · 审查 {admission.reviewedAt}
            </p>
          </div>
        </div>
        <span className="shrink-0 rounded-md border border-rose-200 bg-rose-50 px-1.5 py-0.5 text-[9.5px] font-medium text-rose-700">
          自动发布关闭
        </span>
      </div>

      <div className="mt-3">
        <CapabilityMatrix items={admission.contentKinds} />
      </div>

      <div className="mt-3 flex items-center justify-between gap-2">
        <span className="text-[10px] text-ink-muted">
          人工发布：可用 · 账号类型：{admission.accountTypes.join("、")}
        </span>
        <button
          type="button"
          onClick={() => setExpanded((value) => !value)}
          className="shrink-0 cursor-pointer text-[10px] font-medium text-accent-deep hover:underline"
        >
          {expanded ? "收起依据" : "查看依据"}
        </button>
      </div>

      {expanded && (
        <div className="mt-3 space-y-2.5 border-t border-line pt-3">
          <SourceLinks admission={admission} />
          <p className="text-[10.5px] leading-relaxed text-ink-secondary">{admission.notes}</p>
          {admission.unsupported.length > 0 && (
            <div className="rounded-lg bg-coral-bg/60 px-2.5 py-2">
              <strong className="block text-[10px] font-medium text-coral-text">明确不做</strong>
              <p className="mt-0.5 text-[10px] leading-relaxed text-ink-secondary">
                {admission.unsupported.join("；")}
              </p>
            </div>
          )}
          <div className="grid gap-1.5 sm:grid-cols-2">
            {admission.contentKinds.map((item) => (
              <p key={item.kind} className="text-[9.5px] leading-relaxed text-ink-muted">
                <strong className="text-ink-secondary">{item.label}：</strong>
                {item.publish.reason}
              </p>
            ))}
          </div>
        </div>
      )}
    </article>
  );
}

function AccountCard({ account }: { account: AccountCapabilitySnapshot }) {
  return (
    <article className="rounded-2xl border border-line bg-surface p-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div>
          <h3 className="text-xs font-semibold text-ink">
            {account.platformName} · {account.accountAlias}
          </h3>
          <p className="mt-0.5 text-[10px] text-ink-muted">
            账号 {account.accountStatus} · 准入 {account.admissionStatus} · 分发{" "}
            {account.publishDispatch}
          </p>
        </div>
        <span
          className={`rounded-md border px-1.5 py-0.5 text-[9.5px] font-medium ${
            account.degraded
              ? "border-rose-200 bg-rose-50 text-rose-700"
              : "border-emerald-200 bg-emerald-50 text-emerald-700"
          }`}
        >
          {account.degraded ? "已降级" : "状态正常"}
        </span>
      </div>
      {account.degradationReason && (
        <p className="mt-2 rounded-lg bg-amber-50 px-2.5 py-2 text-[10px] leading-relaxed text-amber-800">
          {account.degradationReason}
        </p>
      )}
      <div className="mt-2 grid gap-1.5 sm:grid-cols-2">
        {account.capabilities.map((capability) => (
          <div
            key={capability.kind}
            className="flex items-center justify-between gap-2 rounded-lg border border-line bg-canvas px-2.5 py-2"
          >
            <div className="min-w-0">
              <span className="block text-[10.5px] font-medium text-ink">{capability.label}</span>
              {capability.reason && (
                <span
                  className="mt-0.5 line-clamp-1 text-[9px] text-ink-muted"
                  title={capability.reason}
                >
                  {capability.reason}
                </span>
              )}
            </div>
            <div className="flex shrink-0 gap-1">
              <LevelBadge level={capability.read} />
              <LevelBadge level={capability.publish} />
            </div>
          </div>
        ))}
      </div>
    </article>
  );
}

function GateRow({ gate }: { gate: MarketplaceGate }) {
  return (
    <div className="flex items-start justify-between gap-3 rounded-xl border border-line bg-canvas px-3 py-2.5">
      <div className="min-w-0">
        <div className="flex flex-wrap items-center gap-1.5">
          <strong className="text-[11px] font-medium text-ink">{gate.label}</strong>
          {gate.required && (
            <span className="rounded bg-sidebar px-1 py-0.5 text-[9px] text-ink-muted">必需</span>
          )}
        </div>
        <p className="mt-0.5 text-[10px] leading-relaxed text-ink-muted">{gate.evidence}</p>
      </div>
      <span
        className={`shrink-0 rounded-md border px-1.5 py-0.5 text-[9.5px] font-medium ${
          gate.passed
            ? "border-emerald-200 bg-emerald-50 text-emerald-700"
            : "border-rose-200 bg-rose-50 text-rose-700"
        }`}
      >
        {gate.passed ? "通过" : "未通过"}
      </span>
    </div>
  );
}

export function EcosystemPanel() {
  const overview = useEcosystemStore((state) => state.overview);
  const loading = useEcosystemStore((state) => state.loading);
  const error = useEcosystemStore((state) => state.error);
  const initialized = useEcosystemStore((state) => state.initialized);
  const initialize = useEcosystemStore((state) => state.initialize);
  const refresh = useEcosystemStore((state) => state.refresh);
  const showToast = useUiStore((state) => state.showToast);
  const [query, setQuery] = useState("");

  useEffect(() => {
    void initialize();
  }, [initialize]);

  const filteredAdmissions = useMemo(() => {
    if (!overview) return [];
    const keyword = query.trim().toLocaleLowerCase();
    if (!keyword) return overview.admissions.platforms;
    return overview.admissions.platforms.filter((platform) =>
      `${platform.platformName} ${platform.platformId} ${platform.admissionStatus}`
        .toLocaleLowerCase()
        .includes(keyword),
    );
  }, [overview, query]);

  async function handleRefresh() {
    await refresh();
    showToast("生态准入与兼容性状态已刷新");
  }

  if (!initialized && loading) {
    return (
      <div className="grid min-h-[320px] place-items-center text-sm text-ink-muted">
        <div className="flex items-center gap-2">
          <RefreshCw size={14} className="animate-spin" />
          正在读取平台准入与兼容性记录
        </div>
      </div>
    );
  }

  if (error || !overview) {
    return (
      <div className="rounded-2xl border border-rose-200 bg-rose-50 p-5 text-sm text-rose-700">
        <div className="flex items-center gap-2 font-medium">
          <AlertTriangle size={16} />
          生态状态读取失败
        </div>
        <p className="mt-2 text-xs leading-relaxed">{error || "没有可用数据"}</p>
        <button
          type="button"
          onClick={() => void refresh()}
          className="mt-3 inline-flex h-8 items-center gap-1.5 rounded-xl bg-rose-600 px-3 text-xs font-medium text-white"
        >
          <RefreshCw size={11} />
          重试
        </button>
      </div>
    );
  }

  const market = overview.marketplace;
  const passedRequired = market.gates.filter((gate) => gate.required && gate.passed).length;
  const requiredGates = market.gates.filter((gate) => gate.required).length;

  return (
    <div className="space-y-6">
      <section className="grid gap-3 md:grid-cols-3">
        <div className="rounded-2xl border border-line bg-surface p-4">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-medium text-ink-muted">平台准入</span>
            <Globe2 size={14} className="text-accent-deep" />
          </div>
          <p className="mt-2 text-2xl font-semibold tracking-tight text-ink">
            {overview.admissions.platforms.length}
          </p>
          <p className="mt-1 text-[10.5px] leading-relaxed text-ink-muted">
            全部已登记官方来源和四形态结果；未验证能力保持实验/不可用。
          </p>
        </div>
        <div className="rounded-2xl border border-line bg-surface p-4">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-medium text-ink-muted">兼容性回归</span>
            {overview.compatibility.passed ? (
              <ShieldCheck size={15} className="text-emerald-600" />
            ) : (
              <ShieldAlert size={15} className="text-rose-600" />
            )}
          </div>
          <p className="mt-2 text-2xl font-semibold tracking-tight text-ink">
            {overview.compatibility.checks.filter((check) => check.passed).length}/
            {overview.compatibility.checks.length}
          </p>
          <p className="mt-1 text-[10.5px] leading-relaxed text-ink-muted">
            Adapter 合约 v{overview.compatibility.contractVersion} ·{" "}
            {overview.compatibility.adapterCount} 个官方适配器
          </p>
        </div>
        <div className="rounded-2xl border border-line bg-surface p-4">
          <div className="flex items-center justify-between">
            <span className="text-[11px] font-medium text-ink-muted">第三方插件市场</span>
            <ShieldAlert size={15} className="text-amber-600" />
          </div>
          <p className="mt-2 text-2xl font-semibold tracking-tight text-ink">关闭</p>
          <p className="mt-1 text-[10.5px] leading-relaxed text-ink-muted">
            {passedRequired}/{requiredGates} 项必需门禁通过；仅允许官方内嵌适配器。
          </p>
        </div>
      </section>

      <section className="space-y-3">
        <div className="flex flex-wrap items-end justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-ink">平台 × 内容形态准入矩阵</h2>
            <p className="mt-1 text-xs leading-relaxed text-ink-muted">
              `supported` 只来自当前账号级运行证据。官方页面可达、Manifest
              声明和演示数据都不会升级为支持。
            </p>
          </div>
          <div className="flex items-center gap-2">
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="筛选平台"
              className="h-8 w-32 rounded-xl border border-line-strong bg-surface px-2.5 text-[11px] text-ink placeholder:text-ink-muted focus:border-accent focus:outline-none"
            />
            <button
              type="button"
              onClick={() => void handleRefresh()}
              disabled={loading}
              className="inline-flex h-8 items-center gap-1.5 rounded-xl border border-line bg-surface px-2.5 text-[11px] font-medium text-ink-secondary hover:bg-subtle disabled:opacity-50"
            >
              <RefreshCw size={10} className={loading ? "animate-spin" : ""} />
              刷新
            </button>
          </div>
        </div>
        <div className="grid gap-3 lg:grid-cols-2 xl:grid-cols-3">
          {filteredAdmissions.map((admission) => (
            <AdmissionCard key={admission.platformId} admission={admission} />
          ))}
        </div>
      </section>

      <section className="space-y-3">
        <div>
          <h2 className="text-sm font-semibold text-ink">账号级能力与降级原因</h2>
          <p className="mt-1 text-xs leading-relaxed text-ink-muted">
            账号授权过期、禁用或平台接口不可用时，能力会自动降级为不可用，并保留可复核原因。
          </p>
        </div>
        {overview.accounts.length > 0 ? (
          <div className="grid gap-3 lg:grid-cols-2">
            {overview.accounts.map((account) => (
              <AccountCard key={account.accountId} account={account} />
            ))}
          </div>
        ) : (
          <div className="rounded-2xl border border-dashed border-line-strong bg-sidebar/60 p-5 text-xs text-ink-muted">
            当前没有受管账号。添加并完成账号状态检查后，这里会显示账号级逐形态能力。
          </div>
        )}
      </section>

      <section className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <div className="rounded-2xl border border-line bg-surface p-4">
          <div className="flex flex-wrap items-start justify-between gap-2">
            <div>
              <h2 className="text-sm font-semibold text-ink">第三方插件市场门禁</h2>
              <p className="mt-1 text-xs leading-relaxed text-ink-muted">{market.rationale}</p>
            </div>
            <span className="rounded-md border border-rose-200 bg-rose-50 px-2 py-1 text-[10px] font-medium text-rose-700">
              市场关闭
            </span>
          </div>
          <div className="mt-3 grid gap-2 sm:grid-cols-2">
            {market.gates.map((gate) => (
              <GateRow key={gate.id} gate={gate} />
            ))}
          </div>
          <p className="mt-3 rounded-xl bg-sidebar px-3 py-2 text-[10.5px] leading-relaxed text-ink-secondary">
            分发方式：{market.allowedDistribution} · 第三方安装：
            {market.thirdPartyInstallAllowed ? "允许" : "禁止"} · 受控试点：
            {market.controlledPilotAllowed ? "允许" : "禁止"}
          </p>
        </div>

        <div className="rounded-2xl border border-line bg-surface p-4">
          <h2 className="text-sm font-semibold text-ink">Adapter 合约与故障隔离</h2>
          <p className="mt-1 text-[10.5px] leading-relaxed text-ink-muted">
            检查结果在 Core 中重新计算；单个适配器失败不影响其他平台。
          </p>
          <div className="mt-3 space-y-2">
            {overview.compatibility.checks.map((check) => (
              <div
                key={check.id}
                className="flex items-start gap-2 rounded-xl border border-line bg-canvas px-2.5 py-2"
              >
                {check.passed ? (
                  <CheckCircle2 size={12} className="mt-0.5 shrink-0 text-emerald-600" />
                ) : (
                  <AlertTriangle size={12} className="mt-0.5 shrink-0 text-rose-600" />
                )}
                <div className="min-w-0">
                  <strong className="block text-[10.5px] font-medium text-ink">
                    {check.label}
                  </strong>
                  <p className="mt-0.5 text-[9.5px] leading-relaxed text-ink-muted">
                    {check.detail}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
