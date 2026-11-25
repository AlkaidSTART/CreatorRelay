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
    <main className="flex h-full min-w-0 flex-col bg-[#FAF7F4]">
      <PageHeader
        eyebrow="CONTENT"
        title="内容中心"
        description="一站式检索你在各平台已发布的作品，随时回溯来源账号、外部链接与真实互动数据。"
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex h-8.5 w-full max-w-sm items-center gap-2 rounded-xl border border-[#EAE2D9] bg-white px-3 text-[#9A8F8A] focus-within:border-[#7B68A8]">
            <Search size={13} className="shrink-0" />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号或外部 ID 检索作品"
              aria-label="检索跨平台作品"
              className="min-w-0 flex-1 bg-transparent text-xs text-[#332B29] placeholder:text-[#B7ABA4] focus:outline-none"
            />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {platforms.map((platform) => (
              <button
                key={platform}
                onClick={() => setSelectedPlatform(platform)}
                className={`h-8 cursor-pointer rounded-xl border px-3 text-xs font-medium transition-colors ${
                  selectedPlatform === platform
                    ? "border-[#7B68A8] bg-[#7B68A8] text-white shadow-[0_2px_6px_rgba(123,104,168,0.22)]"
                    : "border-[#EAE2D9] bg-white text-[#6F6561] hover:bg-[#F7F2EC]"
                }`}
              >
                {platform}
              </button>
            ))}
          </div>
        </div>

        <article className="rounded-2xl border border-[#EAE2D9] bg-white p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="divide-y divide-[#F0E9E1]">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] gap-4 pb-2.5 text-xs font-medium text-[#9A8F8A]">
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
                    <strong className="block truncate text-xs font-medium text-[#332B29]">
                      {item.title}
                    </strong>
                    <small className="text-[11px] text-[#9A8F8A]">
                      ID: {item.externalId} · 发布于 {item.publishedAt}
                    </small>
                  </div>
                  <div className="space-y-1">
                    <div className="flex flex-wrap gap-1.5">
                      <span
                        className={`inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium ${visual.badgeClass}`}
                      >
                        <span
                          className="h-1.5 w-1.5 rounded-full"
                          style={{ backgroundColor: visual.color }}
                        />
                        {item.platform}
                      </span>
                      <span className="rounded-md bg-[#F4EFEA] px-2 py-0.5 text-[11px] text-[#6F6561]">
                        {item.contentType}
                      </span>
                    </div>
                    <small className="block text-[11px] text-[#9A8F8A]">{item.accountAlias}</small>
                  </div>
                  <div>
                    <strong className="block text-xs font-semibold tabular-nums text-[#332B29]">
                      {item.readsLabel}
                    </strong>
                    <small className="text-[11px] text-[#9A8F8A]">
                      {item.readsMetric} · 赞 {item.likes} / 评 {item.comments} / 藏 {item.favorites}
                    </small>
                  </div>
                  <div>
                    <button
                      className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg border border-[#EAE2D9] bg-[#FFFDFA] px-2.5 text-xs font-medium text-[#332B29] hover:border-[#DDD2EE] hover:bg-[#F2ECF9] hover:text-[#5B4888]"
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
