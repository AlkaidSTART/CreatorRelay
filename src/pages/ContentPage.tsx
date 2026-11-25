import { useMemo } from "react";
import { ExternalLink, Search } from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { useContentStore, useUiStore } from "../stores";

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
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="CONTENT"
        title="内容中心"
        description="统一聚合和检索已同步的外部平台作品，保留外部作品 ID、来源账号与独立口径指标。"
        priority="P0"
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex h-9 w-full max-w-sm items-center gap-2 rounded-lg border border-zinc-200 bg-white px-3 text-zinc-400 focus-within:border-zinc-400">
            <Search size={14} className="shrink-0" />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号或外部 ID 检索作品"
              aria-label="检索跨平台作品"
              className="min-w-0 flex-1 bg-transparent text-xs text-zinc-800 placeholder:text-zinc-400 focus:outline-none"
            />
          </div>
          <div className="flex flex-wrap gap-1.5">
            {platforms.map((platform) => (
              <button
                key={platform}
                onClick={() => setSelectedPlatform(platform)}
                className={`h-8 cursor-pointer rounded-lg border px-3 text-xs font-medium transition-colors ${
                  selectedPlatform === platform
                    ? "border-zinc-900 bg-zinc-900 text-white"
                    : "border-zinc-200 bg-white text-zinc-600 hover:bg-zinc-50"
                }`}
              >
                {platform}
              </button>
            ))}
          </div>
        </div>

        <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
          <div className="divide-y divide-zinc-100">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] gap-4 pb-2.5 text-xs font-medium text-zinc-400">
              <span>作品标题 / 外部 ID</span>
              <span>平台 / 账号 / 形态</span>
              <span>核心指标（原始口径）</span>
              <span>操作</span>
            </div>
            {filtered.map((item) => (
              <div
                key={item.id}
                className="grid grid-cols-[minmax(200px,2fr)_minmax(150px,1.2fr)_minmax(180px,1.5fr)_minmax(80px,0.6fr)] items-center gap-4 py-3.5"
              >
                <div className="min-w-0">
                  <strong className="block truncate text-xs font-medium text-zinc-900">
                    {item.title}
                  </strong>
                  <small className="text-[11px] text-zinc-400">
                    ID: {item.externalId} · 发布于 {item.publishedAt}
                  </small>
                </div>
                <div className="space-y-1">
                  <div className="flex flex-wrap gap-1.5">
                    <span className="rounded-md border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[11px] text-zinc-700">
                      {item.platform}
                    </span>
                    <span className="rounded-md bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-600">
                      {item.contentType}
                    </span>
                  </div>
                  <small className="block text-[11px] text-zinc-400">{item.accountAlias}</small>
                </div>
                <div>
                  <strong className="block text-xs font-semibold text-zinc-900">
                    {item.readsLabel}
                  </strong>
                  <small className="text-[11px] text-zinc-400">
                    {item.readsMetric} · 赞 {item.likes} / 评 {item.comments} / 藏 {item.favorites}
                  </small>
                </div>
                <div>
                  <button
                    className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-md border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-50"
                    onClick={() => showToast(`作品 ${item.externalId} 历史指标快照视图开发中`)}
                  >
                    <ExternalLink size={12} />
                    详情
                  </button>
                </div>
              </div>
            ))}
          </div>
        </article>
      </div>
    </main>
  );
}
