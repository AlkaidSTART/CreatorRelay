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
import { PlatformLogo } from "../components/PlatformLogo";
import type { PublishTaskStatus } from "../data/demoData";
import { useContentStore, useUiStore } from "../stores";
import { getPlatformVisual } from "../utils/platformStyle";

const statusMeta: Record<PublishTaskStatus, { label: string; className: string }> = {
  PENDING: { label: "等待执行", className: "bg-sidebar text-ink-secondary" },
  SCHEDULED: { label: "定时发布", className: "bg-sky-bg text-sky-text" },
  PUBLISHING: { label: "发布中", className: "bg-accent-soft text-accent-deep" },
  SUCCESS: { label: "发布成功", className: "bg-mint-bg text-mint-text" },
  FAILED: { label: "需核对重试", className: "bg-coral-bg text-coral-text" },
  NEEDS_MANUAL_ACTION: { label: "待你确认", className: "bg-lemon-bg text-lemon-text" },
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
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="PUBLISHING"
        title="发布中心"
        description="把一篇主稿拆分成各平台独立适配任务，单平台失败互不影响，确认封面与标签后即可发布。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.24)] transition-colors hover:bg-accent-hover"
            onClick={() => {
              showToast("在写作空间顶部的「分发到平台」栏即可一键加入发布队列");
              navigate("/studio");
            }}
          >
            <Plus size={13} />
            从主稿发起分发
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
                    ? "border-accent bg-accent text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)]"
                    : "border-line bg-surface text-ink-secondary hover:bg-subtle"
                }`}
              >
                <span>{option.label}</span>
                <span
                  className={`rounded-full px-1.5 text-[10px] ${
                    filter === option.id ? "bg-surface/20 text-white" : "bg-sidebar text-ink-muted"
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
                className="flex flex-col justify-between gap-4 rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_10px_rgba(74,58,48,0.03)] transition-colors hover:border-line-strong sm:flex-row sm:items-center"
              >
                <div className="flex min-w-0 flex-1 items-start gap-3.5">
                  <div
                    className={`grid h-10 w-10 shrink-0 place-items-center rounded-xl border ${visual.badgeClass}`}
                  >
                    <PlatformLogo platformName={task.platform} size={22} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <strong className="text-sm font-medium text-ink">
                        {task.articleTitle}
                      </strong>
                      <span
                        className={`rounded-md px-2 py-0.5 text-[11px] font-medium ${meta.className}`}
                      >
                        {meta.label}
                      </span>
                    </div>
                    <small className="mt-1 block text-xs text-ink-muted">
                      {task.platform} · {task.accountAlias} · 更新于 {task.updatedAt}
                      {task.scheduledAt ? ` · 计划时间 ${task.scheduledAt}` : ""}
                    </small>
                    <p className="mt-1.5 text-xs leading-relaxed text-ink-secondary">{task.note}</p>
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {task.status === "NEEDS_MANUAL_ACTION" && (
                    <button
                      className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)] transition-colors hover:bg-accent-hover"
                      onClick={() => completePublishTask(task.id)}
                    >
                      <ExternalLink size={12} />
                      确认并发布
                    </button>
                  )}
                  {task.status === "FAILED" && (
                    <button
                      className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-coral-border bg-coral-bg px-3 text-xs font-medium text-coral-text hover:bg-coral-border"
                      onClick={() => retryPublishTask(task.id)}
                    >
                      <RefreshCw size={12} />
                      检查并重试
                    </button>
                  )}
                  {task.status === "SCHEDULED" && (
                    <span className="inline-flex items-center gap-1.5 text-xs text-sky-text">
                      <Clock3 size={12} /> 定时排队中
                    </span>
                  )}
                  {task.status === "SUCCESS" && (
                    <span className="inline-flex items-center gap-1.5 text-xs font-medium text-mint-text">
                      <CheckCircle2 size={12} /> 已归档外部链接
                    </span>
                  )}
                </div>
              </article>
            );
          })}

          {!visibleTasks.length && (
            <div className="flex flex-col items-center justify-center gap-2 rounded-2xl border border-line bg-surface py-14 text-ink-muted">
              <AlertCircle size={20} />
              <strong className="text-xs font-medium text-ink-secondary">
                当前筛选条件下暂无发布任务
              </strong>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}
