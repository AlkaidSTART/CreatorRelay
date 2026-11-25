import {
  AlertCircle,
  CheckCircle2,
  Clock3,
  ExternalLink,
  Plus,
  RefreshCw,
} from "../components/Icons";
import { useNavigate } from "react-router";
import { PageHeader } from "../components/PageHeader";
import type { PublishTaskStatus } from "../data/demoData";
import { useContentStore, useUiStore } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

const statusMeta: Record<PublishTaskStatus, { label: string; className: string }> = {
  PENDING: { label: "等待执行", className: "bg-[#F4F1EA] text-[#646B64]" },
  SCHEDULED: { label: "定时发布", className: "bg-[#EEF4FF] text-[#1E60D5]" },
  PUBLISHING: { label: "发布中", className: "bg-[#EEF4FF] text-[#1E60D5]" },
  SUCCESS: { label: "发布成功", className: "bg-[#E8F6EE] text-[#1E7548]" },
  FAILED: { label: "需核对重试", className: "bg-[#FDEEEE] text-[#C23B3B]" },
  NEEDS_MANUAL_ACTION: { label: "待你确认", className: "bg-[#FDF3DC] text-[#9A6314]" },
};

const filterOptions: Array<{ id: "ALL" | PublishTaskStatus; label: string }> = [
  { id: "ALL", label: "全部任务" },
  { id: "NEEDS_MANUAL_ACTION", label: "待确认" },
  { id: "SCHEDULED", label: "定时队列" },
  { id: "SUCCESS", label: "已发布" },
  { id: "FAILED", label: "需重试" },
];

export function PublishingPage() {
  const navigate = useNavigate();
  const showToast = useUiStore((state) => state.showToast);
  const publishTasks = useContentStore((state) => state.publishTasks);
  const filter = useContentStore((state) => state.publishingFilter);
  const setFilter = useContentStore((state) => state.setPublishingFilter);
  const completePublishTask = useContentStore((state) => state.completePublishTask);
  const retryPublishTask = useContentStore((state) => state.retryPublishTask);

  const visibleTasks =
    filter === "ALL" ? publishTasks : publishTasks.filter((task) => task.status === filter);

  return (
    <main className="flex h-full min-w-0 flex-col bg-[#FAF8F3]">
      <PageHeader
        eyebrow="PUBLISHING RELAY"
        title="发布中心"
        description="把一篇主稿拆分成各平台独立适配任务，单平台失败互不影响，确认封面与标签后即可安心发布。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-[#2F4A36] px-3.5 text-xs font-medium text-white shadow-xs transition-colors hover:bg-[#233A29]"
            onClick={() => {
              showToast("在写作空间顶部的「一键接力分发」栏即可快速加入发布队列");
              navigate("/studio");
            }}
          >
            <Plus size={13} />
            从主稿发起接力
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap gap-1.5" role="tablist" aria-label="任务状态筛选">
          {filterOptions.map((option) => {
            const count =
              option.id === "ALL"
                ? publishTasks.length
                : publishTasks.filter((t) => t.status === option.id).length;
            return (
              <button
                key={option.id}
                role="tab"
                aria-selected={filter === option.id}
                onClick={() => setFilter(option.id)}
                className={`inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border px-3 text-xs font-medium transition-colors ${
                  filter === option.id
                    ? "border-[#2F4A36] bg-[#2F4A36] text-white"
                    : "border-[#E3DED2] bg-white text-[#545C54] hover:border-[#CFC8B9] hover:bg-[#FDFCF9]"
                }`}
              >
                <span>{option.label}</span>
                <span
                  className={`rounded-full px-1.5 py-0.2 text-[10px] ${
                    filter === option.id
                      ? "bg-white/20 text-white"
                      : "bg-[#F4F1EA] text-[#7D847D]"
                  }`}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>

        <div className="space-y-3">
          {visibleTasks.map((task) => {
            const meta = statusMeta[task.status];
            const visual = getPlatformVisual(task.platform);
            return (
              <article
                key={task.id}
                className="flex flex-col justify-between gap-4 rounded-2xl border border-[#E6E2D8] bg-white p-5 transition-all hover:border-[#D4CEC2] sm:flex-row sm:items-center"
              >
                <div className="flex min-w-0 flex-1 items-start gap-3.5">
                  <div
                    className={`grid h-10 w-10 shrink-0 place-items-center rounded-xl text-xs font-semibold ${visual.avatarClass}`}
                  >
                    {task.platformMark}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <strong className="text-sm font-semibold text-[#1E241E]">
                        {task.articleTitle}
                      </strong>
                      <span
                        className={`rounded-full px-2.5 py-0.5 text-[11px] font-medium ${meta.className}`}
                      >
                        {meta.label}
                      </span>
                    </div>
                    <small className="mt-1 block text-xs text-[#858B83]">
                      {task.platform} · {task.accountAlias} · 更新于 {task.updatedAt}
                      {task.scheduledAt ? ` · 计划时间 ${task.scheduledAt}` : ""}
                    </small>
                    <p className="mt-1.5 text-xs leading-relaxed text-[#646B64]">{task.note}</p>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {task.status === "NEEDS_MANUAL_ACTION" && (
                    <button
                      className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl bg-[#2F4A36] px-3.5 text-xs font-medium text-white shadow-xs transition-colors hover:bg-[#233A29]"
                      onClick={() => completePublishTask(task.id)}
                    >
                      <ExternalLink size={12} />
                      确认并发布
                    </button>
                  )}
                  {task.status === "FAILED" && (
                    <button
                      className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl border border-[#E4D1AC] bg-[#FFFBF2] px-3.5 text-xs font-medium text-[#7A5416] transition-colors hover:bg-[#FDF3DC]"
                      onClick={() => retryPublishTask(task.id)}
                    >
                      <RefreshCw size={12} />
                      检查并重试
                    </button>
                  )}
                  {task.status === "SCHEDULED" && (
                    <span className="inline-flex items-center gap-1.5 rounded-xl bg-[#EEF4FF] px-3 py-1.5 text-xs font-medium text-[#1E60D5]">
                      <Clock3 size={12} /> 定时排队中
                    </span>
                  )}
                  {task.status === "SUCCESS" && (
                    <span className="inline-flex items-center gap-1.5 rounded-xl bg-[#E8F6EE] px-3 py-1.5 text-xs font-medium text-[#1E7548]">
                      <CheckCircle2 size={12} /> 已归档外部链接
                    </span>
                  )}
                </div>
              </article>
            );
          })}

          {!visibleTasks.length && (
            <div className="flex flex-col items-center justify-center gap-2 rounded-2xl border border-[#E6E2D8] bg-white py-14 text-[#858B83]">
              <AlertCircle size={20} />
              <strong className="text-xs font-medium text-[#545C54]">
                当前筛选条件下暂无发布任务
              </strong>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}
