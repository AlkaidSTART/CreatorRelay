import { useEffect, useRef, useState } from "react";
import {
  ArrowUp,
  Bot,
  Check,
  ChevronsUpDown,
  Eye,
  FileText,
  KeyRound,
  ShieldCheck,
  Sparkles,
  Terminal,
  X,
} from "../Icons";
import type { AiProvider } from "../../types/ai";
import type { ContentPackageSummary } from "../../types/content";
import type { AssistantContextPreview, McpSettings, ModelContextScope } from "../../types/mcp";

export type ComposerMode = "mcp" | "model";

interface AssistantComposerProps {
  input: string;
  onInputChange: (value: string) => void;
  mode: ComposerMode;
  onModeChange: (mode: ComposerMode) => void;
  providers: AiProvider[];
  providerId: string;
  onProviderChange: (providerId: string) => void;
  packages: ContentPackageSummary[];
  contentId: string;
  onContentChange: (contentId: string) => void;
  scope: ModelContextScope;
  onScopeChange: (scope: ModelContextScope) => void;
  contextPreview: AssistantContextPreview | null;
  settings: McpSettings | null;
  onSaveSettings: (next: McpSettings) => void;
  busyAction: string;
  onSendMcp: () => void;
  onSendModel: () => void;
  onCancelCall: () => void;
  onOpenPrivacyInspector: () => void;
  onNavigateSettings: () => void;
}

const SCOPE_ITEMS: { key: keyof ModelContextScope; label: string }[] = [
  { key: "includeTitle", label: "标题" },
  { key: "includeSummary", label: "摘要与标签" },
  { key: "includeBody", label: "正文" },
  { key: "includeMediaMetadata", label: "素材元数据" },
];

