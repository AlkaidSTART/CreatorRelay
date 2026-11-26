import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Eye, History, PanelLeftOpen, Server, Settings2 } from "../components/Icons";
import { AssistantComposer, type ComposerMode } from "../components/assistant/AssistantComposer";
import {
  AssistantInspectorDrawer,
  type InspectorTab,
} from "../components/assistant/AssistantInspectorDrawer";
import { AssistantMessageStream } from "../components/assistant/AssistantMessageStream";
import { AssistantThreadRail } from "../components/assistant/AssistantThreadRail";
import { ToolAuthorizationModal } from "../components/assistant/ToolAuthorizationModal";
import { useAiStore, useMcpStore, useStudioStore, useUiStore } from "../stores";
import type { AssistantMessage, ModelContextScope } from "../types/mcp";

export function AssistantPage() {
  const navigate = useNavigate();
  const showToast = useUiStore((state) => state.showToast);
  const mcp = useMcpStore();
  const providers = useAiStore((state) => state.providers);
  const initializeAi = useAiStore((state) => state.initialize);
  const packages = useStudioStore((state) => state.packages);
  const initializeStudio = useStudioStore((state) => state.initialize);

  const [input, setInput] = useState("");
  const [mode, setMode] = useState<ComposerMode>("mcp");
  const [threadRailOpen, setThreadRailOpen] = useState(true);
  const [inspectorTab, setInspectorTab] = useState<InspectorTab | null>(null);
  const [providerId, setProviderId] = useState("");
  const [contentId, setContentId] = useState("");
  const [scope, setScope] = useState<ModelContextScope>({
    includeTitle: true,
    includeSummary: true,
    includeBody: false,
    includeMediaMetadata: true,
  });
  const [settingsDraft, setSettingsDraft] = useState(mcp.settings);

  useEffect(() => {
    void mcp.initialize();
    void initializeStudio();
    void initializeAi();
  }, [initializeAi, initializeStudio, mcp.initialize]);

  useEffect(() => {
    setSettingsDraft(mcp.settings);
  }, [mcp.settings]);

  useEffect(() => {
    if (!providerId && providers.length > 0) {
      setProviderId(providers.find((provider) => provider.isDefault)?.id ?? providers[0].id);
    }
  }, [providerId, providers]);

  useEffect(() => {
    if (!contentId && packages.length > 0) {
      setContentId(packages[0].id);
    }
  }, [contentId, packages]);

  useEffect(() => {
    if (!mcp.initialized) return;
    if (inspectorTab !== "privacy" && mode !== "model") return;
    void mcp.loadContextPreview(providerId || null, contentId || null, scope);
  }, [contentId, inspectorTab, mcp.initialized, mcp.loadContextPreview, mode, providerId, scope]);

  const activeMessages = mcp.activeThread?.messages ?? [];
  const isHealthy = mcp.status?.state === "healthy";

  function toggleInspector(target: InspectorTab) {
    setInspectorTab((current) => (current === target ? null : target));
  }

  async function sendMcp(promptText?: string) {
    const text = (promptText ?? input).trim();
    if (!text) return;
    if (!promptText) setInput("");
    await mcp.sendPrompt(text);
  }

  function previewMessage(message: AssistantMessage) {
    const tool = message.toolCall;
    if (!tool?.arguments) return;
    void mcp.previewToolCall(tool.toolName, tool.arguments);
  }

  async function generateReply() {
    const promptText = input.trim();
    const result = await mcp.generateReply({
      prompt: promptText || "请基于我选择的上下文给出简洁总结。",
      providerId: providerId || null,
      contentId: contentId || null,
      scope,
      instruction: null,
    });
    if (promptText) setInput("");
    if (result?.assistantMessage.resultStatus === "success") {
      showToast("模型回答已写入对话历史");
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-surface">
      {/* Unified Workspace Container */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {threadRailOpen && (
          <AssistantThreadRail
            threads={mcp.threads}
            activeThreadId={mcp.activeThread?.thread.id ?? null}
            status={mcp.status}
            toolCount={mcp.tools.length}
            busyAction={mcp.busyAction}
            onCreateThread={() => void mcp.createThread()}
            onSelectThread={(threadId) => void mcp.selectThread(threadId)}
            onDeleteThread={(threadId) => void mcp.deleteThread(threadId)}
            onCollapse={() => setThreadRailOpen(false)}
            onOpenHostInspector={() => setInspectorTab("mcp")}
            onHealthCheck={() => void mcp.healthCheck()}
          />
        )}

        {/* Center Conversation & Composer Stage */}
        <section className="flex min-h-0 min-w-0 flex-1 flex-col bg-surface">
          {/* Slim Single-Row Conversation Toolbar */}
          <header className="flex h-11 shrink-0 items-center justify-between gap-3 border-b border-line bg-surface px-4">
            <div className="flex min-w-0 items-center gap-2">
              {!threadRailOpen && (
                <button
                  type="button"
                  onClick={() => setThreadRailOpen(true)}
                  className="inline-flex h-7 shrink-0 cursor-pointer items-center gap-1.5 rounded-md border border-line bg-surface px-2 text-[11.5px] font-medium text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
                  title="展开任务会话列表"
                  aria-label="展开任务会话列表"
                >
                  <PanelLeftOpen size={12} />
                  <span className="hidden sm:inline">会话</span>
                </button>
              )}
              <h1 className="truncate text-[13px] font-semibold tracking-tight text-ink">
                {mcp.activeThread?.thread.title ?? "AI 助手"}
              </h1>
              <span className="hidden truncate font-mono text-[10.5px] text-ink-muted md:inline">
                {mcp.activeThread?.thread.providerName ?? "本地编排器"} /{" "}
                {mcp.activeThread?.thread.model ?? "deterministic-v1"}
              </span>
            </div>

            {/* Right Controls: Clean On-Demand Inspector Toggles */}
            <div className="flex shrink-0 items-center gap-1">
              <button
                type="button"
                onClick={() => toggleInspector("mcp")}
                className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md border px-2 text-[11.5px] font-medium transition-colors ${
                  inspectorTab === "mcp"
                    ? "border-accent-border bg-accent-soft text-accent-deep"
                    : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                }`}
              >
                <span
                  className={`h-1.5 w-1.5 rounded-full ${
                    isHealthy ? "bg-mint-text" : "bg-coral-text"
                  }`}
                />
                <Server size={11} />
                <span>MCP</span>
                <span className="font-mono text-[10px] opacity-75">{mcp.tools.length}</span>
              </button>

              <button
                type="button"
                onClick={() => toggleInspector("privacy")}
                className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-md border px-2 text-[11.5px] font-medium transition-colors ${
                  inspectorTab === "privacy"
                    ? "border-accent-border bg-accent-soft text-accent-deep"
                    : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                }`}
              >
                <Eye size={11} />
                <span className="hidden sm:inline">披露</span>
              </button>

              <button
                type="button"
                onClick={() => toggleInspector("audit")}
                className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-md border px-2 text-[11.5px] font-medium transition-colors ${
                  inspectorTab === "audit"
                    ? "border-accent-border bg-accent-soft text-accent-deep"
                    : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                }`}
              >
                <History size={11} />
                <span className="hidden sm:inline">审计</span>
                {mcp.audit.length > 0 && (
                  <span className="font-mono text-[10px] opacity-75">{mcp.audit.length}</span>
                )}
              </button>

              <span className="mx-0.5 hidden h-4 w-px bg-line sm:inline-block" aria-hidden="true" />

              <button
                type="button"
                onClick={() => navigate("/settings/ai")}
                title="管理模型与 API Key"
                className="inline-flex h-7 cursor-pointer items-center gap-1 rounded-md border border-line bg-surface px-2 text-[11.5px] font-medium text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
              >
                <Settings2 size={11} />
                <span className="hidden lg:inline">模型配置</span>
              </button>
            </div>
          </header>

          <AssistantMessageStream
            messages={activeMessages}
            loading={mcp.loading}
            busyAction={mcp.busyAction}
            onPreview={previewMessage}
            onQuickSend={(prompt) => {
              setMode("mcp");
              void sendMcp(prompt);
            }}
            onCancelCall={() => void mcp.cancelCall()}
          />

          <AssistantComposer
            input={input}
            onInputChange={setInput}
            mode={mode}
            onModeChange={setMode}
            providers={providers}
            providerId={providerId}
            onProviderChange={setProviderId}
            packages={packages}
            contentId={contentId}
            onContentChange={setContentId}
            scope={scope}
            onScopeChange={setScope}
            contextPreview={mcp.contextPreview}
            settings={mcp.settings}
            onSaveSettings={(next) => void mcp.saveSettings(next)}
            busyAction={mcp.busyAction}
            onSendMcp={() => void sendMcp()}
            onSendModel={() => void generateReply()}
            onCancelCall={() => void mcp.cancelCall()}
            onOpenPrivacyInspector={() => setInspectorTab("privacy")}
            onNavigateSettings={() => navigate("/settings/ai")}
          />
        </section>

        {/* On-Demand Right Inspector Drawer */}
        {inspectorTab && (
          <AssistantInspectorDrawer
            tab={inspectorTab}
            onTabChange={setInspectorTab}
            onClose={() => setInspectorTab(null)}
            status={mcp.status}
            tools={mcp.tools}
            security={mcp.security}
            audit={mcp.audit}
            settingsDraft={settingsDraft}
            onSettingsDraftChange={setSettingsDraft}
            onSaveSettings={(next) => void mcp.saveSettings(next)}
            onStartServer={() => void mcp.startServer()}
            onStopServer={() => void mcp.stopServer()}
            onRunHangCheck={() => void mcp.previewToolCall("mcp_test_hang", { milliseconds: 5000 })}
            providers={providers}
            providerId={providerId}
            onProviderChange={setProviderId}
            packages={packages}
            contentId={contentId}
            onContentChange={setContentId}
            scope={scope}
            onScopeChange={setScope}
            contextPreview={mcp.contextPreview}
            onGenerateReply={() => void generateReply()}
            busyAction={mcp.busyAction}
          />
        )}
      </div>

      {/* Tool Authorization Preview Modal */}
      {mcp.activePreview && (
        <ToolAuthorizationModal
          preview={mcp.activePreview}
          busyAction={mcp.busyAction}
          onClose={mcp.clearPreview}
          onConfirm={() => void mcp.confirmPreview()}
        />
      )}
    </main>
  );
}
