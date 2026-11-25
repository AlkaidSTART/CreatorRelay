import { useState } from "react";
import { BarChart3, TrendingUp } from "lucide-react";
import { PageHeader } from "../components/PageHeader";

const platformSnapshots = [
  {
    platform: "掘金",
    account: "技术主号",
    metricType: "reads",
    sourceMetricName: "文章阅读量",
    value: "24,680",
    delta: "+14.2%",
    followers: "3,420",
    capturedAt: "今天 09:30",
    barPercent: 82,
  },
  {
    platform: "Bilibili",
    account: "林工视频号",
    metricType: "views",
    sourceMetricName: "视频播放量",
    value: "41,920",
    delta: "+22.8%",
    followers: "5,180",
    capturedAt: "今天 08:15",
    barPercent: 95,
  },
  {
    platform: "知乎",
    account: "林工笔记",
    metricType: "reads",
    sourceMetricName: "专栏浏览量",
    value: "12,340",
    delta: "+6.4%",
    followers: "1,890",
    capturedAt: "昨天 21:00",
    barPercent: 54,
  },
  {
    platform: "CSDN",
    account: "Rust 专栏号",
    metricType: "reads",
    sourceMetricName: "博客访问量",
    value: "16,510",
    delta: "+9.1%",
    followers: "2,210",
    capturedAt: "今天 09:00",
    barPercent: 66,
  },
];

const rangeOptions = [
  { id: "7d", label: "近 7 天" },
  { id: "30d", label: "近 30 天" },
  { id: "90d", label: "近 90 天" },
];

export function AnalyticsPage() {
  const [range, setRange] = useState("7d");

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="ANALYTICS"
        title="数据分析"
        description="按账号、内容、平台与时间维度观察趋势，保留各平台原始指标名称、单位与采集时间。"
        priority="P0"
        actions={
          <div className="filter-tabs">
            {rangeOptions.map((item) => (
              <button
                key={item.id}
                className={`filter-tab ${range === item.id ? "active" : ""}`}
                onClick={() => setRange(item.id)}
              >
                {item.label}
              </button>
            ))}
          </div>
        }
      />

      <div className="page-body">
        <div className="callout-bar">
          <span className="note-line" />
          <span>
            统一指标字典：followers / views / reads / likes / comments / shares /
            favorites。每个指标快照均记录 sourceMetricName 与 capturedAt，避免跨平台错误混算。
          </span>
        </div>

        <section className="two-col-grid">
          {platformSnapshots.map((item) => (
            <article className="surface-card" key={item.platform}>
              <div className="card-heading">
                <div>
                  <span className="context-eyebrow">
                    {item.metricType.toUpperCase()} · {item.sourceMetricName}
                  </span>
                  <h2>
                    {item.platform} · {item.account}
                  </h2>
                </div>
                <span className="status-badge ok">
                  <TrendingUp size={12} /> {item.delta}
                </span>
              </div>
              <div className="analytics-kpi">
                <strong>{item.value}</strong>
                <span>粉丝 {item.followers}</span>
              </div>
              <div className="bar-track" aria-hidden="true">
                <div className="bar-fill" style={{ width: `${item.barPercent}%` }} />
              </div>
              <div className="analytics-meta">
                <span>
                  <BarChart3 size={12} /> 统计窗口：
                  {rangeOptions.find((r) => r.id === range)?.label}
                </span>
                <span>采集于 {item.capturedAt}</span>
              </div>
            </article>
          ))}
        </section>
      </div>
    </main>
  );
}
