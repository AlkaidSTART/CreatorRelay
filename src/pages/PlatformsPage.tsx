import { CheckCircle2, Globe2, Plus, RefreshCw, ShieldAlert } from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { plannedPlatforms, type AccountStatus } from "../data/demoData";
import { useContentStore, useUiStore } from "../stores";

const accountStatusLabel: Record<AccountStatus, { label: string; className: string }> = {
  ACTIVE: { label: "会话有效", className: "bg-emerald-50 text-emerald-700" },
  EXPIRED: { label: "会话已过期", className: "bg-red-50 text-red-700" },
  NEEDS_REAUTH: { label: "需重新授权", className: "bg-amber-50 text-amber-700" },
  UNAUTHORIZED: { label: "未登录", className: "bg-zinc-100 text-zinc-600" },
};

const capabilityLabel: Record<string, string> = {
  supported: "支持",
  restricted: "受限",
  experimental: "实验性",
  unsupported: "人工辅助",
};

export function PlatformsPage() {
  const showToast = useUiStore((state) => state.showToast);
  const platformAccounts = useContentStore((state) => state.platformAccounts);
  const checkPlatformAccountStatus = useContentStore(
    (state) => state.checkPlatformAccountStatus,
  );

  return (
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="PLATFORMS"
        title="平台管理"
        description="采用 Platform → Accounts → Platform Sessions 三级模型管理多账号会话隔离与适配器能力声明。"
        priority="P0"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3 text-xs font-medium text-white transition-colors hover:bg-zinc-800"
            onClick={() => showToast("接入新平台流程（选择平台 → 登录验证 → 能力检测）准备中")}
          >
            <Plus size={14} />
            接入平台账号
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex items-center gap-3 rounded-xl border border-zinc-200/80 bg-zinc-50/80 px-4 py-3 text-xs text-zinc-500">
          <span className="h-4 w-0.5 shrink-0 rounded-full bg-zinc-400" />
          <span>
            安全与合规原则：登录状态以实际运行检测为准，不绕过验证码或平台风控；不支持自动发布的平台提供人工发布辅助流程。
          </span>
        </div>

        <section className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          {platformAccounts.map((account) => {
            const status = accountStatusLabel[account.status];
            return (
              <article
                key={account.id}
                className="flex flex-col gap-4 rounded-xl border border-zinc-200/80 bg-white p-5"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-700">
                      <Globe2 size={16} />
                    </div>
                    <div>
                      <h2 className="text-sm font-semibold text-zinc-900">
                        {account.platformName} · {account.alias}
                      </h2>
                      <small className="text-[11px] text-zinc-400">
                        UID: {account.uid} · {account.contentFormats} · 认证：{account.authMode}
                      </small>
                    </div>
                  </div>
                  <span
                    className={`inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-0.5 text-[11px] font-medium ${status.className}`}
                  >
                    {account.status === "ACTIVE" ? (
                      <CheckCircle2 size={11} />
                    ) : (
                      <ShieldAlert size={11} />
                    )}
                    {status.label}
                  </span>
                </div>

                <div className="flex flex-wrap gap-1.5">
                  <span className="rounded-md border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[11px] text-zinc-600">
                    作品读取：{capabilityLabel[account.capabilities.contentRead]}
                  </span>
                  <span className="rounded-md border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[11px] text-zinc-600">
                    发布能力：{capabilityLabel[account.capabilities.contentPublish]}
                  </span>
                  <span className="rounded-md border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[11px] text-zinc-600">
                    数据分析：{capabilityLabel[account.capabilities.analytics]}
                  </span>
                </div>

                <div className="flex items-center justify-between border-t border-zinc-100 pt-3">
                  <small className="text-[11px] text-zinc-400">
                    最近检测/同步：{account.lastSyncedAt}
                  </small>
                  <div className="flex items-center gap-2">
                    <button
                      className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-50"
                      onClick={() => checkPlatformAccountStatus(account.id)}
                    >
                      <RefreshCw size={12} />
                      检测状态
                    </button>
                    {account.status !== "ACTIVE" && (
                      <button
                        className="inline-flex h-7 cursor-pointer items-center rounded-md bg-zinc-900 px-2.5 text-xs font-medium text-white hover:bg-zinc-800"
                        onClick={() =>
                          showToast(`将打开受控浏览器重新登录 ${account.platformName}`)
                        }
                      >
                        重新登录
                      </button>
                    )}
                  </div>
                </div>
              </article>
            );
          })}
        </section>

        <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
          <span className="text-[10px] font-semibold tracking-widest text-zinc-400 uppercase">
            ROADMAP ADAPTERS
          </span>
          <h2 className="mt-0.5 mb-4 text-base font-semibold text-zinc-900">
            首期规划接入的其他平台
          </h2>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {plannedPlatforms.map((platform) => (
              <div
                key={platform.name}
                className="flex flex-col items-start gap-2 rounded-xl border border-dashed border-zinc-200 bg-zinc-50/50 p-3.5"
              >
                <strong className="text-xs font-semibold text-zinc-800">{platform.name}</strong>
                <span className="rounded-md border border-zinc-200 bg-white px-2 py-0.5 text-[10.5px] text-zinc-600">
                  {platform.formats}
                </span>
                <small className="text-[11px] leading-relaxed text-zinc-500">{platform.note}</small>
              </div>
            ))}
          </div>
        </article>
      </div>
    </main>
  );
}
