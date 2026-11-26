import { useEffect } from "react";
import { AlertTriangle, Check, Info, ShieldCheck, X } from "../Icons";
import type { McpAuthorizationPreview } from "../../types/mcp";
import { PERMISSION_META } from "./AssistantMessageStream";

function pretty(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

interface ToolAuthorizationModalProps {
  preview: McpAuthorizationPreview;
  busyAction: string;
  onClose: () => void;
  onConfirm: () => void;
}

export function ToolAuthorizationModal({
  preview,
  busyAction,
  onClose,
  onConfirm,
}: ToolAuthorizationModalProps) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  const perm = PERMISSION_META[preview.permission];

  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/40 p-4 backdrop-blur-[1px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="tool-auth-title"
        className="flex max-h-[86vh] w-full max-w-2xl flex-col overflow-hidden rounded-lg border border-line-strong bg-surface shadow-xl"
      >
        <div className="flex items-center justify-between gap-3 border-b border-line px-5 py-3.5">
          <div className="flex min-w-0 items-center gap-2">
            <ShieldCheck size={15} className="shrink-0 text-accent" />
            <h2 id="tool-auth-title" className="truncate text-[14px] font-semibold text-ink">
              {preview.title}
            </h2>
            <span
              className={`shrink-0 rounded-xs border px-1.5 py-px text-[10px] font-medium ${perm.className}`}
            >
              {perm.label}
            </span>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="grid h-6 w-6 shrink-0 cursor-pointer place-items-center rounded-sm text-ink-muted transition-colors hover:bg-hover hover:text-ink"
            aria-label="关闭预览"
          >
            <X size={14} />
          </button>
        </div>

        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-5">
          <p className="text-[11.5px] text-ink-secondary">
            工具名称：<code className="font-mono font-medium text-ink">{preview.toolName}</code>
          </p>

          <div className="grid divide-y divide-line overflow-hidden rounded-md border border-line bg-surface-soft sm:grid-cols-2 sm:divide-x sm:divide-y-0">
            <div className="px-3.5 py-2.5">
              <span className="text-[10.5px] text-ink-muted">执行目标</span>
              <p className="mt-0.5 text-xs font-medium text-ink">{preview.targetSummary}</p>
            </div>
            <div className="px-3.5 py-2.5">
              <span className="text-[10.5px] text-ink-muted">内容形态</span>
              <p className="mt-0.5 text-xs font-medium text-ink">{preview.contentKind ?? "不适用"}</p>
            </div>
          </div>

          <div className="rounded-md border border-line bg-surface-soft px-3.5 py-2.5">
            <span className="text-[10.5px] text-ink-muted">脱敏参数摘要</span>
            <p className="mt-1 break-words font-mono text-[11px] text-ink-secondary">
              {preview.parameterSummary || "无参数"}
            </p>
          </div>

          <div className="rounded-md border border-line bg-surface-soft px-3.5 py-2.5">
            <span className="text-[10.5px] text-ink-muted">预期副作用</span>
            <ul className="mt-1.5 space-y-1 text-[11.5px] text-ink-secondary">
              {preview.effects.map((effect) => (
                <li key={effect} className="flex gap-1.5">
                  <span className="text-accent">•</span>
                  <span>{effect}</span>
                </li>
              ))}
            </ul>
          </div>

          <details className="rounded-md border border-line bg-surface-soft px-3.5 py-2.5" open>
            <summary className="cursor-pointer text-[11px] font-medium text-ink">
              最终目标与内容预览
            </summary>
            <pre className="mt-2 max-h-56 overflow-auto whitespace-pre-wrap rounded-sm border border-line bg-surface p-2.5 font-mono text-[10.5px] leading-relaxed text-ink-secondary">
              {pretty(preview.details)}
            </pre>
          </details>

          {preview.fieldErrors.length > 0 && (
            <div className="rounded-md border border-coral-border bg-coral-bg p-3">
              <p className="flex items-center gap-1.5 text-[11px] font-medium text-coral-text">
                <AlertTriangle size={12} />
                载荷校验未通过，执行按钮已禁用
              </p>
              <ul className="mt-1 space-y-1 text-[10.5px] text-coral-text">
                {preview.fieldErrors.map((error) => (
                  <li key={`${error.path}-${error.message}`}>
                    <code>{error.path || "root"}</code>：{error.message}
                  </li>
                ))}
              </ul>
            </div>
          )}
        </div>

        <div className="flex items-center gap-2 border-t border-line bg-surface-soft px-5 py-3">
          <p className="flex flex-1 items-center gap-1.5 text-[10.5px] text-ink-muted">
            <Info size={11} className="shrink-0" />
            {preview.requiresConfirmation
              ? "确认令牌 10 分钟内有效且仅可使用一次。"
              : "本次操作不需要二次确认，但仍会记录审计。"}
          </p>
          <button
            type="button"
            onClick={onClose}
            className="h-8 cursor-pointer rounded-md border border-line-strong bg-surface px-3 text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
          >
            取消
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={preview.fieldErrors.length > 0 || Boolean(busyAction)}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md bg-accent px-3.5 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:opacity-50"
          >
            <Check size={12} />
            确认并执行
          </button>
        </div>
      </section>
    </div>
  );
}
