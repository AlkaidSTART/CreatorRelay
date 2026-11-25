import { useMemo, useState } from "react";
import { Bot, PanelLeftClose, Plus, RefreshCw, Search, Server, Trash2 } from "../Icons";
import type { AssistantThread, McpHostStatus } from "../../types/mcp";

function relativeTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  const diffMs = Date.now() - date.getTime();
  const diffMin = Math.floor(diffMs / 60_000);
  if (diffMin < 1) return "刚刚";
  if (diffMin < 60) return `${diffMin} 分钟前`;
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return `${diffHr} 小时前`;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
  }).format(date);
}

interface AssistantThreadRailProps {
  threads: AssistantThread[];
  activeThreadId: string | null;
  status: McpHostStatus | null;
  toolCount: number;
  busyAction: string;
  onCreateThread: () => void;
  onSelectThread: (threadId: string) => void;
  onDeleteThread: (threadId: string) => void;
  onCollapse: () => void;
  onOpenHostInspector: () => void;
  onHealthCheck: () => void;
}

export function AssistantThreadRail({
  threads,
  activeThreadId,
  status,
  toolCount,
  busyAction,
  onCreateThread,
  onSelectThread,
  onDeleteThread,
  onCollapse,
  onOpenHostInspector,
  onHealthCheck,
}: AssistantThreadRailProps) {
  const [query, setQuery] = useState("");

  const filteredThreads = useMemo(() => {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) return threads;
    return threads.filter(
      (thread) =>
        thread.title.toLowerCase().includes(trimmed) ||
        thread.providerName.toLowerCase().includes(trimmed) ||
        thread.model.toLowerCase().includes(trimmed),
    );
  }, [query, threads]);

  const isHealthy = status?.state === "healthy";

  return (
    <aside
      className="flex h-full w-[248px] shrink-0 flex-col border-r border-line bg-sidebar/60"
      aria-label="对话任务列表"
    >
      {/* Rail Header */}
      <div className="flex items-center justify-between gap-2 px-3 pt-3 pb-2">
        <span className="text-[11px] font-semibold tracking-wider text-ink-muted uppercase">
          任务会话
        </span>
        <button
          type="button"
          onClick={onCollapse}
          className="grid h-6 w-6 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink"
          title="收起会话列表"
          aria-label="收起会话列表"
        >
          <PanelLeftClose size={12} />
        </button>
      </div>

      {/* New Task Button */}
      <div className="px-3 pb-2.5">
        <button
          type="button"
          onClick={onCreateThread}
          disabled={Boolean(busyAction)}
          className="flex h-8.5 w-full cursor-pointer items-center justify-center gap-1.5 rounded-xl border border-line-strong bg-surface text-xs font-medium text-ink shadow-[0_1px_2px_rgba(51,43,41,0.04)] transition-colors hover:border-accent-border hover:bg-accent-soft/50 hover:text-accent-deep disabled:opacity-50"
        >
          <Plus size={12} />
          新建任务
        </button>

        {threads.length > 3 && (
          <div className="relative mt-2">
            <Search
              size={11}
              className="pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2 text-ink-muted"
            />
            <input
              type="text"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="筛选会话…"
              className="h-7.5 w-full rounded-lg border border-line bg-surface/80 pr-2.5 pl-7 text-[11.5px] text-ink placeholder:text-ink-muted outline-none focus:border-accent"
            />
          </div>
        )}
      </div>

      {/* Thread List */}
      <div className="min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2 pb-3">
        {filteredThreads.map((thread) => {
          const isActive = activeThreadId === thread.id;
          return (
            <div
              key={thread.id}
              className={`group relative rounded-xl transition-colors ${
                isActive
                  ? "bg-surface text-ink shadow-[0_1px_4px_rgba(51,43,41,0.06)] ring-1 ring-line"
                  : "text-ink-secondary hover:bg-hover/70 hover:text-ink"
              }`}
            >
              <button
                type="button"
                onClick={() => onSelectThread(thread.id)}
                className="flex w-full cursor-pointer flex-col gap-1 rounded-xl py-2 pr-8 pl-2.5 text-left outline-none focus-visible:ring-2 focus-visible:ring-accent/40"
              >
                <div className="flex items-start gap-2">
                  <Bot
                    size={12}
                    className={`mt-0.5 shrink-0 ${
                      isActive ? "text-accent" : "text-ink-muted group-hover:text-ink-secondary"
                    }`}
                  />
                  <span className="line-clamp-2 min-w-0 flex-1 text-[12px] leading-snug font-medium">
                    {thread.title}
                  </span>
                </div>
                <div className="flex items-center justify-between gap-2 pl-5 text-[10px] text-ink-muted">
                  <span className="truncate">{thread.providerName}</span>
                  <span className="shrink-0">{relativeTime(thread.updatedAt)}</span>
                </div>
              </button>
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  onDeleteThread(thread.id);
                }}
                disabled={Boolean(busyAction)}
                title="删除任务"
                aria-label={`删除任务：${thread.title}`}
                className="absolute top-1.5 right-1.5 grid h-6 w-6 cursor-pointer place-items-center rounded-lg text-ink-muted opacity-0 transition-all hover:bg-coral-bg hover:text-coral-text focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-coral-border group-hover:opacity-100 group-focus-within:opacity-100 disabled:pointer-events-none disabled:opacity-30"
              >
                <Trash2 size={11} />
              </button>
            </div>
          );
        })}

        {filteredThreads.length === 0 && (
          <div className="px-3 py-6 text-center text-[11px] text-ink-muted">
            {query.trim() ? "没有匹配的会话" : "暂无任务会话"}
          </div>
        )}
      </div>

      {/* Ambient MCP Host Status Footer */}
      <div className="border-t border-line p-2.5">
        <div className="flex items-center gap-2 rounded-xl border border-line bg-surface px-2.5 py-2">
          <button
            type="button"
            onClick={onOpenHostInspector}
            className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left"
            title="打开 MCP Host 面板"
          >
            <span className="relative flex h-2 w-2 shrink-0">
              {isHealthy && (
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-mint-text opacity-40" />
              )}
              <span
                className={`relative inline-flex h-2 w-2 rounded-full ${
                  isHealthy ? "bg-mint-text" : "bg-coral-text"
                }`}
              />
            </span>
            <span className="min-w-0 flex-1">
              <span className="flex items-center gap-1 text-[11px] font-medium text-ink">
                <Server size={10} className="text-ink-muted" />
                <span className="truncate">MCP Host</span>
              </span>
              <span className="block truncate text-[9.5px] text-ink-muted">
                {isHealthy ? `就绪 · ${toolCount} 个工具` : "未就绪 · 点击检查"}
              </span>
            </span>
          </button>
          <button
            type="button"
            onClick={onHealthCheck}
            disabled={Boolean(busyAction)}
            title="执行健康检查"
            aria-label="执行健康检查"
            className="grid h-6 w-6 shrink-0 cursor-pointer place-items-center rounded-lg text-ink-muted transition-colors hover:bg-hover hover:text-ink disabled:opacity-40"
          >
            <RefreshCw size={11} className={busyAction === "health" ? "animate-spin" : ""} />
          </button>
        </div>
      </div>
    </aside>
  );
}
