import { AlertCircle, CheckCircle2, Clock3, ExternalLink, Plus, RefreshCw } from "lucide-react";
import { useNavigate } from "react-router";
import { PageHeader } from "../components/PageHeader";
import type { PublishTaskStatus } from "../data/demoData";
import { useContentStore, useUiStore } from "../stores";

const statusMeta: Record<PublishTaskStatus, { label: string; className: string }> = {
  PENDING: { label: "等待执行", className: "bg-zinc-100 text-zinc-600" },
  SCHEDULED: { label: "定时发布", className: "bg-blue-50 text-blue-700" },
  PUBLISHING: { label: "发布中", className: "bg-blue-50 text-blue-700" },
  SUCCESS: { label: "发布成功", className: "bg-emerald-50 text-emerald-700" },
  FAILED: { label: "发布失败", className: "bg-red-50 text-red-700" },
  NEEDS_MANUAL_ACTION: { label: "需人工确认", className: "bg-amber-50 text-amber-700" },
};

const filterOptions: Array<{ id: "ALL" | PublishTaskStatus; label: string }> = [
  { id: "ALL", label: "全部任务" },
  { id: "NEEDS_MANUAL_ACTION", label: "需人工确认" },
  { id: "SCHEDULED", label: "定时发布" },
  { id: "SUCCESS", label: "已完成" },
  { id: "FAILED", label: "失败待核对" },
];

export function PublishingPage() {
  const navigate = useNavigate();
  const showToast = useUiStore((state) => state.showToast);
  const publishTasks = useContentStore((state) => state.publishTasks);
  const filter = useContentStore((state) => state.publishingFilter);
  const setFilter = useContentStore((state) => state.setPublishingFilter);

  const visibleTasks =
    filter === "ALL" ? publishTasks : publishTasks.filter((task) => task.status === filter);

  return (
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="PUBLISHING"
        title="发布中心"
        description="从主稿或平台稿创建独立发布任务，支持发布前确认、定时计划、失败核对与人工发布辅助。"
        priority="P0/P1"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3 text-xs font-medium text-white transition-colors hover:bg-zinc-800"
            onClick={() => {
              showToast("请先在写作空间选择主稿或平台稿发起发布");
              navigate("/studio");
            }}
          >
            <Plus size={14} />
            从主稿创建任务
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex items-center gap-3 rounded-xl border border-zinc-200/80 bg-zinc-50/80 px-4 py-3 text-xs text-zinc-500">
          <span className="h-4 w-0.5 shrink-0 rounded-full bg-zinc-400" />
          <span>
            业务约束：多平台分发拆分为多个独立任务，单平台失败不影响其他任务；请求超时后先核对平台作品状态，避免重复发布。
          </span>
        </div>

        <div className="flex flex-wrap gap-1.5" role="tablist" aria-label="任务状态筛选">
          {filterOptions.map((option) => (
            <button
              key={option.id}
              role="tab"
              aria-selected={filter === option.id}
              onClick={() => setFilter(option.id)}
              className={`h-8 cursor-pointer rounded-lg border px-3 text-xs font-medium transition-colors ${
                filter === option.id
                  ? "border-zinc-900 bg-zinc-900 text-white"
                  : "border-zinc-200 bg-white text-zinc-600 hover:bg-zinc-50"
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>

        <div className="space-y-3">
          {visibleTasks.map((task) => {
            const meta = statusMeta[task.status];
            return (
              <article
                key={task.id}
                className="flex flex-col justify-between gap-4 rounded-xl border border-zinc-200/80 bg-white p-5 sm:flex-row sm:items-center"
              >
                <div className="flex min-w-0 flex-1 items-start gap-3.5">
                  <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-zinc-100 text-xs font-semibold text-zinc-700">
                    {task.platformMark}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <strong className="text-sm font-medium text-zinc-900">
                        {task.articleTitle}
                      </strong>
                      <span
                        className={`rounded-md px-2 py-0.5 text-[11px] font-medium ${meta.className}`}
                      >
                        {meta.label}
                      </span>
                    </div>
                    <small className="mt-1 block text-xs text-zinc-400">
                      {task.platform} · {task.accountAlias} · {task.draftType} · 更新于{" "}
                      {task.updatedAt}
                      {task.scheduledAt ? ` · 计划时间 ${task.scheduledAt}` : ""}
                    </small>
                    <p className="mt-1.5 text-xs leading-relaxed text-zinc-500">{task.note}</p>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {task.status === "NEEDS_MANUAL_ACTION" && (
                    <button
                      className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3 text-xs font-medium text-white hover:bg-zinc-800"
                      onClick={() => showToast(`已打开 ${task.platform} 人工发布辅助核对清单`)}
                    >
                      <ExternalLink size={13} />
                      人工辅助发布
                    </button>
                  )}
                  {task.status === "FAILED" && (
                    <button
                      className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-3 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                      onClick={() => showToast("请先在平台管理中验证账号会话，再核对发布结果")}
                    >
                      <RefreshCw size={13} />
                      检查并重试
                    </button>
                  )}
                  {task.status === "SCHEDULED" && (
                    <span className="inline-flex items-center gap-1.5 text-xs text-zinc-500">
                      <Clock3 size={13} /> 排队中
                    </span>
                  )}
                  {task.status === "SUCCESS" && (
                    <span className="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-600">
                      <CheckCircle2 size={13} /> 已记录外部 ID
                    </span>
                  )}
                </div>
              </article>
            );
          })}

          {!visibleTasks.length && (
            <div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-zinc-200/80 bg-white py-14 text-zinc-400">
              <AlertCircle size={20} />
              <strong className="text-xs font-medium text-zinc-600">
                当前筛选条件下暂无发布任务
              </strong>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}
