import { useEffect, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  AlertTriangle,
  ArrowRight,
  BarChart3,
  CheckCircle2,
  FileText,
  Globe2,
  LoaderCircle,
  Play,
  ShieldCheck,
  Terminal,
  X,
} from "../Icons";
import type { AssistantMessage, ToolPermission } from "../../types/mcp";

export const STARTER_TASKS = [
  {
    title: "检查账号登录状态",
    description: "读取已绑定平台的脱敏会话状态与可用性",
    prompt: "查询所有已绑定账号的登录状态",
    tag: "只读",
    icon: Globe2,
  },
  {
    title: "检索本地内容草稿",
    description: "列出工作区中的四类内容包与最近编辑进度",
    prompt: "检索本地内容草稿",
    tag: "只读",
    icon: FileText,
  },
  {
    title: "查看 7 天指标趋势",
    description: "汇总跨平台曝光、互动与粉丝变化趋势",
    prompt: "查询最近 7 天指标趋势",
    tag: "只读",
    icon: BarChart3,
  },
  {
    title: "运行 MCP 回显自检",
    description: "验证本地独立 stdio MCP Server 连通性与回显",
    prompt: "运行 MCP 回显自检",
    tag: "自检",
    icon: ShieldCheck,
  },
] as const;

export const PERMISSION_META: Record<ToolPermission, { label: string; className: string }> = {
  read: { label: "只读", className: "bg-sky-bg text-sky-text border-sky-border" },
  write: { label: "写入", className: "bg-lemon-bg text-lemon-text border-lemon-border" },
  externalAction: {
    label: "外部操作",
    className: "bg-coral-bg text-coral-text border-coral-border",
  },
};

export const STATUS_META: Record<string, { label: string; className: string }> = {
  proposed: { label: "待确认", className: "bg-lemon-bg text-lemon-text" },
  success: { label: "已完成", className: "bg-mint-bg text-mint-text" },
  partial: { label: "部分完成", className: "bg-lemon-bg text-lemon-text" },
  failed: { label: "失败", className: "bg-coral-bg text-coral-text" },
  cancelled: { label: "已取消", className: "bg-subtle text-ink-secondary" },
  timedOut: { label: "已超时", className: "bg-coral-bg text-coral-text" },
  no_tool: { label: "无工具调用", className: "bg-subtle text-ink-secondary" },
  no_provider: { label: "未配置模型", className: "bg-lemon-bg text-lemon-text" },
};

