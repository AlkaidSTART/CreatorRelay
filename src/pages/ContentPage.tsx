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
    <main className="flex h-full min-w-0 flex-col bg-[#FAF8F3]">
      <PageHeader
        eyebrow="CONTENT ARCHIVE"
        title="内容中心"
        description="一站式检索你在各平台已发布的作品，随时回溯来源账号、外部链接与真实互动数据。"
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex h-9 w-full max-w-sm items-center gap-2 rounded-xl border border-[#E3DED2] bg-white px-3 text-[#858B83] focus-within:border-[#2F4A36]/50">
            <Search size={13} className="shrink-0" />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号或外部 ID 检索作品"
              aria-label="检索跨平台作品"
              className="min-w-0 flex-1 bg-transparent text-xs text-[#1E241E] placeholder:text-[#9CA19A] focus:outline-none"
            />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {platforms.map((platform) => (
              <button
                key={platform}
                onClick={() => setSelectedPlatform(platform)}
                className={`h-8 cursor-pointer rounded-xl border px-3 text-xs font-medium transition-colors ${
                  selectedPlatform === platform
                    ? "border-[#2F4A36] bg-[#2F4A36] text-white"
                    : "border-[#E3DED2] bg-white text-[#545C54] hover:border-[#CFC8B9] hover:bg-[#FDFCF9]"
                }`}
              >
                {platform}
              </button>
            ))}
          </div>
        </div>

        <article className="rounded-2xl border border-[#E6E2D8] bg-white p-5">
          <div className="divide-y divide-[#F2EFE8]">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] gap-4 pb-2.5 text-xs font-medium text-[#858B83]">
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
                    <strong className="block truncate text-xs font-medium text-[#1E241E]">
                      {item.title}
                    </strong>
                    <small className="text-[11px] text-[#858B83]">
                      ID: {item.externalId} · 发布于 {item.publishedAt}
                    </small>
                  </div>
                  <div className="space-y-1">
                    <div className="flex flex-wrap gap-1.5">
                      <span
                        className={`rounded-md border px-2 py-0.5 text-[11px] font-medium ${visual.badgeClass}`}
                      >
                        {item.platform}
                      </span>
                      <span className="rounded-md bg-[#F4F1EA] px-2 py-0.5 text-[11px] text-[#5C645C]">
                        {item.contentType}
                      </span>
                    </div>
                    <small className="block text-[11px] text-[#858B83]">{item.accountAlias}</small>
                  </div>
                  <div>
                    <strong className="block text-xs font-semibold tabular-nums text-[#1E241E]">
                      {item.readsLabel}
                    </strong>
                    <small className="text-[11px] text-[#858B83]">
                      {item.readsMetric} · 赞 {item.likes} / 评 {item.comments} / 藏 {item.favorites}
                    </small>
                  </div>
                  <div>
                    <button
                      className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg border border-[#E3DED2] bg-[#FDFCF9] px-2.5 text-xs font-medium text-[#2F4A36] transition-colors hover:bg-[#EAF0EA]"
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
