import { useEffect, useMemo, useState } from "react";
import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { useNavigate } from "react-router";
import {
  AlertCircle,
  AlertTriangle,
  CheckCircle2,
  Clock3,
  ExternalLink,
  FolderOpen,
  Play,
  Plus,
  RefreshCw,
  X,
} from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { useOperationsStore, useUiStore } from "../stores";
import type { PublishTask, PublishTaskStatus, ResolveUncertainInput } from "../types/operations";
import {
  PUBLISH_STATUS_META,
  contentKindText,
  platformHomeUrl,
  taskActionSummary,
} from "../utils/operations";

const filterOptions: Array<{ id: "ALL" | PublishTaskStatus; label: string }> = [
  { id: "ALL", label: "全部任务" },
  { id: "CREATED", label: "待确认" },
  { id: "NEEDS_MANUAL_ACTION", label: "人工处理" },
  { id: "SCHEDULED", label: "定时队列" },
  { id: "SUCCESS", label: "已发布" },
  { id: "FAILED", label: "失败" },
];

function formatTime(value?: string | null) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function isUncertain(task: PublishTask) {
  return task.errorCode === "RESULT_UNCERTAIN" || task.errorCode === "RESULT_MISMATCH";
}

export function PublishingPage() {
  const navigate = useNavigate();
  const showToast = useUiStore((state) => state.showToast);
  const tasks = useOperationsStore((state) => state.tasks);
  const preview = useOperationsStore((state) => state.preview);
  const initialize = useOperationsStore((state) => state.initialize);
  const loadTasks = useOperationsStore((state) => state.loadTasks);
  const prepareTask = useOperationsStore((state) => state.prepareTask);
  const confirmTask = useOperationsStore((state) => state.confirmTask);
  const cancelTask = useOperationsStore((state) => state.cancelTask);
  const retryTask = useOperationsStore((state) => state.retryTask);
  const updateTaskSchedule = useOperationsStore((state) => state.updateTaskSchedule);
  const runDueTasks = useOperationsStore((state) => state.runDueTasks);
  const exportManualPackage = useOperationsStore((state) => state.exportManualPackage);
  const completeManualTask = useOperationsStore((state) => state.completeManualTask);
  const resolveUncertainTask = useOperationsStore((state) => state.resolveUncertainTask);
  const resumeAccountTasks = useOperationsStore((state) => state.resumeAccountTasks);
  const clearPreview = useOperationsStore((state) => state.clearPreview);
  const [filter, setFilter] = useState<"ALL" | PublishTaskStatus>("ALL");
  const [manualTask, setManualTask] = useState<PublishTask | null>(null);
  const [resolveTask, setResolveTask] = useState<PublishTask | null>(null);
  const [scheduleTask, setScheduleTask] = useState<PublishTask | null>(null);
  const [scheduleValue, setScheduleValue] = useState("");
  const [externalId, setExternalId] = useState("");
  const [externalUrl, setExternalUrl] = useState("");
  const [working, setWorking] = useState(false);
  const [confirming, setConfirming] = useState(false);

  useEffect(() => {
    void initialize();
    void loadTasks();
  }, [initialize, loadTasks]);

  const visibleTasks = useMemo(
    () => (filter === "ALL" ? tasks : tasks.filter((task) => task.status === filter)),
    [filter, tasks],
  );

  async function handlePreview(task: PublishTask) {
    setWorking(true);
    try {
      const result = await prepareTask(task.id);
      if (result && !result.validation.ok) {
        showToast("发布前检查未通过，请先处理预览中的问题");
      }
    } finally {
      setWorking(false);
    }
  }

  async function handleConfirm() {
    if (!preview?.confirmationToken) return;
    setConfirming(true);
    try {
      const task = await confirmTask(preview.taskId, preview.confirmationToken);
      if (task) {
        showToast(
          task.status === "SUCCESS"
            ? "发布成功，外部作品已归档"
            : task.status === "SCHEDULED"
              ? "已进入定时发布队列"
              : "已确认最终内容包，可继续人工发布",
        );
      }
    } finally {
      setConfirming(false);
    }
  }

  async function handleExport(task: PublishTask) {
    const result = await exportManualPackage(task.id);
    if (!result) return;
    try {
      await openPath(result.directory);
    } catch {
      // The path remains available in the toast if the opener is unavailable.
    }
    showToast(`人工发布包已导出：${result.directory}`);
  }

  async function handleOpenPlatform(task: PublishTask) {
    try {
      await openUrl(platformHomeUrl(task.platformId));
    } catch (error) {
      showToast(`无法打开平台：${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async function handleOpenResult(task: PublishTask) {
    if (!task.externalUrl) {
      showToast(task.externalId ? `外部作品 ID：${task.externalId}` : "平台未返回外部链接");
      return;
    }
    try {
      await openUrl(task.externalUrl);
    } catch (error) {
      showToast(`无法打开作品：${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async function handleManualComplete(event: React.FormEvent) {
    event.preventDefault();
    if (!manualTask) return;
    const task = await completeManualTask(manualTask.id, {
      externalId: externalId.trim() || null,
      externalUrl: externalUrl.trim() || null,
    });
    if (task) {
      showToast("人工发布结果已归档");
      setManualTask(null);
      setExternalId("");
      setExternalUrl("");
    }
  }

  async function handleResolve(input: ResolveUncertainInput) {
    if (!resolveTask) return;
    const task = await resolveUncertainTask(resolveTask.id, input);
    if (task) {
      showToast("已记录人工核对结果，不会自动重复提交");
      setResolveTask(null);
      setExternalId("");
      setExternalUrl("");
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="PUBLISHING"
        title="发布中心"
        description="每个目标平台与账号都是独立任务；先预览并确认最终内容包，再执行自动发布或导出人工发布包。"
        actions={
          <div className="flex items-center gap-2">
            <button
              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle"
              onClick={async () => {
                const executed = await runDueTasks();
                showToast(
                  executed.length
                    ? `已执行 ${executed.length} 个到期任务`
                    : "当前没有到期的定时任务",
                );
              }}
            >
              <Play size={12} />
              运行到期任务
            </button>
            <button
              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.24)] transition-colors hover:bg-accent-hover"
              onClick={() => navigate("/studio")}
            >
              <Plus size={13} />
              从平台稿发起
            </button>
          </div>
        }
      />

      <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap gap-1.5" role="tablist" aria-label="任务状态筛选">
          {filterOptions.map((option) => {
            const count =
              option.id === "ALL"
                ? tasks.length
                : tasks.filter((task) => task.status === option.id).length;
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
                <span className="rounded-full bg-surface/20 px-1.5 text-[10px]">{count}</span>
              </button>
            );
          })}
        </div>

        <div className="space-y-3">
          {visibleTasks.map((task) => {
            const meta = PUBLISH_STATUS_META[task.status];
            return (
              <article
                key={task.id}
                className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_10px_rgba(74,58,48,0.03)]"
              >
                <div className="flex flex-col justify-between gap-4 lg:flex-row lg:items-start">
                  <div className="flex min-w-0 flex-1 items-start gap-3.5">
                    <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-line bg-white">
                      <PlatformLogo platformName={task.platformName} size={28} />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <strong className="text-sm font-medium text-ink">{task.title}</strong>
                        <span
                          className={`rounded-md px-2 py-0.5 text-[11px] font-medium ${meta.tone}`}
                        >
                          {meta.label}
                        </span>
                        <span className="rounded-md bg-sidebar px-2 py-0.5 text-[11px] text-ink-secondary">
                          {contentKindText(task.contentKind)}
                        </span>
                        {task.paused && (
                          <span className="inline-flex items-center gap-1 rounded-md bg-coral-bg px-2 py-0.5 text-[11px] text-coral-text">
                            <AlertTriangle size={10} /> 已暂停
                          </span>
                        )}
                      </div>
                      <p className="mt-1.5 text-xs leading-relaxed text-ink-secondary">
                        {taskActionSummary(task)}
                      </p>
                      <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[11px] text-ink-muted">
                        <span>
                          {task.platformName} · {task.accountAlias}
                        </span>
                        <span>{task.executionMode === "auto" ? "自动发布" : "人工发布"}</span>
                        <span>内容版本 {task.contentVersion.slice(0, 10)}</span>
                        <span>提交 {task.attemptCount} 次</span>
                        <span>更新 {formatTime(task.updatedAt)}</span>
                        {task.scheduledAt && <span>计划 {formatTime(task.scheduledAt)}</span>}
                      </div>
                      {(task.externalId || task.externalUrl) && (
                        <div className="mt-2 rounded-lg bg-mint-bg/60 px-2.5 py-1.5 text-[11px] text-mint-text">
                          外部作品：{task.externalId ?? "未提供 ID"}
                          {task.externalUrl ? ` · ${task.externalUrl}` : ""}
                        </div>
                      )}
                    </div>
                  </div>

                  <div className="flex shrink-0 flex-wrap items-center gap-2">
                    {task.status === "CREATED" && (
                      <>
                        <button
                          className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on hover:bg-accent-hover"
                          disabled={working}
                          onClick={() => void handlePreview(task)}
                        >
                          <ExternalLink size={12} /> 预览并确认
                        </button>
                        <button
                          className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
                          onClick={() => {
                            setScheduleTask(task);
                            setScheduleValue("");
                          }}
                        >
                          <Clock3 size={12} /> 定时
                        </button>
                      </>
                    )}
                    {task.status === "SCHEDULED" && (
                      <span className="inline-flex items-center gap-1.5 text-xs text-sky-text">
                        <Clock3 size={12} /> 等待计划时间
                      </span>
                    )}
                    {task.status === "SUCCESS" && (
                      <button
                        className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-mint-border bg-mint-bg px-3 text-xs font-medium text-mint-text"
                        onClick={() => void handleOpenResult(task)}
                      >
                        <CheckCircle2 size={12} /> 查看外部作品
                      </button>
                    )}
                    {task.status === "FAILED" && (
                      <button
                        className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-coral-border bg-coral-bg px-3 text-xs font-medium text-coral-text"
                        onClick={async () => {
                          const updated = await retryTask(task.id);
                          if (updated) showToast("已重新进入预览确认，不会自动提交");
                        }}
                      >
                        <RefreshCw size={12} /> 检查并重新准备
                      </button>
                    )}
                    {task.status === "NEEDS_MANUAL_ACTION" && (
                      <>
                        {task.paused ? (
                          <button
                            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-coral-border bg-coral-bg px-3 text-xs font-medium text-coral-text"
                            onClick={async () => {
                              const resumed = await resumeAccountTasks(task.accountId);
                              showToast(
                                resumed.length
                                  ? "账号已恢复，可重新预览依赖任务"
                                  : "账号仍不可用，请先在平台管理中重新授权",
                              );
                            }}
                          >
                            <RefreshCw size={12} /> 恢复账号任务
                          </button>
                        ) : isUncertain(task) ? (
                          <button
                            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on hover:bg-accent-hover"
                            onClick={() => {
                              setResolveTask(task);
                              setExternalId(task.externalId ?? "");
                              setExternalUrl(task.externalUrl ?? "");
                            }}
                          >
                            <AlertCircle size={12} /> 核对平台结果
                          </button>
                        ) : (
                          <>
                            <button
                              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on hover:bg-accent-hover"
                              onClick={() => void handleExport(task)}
                            >
                              <FolderOpen size={12} /> 导出发布包
                            </button>
                            <button
                              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
                              onClick={() => void handleOpenPlatform(task)}
                            >
                              <ExternalLink size={12} /> 打开平台
                            </button>
                            <button
                              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-mint-border bg-mint-bg px-3 text-xs font-medium text-mint-text"
                              onClick={() => {
                                setManualTask(task);
                                setExternalId("");
                                setExternalUrl("");
                              }}
                            >
                              <CheckCircle2 size={12} /> 标记完成
                            </button>
                          </>
                        )}
                      </>
                    )}
                    {!["SUCCESS", "CANCELLED", "PUBLISHING"].includes(task.status) && (
                      <button
                        className="inline-flex h-8 cursor-pointer items-center gap-1 rounded-xl border border-line px-2.5 text-xs text-ink-muted hover:border-coral-border hover:bg-coral-bg hover:text-coral-text"
                        onClick={async () => {
                          const updated = await cancelTask(task.id, "用户从发布中心取消");
                          if (updated) showToast("发布任务已取消");
                        }}
                      >
                        <X size={12} /> 取消
                      </button>
                    )}
                  </div>
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

      {preview && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-ink/35 p-5 backdrop-blur-sm">
          <div className="max-h-[88vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-line bg-surface p-6 shadow-2xl">
            <div className="flex items-start justify-between gap-4">
              <div>
                <p className="text-[11px] font-semibold tracking-[0.18em] text-accent">
                  最终发布确认
                </p>
                <h2 className="mt-1 text-lg font-semibold text-ink">{preview.task.title}</h2>
                <p className="mt-1 text-xs text-ink-secondary">
                  {contentKindText(preview.task.contentKind)} · {preview.task.platformName} ·{" "}
                  {preview.task.accountAlias}
                </p>
              </div>
              <button
                className="cursor-pointer text-ink-muted hover:text-ink"
                onClick={clearPreview}
              >
                <X size={18} />
              </button>
            </div>

            <div className="mt-5 grid gap-3 sm:grid-cols-3">
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">执行方式</span>
                <strong className="mt-1 block text-sm text-ink">
                  {preview.executionMode === "auto" ? "自动发布" : "人工发布"}
                </strong>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">运行时能力</span>
                <strong className="mt-1 block text-sm text-ink">{preview.capability}</strong>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[11px] text-ink-muted">内容版本</span>
                <strong className="mt-1 block truncate font-mono text-xs text-ink">
                  {preview.task.contentVersion}
                </strong>
              </div>
            </div>

            {preview.capabilityReason && (
              <p className="mt-3 rounded-xl border border-lemon-border bg-lemon-bg px-3 py-2 text-xs text-lemon-text">
                {preview.capabilityReason}
              </p>
            )}

            <div className="mt-4 rounded-xl border border-line bg-canvas p-4">
              <h3 className="text-xs font-semibold text-ink">正文 / 短文案</h3>
              <pre className="mt-2 max-h-40 overflow-y-auto whitespace-pre-wrap font-sans text-xs leading-relaxed text-ink-secondary">
                {preview.task.body || "该形态没有 Markdown 正文；请核对下方媒体文件。"}
              </pre>
            </div>

            <div className="mt-3 rounded-xl border border-line bg-canvas p-4">
              <h3 className="text-xs font-semibold text-ink">媒体顺序与封面</h3>
              <ol className="mt-2 space-y-1.5">
                {preview.task.media
                  .reduce<typeof preview.task.media>((ordered, asset) => {
                    const index = ordered.findIndex((item) => item.order > asset.order);
                    if (index < 0) ordered.push(asset);
                    else ordered.splice(index, 0, asset);
                    return ordered;
                  }, [])
                  .map((asset) => (
                    <li
                      key={asset.id}
                      className="flex items-center gap-2 text-xs text-ink-secondary"
                    >
                      <span className="grid h-5 w-5 place-items-center rounded bg-surface text-[10px] text-ink-muted">
                        {asset.order}
                      </span>
                      <span className="min-w-0 flex-1 truncate">{asset.originalName}</span>
                      <span className="text-[10px] text-ink-muted">{asset.mimeType}</span>
                      {preview.task.cover?.id === asset.id && (
                        <span className="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] text-accent-deep">
                          封面
                        </span>
                      )}
                    </li>
                  ))}
              </ol>
              {!preview.task.media.length && (
                <p className="text-xs text-ink-muted">
                  此形态无媒体文件；正文和元数据将作为发布载荷。
                </p>
              )}
              {preview.task.cover &&
                !preview.task.media.some((asset) => asset.id === preview.task.cover?.id) && (
                  <p className="mt-2 text-xs text-ink-secondary">
                    独立封面：{preview.task.cover.originalName}
                  </p>
                )}
            </div>

            {preview.validation.issues.length > 0 && (
              <div className="mt-3 rounded-xl border border-coral-border bg-coral-bg p-3 text-xs text-coral-text">
                {preview.validation.issues.map((issue) => (
                  <p key={`${issue.field}-${issue.code}`}>· {issue.message}</p>
                ))}
              </div>
            )}

            <div className="mt-5 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
              <button
                className="cursor-pointer text-xs text-ink-muted hover:text-coral-text"
                onClick={async () => {
                  const updated = await cancelTask(preview.taskId, "用户在发布确认界面取消");
                  if (updated) showToast("发布任务已取消");
                }}
              >
                取消任务
              </button>
              <div className="flex items-center gap-2">
                <button
                  className="inline-flex h-9 cursor-pointer items-center rounded-xl border border-line px-3 text-xs text-ink-secondary hover:bg-subtle"
                  onClick={clearPreview}
                >
                  返回检查
                </button>
                <button
                  className="inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-4 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-50"
                  disabled={!preview.validation.ok || !preview.confirmationToken || confirming}
                  onClick={() => void handleConfirm()}
                >
                  <CheckCircle2 size={13} />
                  {confirming
                    ? "正在执行..."
                    : preview.executionMode === "auto"
                      ? "确认并自动发布"
                      : "确认并进入人工发布"}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {manualTask && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-ink/35 p-5 backdrop-blur-sm">
          <form
            className="w-full max-w-md rounded-2xl border border-line bg-surface p-6 shadow-2xl"
            onSubmit={handleManualComplete}
          >
            <h2 className="text-base font-semibold text-ink">记录人工发布结果</h2>
            <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
              已完成平台操作后填写外部作品 ID 和链接。可留空，系统会生成可追溯的人工记录 ID。
            </p>
            <label className="mt-4 block text-xs text-ink-secondary">
              外部作品 ID
              <input
                value={externalId}
                onChange={(event) => setExternalId(event.target.value)}
                className="mt-1 h-9 w-full rounded-xl border border-line bg-canvas px-3 text-xs text-ink outline-none focus:border-accent"
                placeholder="例如 bv1xx..."
              />
            </label>
            <label className="mt-3 block text-xs text-ink-secondary">
              外部作品 URL
              <input
                value={externalUrl}
                onChange={(event) => setExternalUrl(event.target.value)}
                className="mt-1 h-9 w-full rounded-xl border border-line bg-canvas px-3 text-xs text-ink outline-none focus:border-accent"
                placeholder="https://..."
              />
            </label>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                className="h-8 cursor-pointer rounded-xl border border-line px-3 text-xs text-ink-secondary"
                onClick={() => setManualTask(null)}
              >
                取消
              </button>
              <button className="h-8 cursor-pointer rounded-xl bg-accent px-3 text-xs font-medium text-accent-on">
                标记为已发布
              </button>
            </div>
          </form>
        </div>
      )}

      {resolveTask && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-ink/35 p-5 backdrop-blur-sm">
          <div className="w-full max-w-lg rounded-2xl border border-line bg-surface p-6 shadow-2xl">
            <div className="flex items-start gap-3">
              <AlertTriangle size={18} className="mt-0.5 shrink-0 text-lemon-text" />
              <div>
                <h2 className="text-base font-semibold text-ink">核对平台实际结果</h2>
                <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
                  上次请求超时或返回不一致。请先到平台作品列表确认，再记录结果；系统不会自动重复提交。
                </p>
              </div>
            </div>
            <div className="mt-4 grid gap-2 sm:grid-cols-2">
              <input
                value={externalId}
                onChange={(event) => setExternalId(event.target.value)}
                className="h-9 rounded-xl border border-line bg-canvas px-3 text-xs text-ink outline-none focus:border-accent"
                placeholder="已找到时填写外部作品 ID"
              />
              <input
                value={externalUrl}
                onChange={(event) => setExternalUrl(event.target.value)}
                className="h-9 rounded-xl border border-line bg-canvas px-3 text-xs text-ink outline-none focus:border-accent"
                placeholder="外部作品 URL（可选）"
              />
            </div>
            <div className="mt-5 flex flex-wrap justify-end gap-2">
              <button
                className="h-8 cursor-pointer rounded-xl border border-line px-3 text-xs text-ink-secondary"
                onClick={() => setResolveTask(null)}
              >
                稍后处理
              </button>
              <button
                className="h-8 cursor-pointer rounded-xl border border-coral-border bg-coral-bg px-3 text-xs font-medium text-coral-text"
                onClick={() =>
                  void handleResolve({ outcome: "failed", note: "平台后台未找到作品" })
                }
              >
                确认未发布
              </button>
              <button
                className="h-8 cursor-pointer rounded-xl bg-accent px-3 text-xs font-medium text-accent-on"
                onClick={() =>
                  void handleResolve({
                    outcome: "success",
                    externalId: externalId.trim() || null,
                    externalUrl: externalUrl.trim() || null,
                    note: "平台后台确认作品已发布",
                  })
                }
              >
                确认已发布
              </button>
            </div>
          </div>
        </div>
      )}

      {scheduleTask && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-ink/35 p-5 backdrop-blur-sm">
          <form
            className="w-full max-w-md rounded-2xl border border-line bg-surface p-6 shadow-2xl"
            onSubmit={async (event) => {
              event.preventDefault();
              if (!scheduleValue) return;
              const scheduledAt = new Date(scheduleValue).toISOString();
              const updated = await updateTaskSchedule(scheduleTask.id, scheduledAt);
              if (updated) {
                showToast("计划时间已保存；确认最终内容包后才会进入定时队列");
                setScheduleTask(null);
                await handlePreview(updated);
              }
            }}
          >
            <h2 className="text-base font-semibold text-ink">设置计划发布时间</h2>
            <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
              只有账号与形态支持自动发布时才会在确认后进入定时队列；不支持时仍走人工发布包。
            </p>
            <label className="mt-4 block text-xs text-ink-secondary">
              本地时间
              <input
                type="datetime-local"
                value={scheduleValue}
                onChange={(event) => setScheduleValue(event.target.value)}
                className="mt-1 h-9 w-full rounded-xl border border-line bg-canvas px-3 text-xs text-ink outline-none focus:border-accent"
              />
            </label>
            <div className="mt-5 flex justify-end gap-2">
              <button
                type="button"
                className="h-8 cursor-pointer rounded-xl border border-line px-3 text-xs text-ink-secondary"
                onClick={() => setScheduleTask(null)}
              >
                取消
              </button>
              <button
                disabled={!scheduleValue}
                className="h-8 cursor-pointer rounded-xl bg-accent px-3 text-xs font-medium text-accent-on disabled:opacity-50"
              >
                保存并预览
              </button>
            </div>
          </form>
        </div>
      )}
    </main>
  );
}
