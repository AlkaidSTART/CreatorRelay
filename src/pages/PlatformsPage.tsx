import { CheckCircle2, Plus, RefreshCw, ShieldAlert } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { PLATFORM_CATEGORIES, type AccountStatus, type PlatformCategory } from "../data/demoData";
import { useContentStore, useUiStore } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

const accountStatusLabel: Record<AccountStatus, { label: string; className: string }> = {
  ACTIVE: { label: "会话有效", className: "bg-mint-bg text-mint-text" },
  EXPIRED: { label: "会话已过期", className: "bg-coral-bg text-coral-text" },
  NEEDS_REAUTH: { label: "需重新授权", className: "bg-lemon-bg text-lemon-text" },
  UNAUTHORIZED: { label: "未登录", className: "bg-sidebar text-ink-secondary" },
};

const capabilityLabel: Record<string, string> = {
  supported: "自动支持",
  restricted: "受限",
  experimental: "实验性",
  unsupported: "人工辅助",
};

const categoryFilterOptions: Array<{ id: "ALL" | PlatformCategory; label: string }> = [
  { id: "ALL", label: "全部类型" },
  ...PLATFORM_CATEGORIES.map((category) => ({
    id: category.id,
    label: category.label,
  })),
];

export function PlatformsPage() {
  const showToast = useUiStore((state) => state.showToast);
  const platformAccounts = useContentStore((state) => state.platformAccounts);
  const categoryFilter = useContentStore((state) => state.platformCategoryFilter);
  const setCategoryFilter = useContentStore((state) => state.setPlatformCategoryFilter);
  const checkPlatformAccountStatus = useContentStore((state) => state.checkPlatformAccountStatus);
  const reauthPlatformAccount = useContentStore((state) => state.reauthPlatformAccount);

  const visibleCategories = PLATFORM_CATEGORIES.filter(
    (category) => categoryFilter === "ALL" || category.id === categoryFilter,
  )
    .map((category) => ({
      ...category,
      accounts: platformAccounts.filter((account) => account.category === category.id),
    }))
    .filter((group) => group.accounts.length > 0);

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="PLATFORMS"
        title="平台管理"
        description="按内容类型统一管理已支持的全部创作平台与账号会话，会话隔离互不干扰，随时一键重连。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.24)] transition-colors hover:bg-accent-hover"
            onClick={() => showToast("支持通过受控浏览器或 OAuth 接入更多平台账号")}
          >
            <Plus size={13} />
            接入平台账号
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap gap-1.5" role="tablist" aria-label="平台类型筛选">
          {categoryFilterOptions.map((option) => {
            const count =
              option.id === "ALL"
                ? platformAccounts.length
                : platformAccounts.filter((account) => account.category === option.id).length;
            const selected = categoryFilter === option.id;
            return (
              <button
                key={option.id}
                role="tab"
                aria-selected={selected}
                onClick={() => setCategoryFilter(option.id)}
                className={`inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border px-3 text-xs font-medium transition-colors ${
                  selected
                    ? "border-accent bg-accent text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)]"
                    : "border-line bg-surface text-ink-secondary hover:bg-subtle"
                }`}
              >
                <span>{option.label}</span>
                <span
                  className={`rounded-full px-1.5 text-[10px] ${
                    selected ? "bg-surface/20 text-white" : "bg-sidebar text-ink-muted"
                  }`}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>

        <div className="space-y-6">
          {visibleCategories.map((group) => (
            <section key={group.id} className="space-y-3">
              <div className="flex flex-wrap items-baseline justify-between gap-2">
                <div className="flex items-center gap-2">
                  <h2 className="text-sm font-semibold text-ink">{group.label}</h2>
                  <span className="rounded-md border border-line bg-surface px-2 py-0.5 text-[11px] font-medium text-ink-secondary">
                    {group.accounts.length} 个平台
                  </span>
                </div>
                <p className="text-xs text-ink-muted">{group.description}</p>
              </div>

              <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                {group.accounts.map((account) => {
                  const status = accountStatusLabel[account.status];
                  const visual = getPlatformVisual(account.platformName);
                  return (
                    <article
                      key={account.id}
                      className="flex flex-col gap-4 rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]"
                    >
                      <div className="flex items-start justify-between gap-3">
                        <div className="flex items-center gap-3">
                          <div
                            className={`grid h-10 w-10 shrink-0 place-items-center rounded-xl border ${visual.badgeClass}`}
                          >
                            <PlatformLogo
                              platformId={account.platformId}
                              platformName={account.platformName}
                              size={22}
                            />
                          </div>
                          <div>
                            <h3 className="text-sm font-semibold text-ink">
                              {account.platformName} · {account.alias}
                            </h3>
                            <small className="text-[11px] text-ink-muted">
                              UID: {account.uid} · {account.contentFormats} · 认证：
                              {account.authMode}
                            </small>
                          </div>
                        </div>
                        <span
                          className={`inline-flex shrink-0 items-center gap-1 rounded-lg px-2 py-0.5 text-[11px] font-medium ${status.className}`}
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
                        <span className="rounded-lg border border-line bg-canvas px-2 py-0.5 text-[11px] text-ink-secondary">
                          作品读取：{capabilityLabel[account.capabilities.contentRead]}
                        </span>
                        <span className="rounded-lg border border-line bg-canvas px-2 py-0.5 text-[11px] text-ink-secondary">
                          发布能力：{capabilityLabel[account.capabilities.contentPublish]}
                        </span>
                        <span className="rounded-lg border border-line bg-canvas px-2 py-0.5 text-[11px] text-ink-secondary">
                          数据分析：{capabilityLabel[account.capabilities.analytics]}
                        </span>
                      </div>

                      <div className="flex items-center justify-between border-t border-line pt-3">
                        <small className="text-[11px] text-ink-muted">
                          最近检测/同步：{account.lastSyncedAt}
                        </small>
                        <div className="flex items-center gap-2">
                          <button
                            className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg border border-line-strong bg-surface px-2.5 text-xs font-medium text-ink-secondary hover:bg-subtle"
                            onClick={() => checkPlatformAccountStatus(account.id)}
                          >
                            <RefreshCw size={11} />
                            检测状态
                          </button>
                          {account.status !== "ACTIVE" && (
                            <button
                              className="inline-flex h-7 cursor-pointer items-center rounded-lg bg-accent px-2.5 text-xs font-medium text-accent-on hover:bg-accent-hover"
                              onClick={() => reauthPlatformAccount(account.id)}
                            >
                              {account.status === "UNAUTHORIZED" ? "立即连接" : "重新连接"}
                            </button>
                          )}
                        </div>
                      </div>
                    </article>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      </div>
    </main>
  );
}
