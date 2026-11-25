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
      className="fixed inset-0 z-50 grid place-items-center bg-ink/35 p-5 backdrop-blur-[2px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="tool-auth-title"
        className="max-h-[86vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-line bg-surface p-5 shadow-2xl"
      >
        <div className="flex items-start gap-3">
          <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent">
            <ShieldCheck size={17} />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 id="tool-auth-title" className="text-[15px] font-semibold text-ink">
                {preview.title}
              </h2>
              <span
                className={`rounded-md border px-1.5 py-0.5 text-[10px] font-medium ${perm.className}`}
              >
                {perm.label}
              </span>
            </div>
            <p className="mt-1 text-[11px] text-ink-secondary">
              工具 <code className="font-mono font-medium text-ink">{preview.toolName}</code>
            </p>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-muted transition-colors hover:bg-hover hover:text-ink"
            aria-label="关闭预览"
          >
            <X size={15} />
          </button>
        </div>

        <div className="mt-4 grid gap-2 sm:grid-cols-2">
          <div className="rounded-xl border border-line bg-canvas p-3">
            <span className="text-[10px] text-ink-muted">执行目标</span>
            <p className="mt-1 text-xs font-medium text-ink">{preview.targetSummary}</p>
          </div>
          <div className="rounded-xl border border-line bg-canvas p-3">
            <span className="text-[10px] text-ink-muted">内容形态</span>
            <p className="mt-1 text-xs font-medium text-ink">{preview.contentKind ?? "不适用"}</p>
          </div>
        </div>

        <div className="mt-2 rounded-xl border border-line bg-canvas p-3">
          <span className="text-[10px] text-ink-muted">脱敏参数摘要</span>
          <p className="mt-1 break-words font-mono text-[11px] text-ink-secondary">
            {preview.parameterSummary || "无参数"}
          </p>
        </div>

        <div className="mt-2 rounded-xl border border-line bg-canvas p-3">
          <span className="text-[10px] text-ink-muted">预期副作用</span>
          <ul className="mt-1.5 space-y-1 text-[11.5px] text-ink-secondary">
            {preview.effects.map((effect) => (
              <li key={effect} className="flex gap-1.5">
                <span className="text-accent">•</span>
                <span>{effect}</span>
              </li>
            ))}
          </ul>
        </div>

        <details className="mt-2 rounded-xl border border-line bg-canvas p-3" open>
          <summary className="cursor-pointer text-[10.5px] font-medium text-ink">
            最终目标与内容预览
          </summary>
          <pre className="mt-2 max-h-60 overflow-auto whitespace-pre-wrap rounded-lg bg-surface p-2.5 font-mono text-[10px] leading-relaxed text-ink-secondary">
            {pretty(preview.details)}
          </pre>
        </details>

        {preview.fieldErrors.length > 0 && (
          <div className="mt-2.5 rounded-xl border border-coral-border bg-coral-bg p-3">
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

        <div className="mt-4 flex items-center gap-2 border-t border-line pt-3.5">
          <p className="flex flex-1 items-center gap-1.5 text-[10.5px] text-ink-muted">
            <Info size={11} className="shrink-0" />
            {preview.requiresConfirmation
              ? "确认令牌 10 分钟内有效且仅可使用一次。"
              : "本次操作不需要二次确认，但仍会记录审计。"}
          </p>
          <button
            type="button"
            onClick={onClose}
            className="cursor-pointer rounded-xl border border-line bg-surface px-3.5 py-2 text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
          >
            取消
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={preview.fieldErrors.length > 0 || Boolean(busyAction)}
            className="inline-flex cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3.5 py-2 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:opacity-50"
          >
            <Check size={12} />
            确认并执行
          </button>
        </div>
      </section>
    </div>
  );
}
