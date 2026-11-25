import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import { Eye, History, PanelLeftOpen, Server, ShieldCheck, Sparkles } from "../components/Icons";
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
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      {/* Slim Codex-style Top Workspace Bar */}
      <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line bg-surface-soft px-4">
        <div className="flex min-w-0 items-center gap-2.5">
          {!threadRailOpen && (
            <button
              type="button"
              onClick={() => setThreadRailOpen(true)}
              className="grid h-7.5 w-7.5 shrink-0 cursor-pointer place-items-center rounded-lg border border-line bg-surface text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
              title="展开任务会话列表"
              aria-label="展开任务会话列表"
            >
              <PanelLeftOpen size={13} />
            </button>
          )}
          <div className="flex min-w-0 items-center gap-2">
            <h1 className="truncate text-[13.5px] font-semibold tracking-tight text-ink">
              {mcp.activeThread?.thread.title ?? "AI 助手"}
            </h1>
            <span className="hidden rounded-md border border-line bg-surface px-2 py-0.5 font-mono text-[10px] text-ink-muted sm:inline-block">
              {mcp.activeThread?.thread.providerName ?? "本地编排器"} ·{" "}
              {mcp.activeThread?.thread.model ?? "deterministic-v1"}
            </span>
          </div>
        </div>

        {/* Right Controls: Inspector Toggles & Quick Actions */}
        <div className="flex shrink-0 items-center gap-1.5">
          <div
            className="hidden items-center gap-0.5 rounded-xl border border-line bg-canvas p-0.5 md:inline-flex"
            role="group"
            aria-label="检查面板"
          >
            <button
              type="button"
              onClick={() => toggleInspector("mcp")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-[11px] font-medium transition-colors ${
                inspectorTab === "mcp"
                  ? "bg-surface text-ink shadow-xs"
                  : "text-ink-secondary hover:text-ink"
              }`}
            >
              <span
                className={`h-1.5 w-1.5 rounded-full ${
                  isHealthy ? "bg-mint-text" : "bg-coral-text"
                }`}
              />
              <Server size={10} />
              <span>MCP ({mcp.tools.length})</span>
            </button>
            <button
              type="button"
              onClick={() => toggleInspector("privacy")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-[11px] font-medium transition-colors ${
                inspectorTab === "privacy"
                  ? "bg-surface text-ink shadow-xs"
                  : "text-ink-secondary hover:text-ink"
              }`}
            >
              <Eye size={10} />
              <span>发送披露</span>
            </button>
            <button
              type="button"
              onClick={() => toggleInspector("audit")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-[11px] font-medium transition-colors ${
                inspectorTab === "audit"
                  ? "bg-surface text-ink shadow-xs"
                  : "text-ink-secondary hover:text-ink"
              }`}
            >
              <History size={10} />
              <span>审计 ({mcp.audit.length})</span>
            </button>
          </div>

          <button
            type="button"
            onClick={() => void mcp.healthCheck()}
            disabled={Boolean(mcp.busyAction)}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-2.5 text-[11.5px] text-ink-secondary transition-colors hover:border-line-strong hover:text-ink disabled:opacity-50"
            title="执行 MCP Host 健康检查"
          >
            <ShieldCheck size={12} />
            <span className="hidden lg:inline">健康检查</span>
          </button>

          <button
            type="button"
            onClick={() => navigate("/settings/ai")}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-accent-border bg-accent-soft px-2.5 text-[11.5px] font-medium text-accent-deep transition-colors hover:opacity-90"
          >
            <Sparkles size={11} />
            <span className="hidden sm:inline">模型 Key</span>
          </button>
        </div>
      </header>

      {/* Main Codex 3-Zone Workspace */}
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

        {/* Center Conversation & Strategy Composer Stage */}
        <section className="flex min-h-0 min-w-0 flex-1 flex-col">
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

        {/* Collapsible Right Inspector Drawer */}
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
