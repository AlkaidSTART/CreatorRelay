import { useMemo, useState } from "react";
import { ExternalLink, Search } from "lucide-react";
import { useOutletContext } from "react-router";
import { PageHeader } from "../components/PageHeader";
import { demoPublishedContents } from "../data/demoData";
import type { AppOutletContext } from "../layouts/AppLayout";

const platforms = ["全部平台", "掘金", "知乎", "Bilibili", "CSDN"];

export function ContentPage() {
  const { showToast } = useOutletContext<AppOutletContext>();
  const [keyword, setKeyword] = useState("");
  const [selectedPlatform, setSelectedPlatform] = useState("全部平台");

  const filtered = useMemo(() => {
    const term = keyword.trim().toLocaleLowerCase();
    return demoPublishedContents.filter((item) => {
      const matchPlatform = selectedPlatform === "全部平台" || item.platform === selectedPlatform;
      const matchKeyword =
        !term ||
        `${item.title} ${item.platform} ${item.accountAlias} ${item.externalId}`
          .toLocaleLowerCase()
          .includes(term);
      return matchPlatform && matchKeyword;
    });
  }, [keyword, selectedPlatform]);

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="CONTENT"
        title="内容中心"
        description="统一聚合和检索已同步的外部平台作品，保留外部作品 ID、来源账号与独立口径指标。"
        priority="P0"
      />

      <div className="page-body">
        <div className="toolbar-row">
          <div className="rail-search content-search">
            <Search size={14} />
            <input
              value={keyword}
              onChange={(event) => setKeyword(event.target.value)}
              placeholder="按标题、平台、账号或外部 ID 检索作品"
              aria-label="检索跨平台作品"
            />
          </div>
          <div className="filter-tabs">
            {platforms.map((platform) => (
              <button
                key={platform}
                className={`filter-tab ${selectedPlatform === platform ? "active" : ""}`}
                onClick={() => setSelectedPlatform(platform)}
              >
                {platform}
              </button>
            ))}
          </div>
        </div>

        <article className="surface-card">
          <div className="data-table">
            <div className="table-head">
              <span>作品标题 / 外部 ID</span>
              <span>平台 / 账号 / 形态</span>
              <span>核心指标（原始口径）</span>
              <span>操作</span>
            </div>
            {filtered.map((item) => (
              <div className="table-row" key={item.id}>
                <div className="table-primary">
                  <strong>{item.title}</strong>
                  <small>
                    ID: {item.externalId} · 发布于 {item.publishedAt}
                  </small>
                </div>
                <div className="table-primary">
                  <div>
                    <span className="tag-chip">{item.platform}</span>{" "}
                    <span className="tag-chip">{item.contentType}</span>
                  </div>
                  <small>{item.accountAlias}</small>
                </div>
                <div className="table-primary">
                  <strong>{item.readsLabel}</strong>
                  <small>
                    {item.readsMetric} · 赞 {item.likes} / 评 {item.comments} / 藏 {item.favorites}
                  </small>
                </div>
                <div>
                  <button
                    className="secondary-button"
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
