import { CheckCircle2, Plus, RefreshCw, ShieldAlert } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { plannedPlatforms, type AccountStatus } from "../data/demoData";
import { useContentStore, useUiStore } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

const accountStatusLabel: Record<AccountStatus, { label: string; className: string }> = {
  ACTIVE: { label: "会话有效", className: "bg-[#E8F6EE] text-[#1E7548]" },
  EXPIRED: { label: "会话已过期", className: "bg-[#FDEEEE] text-[#C23B3B]" },
  NEEDS_REAUTH: { label: "需重新授权", className: "bg-[#FDF3DC] text-[#9A6314]" },
  UNAUTHORIZED: { label: "未登录", className: "bg-[#F4F1EA] text-[#646B64]" },
};

const capabilityLabel: Record<string, string> = {
  supported: "自动支持",
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
  const reauthPlatformAccount = useContentStore((state) => state.reauthPlatformAccount);

  return (
    <main className="flex h-full min-w-0 flex-col bg-[#FAF8F3]">
      <PageHeader
        eyebrow="PLATFORMS"
        title="平台管理"
        description="统一管理你的多平台账号会话与发布能力，会话隔离互不干扰，随时一键重连。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-[#2F4A36] px-3.5 text-xs font-medium text-white shadow-xs transition-colors hover:bg-[#233A29]"
            onClick={() => showToast("支持通过受控浏览器或 OAuth 接入更多平台账号")}
          >
            <Plus size={13} />
            接入平台账号
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <section className="grid grid-cols-1 gap-4 lg:grid-cols-2">
          {platformAccounts.map((account) => {
            const status = accountStatusLabel[account.status];
            const visual = getPlatformVisual(account.platformName);
            return (
              <article
                key={account.id}
                className="flex flex-col gap-4 rounded-2xl border border-[#E6E2D8] bg-white p-5 transition-all hover:border-[#D4CEC2]"
              >
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-3">
                    <div
                      className={`grid h-10 w-10 shrink-0 place-items-center rounded-xl text-xs font-semibold ${visual.avatarClass}`}
                    >
                      {visual.mark}
                    </div>
                    <div>
                      <h2 className="text-sm font-semibold text-[#1E241E]">
                        {account.platformName} · {account.alias}
                      </h2>
                      <small className="text-[11px] text-[#858B83]">
                        UID: {account.uid} · {account.contentFormats} · 认证：{account.authMode}
                      </small>
                    </div>
                  </div>
                  <span
                    className={`inline-flex shrink-0 items-center gap-1 rounded-full px-2.5 py-0.5 text-[11px] font-medium ${status.className}`}
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
                  <span className="rounded-lg border border-[#EAE6DC] bg-[#FAF8F3] px-2.5 py-0.5 text-[11px] text-[#545C54]">
                    作品读取：{capabilityLabel[account.capabilities.contentRead]}
                  </span>
                  <span className="rounded-lg border border-[#EAE6DC] bg-[#FAF8F3] px-2.5 py-0.5 text-[11px] text-[#545C54]">
                    发布能力：{capabilityLabel[account.capabilities.contentPublish]}
                  </span>
                  <span className="rounded-lg border border-[#EAE6DC] bg-[#FAF8F3] px-2.5 py-0.5 text-[11px] text-[#545C54]">
                    数据分析：{capabilityLabel[account.capabilities.analytics]}
                  </span>
                </div>

                <div className="flex items-center justify-between border-t border-[#F2EFE8] pt-3">
                  <small className="text-[11px] text-[#858B83]">
                    最近检测/同步：{account.lastSyncedAt}
                  </small>
                  <div className="flex items-center gap-2">
                    <button
                      className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-lg border border-[#DCD7CB] bg-white px-2.5 text-xs font-medium text-[#545C54] hover:bg-[#F4F1EA]"
                      onClick={() => checkPlatformAccountStatus(account.id)}
                    >
                      <RefreshCw size={11} />
                      检测状态
                    </button>
                    {account.status !== "ACTIVE" && (
                      <button
                        className="inline-flex h-7.5 cursor-pointer items-center rounded-lg bg-[#2F4A36] px-3 text-xs font-medium text-white hover:bg-[#233A29]"
                        onClick={() => reauthPlatformAccount(account.id)}
                      >
                        重新连接
                      </button>
                    )}
                  </div>
                </div>
              </article>
            );
          })}
        </section>

        <article className="rounded-2xl border border-[#E6E2D8] bg-white p-5">
          <span className="text-[10px] font-semibold tracking-widest text-[#4C6450] uppercase">
            COMING SOON
          </span>
          <h2 className="mt-0.5 mb-4 text-base font-semibold text-[#1E241E]">
            即将支持的更多创作平台
          </h2>
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {plannedPlatforms.map((platform) => {
              const visual = getPlatformVisual(platform.name);
              return (
                <div
                  key={platform.name}
                  className="flex flex-col items-start gap-2 rounded-xl border border-dashed border-[#D6D0C2] bg-[#FCFBF8] p-3.5"
                >
                  <div className="flex items-center gap-2">
                    <span
                      className={`grid h-6 w-6 place-items-center rounded-md text-[11px] font-semibold ${visual.avatarClass}`}
                    >
                      {visual.mark}
                    </span>
                    <strong className="text-xs font-semibold text-[#1E241E]">{platform.name}</strong>
                  </div>
                  <span className="rounded-md border border-[#E6E2D8] bg-white px-2 py-0.5 text-[10.5px] text-[#5C645C]">
                    {platform.formats}
                  </span>
                  <small className="text-[11px] leading-relaxed text-[#7D847D]">{platform.note}</small>
                </div>
              );
            })}
          </div>
        </article>
      </div>
    </main>
  );
}
