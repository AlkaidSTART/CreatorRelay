import { useMemo } from "react";
import { ExternalLink, Search } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { useContentStore, useUiStore } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

export function ContentPage() {
  const showToast = useUiStore((state) => state.showToast);
  const platformAccounts = useContentStore((state) => state.platformAccounts);
  const publishedContents = useContentStore((state) => state.publishedContents);
  const keyword = useContentStore((state) => state.contentKeyword);
  const setKeyword = useContentStore((state) => state.setContentKeyword);
  const selectedPlatform = useContentStore((state) => state.contentPlatformFilter);
  const setSelectedPlatform = useContentStore((state) => state.setContentPlatformFilter);

  const platforms = useMemo(
    () => [
      "全部平台",
      ...Array.from(new Set(platformAccounts.map((account) => account.platformName))),
    ],
    [platformAccounts],
  );

  const filtered = useMemo(() => {
    const term = keyword.trim().toLocaleLowerCase();
    return publishedContents.filter((item) => {
      const matchPlatform = selectedPlatform === "全部平台" || item.platform === selectedPlatform;
      const matchKeyword =
        !term ||
        `${item.title} ${item.platform} ${item.accountAlias} ${item.externalId}`
          .toLocaleLowerCase()
          .includes(term);
      return matchPlatform && matchKeyword;
    });
  }, [keyword, publishedContents, selectedPlatform]);

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="CONTENT"
        title="内容中心"
        description="一站式检索你在各平台已发布的作品，随时回溯来源账号、外部链接与真实互动数据。"
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex h-8.5 w-full max-w-sm items-center gap-2 rounded-xl border border-line bg-surface px-3 text-ink-muted focus-within:border-accent">
            <Search size={13} className="shrink-0" />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号或外部 ID 检索作品"
              aria-label="检索跨平台作品"
              className="min-w-0 flex-1 bg-transparent text-xs text-ink placeholder:text-ink-muted focus:outline-none"
            />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {platforms.map((platform) => (
              <button
                key={platform}
                onClick={() => setSelectedPlatform(platform)}
                className={`inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border px-3 text-xs font-medium transition-colors ${
                  selectedPlatform === platform
                    ? "border-accent bg-accent text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)]"
                    : "border-line bg-surface text-ink-secondary hover:bg-subtle"
                }`}
              >
                {platform !== "全部平台" && <PlatformLogo platformName={platform} size={13} />}
                {platform}
              </button>
            ))}
          </div>
        </div>

        <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="divide-y divide-line">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] gap-4 pb-2.5 text-xs font-medium text-ink-muted">
              <span>作品标题 / 外部 ID</span>
              <span>平台 / 账号 / 形态</span>
              <span>核心指标（原始口径）</span>
              <span>操作</span>
            </div>
            {filtered.map((item) => {
              const visual = getPlatformVisual(item.platform);
              return (
                <div
                  key={item.id}
                  className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] items-center gap-4 py-3.5"
                >
                  <div className="min-w-0">
                    <strong className="block truncate text-xs font-medium text-ink">
                      {item.title}
                    </strong>
                    <small className="text-[11px] text-ink-muted">
                      ID: {item.externalId} · 发布于 {item.publishedAt}
                    </small>
                  </div>
                  <div className="space-y-1">
                    <div className="flex flex-wrap gap-1.5">
                      <span
                        className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium ${visual.badgeClass}`}
                      >
                        <PlatformLogo platformName={item.platform} size={13} />
                        {item.platform}
                      </span>
                      <span className="rounded-md bg-sidebar px-2 py-0.5 text-[11px] text-ink-secondary">
                        {item.contentType}
                      </span>
                    </div>
                    <small className="block text-[11px] text-ink-muted">{item.accountAlias}</small>
                  </div>
                  <div>
                    <strong className="block text-xs font-semibold tabular-nums text-ink">
                      {item.readsLabel}
                    </strong>
                    <small className="text-[11px] text-ink-muted">
                      {item.readsMetric} · 赞 {item.likes} / 评 {item.comments} / 藏{" "}
                      {item.favorites}
                    </small>
                  </div>
                  <div>
                    <button
                      className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg border border-line bg-surface-soft px-2.5 text-xs font-medium text-ink hover:border-accent-border hover:bg-accent-soft hover:text-accent-deep"
                      onClick={() =>
                        showToast(`正在查看《${item.title}》的 ${item.platform} 指标快照`)
                      }
                    >
                      <ExternalLink size={11} />
                      详情
                    </button>
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
