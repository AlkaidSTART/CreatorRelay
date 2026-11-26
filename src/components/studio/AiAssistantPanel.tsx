import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { Check, Copy, LoaderCircle, RotateCcw, Shield, Sparkles, X } from "../Icons";
import {
  useAiStore,
  aiOperationTarget,
  buildAiMetadata,
  operationsForKind,
} from "../../stores/aiStore";
import { useStudioStore } from "../../stores/studioStore";
import { useUiStore } from "../../stores/uiStore";
import type { AiGenerateRequest, AiGenerateResult, AiOperation } from "../../types/ai";

interface UndoState {
  target: "title" | "summary" | "body";
  previous: string;
}

export function AiAssistantPanel() {
  const navigate = useNavigate();
  const editor = useStudioStore((state) => state.editor);
  const updateEditor = useStudioStore((state) => state.updateEditor);
  const providers = useAiStore((state) => state.providers);
  const initialize = useAiStore((state) => state.initialize);
  const generate = useAiStore((state) => state.generate);
  const busyAction = useAiStore((state) => state.busyAction);
  const showToast = useUiStore((state) => state.showToast);
  const [providerId, setProviderId] = useState("");
  const [pendingOperation, setPendingOperation] = useState<AiOperation | null>(null);
  const [instruction, setInstruction] = useState("");
  const [includeTitle, setIncludeTitle] = useState(true);
  const [includeSummary, setIncludeSummary] = useState(true);
  const [includeBody, setIncludeBody] = useState(false);
  const [includeMedia, setIncludeMedia] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [result, setResult] = useState<AiGenerateResult | null>(null);
  const [undo, setUndo] = useState<UndoState | null>(null);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (!providerId) {
      const preferred =
        providers.find((provider) => provider.isDefault && provider.configured) ??
        providers.find((provider) => provider.configured) ??
        providers[0];
      if (preferred) setProviderId(preferred.id);
    }
  }, [providers, providerId]);

  const operations = useMemo(() => (editor ? operationsForKind(editor.kind) : []), [editor]);
  const provider = providers.find((item) => item.id === providerId) ?? null;

  if (!editor) return null;
  const activeEditor = editor;

  function chooseOperation(operation: AiOperation) {
    const target = aiOperationTarget(operation);
    setPendingOperation(operation);
    setIncludeTitle(target !== "body");
    setIncludeSummary(target !== "body");
    setIncludeBody(target === "body");
    setIncludeMedia(editor?.kind === "video" || editor?.kind === "imageText");
    setInstruction("");
    setResult(null);
    setUndo(null);
  }

  const request: AiGenerateRequest | null =
    pendingOperation && provider
      ? {
          providerId: provider.id,
          operation: pendingOperation,
          contentKind: editor.kind,
          title: includeTitle ? editor.title : "",
          context: [
            includeTitle && editor.title ? `标题：${editor.title}` : "",
            includeSummary && editor.summary ? `摘要/短文案：${editor.summary}` : "",
            includeBody && editor.body ? `正文：\n${editor.body}` : "",
          ]
            .filter(Boolean)
            .join("\n\n"),
          mediaMetadata: includeMedia ? buildAiMetadata(editor.media) : [],
          instruction: instruction.trim() || null,
        }
      : null;

  async function runGeneration() {
    if (!request) return;
    const generated = await generate(request);
    if (generated) {
      setResult(generated);
      setConfirmOpen(false);
      showToast("AI 结果已生成；应用前不会修改主内容");
    }
  }

  function applyResult() {
    if (!result || !pendingOperation) return;
    const target = aiOperationTarget(pendingOperation);
    const previous =
      target === "title"
        ? activeEditor.title
        : target === "summary"
          ? activeEditor.summary
          : activeEditor.body;
    if (target === "title") updateEditor({ title: result.content });
    if (target === "summary") updateEditor({ summary: result.content });
    if (target === "body") updateEditor({ body: result.content });
    setUndo({ target, previous });
    setResult(null);
    showToast("已应用 AI 结果，可在本次操作中撤销");
  }

  function undoResult() {
    if (!undo) return;
    if (undo.target === "title") updateEditor({ title: undo.previous });
    if (undo.target === "summary") updateEditor({ summary: undo.previous });
    if (undo.target === "body") updateEditor({ body: undo.previous });
    setUndo(null);
    showToast("已撤销本次 AI 操作");
  }

  if (!provider || !provider.configured) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <div className="grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-accent-deep">
          <Sparkles size={20} />
        </div>
        <h3 className="text-sm font-semibold text-ink">尚未配置 BYOK 模型</h3>
        <p className="text-[11px] leading-relaxed text-ink-muted">
          添加供应商、兼容端点或本地模型，并测试连接后才能使用形态化 AI 辅助。API Key
          仅保存在系统钥匙串。
        </p>
        <button
          type="button"
          onClick={() => navigate("/settings/ai")}
          className="rounded-xl bg-accent px-3 py-2 text-xs font-medium text-accent-on"
        >
          前往 AI 模型配置
        </button>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-line px-4 py-3.5">
        <div className="flex items-center justify-between gap-2">
          <div>
            <h3 className="text-[13px] font-semibold text-ink">形态化 AI 辅助</h3>
            <p className="mt-0.5 text-[11px] text-ink-muted">结果需预览并手动应用</p>
          </div>
          <span className="rounded-md bg-mint-bg px-2 py-0.5 text-[10px] font-medium text-mint-text">
            BYOK
          </span>
        </div>
        <label className="mt-3 block text-[11px] font-medium text-ink-muted">
          当前模型
          <select
            value={providerId}
            onChange={(event) => setProviderId(event.target.value)}
            className="mt-1.5 w-full rounded-xl border border-line bg-surface px-2.5 py-2 text-xs text-ink outline-none transition-colors focus:border-accent"
          >
            {providers.map((item) => (
              <option key={item.id} value={item.id} disabled={!item.configured}>
                {item.name} · {item.model}
                {item.configured ? "" : "（未配置 Key）"}
              </option>
            ))}
          </select>
        </label>
        <p className="mt-1.5 truncate text-[10px] text-ink-faint">{provider.baseUrl}</p>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        <div className="grid gap-2">
          {operations.map((operation) => (
            <button
              key={operation.operation}
              type="button"
              onClick={() => chooseOperation(operation.operation)}
              className={`flex cursor-pointer items-start gap-2.5 rounded-xl border p-3 text-left transition-all ${
                pendingOperation === operation.operation
                  ? "border-accent bg-accent-soft/45 shadow-xs"
                  : "border-line bg-surface hover:border-accent-border"
              }`}
            >
              <Sparkles size={12} className="mt-0.5 shrink-0 text-accent" />
              <div className="min-w-0">
                <strong className="block text-xs font-medium text-ink">{operation.label}</strong>
                <span className="mt-0.5 block text-[10.5px] leading-snug text-ink-muted">
                  {operation.description}
                </span>
              </div>
            </button>
          ))}
        </div>

        {undo && (
          <button
            type="button"
            onClick={undoResult}
            className="mt-3 inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-line px-2 py-1.5 text-[10.5px] text-ink-secondary hover:border-accent"
          >
            <RotateCcw size={10} /> 撤销本次操作
          </button>
        )}

        {result && (
          <div className="mt-3 rounded-xl border border-accent-border bg-accent-soft/40 p-2.5">
            <div className="flex items-center justify-between">
              <span className="text-[10.5px] font-medium text-ink">AI 结果预览</span>
              <button
                type="button"
                aria-label="丢弃 AI 结果"
                onClick={() => setResult(null)}
                className="text-ink-muted hover:text-coral-text"
              >
                <X size={11} />
              </button>
            </div>
            <p className="mt-1 text-[9.5px] text-ink-muted">
              {result.providerName} · {result.model}
            </p>
            <textarea
              readOnly
              value={result.content}
              className="mt-2 min-h-32 w-full resize-y rounded-lg border border-line bg-surface p-2 text-[11px] leading-relaxed text-ink"
            />
            {aiOperationTarget(pendingOperation ?? "custom") !== "body" ||
            pendingOperation === "contentCheck" ? null : (
              <p className="mt-1 text-[9.5px] leading-relaxed text-ink-muted">
                应用后会替换编辑区正文；未应用前主内容保持不变。
              </p>
            )}
            <div className="mt-2 flex gap-1.5">
              <button
                type="button"
                onClick={applyResult}
                disabled={pendingOperation === "contentCheck"}
                className="flex-1 rounded bg-accent py-1.5 text-[10px] font-medium text-accent-on disabled:opacity-50"
              >
                {pendingOperation === "contentCheck" ? "检查结果仅供查看" : "应用结果"}
              </button>
              <button
                type="button"
                onClick={() => {
                  void navigator.clipboard?.writeText(result.content);
                  showToast("AI 结果已复制");
                }}
                className="rounded border border-line px-2 py-1.5 text-[10px] text-ink-secondary"
              >
                <Copy size={10} />
              </button>
            </div>
          </div>
        )}
      </div>

      {pendingOperation && !confirmOpen && !result && (
        <div className="border-t border-line p-3">
          <button
            type="button"
            onClick={() => setConfirmOpen(true)}
            disabled={!request || busyAction === "generate"}
            className="inline-flex w-full items-center justify-center gap-1.5 rounded-xl bg-accent py-2 text-xs font-medium text-accent-on disabled:opacity-50"
          >
            <Shield size={11} /> 查看发送内容并确认
          </button>
        </div>
      )}

      {confirmOpen && request && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-black/35 p-4 backdrop-blur-[2px]">
          <div className="max-h-[88vh] w-[min(680px,100%)] overflow-y-auto rounded-2xl border border-line bg-surface p-5 shadow-2xl">
            <div className="flex items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold text-ink">确认发送到模型服务商</h3>
                <p className="mt-1 text-[11px] leading-relaxed text-ink-secondary">
                  未确认前不会发出任何请求。以下内容会发送给所选 BYOK 服务商，不包含工作区路径、
                  未选素材或整个媒体库。
                </p>
              </div>
              <button
                type="button"
                onClick={() => setConfirmOpen(false)}
                className="text-ink-muted hover:text-ink"
              >
                <X size={14} />
              </button>
            </div>

            <div className="mt-3 grid gap-2 text-[11px] sm:grid-cols-3">
              <div className="rounded-lg bg-canvas p-2">
                <span className="block text-ink-muted">内容形态</span>
                <strong className="text-ink">{editor.kind}</strong>
              </div>
              <div className="rounded-lg bg-canvas p-2">
                <span className="block text-ink-muted">服务商</span>
                <strong className="text-ink">{provider.name}</strong>
              </div>
              <div className="rounded-lg bg-canvas p-2">
                <span className="block text-ink-muted">模型</span>
                <strong className="break-all text-ink">{provider.model}</strong>
              </div>
            </div>

            <div className="mt-3 grid grid-cols-2 gap-1.5 text-[10.5px]">
              {[
                ["包含标题", includeTitle, setIncludeTitle],
                ["包含摘要/短文案", includeSummary, setIncludeSummary],
                ["包含正文", includeBody, setIncludeBody],
                ["包含素材元数据", includeMedia, setIncludeMedia],
              ].map(([label, checked, setter]) => (
                <label
                  key={String(label)}
                  className="flex items-center gap-1.5 rounded border border-line px-2 py-1.5"
                >
                  <input
                    type="checkbox"
                    checked={Boolean(checked)}
                    onChange={(event) => (setter as (value: boolean) => void)(event.target.checked)}
                    className="accent-[var(--color-accent)]"
                  />
                  {String(label)}
                </label>
              ))}
            </div>

            {(pendingOperation === "custom" || pendingOperation === "tone") && (
              <label className="mt-3 block text-[10.5px] text-ink-muted">
                附加要求
                <textarea
                  value={instruction}
                  onChange={(event) => setInstruction(event.target.value)}
                  placeholder="例如：转换为更直接、技术性的语气"
                  className="mt-1 min-h-16 w-full rounded-lg border border-line bg-canvas p-2 text-[11px] text-ink outline-none focus:border-accent"
                />
              </label>
            )}

            <div className="mt-3">
              <span className="text-[10.5px] font-medium text-ink">
                实际发送载荷（节选正文与素材元数据）
              </span>
              <pre className="mt-1 max-h-52 overflow-auto rounded-lg border border-line bg-canvas p-2 font-mono text-[9.5px] leading-relaxed text-ink-secondary">
                {JSON.stringify(request, null, 2)}
              </pre>
            </div>

            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setConfirmOpen(false)}
                className="rounded-lg border border-line px-3 py-1.5 text-xs text-ink-secondary"
              >
                取消
              </button>
              <button
                type="button"
                onClick={() => void runGeneration()}
                disabled={busyAction === "generate"}
                className="inline-flex items-center gap-1.5 rounded-lg bg-accent px-3 py-1.5 text-xs font-medium text-accent-on disabled:opacity-50"
              >
                {busyAction === "generate" ? <LoaderCircle size={11} /> : <Check size={11} />}
                {busyAction === "generate" ? "生成中..." : "确认并发送"}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
