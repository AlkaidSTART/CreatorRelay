import { BarChart3, TrendingUp } from "lucide-react";
import { PageHeader } from "../components/PageHeader";
import { useContentStore, type AnalyticsRange } from "../stores";

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

const rangeOptions: Array<{ id: AnalyticsRange; label: string }> = [
  { id: "7d", label: "近 7 天" },
  { id: "30d", label: "近 30 天" },
  { id: "90d", label: "近 90 天" },
];

export function AnalyticsPage() {
  const range = useContentStore((state) => state.analyticsRange);
  const setRange = useContentStore((state) => state.setAnalyticsRange);

  return (
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="ANALYTICS"
        title="数据分析"
        description="按账号、内容、平台与时间维度观察趋势，保留各平台原始指标名称、单位与采集时间。"
        priority="P0"
        actions={
          <div className="flex items-center rounded-lg bg-zinc-100 p-0.5">
            {rangeOptions.map((item) => (
              <button
                key={item.id}
                onClick={() => setRange(item.id)}
                className={`h-7 cursor-pointer rounded-md px-2.5 text-xs font-medium transition-all ${
                  range === item.id
                    ? "bg-white text-zinc-900 shadow-2xs"
                    : "text-zinc-500 hover:text-zinc-800"
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex items-center gap-3 rounded-xl border border-zinc-200/80 bg-zinc-50/80 px-4 py-3 text-xs text-zinc-500">
          <span className="h-4 w-0.5 shrink-0 rounded-full bg-zinc-400" />
          <span>
            统一指标字典：followers / views / reads / likes / comments / shares /
            favorites。每个指标快照均记录 sourceMetricName 与 capturedAt，避免跨平台错误混算。
          </span>
        </div>

        <section className="grid grid-cols-1 gap-5 md:grid-cols-2">
          {platformSnapshots.map((item) => (
            <article
              key={item.platform}
              className="rounded-xl border border-zinc-200/80 bg-white p-5"
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <span className="text-[10px] font-semibold tracking-widest text-zinc-400 uppercase">
                    {item.metricType} · {item.sourceMetricName}
                  </span>
                  <h2 className="mt-1 text-base font-semibold text-zinc-900">
                    {item.platform} · {item.account}
                  </h2>
                </div>
                <span className="inline-flex items-center gap-1 rounded-md bg-emerald-50 px-2 py-0.5 text-xs font-medium text-emerald-700">
                  <TrendingUp size={12} /> {item.delta}
                </span>
              </div>

              <div className="mt-4 mb-3 flex items-baseline justify-between">
                <strong className="text-3xl font-semibold tracking-tight text-zinc-900">
                  {item.value}
                </strong>
                <span className="text-xs text-zinc-500">粉丝 {item.followers}</span>
              </div>

              <div className="h-2 overflow-hidden rounded-full bg-zinc-100" aria-hidden="true">
                <div
                  className="h-full rounded-full bg-zinc-900"
                  style={{ width: `${item.barPercent}%` }}
                />
              </div>

              <div className="mt-3.5 flex items-center justify-between text-[11px] text-zinc-400">
                <span className="inline-flex items-center gap-1">
                  <BarChart3 size={12} />
                  统计窗口：{rangeOptions.find((r) => r.id === range)?.label}
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
