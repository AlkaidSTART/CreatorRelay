import { BarChart3, TrendingUp } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useContentStore, type AnalyticsRange } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

const platformSnapshots = [
  {
    platform: "掘金",
    account: "技术主号",
    sourceMetricName: "文章阅读量",
    value: "24,680",
    delta: "+14.2%",
    followers: "3,420",
    capturedAt: "今天 09:30",
    barPercent: 82,
    goalLabel: "月目标达成",
  },
  {
    platform: "Bilibili",
    account: "林工视频号",
    sourceMetricName: "视频播放量",
    value: "41,920",
    delta: "+22.8%",
    followers: "5,180",
    capturedAt: "今天 08:15",
    barPercent: 95,
    goalLabel: "月目标达成",
  },
  {
    platform: "知乎",
    account: "林工笔记",
    sourceMetricName: "专栏浏览量",
    value: "12,340",
    delta: "+6.4%",
    followers: "1,890",
    capturedAt: "昨天 21:00",
    barPercent: 54,
    goalLabel: "月目标达成",
  },
  {
    platform: "CSDN",
    account: "Rust 专栏号",
    sourceMetricName: "博客访问量",
    value: "16,510",
    delta: "+9.1%",
    followers: "2,210",
    capturedAt: "今天 09:00",
    barPercent: 66,
    goalLabel: "月目标达成",
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
    <main className="flex h-full min-w-0 flex-col bg-[#FAF7F4]">
      <PageHeader
        eyebrow="ANALYTICS"
        title="数据分析"
        description="按账号、平台与时间维度观察创作增长，保留各平台真实指标名称与采集时间。"
        actions={
          <div className="flex items-center rounded-xl bg-[#F4EFEA] p-0.5">
            {rangeOptions.map((item) => (
              <button
                key={item.id}
                onClick={() => setRange(item.id)}
                className={`h-7 cursor-pointer rounded-lg px-2.5 text-xs font-medium transition-all ${
                  range === item.id
                    ? "bg-white text-[#332B29] shadow-[0_1px_3px_rgba(74,58,48,0.08)]"
                    : "text-[#6F6561] hover:text-[#332B29]"
                }`}
              >
                {item.label}
              </button>
            ))}
          </div>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <section className="grid grid-cols-1 gap-5 md:grid-cols-2">
          {platformSnapshots.map((item) => {
            const visual = getPlatformVisual(item.platform);
            return (
              <article
                key={item.platform}
                className="rounded-2xl border border-[#EAE2D9] bg-white p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]"
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="flex items-center gap-1.5 text-[11px] text-[#9A8F8A]">
                      <span
                        className="h-2 w-2 rounded-full"
                        style={{ backgroundColor: visual.color }}
                      />
                      <span>{item.sourceMetricName}</span>
                    </div>
                    <h2 className="mt-1 text-base font-semibold text-[#332B29]">
                      {item.platform} · {item.account}
                    </h2>
                  </div>
                  <span className="inline-flex items-center gap-1 rounded-lg bg-[#E7F4EE] px-2 py-0.5 text-xs font-medium text-[#286B51]">
                    <TrendingUp size={11} /> {item.delta}
                  </span>
                </div>

                <div className="mt-4 mb-2.5 flex items-baseline justify-between">
                  <strong className="text-3xl font-semibold tracking-tight tabular-nums text-[#332B29]">
                    {item.value}
                  </strong>
                  <span className="text-xs text-[#6F6561]">粉丝 {item.followers}</span>
                </div>

                <div className="mb-1.5 flex items-center justify-between text-[11px] text-[#9A8F8A]">
                  <span>{item.goalLabel}</span>
                  <span className="font-mono">{item.barPercent}%</span>
                </div>
                <div className="h-2 overflow-hidden rounded-full bg-[#F4EFEA]" aria-hidden="true">
                  <div
                    className="h-full rounded-full"
                    style={{ width: `${item.barPercent}%`, backgroundColor: visual.barColor }}
                  />
                </div>

                <div className="mt-3.5 flex items-center justify-between border-t border-[#F0E9E1] pt-3 text-[11px] text-[#9A8F8A]">
                  <span className="inline-flex items-center gap-1">
                    <BarChart3 size={11} />
                    统计窗口：{rangeOptions.find((r) => r.id === range)?.label}
                  </span>
                  <span>采集于 {item.capturedAt}</span>
                </div>
              </article>
            );
          })}
        </section>
      </div>
    </main>
  );
}
