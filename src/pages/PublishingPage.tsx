import { useState } from "react";
import { AlertCircle, CheckCircle2, Clock3, ExternalLink, Plus, RefreshCw } from "lucide-react";
import { useNavigate, useOutletContext } from "react-router";
import { PageHeader } from "../components/PageHeader";
import { demoPublishTasks, type PublishTaskStatus } from "../data/demoData";
import type { AppOutletContext } from "../layouts/AppLayout";

const statusMeta: Record<PublishTaskStatus, { label: string; tone: string }> = {
  PENDING: { label: "等待执行", tone: "neutral" },
  SCHEDULED: { label: "定时发布", tone: "info" },
  PUBLISHING: { label: "发布中", tone: "info" },
  SUCCESS: { label: "发布成功", tone: "ok" },
  FAILED: { label: "发布失败", tone: "danger" },
  NEEDS_MANUAL_ACTION: { label: "需人工确认", tone: "warn" },
};

const filterOptions: Array<{ id: "ALL" | PublishTaskStatus; label: string }> = [
  { id: "ALL", label: "全部任务" },
  { id: "NEEDS_MANUAL_ACTION", label: "需人工确认" },
  { id: "SCHEDULED", label: "定时发布" },
  { id: "SUCCESS", label: "已完成" },
  { id: "FAILED", label: "失败待核对" },
];

export function PublishingPage() {
  const { showToast } = useOutletContext<AppOutletContext>();
  const navigate = useNavigate();
  const [filter, setFilter] = useState<"ALL" | PublishTaskStatus>("ALL");

  const visibleTasks =
    filter === "ALL" ? demoPublishTasks : demoPublishTasks.filter((task) => task.status === filter);

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="PUBLISHING"
        title="发布中心"
        description="从主稿或平台稿创建独立发布任务，支持发布前确认、定时计划、失败核对与人工发布辅助。"
        priority="P0/P1"
        actions={
          <button
            className="publish-button"
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

      <div className="page-body">
        <div className="callout-bar">
          <span className="note-line" />
          <span>
            业务约束：多平台分发拆分为多个独立任务，单平台失败不影响其他任务；请求超时后先核对平台作品状态，避免重复发布。
          </span>
        </div>

        <div className="filter-tabs" role="tablist" aria-label="任务状态筛选">
          {filterOptions.map((option) => (
            <button
              key={option.id}
              role="tab"
              aria-selected={filter === option.id}
              className={`filter-tab ${filter === option.id ? "active" : ""}`}
              onClick={() => setFilter(option.id)}
            >
              {option.label}
            </button>
          ))}
        </div>

        <div className="stack-list">
          {visibleTasks.map((task) => {
            const meta = statusMeta[task.status];
            return (
              <article className="surface-card task-card" key={task.id}>
                <div className="task-main">
                  <div className="platform-mark juejin-mark">{task.platformMark}</div>
                  <div className="list-row-copy">
                    <div className="task-title-line">
                      <strong>{task.articleTitle}</strong>
                      <span className={`status-badge ${meta.tone}`}>{meta.label}</span>
                    </div>
                    <small>
                      {task.platform} · {task.accountAlias} · {task.draftType} · 更新于{" "}
                      {task.updatedAt}
                      {task.scheduledAt ? ` · 计划时间 ${task.scheduledAt}` : ""}
                    </small>
                    <p className="task-note">{task.note}</p>
                  </div>
                </div>
                <div className="task-actions">
                  {task.status === "NEEDS_MANUAL_ACTION" && (
                    <button
                      className="primary-button"
                      onClick={() => showToast(`已打开 ${task.platform} 人工发布辅助核对清单`)}
                    >
                      <ExternalLink size={13} />
                      人工辅助发布
                    </button>
                  )}
                  {task.status === "FAILED" && (
                    <button
                      className="secondary-button"
                      onClick={() => showToast("请先在平台管理中验证账号会话，再核对发布结果")}
                    >
                      <RefreshCw size={13} />
                      检查并重试
                    </button>
                  )}
                  {task.status === "SCHEDULED" && (
                    <span className="muted-inline">
                      <Clock3 size={13} /> 排队中
                    </span>
                  )}
                  {task.status === "SUCCESS" && (
                    <span className="ok-inline">
                      <CheckCircle2 size={13} /> 已记录外部 ID
                    </span>
                  )}
                </div>
              </article>
            );
          })}
          {!visibleTasks.length && (
            <div className="surface-card empty-state-card">
              <AlertCircle size={20} />
              <strong>当前筛选条件下暂无发布任务</strong>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}
