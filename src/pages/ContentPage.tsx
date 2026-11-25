import { useMemo } from "react";
import { ExternalLink, Search } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useContentStore, useUiStore } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

const platforms = ["全部平台", "掘金", "知乎", "Bilibili", "CSDN"];

export function ContentPage() {
  const showToast = useUiStore((state) => state.showToast);
  const publishedContents = useContentStore((state) => state.publishedContents);
  const keyword = useContentStore((state) => state.contentKeyword);
  const setKeyword = useContentStore((state) => state.setContentKeyword);
  const selectedPlatform = useContentStore((state) => state.contentPlatformFilter);
  const setSelectedPlatform = useContentStore((state) => state.setContentPlatformFilter);

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
    <main className="flex h-full min-w-0 flex-col bg-[#FAFAFB]">
      <PageHeader
        eyebrow="CONTENT"
        title="内容中心"
        description="一站式检索你在各平台已发布的作品，随时回溯来源账号、外部链接与真实互动数据。"
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex h-8.5 w-full max-w-sm items-center gap-2 rounded-lg border border-black/[0.08] bg-white px-3 text-[#8E8E93] focus-within:border-[#2563EB]/60">
            <Search size={13} className="shrink-0" />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号或外部 ID 检索作品"
              aria-label="检索跨平台作品"
              className="min-w-0 flex-1 bg-transparent text-xs text-[#1D1D1F] placeholder:text-[#AEAEB2] focus:outline-none"
            />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {platforms.map((platform) => (
              <button
                key={platform}
                onClick={() => setSelectedPlatform(platform)}
                className={`h-8 cursor-pointer rounded-lg border px-3 text-xs font-medium transition-colors ${
                  selectedPlatform === platform
                    ? "border-[#1D1D1F] bg-[#1D1D1F] text-white"
                    : "border-black/[0.08] bg-white text-[#58585D] hover:bg-[#F5F5F7]"
                }`}
              >
                {platform}
              </button>
            ))}
          </div>
        </div>

        <article className="rounded-xl border border-black/[0.07] bg-white p-5">
          <div className="divide-y divide-black/[0.05]">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] gap-4 pb-2.5 text-xs font-medium text-[#8E8E93]">
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
                    <strong className="block truncate text-xs font-medium text-[#1D1D1F]">
                      {item.title}
                    </strong>
                    <small className="text-[11px] text-[#8E8E93]">
                      ID: {item.externalId} · 发布于 {item.publishedAt}
                    </small>
                  </div>
                  <div className="space-y-1">
                    <div className="flex flex-wrap gap-1.5">
                      <span className="inline-flex items-center gap-1.5 rounded-md border border-black/[0.08] bg-[#FAFAFB] px-2 py-0.5 text-[11px] font-medium text-[#1D1D1F]">
                        <span
                          className="h-1.5 w-1.5 rounded-full"
                          style={{ backgroundColor: visual.color }}
                        />
                        {item.platform}
                      </span>
                      <span className="rounded-md bg-[#F5F5F7] px-2 py-0.5 text-[11px] text-[#6E6E73]">
                        {item.contentType}
                      </span>
                    </div>
                    <small className="block text-[11px] text-[#8E8E93]">{item.accountAlias}</small>
                  </div>
                  <div>
                    <strong className="block text-xs font-semibold tabular-nums text-[#1D1D1F]">
                      {item.readsLabel}
                    </strong>
                    <small className="text-[11px] text-[#8E8E93]">
                      {item.readsMetric} · 赞 {item.likes} / 评 {item.comments} / 藏 {item.favorites}
                    </small>
                  </div>
                  <div>
                    <button
                      className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-md border border-black/[0.08] bg-white px-2.5 text-xs font-medium text-[#1D1D1F] hover:bg-[#F5F5F7]"
                      onClick={() => showToast(`正在查看《${item.title}》的 ${item.platform} 指标快照`)}
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