function formatTimestamp(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function MessageTurn({
  message,
  onPreview,
}: {
  message: AssistantMessage;
  onPreview: (message: AssistantMessage) => void;
}) {
  const isUser = message.role === "user";
  const tool = message.toolCall;
  const canExecute = tool?.status === "proposed" && tool.arguments && tool.permission !== undefined;

  if (isUser) {
    return (
      <article className="group flex flex-col items-end gap-1">
        <div className="max-w-[85%] rounded-lg border border-line bg-subtle px-3.5 py-2.5 text-[13.5px] leading-relaxed text-ink">
          <p className="whitespace-pre-wrap">{message.content}</p>
        </div>
        <span className="px-1 font-mono text-[10px] text-ink-faint opacity-0 transition-opacity group-hover:opacity-100">
          {formatTimestamp(message.createdAt)}
        </span>
      </article>
    );
  }

  const toolStatus = tool?.status ? STATUS_META[tool.status] : null;
  const perm = tool ? PERMISSION_META[tool.permission] : null;

  return (
    <article className="flex flex-col gap-2">
      {/* Minimal Editorial Assistant Header */}
      <div className="flex flex-wrap items-center gap-2 text-[11px]">
        <span className="h-2 w-2 rounded-xs bg-accent" aria-hidden="true" />
        <span className="font-semibold text-ink">CreatorRelay</span>
        <span className="font-mono text-[10.5px] text-ink-muted">
          {message.providerName ?? "本地编排器"} · {message.model ?? "deterministic-v1"}
        </span>
        <span className="ml-auto font-mono text-[10px] text-ink-faint">
          {formatTimestamp(message.createdAt)}
        </span>
      </div>

      {/* Assistant Markdown Body */}
      <div className="pl-4">
        <div className="prose-assistant space-y-2 text-[13.5px] leading-[1.75] text-ink">
          <ReactMarkdown
            remarkPlugins={[remarkGfm]}
            components={{
              p: ({ children }) => <p className="my-0 whitespace-pre-wrap">{children}</p>,
              ul: ({ children }) => <ul className="my-1.5 list-disc space-y-1 pl-5">{children}</ul>,
              ol: ({ children }) => (
                <ol className="my-1.5 list-decimal space-y-1 pl-5">{children}</ol>
              ),
              code: ({ className, children }) => {
                const isBlock = Boolean(className);
                if (isBlock) {
                  return (
                    <code className="block overflow-x-auto rounded-md border border-line bg-subtle p-3 font-mono text-[11.5px] leading-relaxed text-ink">
                      {children}
                    </code>
                  );
                }
                return (
                  <code className="rounded-xs border border-line bg-subtle px-1 py-px font-mono text-[12px] text-ink">
                    {children}
                  </code>
                );
              },
              pre: ({ children }) => <pre className="my-2">{children}</pre>,
              a: ({ href, children }) => (
                <a
                  href={href}
                  target="_blank"
                  rel="noreferrer"
                  className="text-accent underline decoration-accent/40 underline-offset-2 hover:text-accent-hover"
                >
                  {children}
                </a>
              ),
            }}
          >
            {message.content}
          </ReactMarkdown>
        </div>

        {/* Crisp Inline Tool Execution Block */}
        {tool && (
          <div className="mt-3 overflow-hidden rounded-md border border-line bg-surface-soft">
            <div className="flex flex-wrap items-center gap-2 border-b border-line bg-subtle/70 px-3 py-2">
              <Terminal size={12} className="text-ink-muted" />
              <code className="font-mono text-[11.5px] font-semibold text-ink">
                {tool.toolName}
              </code>
              {perm && (
                <span
                  className={`rounded-xs border px-1.5 py-px text-[9.5px] font-medium ${perm.className}`}
                >
                  {perm.label}
                </span>
              )}
              {tool.status && (
                <span
                  className={`ml-auto inline-flex items-center gap-1 rounded-xs px-1.5 py-px text-[10px] font-medium ${
                    toolStatus?.className ?? "bg-subtle text-ink-secondary"
                  }`}
                >
                  {tool.status === "success" && <CheckCircle2 size={10} />}
                  {tool.status === "failed" && <AlertTriangle size={10} />}
                  {toolStatus?.label ?? tool.status}
                </span>
              )}
            </div>

            <div className="space-y-1.5 px-3 py-2.5 text-[11.5px]">
              <div className="flex items-baseline gap-3">
                <span className="w-14 shrink-0 text-[10.5px] text-ink-muted">执行目标</span>
                <span className="min-w-0 flex-1 text-ink">{tool.targetSummary}</span>
              </div>
              <div className="flex items-baseline gap-3">
                <span className="w-14 shrink-0 text-[10.5px] text-ink-muted">参数摘要</span>
                <code className="min-w-0 flex-1 break-words rounded-xs bg-surface px-1.5 py-0.5 font-mono text-[11px] text-ink-secondary">
                  {tool.parameterSummary || "无参数"}
                </code>
              </div>

              {tool.errorMessage && (
                <div className="mt-2 flex items-start gap-2 rounded-sm border border-coral-border bg-coral-bg px-2.5 py-2 text-[11px] text-coral-text">
                  <AlertTriangle size={12} className="mt-0.5 shrink-0" />
                  <span>
                    {tool.errorCode ? `${tool.errorCode}：` : ""}
                    {tool.errorMessage}
                  </span>
                </div>
              )}
            </div>

            {canExecute && (
              <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line bg-surface px-3 py-2">
                <span className="text-[10.5px] text-ink-muted">
                  点击后展示预期副作用与脱敏载荷，确认后再执行
                </span>
                <button
                  type="button"
                  onClick={() => onPreview(message)}
                  className="inline-flex cursor-pointer items-center gap-1.5 rounded-md bg-accent px-2.5 py-1 text-[11px] font-medium text-accent-on transition-colors hover:bg-accent-hover"
                >
                  <Play size={10} />
                  预览并授权执行
                </button>
              </div>
            )}
          </div>
        )}

        {message.errorMessage && !tool?.errorMessage && (
          <div className="mt-2.5 flex items-start gap-2 rounded-md border border-coral-border bg-coral-bg px-3 py-2 text-[11.5px] text-coral-text">
            <AlertTriangle size={12} className="mt-0.5 shrink-0" />
            <span>{message.errorMessage}</span>
          </div>
        )}
      </div>
    </article>
  );
}

interface AssistantMessageStreamProps {
  messages: AssistantMessage[];
  loading: boolean;
  busyAction: string;
  onPreview: (message: AssistantMessage) => void;
  onQuickSend: (prompt: string) => void;
  onCancelCall: () => void;
}

export function AssistantMessageStream({
  messages,
  loading,
  busyAction,
  onPreview,
  onQuickSend,
  onCancelCall,
}: AssistantMessageStreamProps) {
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [messages.length, busyAction]);

  const busyLabel =
    busyAction === "prompt"
      ? "正在规划本地工具调用…"
      : busyAction === "generate"
        ? "正在等待模型返回回答…"
        : busyAction === "execute"
          ? "正在受控执行 MCP 工具…"
          : busyAction === "preview"
            ? "正在生成执行前授权预览…"
            : "";

  return (
    <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-6 pb-6">
      <div className="mx-auto flex w-full max-w-[740px] flex-col">
        {loading && messages.length === 0 && (
          <div className="flex flex-col items-center justify-center gap-2 py-24 text-xs text-ink-muted">
            <LoaderCircle size={16} className="animate-spin text-accent" />
            <span>正在连接本地 MCP Host…</span>
          </div>
        )}

        {!loading && messages.length === 0 && (
          <div className="my-auto py-10">
            <span className="font-mono text-[10px] font-semibold tracking-[0.08em] text-accent uppercase">
              LOCAL MCP HOST · CONTROLLED EXECUTION
            </span>
            <h2 className="mt-1.5 text-[20px] font-semibold tracking-[-0.02em] text-ink">
              今天想推进什么创作任务？
            </h2>
            <p className="mt-1.5 max-w-xl text-[12.5px] leading-relaxed text-ink-secondary">
              通过本地受控工具查询账号、内容与数据指标，或切换至 BYOK 模型并按显式披露范围生成建议。
            </p>

            {/* Hairline-Divided 2x2 Starter Grid */}
            <div className="mt-6 grid grid-cols-1 overflow-hidden rounded-lg border border-line bg-surface sm:grid-cols-2">
              {STARTER_TASKS.map((task, idx) => {
                const Icon = task.icon;
                const borderClass = [
                  idx >= 2 ? "sm:border-t sm:border-line" : "",
                  idx % 2 === 1 ? "sm:border-l sm:border-line" : "",
                  idx > 0 ? "border-t border-line sm:border-t-0" : "",
                ]
                  .filter(Boolean)
                  .join(" ");
                return (
                  <button
                    key={task.title}
                    type="button"
                    onClick={() => onQuickSend(task.prompt)}
                    disabled={Boolean(busyAction)}
                    className={`group flex cursor-pointer items-start gap-3 px-4 py-3.5 text-left transition-colors hover:bg-subtle/70 disabled:opacity-50 ${borderClass}`}
                  >
                    <Icon
                      size={14}
                      className="mt-0.5 shrink-0 text-ink-muted transition-colors group-hover:text-accent"
                    />
                    <span className="min-w-0 flex-1">
                      <span className="flex items-center gap-1.5">
                        <strong className="truncate text-[12.5px] font-medium text-ink">
                          {task.title}
                        </strong>
                        <span className="rounded-xs bg-subtle px-1 py-px font-mono text-[9.5px] text-ink-muted">
                          {task.tag}
                        </span>
                      </span>
                      <span className="mt-0.5 block text-[11px] leading-snug text-ink-muted">
                        {task.description}
                      </span>
                    </span>
                    <ArrowRight
                      size={11}
                      className="mt-1 shrink-0 text-ink-faint transition-transform group-hover:translate-x-0.5 group-hover:text-ink-secondary"
                    />
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {messages.length > 0 && (
          <div className="space-y-6">
            {messages.map((message) => (
              <MessageTurn key={message.id} message={message} onPreview={onPreview} />
            ))}
          </div>
        )}

        {busyLabel && (
          <div className="mt-5 flex items-center gap-2 pl-4 text-xs text-ink-secondary">
            <LoaderCircle size={13} className="animate-spin text-accent" />
            <span>{busyLabel}</span>
            {busyAction === "execute" && (
              <button
                type="button"
                onClick={onCancelCall}
                className="ml-2 inline-flex cursor-pointer items-center gap-1 rounded-sm border border-coral-border bg-coral-bg px-2 py-0.5 text-[10.5px] font-medium text-coral-text"
              >
                <X size={10} />
                取消调用
              </button>
            )}
          </div>
        )}

        <div ref={bottomRef} className="h-2" />
      </div>
    </div>
  );
}
