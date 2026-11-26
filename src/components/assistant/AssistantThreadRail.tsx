import { useMemo, useState } from "react";
import { PanelLeftClose, Plus, RefreshCw, Search, Trash2 } from "../Icons";
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
      className="flex h-full w-[224px] shrink-0 flex-col border-r border-line bg-surface-soft"
      aria-label="对话任务列表"
    >
      {/* Slim Header Matching Content Studio */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-line px-3">
        <div className="flex min-w-0 items-center gap-1.5">
          <span className="text-[12.5px] font-semibold tracking-tight text-ink">会话</span>
          <span className="font-mono text-[10.5px] text-ink-muted">{threads.length}</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={onCreateThread}
            disabled={Boolean(busyAction)}
            className="grid h-6.5 w-6.5 cursor-pointer place-items-center rounded-md text-ink-secondary transition-colors hover:bg-hover hover:text-ink disabled:opacity-40"
            title="新建任务会话"
            aria-label="新建任务会话"
          >
            <Plus size={13} />
          </button>
          <button
            type="button"
            onClick={onCollapse}
            className="grid h-6.5 w-6.5 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink"
            title="收起会话列表"
            aria-label="收起会话列表"
          >
            <PanelLeftClose size={13} />
          </button>
        </div>
      </div>

      {/* Optional Search Filter */}
      {threads.length > 3 && (
        <div className="border-b border-line px-2.5 py-2">
          <div className="flex h-7 items-center gap-1.5 rounded-md border border-line bg-surface px-2 focus-within:border-accent">
            <Search size={11} className="shrink-0 text-ink-muted" />
            <input
              type="text"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="筛选会话..."
              className="min-w-0 flex-1 bg-transparent text-[11.5px] text-ink placeholder:text-ink-muted outline-none"
            />
          </div>
        </div>
      )}

      {/* Hairline-Divided Thread List */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {filteredThreads.length > 0 ? (
          <div className="divide-y divide-line/60">
            {filteredThreads.map((thread) => {
              const isActive = activeThreadId === thread.id;
              return (
                <div
                  key={thread.id}
                  className={`group relative transition-colors ${
                    isActive ? "bg-surface" : "hover:bg-hover/50"
                  }`}
                >
                  {isActive && (
                    <span
                      aria-hidden="true"
                      className="pointer-events-none absolute top-0 bottom-0 left-0 w-[2.5px] bg-accent"
                    />
                  )}
                  <button
                    type="button"
                    onClick={() => onSelectThread(thread.id)}
                    className="flex w-full cursor-pointer flex-col gap-1 py-2.5 pr-8 pl-3 text-left outline-none focus-visible:ring-1 focus-visible:ring-accent"
                  >
                    <span
                      className={`line-clamp-1 text-[12px] leading-snug ${
                        isActive ? "font-semibold text-ink" : "font-medium text-ink-secondary"
                      }`}
                    >
                      {thread.title}
                    </span>
                    <div className="flex items-center justify-between gap-2 text-[10px] text-ink-muted">
                      <span className="truncate">{thread.providerName}</span>
                      <span className="shrink-0 font-mono">{relativeTime(thread.updatedAt)}</span>
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
                    className="absolute top-2 right-1.5 grid h-6 w-6 cursor-pointer place-items-center rounded-sm text-ink-muted opacity-0 transition-opacity hover:bg-coral-bg hover:text-coral-text focus-visible:opacity-100 group-hover:opacity-100 group-focus-within:opacity-100 disabled:pointer-events-none disabled:opacity-30"
                  >
                    <Trash2 size={11} />
                  </button>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="px-3 py-8 text-center text-[11px] text-ink-muted">
            {query.trim() ? "没有匹配的会话" : "暂无任务会话"}
          </div>
        )}
      </div>

      {/* Compact Single-Line MCP Host Status Strip */}
      <div className="flex h-9 shrink-0 items-center justify-between gap-2 border-t border-line bg-surface-soft px-3">
        <button
          type="button"
          onClick={onOpenHostInspector}
          className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 text-left transition-colors hover:text-ink"
          title="查看 MCP Host 状态"
        >
          <span
            className={`h-1.5 w-1.5 shrink-0 rounded-full ${
              isHealthy ? "bg-mint-text" : "bg-coral-text"
            }`}
          />
          <span className="truncate text-[11px] font-medium text-ink-secondary">
            MCP {isHealthy ? `就绪 · ${toolCount} 工具` : "未就绪"}
          </span>
        </button>
        <button
          type="button"
          onClick={onHealthCheck}
          disabled={Boolean(busyAction)}
          title="执行健康检查"
          aria-label="执行健康检查"
          className="grid h-6 w-6 shrink-0 cursor-pointer place-items-center rounded-sm text-ink-muted transition-colors hover:bg-hover hover:text-ink disabled:opacity-40"
        >
          <RefreshCw size={11} className={busyAction === "health" ? "animate-spin" : ""} />
        </button>
      </div>
    </aside>
  );
}