export function AssistantComposer({
  input,
  onInputChange,
  mode,
  onModeChange,
  providers,
  providerId,
  onProviderChange,
  packages,
  contentId,
  onContentChange,
  scope,
  onScopeChange,
  contextPreview,
  settings,
  onSaveSettings,
  busyAction,
  onSendMcp,
  onSendModel,
  onCancelCall,
  onOpenPrivacyInspector,
  onNavigateSettings,
}: AssistantComposerProps) {
  const [openPopover, setOpenPopover] = useState<"model" | "context" | "policy" | null>(null);
  const [policyDraft, setPolicyDraft] = useState<McpSettings | null>(settings);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const textareaRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    setPolicyDraft(settings);
  }, [settings]);

  useEffect(() => {
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpenPopover(null);
      }
    }
    if (openPopover) {
      window.addEventListener("mousedown", handleClickOutside);
      return () => window.removeEventListener("mousedown", handleClickOutside);
    }
    return undefined;
  }, [openPopover]);

  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${Math.min(el.scrollHeight, 168)}px`;
  }, [input]);

  const selectedProvider = providers.find((item) => item.id === providerId) ?? null;
  const selectedPackage = packages.find((item) => item.id === contentId) ?? null;
  const enabledScopeLabels = SCOPE_ITEMS.filter((item) => scope[item.key]).map(
    (item) => item.label,
  );

  function handleSubmit() {
    if (busyAction) return;
    if (mode === "mcp") {
      if (!input.trim()) return;
      onSendMcp();
    } else {
      onSendModel();
    }
  }

  const writePolicyLabel = settings?.writePolicy === "allow" ? "写入：自动" : "写入：确认";

  return (
    <div className="shrink-0 border-t border-line/70 bg-surface px-6 pt-3 pb-4">
      <div ref={containerRef} className="relative mx-auto w-full max-w-[740px]">
        {/* Popover: Model Picker */}
        {openPopover === "model" && (
          <div className="absolute bottom-full left-0 z-30 mb-2 w-72 rounded-md border border-line-strong bg-surface p-1.5 shadow-lg">
            <div className="px-2 py-1 text-[10px] font-semibold tracking-wider text-ink-muted uppercase">
              执行引擎 / 模型
            </div>
            <button
              type="button"
              onClick={() => {
                onModeChange("mcp");
                onProviderChange("");
                setOpenPopover(null);
              }}
              className={`flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs transition-colors ${
                mode === "mcp"
                  ? "bg-accent-soft text-accent-deep"
                  : "text-ink-secondary hover:bg-hover"
              }`}
            >
              <Terminal size={12} />
              <span className="min-w-0 flex-1">
                <strong className="block font-medium">本地编排器 (MCP)</strong>
                <small className="font-mono text-[10px] text-ink-muted">
                  deterministic-v1 · 不经云端
                </small>
              </span>
              {mode === "mcp" && <Check size={12} />}
            </button>

            {providers.length > 0 && (
              <div className="mt-1.5 border-t border-line pt-1.5">
                <div className="px-2 py-1 text-[10px] text-ink-muted">BYOK 模型对话</div>
                <div className="max-h-44 space-y-0.5 overflow-y-auto">
                  {providers.map((provider) => {
                    const active = mode === "model" && providerId === provider.id;
                    return (
                      <button
                        key={provider.id}
                        type="button"
                        onClick={() => {
                          onProviderChange(provider.id);
                          onModeChange("model");
                          setOpenPopover(null);
                        }}
                        className={`flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs transition-colors ${
                          active
                            ? "bg-accent-soft text-accent-deep"
                            : "text-ink-secondary hover:bg-hover"
                        }`}
                      >
                        <Sparkles size={12} />
                        <span className="min-w-0 flex-1">
                          <strong className="block truncate font-medium">{provider.name}</strong>
                          <small className="block truncate font-mono text-[10px] text-ink-muted">
                            {provider.model}
                          </small>
                        </span>
                        {active && <Check size={12} />}
                      </button>
                    );
                  })}
                </div>
              </div>
            )}

            <div className="mt-1.5 border-t border-line pt-1.5">
              <button
                type="button"
                onClick={() => {
                  setOpenPopover(null);
                  onNavigateSettings();
                }}
                className="flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-[11px] text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
              >
                <KeyRound size={11} />
                管理模型与 API Key…
              </button>
            </div>
          </div>
        )}

        {/* Popover: Context Attachment & Scope */}
        {openPopover === "context" && (
          <div className="absolute bottom-full left-0 z-30 mb-2 w-80 rounded-md border border-line-strong bg-surface p-3 shadow-lg">
            <div className="flex items-center justify-between">
              <span className="text-[11.5px] font-semibold text-ink">附加内容上下文</span>
              <button
                type="button"
                onClick={() => {
                  setOpenPopover(null);
                  onOpenPrivacyInspector();
                }}
                className="inline-flex cursor-pointer items-center gap-1 text-[10.5px] font-medium text-accent hover:underline"
              >
                <Eye size={10} />
                完整披露预览
              </button>
            </div>

            <label className="mt-2.5 block text-[10.5px] text-ink-secondary">
              选择内容包
              <select
                value={contentId}
                onChange={(event) => onContentChange(event.target.value)}
                className="mt-1 w-full rounded-sm border border-line bg-canvas px-2 py-1.5 text-xs text-ink outline-none focus:border-accent"
              >
                <option value="">不附加内容包（仅发送输入文本）</option>
                {packages.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.title} · {item.kind}
                  </option>
                ))}
              </select>
            </label>

            <div className="mt-2.5">
              <span className="text-[10.5px] text-ink-secondary">允许发送字段</span>
              <div className="mt-1.5 grid grid-cols-2 gap-1.5">
                {SCOPE_ITEMS.map((item) => (
                  <label
                    key={item.key}
                    className="flex cursor-pointer items-center gap-1.5 rounded-sm border border-line bg-canvas px-2 py-1.5 text-[11px] text-ink-secondary hover:border-line-strong"
                  >
                    <input
                      type="checkbox"
                      checked={Boolean(scope[item.key])}
                      onChange={(event) =>
                        onScopeChange({ ...scope, [item.key]: event.target.checked })
                      }
                    />
                    {item.label}
                  </label>
                ))}
              </div>
            </div>

            <p className="mt-2.5 text-[10px] leading-relaxed text-ink-muted">
              工具调用始终本地执行；仅在模型模式下发送时，上述字段才会传给模型服务商。
            </p>
          </div>
        )}

        {/* Popover: Approval Policy */}
        {openPopover === "policy" && policyDraft && (
          <div className="absolute bottom-full left-0 z-30 mb-2 w-76 rounded-md border border-line-strong bg-surface p-3 shadow-lg">
            <div className="flex items-center gap-1.5 text-[11.5px] font-semibold text-ink">
              <ShieldCheck size={12} className="text-accent" />
              执行授权策略
            </div>
            <div className="mt-2.5 space-y-2.5 text-[11.5px] text-ink-secondary">
              <label className="flex cursor-pointer items-center justify-between gap-2">
                <span>允许只读工具</span>
                <input
                  type="checkbox"
                  checked={policyDraft.readEnabled}
                  onChange={(event) =>
                    setPolicyDraft({ ...policyDraft, readEnabled: event.target.checked })
                  }
                />
              </label>
              <label className="flex cursor-pointer items-center justify-between gap-2">
                <span>异常自动重启 MCP Host</span>
                <input
                  type="checkbox"
                  checked={policyDraft.autoRestart}
                  onChange={(event) =>
                    setPolicyDraft({ ...policyDraft, autoRestart: event.target.checked })
                  }
                />
              </label>
              <label className="block">
                <span className="text-[10.5px] text-ink-muted">本地写操作授权</span>
                <select
                  value={policyDraft.writePolicy}
                  onChange={(event) =>
                    setPolicyDraft({
                      ...policyDraft,
                      writePolicy: event.target.value as "confirm" | "allow",
                    })
                  }
                  className="mt-1 w-full rounded-sm border border-line bg-canvas px-2 py-1.5 text-xs text-ink outline-none focus:border-accent"
                >
                  <option value="confirm">每次确认（推荐）</option>
                  <option value="allow">允许已授权写入</option>
                </select>
              </label>
              <p className="text-[10px] text-ink-muted">
                外部发布（externalAction）始终强制每次确认，不可关闭。
              </p>
              <button
                type="button"
                disabled={Boolean(busyAction)}
                onClick={() => {
                  onSaveSettings(policyDraft);
                  setOpenPopover(null);
                }}
                className="w-full cursor-pointer rounded-md bg-accent py-1.5 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:opacity-50"
              >
                保存策略
              </button>
            </div>
          </div>
        )}

        {/* Unified Crisp Composer Box */}
        <div className="overflow-hidden rounded-lg border border-line-strong bg-surface transition-colors focus-within:border-accent">
          {/* Compact Model Disclosure Strip (Only in Model Mode) */}
          {mode === "model" && (
            <div className="flex flex-wrap items-center gap-2 border-b border-line bg-surface-soft px-3 py-1.5 text-[11px]">
              <span className="font-medium text-accent-deep">模型模式</span>
              <span className="text-ink-faint">·</span>
              <span className="max-w-[200px] truncate text-ink-secondary">
                {selectedPackage ? selectedPackage.title : "未附加内容包"}
              </span>
              {enabledScopeLabels.length > 0 && selectedPackage && (
                <span className="text-[10.5px] text-ink-muted">
                  ({enabledScopeLabels.join(" / ")})
                </span>
              )}
              {contextPreview && (
                <span className="font-mono text-[10px] text-ink-muted">
                  {contextPreview.characterCount} 字
                </span>
              )}
              <button
                type="button"
                onClick={onOpenPrivacyInspector}
                className="ml-auto inline-flex cursor-pointer items-center gap-1 text-[10.5px] font-medium text-accent hover:underline"
              >
                <Eye size={10} />
                披露详情
              </button>
            </div>
          )}

          {/* Input Area */}
          <div className="px-3.5 pt-2.5 pb-1.5">
            <textarea
              ref={textareaRef}
              value={input}
              onChange={(event) => onInputChange(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  handleSubmit();
                }
              }}
              rows={2}
              placeholder={
                mode === "mcp"
                  ? "输入本地任务，例如：查询账号状态、检索草稿，或运行 MCP 自检…"
                  : "输入想向模型咨询的问题，将按披露范围发送…"
              }
              className="max-h-42 min-h-[44px] w-full resize-none bg-transparent text-[13.5px] leading-relaxed text-ink placeholder:text-ink-faint outline-none"
            />
          </div>

          {/* Clean Bottom Control Strip */}
          <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line/70 bg-surface-soft/60 px-2.5 py-1.5">
            <div className="flex flex-wrap items-center gap-1">
              {/* Compact Segmented Mode Switch */}
              <div
                className="inline-flex items-center rounded-md border border-line bg-subtle p-0.5"
                role="tablist"
                aria-label="执行模式"
              >
                <button
                  type="button"
                  role="tab"
                  aria-selected={mode === "mcp"}
                  onClick={() => onModeChange("mcp")}
                  className={`inline-flex cursor-pointer items-center gap-1 rounded-xs px-2 py-0.5 text-[11px] font-medium transition-colors ${
                    mode === "mcp"
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-muted hover:text-ink"
                  }`}
                >
                  <Terminal size={10} />
                  MCP
                </button>
                <button
                  type="button"
                  role="tab"
                  aria-selected={mode === "model"}
                  onClick={() => onModeChange("model")}
                  className={`inline-flex cursor-pointer items-center gap-1 rounded-xs px-2 py-0.5 text-[11px] font-medium transition-colors ${
                    mode === "model"
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-muted hover:text-ink"
                  }`}
                >
                  <Sparkles size={10} />
                  模型
                </button>
              </div>

              {/* Engine / Model Selector */}
              <button
                type="button"
                onClick={() => setOpenPopover(openPopover === "model" ? null : "model")}
                className="inline-flex h-6.5 max-w-[180px] cursor-pointer items-center gap-1 rounded-md px-2 text-[11px] text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                title="切换执行引擎或模型"
              >
                <Bot size={11} className="shrink-0 text-ink-muted" />
                <span className="truncate">
                  {mode === "mcp"
                    ? "本地编排器"
                    : selectedProvider
                      ? `${selectedProvider.name} · ${selectedProvider.model}`
                      : "选择模型"}
                </span>
                <ChevronsUpDown size={10} className="shrink-0 text-ink-muted" />
              </button>

              {/* Context Button */}
              <button
                type="button"
                onClick={() => setOpenPopover(openPopover === "context" ? null : "context")}
                className={`inline-flex h-6.5 cursor-pointer items-center gap-1 rounded-md px-2 text-[11px] transition-colors ${
                  selectedPackage && mode === "model"
                    ? "bg-accent-soft text-accent-deep"
                    : "text-ink-secondary hover:bg-hover hover:text-ink"
                }`}
                title="配置发送给模型的上下文范围"
              >
                <FileText size={10} />
                <span>上下文</span>
              </button>

              {/* Policy Button */}
              <button
                type="button"
                onClick={() => setOpenPopover(openPopover === "policy" ? null : "policy")}
                className="inline-flex h-6.5 cursor-pointer items-center gap-1 rounded-md px-2 text-[11px] text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                title="调整 MCP 写入与只读授权策略"
              >
                <ShieldCheck size={11} className="text-mint-text" />
                <span>{writePolicyLabel}</span>
              </button>
            </div>

            {/* Submit / Cancel */}
            <div className="flex items-center gap-1.5">
              {busyAction === "execute" ? (
                <button
                  type="button"
                  onClick={onCancelCall}
                  className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-md border border-coral-border bg-coral-bg px-2.5 text-[11.5px] font-medium text-coral-text"
                >
                  <X size={11} />
                  停止
                </button>
              ) : (
                <button
                  type="button"
                  onClick={handleSubmit}
                  disabled={Boolean(busyAction) || (mode === "mcp" && !input.trim())}
                  className="inline-flex h-7 cursor-pointer items-center justify-center gap-1 rounded-md bg-accent px-2.5 text-[11.5px] font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-40"
                  aria-label={mode === "mcp" ? "发送任务" : "确认并发送到模型"}
                >
                  {mode === "model" && <span>发送</span>}
                  <ArrowUp size={12} />
                </button>
              )}
            </div>
          </div>
        </div>

        <div className="mt-1.5 flex items-center justify-between px-0.5 text-[10px] text-ink-faint">
          <span>Enter 发送 · Shift+Enter 换行</span>
          <span>本地工具调用不经云端 · 外部发布始终逐次确认</span>
        </div>
      </div>
    </div>
  );
}
