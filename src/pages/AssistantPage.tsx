import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
import {
  AlertTriangle,
  Bot,
  Check,
  Clock3,
  Info,
  Play,
  Send,
  ShieldCheck,
  Sparkles,
  Terminal,
  X,
} from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useAiStore, useMcpStore, useStudioStore, useUiStore } from "../stores";
import type { AssistantMessage, ModelContextScope, ToolPermission } from "../types/mcp";

const quickPrompts = [
  "查询所有已绑定账号的登录状态",
  "检索本地内容草稿",
  "查询最近 7 天指标趋势",
  "运行 MCP 回显自检",
];

const permissionMeta: Record<ToolPermission, { label: string; className: string }> = {
  read: { label: "只读", className: "bg-sky-bg text-sky-text" },
  write: { label: "写入", className: "bg-lemon-bg text-lemon-text" },
  externalAction: { label: "外部操作", className: "bg-coral-bg text-coral-text" },
};

const statusMeta: Record<string, string> = {
  proposed: "bg-lemon-bg text-lemon-text",
  success: "bg-mint-bg text-mint-text",
  partial: "bg-lemon-bg text-lemon-text",
  failed: "bg-coral-bg text-coral-text",
  cancelled: "bg-sidebar text-ink-secondary",
  timedOut: "bg-coral-bg text-coral-text",
  no_tool: "bg-sidebar text-ink-secondary",
  no_provider: "bg-lemon-bg text-lemon-text",
};

function timeLabel(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function pretty(value: unknown) {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function MessageCard({
  message,
  onPreview,
}: {
  message: AssistantMessage;
  onPreview: (message: AssistantMessage) => void;
}) {
  const isUser = message.role === "user";
  const tool = message.toolCall;
  const canExecute = tool?.status === "proposed" && tool.arguments && tool.permission !== undefined;
  return (
    <article
      className={`rounded-2xl border px-4 py-3 ${
        isUser
          ? "ml-10 border-accent bg-accent text-accent-on shadow-[0_2px_8px_rgba(123,104,168,0.18)]"
          : "border-line bg-surface text-ink"
      }`}
    >
      <div className="mb-2 flex flex-wrap items-center gap-2 text-[11px]">
        <span className={isUser ? "text-white/80" : "font-medium text-ink"}>
          {isUser ? "创作者" : "CreatorRelay Assistant"}
        </span>
        {!isUser && (
          <span className="rounded-md bg-canvas px-1.5 py-0.5 text-[10px] text-ink-muted">
            {message.providerName ?? "本地编排器"} · {message.model ?? "deterministic-v1"}
          </span>
        )}
        <span className={`ml-auto text-[10px] ${isUser ? "text-white/60" : "text-ink-muted"}`}>
          {timeLabel(message.createdAt)}
        </span>
      </div>
      <p className="whitespace-pre-wrap text-[13px] leading-relaxed">{message.content}</p>
      {tool && (
        <div className="mt-3 rounded-xl border border-line bg-canvas p-3">
          <div className="flex flex-wrap items-center gap-2">
            <Terminal size={12} className="text-accent" />
            <code className="font-mono text-[11px] font-medium text-ink">{tool.toolName}</code>
            <span
              className={`rounded-md px-1.5 py-0.5 text-[9.5px] font-medium ${
                permissionMeta[tool.permission]?.className ?? "bg-sidebar text-ink-secondary"
              }`}
            >
              {permissionMeta[tool.permission]?.label ?? tool.permission}
            </span>
            {tool.status && (
              <span
                className={`rounded-md px-1.5 py-0.5 text-[9.5px] ${
                  statusMeta[tool.status] ?? "bg-sidebar text-ink-secondary"
                }`}
              >
                {tool.status}
              </span>
            )}
          </div>
          <dl className="mt-2 grid gap-1.5 text-[10.5px] text-ink-secondary">
            <div className="flex gap-2">
              <dt className="shrink-0 text-ink-muted">目标</dt>
              <dd>{tool.targetSummary}</dd>
            </div>
            <div className="flex gap-2">
              <dt className="shrink-0 text-ink-muted">参数摘要</dt>
              <dd className="min-w-0 break-words font-mono">{tool.parameterSummary}</dd>
            </div>
          </dl>
          {tool.errorMessage && (
            <p className="mt-2 rounded-lg bg-coral-bg px-2 py-1.5 text-[10.5px] text-coral-text">
              {tool.errorCode ? `${tool.errorCode}：` : ""}
              {tool.errorMessage}
            </p>
          )}
          {canExecute && (
            <button
              type="button"
              onClick={() => onPreview(message)}
              className="mt-2.5 inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-accent-border bg-accent-soft px-2.5 py-1.5 text-[10.5px] font-medium text-accent-deep"
            >
              <Play size={9} />
              预览并执行
            </button>
          )}
        </div>
      )}
      {message.errorMessage && !tool?.errorMessage && (
        <p className="mt-2 rounded-lg bg-coral-bg px-2 py-1.5 text-[10.5px] text-coral-text">
          {message.errorMessage}
        </p>
      )}
    </article>
  );
}

export function AssistantPage() {
  const navigate = useNavigate();
  const showToast = useUiStore((state) => state.showToast);
  const mcp = useMcpStore();
  const providers = useAiStore((state) => state.providers);
  const initializeAi = useAiStore((state) => state.initialize);
  const packages = useStudioStore((state) => state.packages);
  const initializeStudio = useStudioStore((state) => state.initialize);
  const [input, setInput] = useState("");
  const [panel, setPanel] = useState<"mcp" | "privacy" | "audit">("mcp");
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
    if (!contentId && packages.length > 0) setContentId(packages[0].id);
  }, [contentId, packages]);

  useEffect(() => {
    if (panel !== "privacy" || !mcp.initialized) return;
    void mcp.loadContextPreview(providerId || null, contentId || null, scope);
  }, [contentId, mcp.initialized, mcp.loadContextPreview, panel, providerId, scope]);

  const activeMessages = mcp.activeThread?.messages ?? [];
  const groupedTools = useMemo(
    () => ({
      read: mcp.tools.filter((tool) => tool.permission === "read"),
      write: mcp.tools.filter((tool) => tool.permission === "write"),
      externalAction: mcp.tools.filter((tool) => tool.permission === "externalAction"),
    }),
    [mcp.tools],
  );

  async function send() {
    if (!input.trim()) return;
    const prompt = input.trim();
    setInput("");
    await mcp.sendPrompt(prompt);
  }

  function previewMessage(message: AssistantMessage) {
    const tool = message.toolCall;
    if (!tool?.arguments) return;
    void mcp.previewToolCall(tool.toolName, tool.arguments);
  }

  async function generateReply() {
    const result = await mcp.generateReply({
      prompt: input.trim() || "请基于我选择的上下文给出简洁总结。",
      providerId: providerId || null,
      contentId: contentId || null,
      scope,
      instruction: null,
    });
    if (result?.assistantMessage.resultStatus === "success") {
      showToast("模型回答已写入对话历史");
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="ASSISTANT / MCP HOST"
        title="AI 助手"
        description="通过受控本地工具查询账号、内容与指标，创建四类内容草稿和发布任务；写入与外部发布均有可见授权边界。"
        actions={
          <div className="flex items-center gap-2">
            <button
              onClick={() => navigate("/settings")}
              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-accent-border bg-accent-soft px-3 text-xs font-medium text-accent-deep"
            >
              <Sparkles size={12} />
              配置模型 Key
            </button>
            <button
              onClick={() => void mcp.healthCheck()}
              disabled={Boolean(mcp.busyAction)}
              className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs text-ink-secondary hover:text-ink disabled:opacity-50"
            >
              <ShieldCheck size={12} />
              健康检查
            </button>
          </div>
        }
      />

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-4 overflow-hidden px-6 py-5 xl:grid-cols-[210px_minmax(0,1fr)_340px]">
        <aside className="flex min-h-0 flex-col rounded-2xl border border-line bg-surface p-3">
          <button
            onClick={() => void mcp.createThread()}
            className="flex h-8 cursor-pointer items-center justify-center gap-1.5 rounded-xl border border-accent-border bg-accent-soft text-xs font-medium text-accent-deep"
          >
            <Bot size={12} />
            新建对话
          </button>
          <div className="mt-3 space-y-1 overflow-y-auto">
            {mcp.threads.map((thread) => (
              <button
                key={thread.id}
                onClick={() => void mcp.selectThread(thread.id)}
                className={`w-full cursor-pointer rounded-xl px-2.5 py-2 text-left transition-colors ${
                  mcp.activeThread?.thread.id === thread.id
                    ? "bg-accent-soft text-accent-deep"
                    : "text-ink-secondary hover:bg-hover"
                }`}
              >
                <span className="line-clamp-2 text-[11.5px] font-medium">{thread.title}</span>
                <span className="mt-1 block truncate text-[9.5px] text-ink-muted">
                  {thread.providerName} · {thread.model}
                </span>
              </button>
            ))}
          </div>
        </aside>

        <section className="flex min-h-0 flex-col rounded-2xl border border-line bg-surface">
          <header className="flex items-center gap-3 border-b border-line px-4 py-3">
            <div className="grid h-8 w-8 place-items-center rounded-xl bg-accent-soft text-accent">
              <Bot size={15} />
            </div>
            <div className="min-w-0">
              <h2 className="truncate text-[13px] font-semibold text-ink">
                {mcp.activeThread?.thread.title ?? "AI 助手"}
              </h2>
              <p className="truncate text-[10px] text-ink-muted">
                {mcp.activeThread?.thread.providerName ?? "本地编排器"} ·{" "}
                {mcp.activeThread?.thread.model ?? "deterministic-v1"}
              </p>
            </div>
            {mcp.busyAction === "execute" && (
              <button
                onClick={() => void mcp.cancelCall()}
                className="ml-auto inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg border border-coral-text/30 bg-coral-bg px-2.5 text-[10.5px] text-coral-text"
              >
                <X size={10} />
                取消当前调用
              </button>
            )}
          </header>
          <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-4 py-4">
            {mcp.loading && activeMessages.length === 0 && (
              <div className="grid h-32 place-items-center text-xs text-ink-muted">
                正在连接本地 MCP Host…
              </div>
            )}
            {activeMessages.map((message) => (
              <MessageCard key={message.id} message={message} onPreview={previewMessage} />
            ))}
            {!mcp.loading && activeMessages.length === 0 && (
              <div className="grid h-40 place-items-center rounded-2xl border border-dashed border-line bg-canvas/60 text-center text-xs text-ink-muted">
                输入自然语言任务，助手会先给出工具计划，再进入预览与授权。
              </div>
            )}
          </div>
          <div className="border-t border-line px-4 py-3">
            <div className="mb-2 flex flex-wrap gap-1.5">
              {quickPrompts.map((prompt) => (
                <button
                  key={prompt}
                  onClick={() => setInput(prompt)}
                  className="cursor-pointer rounded-lg border border-line bg-canvas px-2 py-1 text-[10px] text-ink-secondary hover:border-accent-border hover:text-accent-deep"
                >
                  {prompt}
                </button>
              ))}
            </div>
            <form
              className="flex gap-2"
              onSubmit={(event) => {
                event.preventDefault();
                void send();
              }}
            >
              <textarea
                value={input}
                onChange={(event) => setInput(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && !event.shiftKey) {
                    event.preventDefault();
                    void send();
                  }
                }}
                rows={2}
                placeholder="例如：查询所有账号状态、检索本地草稿，或运行 MCP 超时自检…"
                className="min-h-12 flex-1 resize-none rounded-xl border border-line bg-canvas px-3 py-2 text-xs text-ink outline-none focus:border-accent"
              />
              <button
                type="submit"
                disabled={Boolean(mcp.busyAction) || !input.trim()}
                className="inline-flex w-12 cursor-pointer items-center justify-center rounded-xl bg-accent text-accent-on disabled:opacity-50"
                aria-label="发送消息"
              >
                <Send size={14} />
              </button>
            </form>
          </div>
        </section>

        <aside className="flex min-h-0 flex-col rounded-2xl border border-line bg-surface">
          <div className="grid grid-cols-3 gap-1 border-b border-line p-2">
            {[
              ["mcp", "MCP Host"],
              ["privacy", "模型与隐私"],
              ["audit", "调用审计"],
            ].map(([id, label]) => (
              <button
                key={id}
                onClick={() => setPanel(id as typeof panel)}
                className={`cursor-pointer rounded-lg px-1.5 py-1.5 text-[10px] font-medium ${
                  panel === id ? "bg-accent-soft text-accent-deep" : "text-ink-muted hover:bg-hover"
                }`}
              >
                {label}
              </button>
            ))}
          </div>

          <div className="min-h-0 flex-1 overflow-y-auto p-3">
            {panel === "mcp" && (
              <div className="space-y-3">
                <section className="rounded-xl border border-line bg-canvas p-3">
                  <div className="flex items-center gap-2">
                    <span
                      className={`h-2 w-2 rounded-full ${
                        mcp.status?.state === "healthy" ? "bg-mint-text" : "bg-coral-text"
                      }`}
                    />
                    <strong className="text-[11.5px] text-ink">
                      {mcp.status?.state === "healthy" ? "MCP Host 健康" : "MCP Host 未就绪"}
                    </strong>
                    <span className="ml-auto text-[9px] text-ink-muted">
                      PID {mcp.status?.pid ?? "—"}
                    </span>
                  </div>
                  <dl className="mt-2 grid grid-cols-2 gap-1 text-[9.5px] text-ink-secondary">
                    <dt>Server</dt>
                    <dd className="truncate text-right">{mcp.status?.serverName ?? "—"}</dd>
                    <dt>协议</dt>
                    <dd className="text-right">{mcp.status?.protocolVersion ?? "—"}</dd>
                    <dt>重启</dt>
                    <dd className="text-right">{mcp.status?.restartCount ?? 0} 次</dd>
                    <dt>工具</dt>
                    <dd className="text-right">{mcp.tools.length} 个</dd>
                  </dl>
                  {mcp.status?.lastError && (
                    <p className="mt-2 rounded-lg bg-coral-bg px-2 py-1.5 text-[9.5px] text-coral-text">
                      {mcp.status.lastError}
                    </p>
                  )}
                  <div className="mt-2 grid grid-cols-2 gap-1.5">
                    <button
                      onClick={() => void mcp.startServer()}
                      disabled={Boolean(mcp.busyAction)}
                      className="cursor-pointer rounded-lg border border-line bg-surface px-2 py-1 text-[9.5px] text-ink-secondary disabled:opacity-50"
                    >
                      启动 / 重启
                    </button>
                    <button
                      onClick={() => void mcp.stopServer()}
                      disabled={Boolean(mcp.busyAction)}
                      className="cursor-pointer rounded-lg border border-line bg-surface px-2 py-1 text-[9.5px] text-ink-secondary disabled:opacity-50"
                    >
                      停止 Server
                    </button>
                  </div>
                </section>

                <section className="rounded-xl border border-line bg-canvas p-3">
                  <h3 className="flex items-center gap-1.5 text-[11px] font-semibold text-ink">
                    <ShieldCheck size={11} className="text-accent" />
                    权限策略
                  </h3>
                  {settingsDraft && (
                    <div className="mt-2 space-y-2 text-[10px] text-ink-secondary">
                      <label className="flex items-center justify-between gap-2">
                        允许只读工具
                        <input
                          type="checkbox"
                          checked={settingsDraft.readEnabled}
                          onChange={(event) =>
                            setSettingsDraft({
                              ...settingsDraft,
                              readEnabled: event.target.checked,
                            })
                          }
                        />
                      </label>
                      <label className="flex items-center justify-between gap-2">
                        自动重启
                        <input
                          type="checkbox"
                          checked={settingsDraft.autoRestart}
                          onChange={(event) =>
                            setSettingsDraft({
                              ...settingsDraft,
                              autoRestart: event.target.checked,
                            })
                          }
                        />
                      </label>
                      <label className="block">
                        写操作授权
                        <select
                          value={settingsDraft.writePolicy}
                          onChange={(event) =>
                            setSettingsDraft({
                              ...settingsDraft,
                              writePolicy: event.target.value as "confirm" | "allow",
                            })
                          }
                          className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5"
                        >
                          <option value="confirm">每次确认</option>
                          <option value="allow">允许已授权写入</option>
                        </select>
                      </label>
                      <p className="rounded-lg bg-surface px-2 py-1.5 text-[9px] text-ink-muted">
                        外部发布始终每次确认；该策略不可关闭。
                      </p>
                      <button
                        onClick={() => void mcp.saveSettings(settingsDraft)}
                        disabled={Boolean(mcp.busyAction)}
                        className="w-full cursor-pointer rounded-lg bg-accent px-2 py-1.5 text-[10px] font-medium text-accent-on disabled:opacity-50"
                      >
                        保存策略
                      </button>
                    </div>
                  )}
                </section>

                <section>
                  <h3 className="mb-2 text-[11px] font-semibold text-ink">已发现工具</h3>
                  <div className="space-y-1.5">
                    {(["read", "write", "externalAction"] as const).map((permission) =>
                      groupedTools[permission].map((tool) => (
                        <div
                          key={tool.name}
                          className="rounded-lg border border-line bg-canvas p-2"
                        >
                          <div className="flex items-center gap-1.5">
                            <code className="min-w-0 flex-1 truncate font-mono text-[9.5px] text-ink">
                              {tool.name}
                            </code>
                            <span
                              className={`rounded px-1 py-0.5 text-[8.5px] ${permissionMeta[permission].className}`}
                            >
                              {permissionMeta[permission].label}
                            </span>
                          </div>
                          <p className="mt-1 line-clamp-2 text-[9px] leading-relaxed text-ink-muted">
                            {tool.description}
                          </p>
                        </div>
                      )),
                    )}
                    {mcp.tools.length === 0 && (
                      <p className="rounded-lg border border-dashed border-line p-3 text-center text-[10px] text-ink-muted">
                        启动 MCP Server 后显示工具目录。
                      </p>
                    )}
                  </div>
                </section>

                <button
                  onClick={() => void mcp.previewToolCall("mcp_test_hang", { milliseconds: 5000 })}
                  className="flex w-full cursor-pointer items-center justify-center gap-1.5 rounded-lg border border-line bg-canvas px-2 py-1.5 text-[9.5px] text-ink-secondary"
                >
                  <Clock3 size={10} />
                  运行超时 / 取消自检
                </button>
              </div>
            )}

            {panel === "privacy" && (
              <div className="space-y-3">
                <section className="rounded-xl border border-line bg-canvas p-3">
                  <h3 className="text-[11px] font-semibold text-ink">发送前披露</h3>
                  <p className="mt-1 text-[9.5px] leading-relaxed text-ink-muted">
                    工具调用不经过云端模型。只有点击下方确认按钮时，选定范围才会发送给模型服务商。
                  </p>
                  <label className="mt-2 block text-[9.5px] text-ink-secondary">
                    模型服务商
                    <select
                      value={providerId}
                      onChange={(event) => setProviderId(event.target.value)}
                      className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 text-[10px]"
                    >
                      <option value="">本地编排器</option>
                      {providers.map((provider) => (
                        <option key={provider.id} value={provider.id}>
                          {provider.name} · {provider.model}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="mt-2 block text-[9.5px] text-ink-secondary">
                    内容来源
                    <select
                      value={contentId}
                      onChange={(event) => setContentId(event.target.value)}
                      className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 text-[10px]"
                    >
                      <option value="">不发送内容包</option>
                      {packages.map((item) => (
                        <option key={item.id} value={item.id}>
                          {item.title} · {item.kind}
                        </option>
                      ))}
                    </select>
                  </label>
                  <div className="mt-2 grid grid-cols-2 gap-1.5 text-[9.5px]">
                    {[
                      ["标题", "includeTitle"],
                      ["摘要与标签", "includeSummary"],
                      ["正文", "includeBody"],
                      ["素材元数据", "includeMediaMetadata"],
                    ].map(([label, key]) => (
                      <label
                        key={key}
                        className="flex items-center gap-1.5 rounded-lg border border-line bg-surface px-2 py-1.5 text-ink-secondary"
                      >
                        <input
                          type="checkbox"
                          checked={Boolean(scope[key as keyof ModelContextScope])}
                          onChange={(event) => setScope({ ...scope, [key]: event.target.checked })}
                        />
                        {label}
                      </label>
                    ))}
                  </div>
                </section>

                <section className="rounded-xl border border-line bg-canvas p-3">
                  <div className="flex items-center justify-between gap-2">
                    <h3 className="text-[11px] font-semibold text-ink">实际发送上下文</h3>
                    {mcp.contextPreview && (
                      <span className="rounded bg-surface px-1.5 py-0.5 text-[8.5px] text-ink-muted">
                        {mcp.contextPreview.characterCount} 字
                      </span>
                    )}
                  </div>
                  {mcp.contextPreview ? (
                    <>
                      <p className="mt-1 text-[9px] text-ink-muted">
                        服务商 {mcp.contextPreview.providerName} · 模型 {mcp.contextPreview.model}
                      </p>
                      <div className="mt-2 flex flex-wrap gap-1">
                        {mcp.contextPreview.includedFields.map((field) => (
                          <span
                            key={field}
                            className="rounded bg-mint-bg px-1.5 py-0.5 text-[8.5px] text-mint-text"
                          >
                            包含 {field}
                          </span>
                        ))}
                        {mcp.contextPreview.excludedFields.map((field) => (
                          <span
                            key={field}
                            className="rounded bg-sidebar px-1.5 py-0.5 text-[8.5px] text-ink-muted"
                          >
                            排除 {field}
                          </span>
                        ))}
                      </div>
                      <pre className="mt-2 max-h-56 overflow-auto whitespace-pre-wrap rounded-lg border border-line bg-surface p-2 font-mono text-[9px] leading-relaxed text-ink-secondary">
                        {mcp.contextPreview.context || "未选择内容包；仅发送用户请求。"}
                      </pre>
                      <button
                        onClick={() => void generateReply()}
                        disabled={Boolean(mcp.busyAction)}
                        className="mt-2 flex w-full cursor-pointer items-center justify-center gap-1.5 rounded-lg bg-accent px-2 py-1.5 text-[10px] font-medium text-accent-on disabled:opacity-50"
                      >
                        <Send size={10} />
                        确认并发送到模型
                      </button>
                    </>
                  ) : (
                    <p className="mt-2 rounded-lg border border-dashed border-line p-3 text-center text-[9.5px] text-ink-muted">
                      选择内容与范围后显示实际发送载荷。
                    </p>
                  )}
                </section>
              </div>
            )}

            {panel === "audit" && (
              <div className="space-y-2">
                <div className="rounded-xl border border-line bg-canvas p-3">
                  <h3 className="text-[11px] font-semibold text-ink">调用日志</h3>
                  <p className="mt-1 text-[9px] leading-relaxed text-ink-muted">
                    参数仅保存脱敏摘要；正文以字符数代替，不保存 API Key、Cookie、Token 或凭据引用。
                  </p>
                </div>
                {mcp.audit.map((record) => (
                  <div key={record.id} className="rounded-lg border border-line bg-canvas p-2.5">
                    <div className="flex items-center gap-1.5">
                      <code className="min-w-0 flex-1 truncate font-mono text-[9.5px] text-ink">
                        {record.toolName}
                      </code>
                      <span
                        className={`rounded px-1 py-0.5 text-[8px] ${statusMeta[record.status] ?? "bg-sidebar text-ink-secondary"}`}
                      >
                        {record.status}
                      </span>
                    </div>
                    <p className="mt-1 text-[9px] text-ink-muted">
                      {timeLabel(record.startedAt)} · {record.targetSummary}
                    </p>
                    <p className="mt-1 line-clamp-3 break-words font-mono text-[8.5px] text-ink-secondary">
                      {record.parameterSummary || "无参数摘要"}
                    </p>
                    {record.errorMessage && (
                      <p className="mt-1 text-[8.5px] text-coral-text">{record.errorMessage}</p>
                    )}
                  </div>
                ))}
                {mcp.audit.length === 0 && (
                  <p className="rounded-lg border border-dashed border-line p-4 text-center text-[10px] text-ink-muted">
                    暂无工具调用记录。
                  </p>
                )}
              </div>
            )}
          </div>
        </aside>
      </div>

      {mcp.activePreview && (
        <div className="fixed inset-0 z-50 grid place-items-center bg-ink/35 p-5 backdrop-blur-[2px]">
          <section className="max-h-[86vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-line bg-surface p-5 shadow-2xl">
            <div className="flex items-start gap-3">
              <div className="grid h-9 w-9 place-items-center rounded-xl bg-accent-soft text-accent">
                <ShieldCheck size={17} />
              </div>
              <div className="min-w-0 flex-1">
                <h2 className="text-[15px] font-semibold text-ink">{mcp.activePreview.title}</h2>
                <p className="mt-1 text-[10.5px] text-ink-secondary">
                  工具 <code className="font-mono">{mcp.activePreview.toolName}</code> · 权限{" "}
                  {permissionMeta[mcp.activePreview.permission].label}
                </p>
              </div>
              <button
                onClick={mcp.clearPreview}
                className="cursor-pointer text-ink-muted hover:text-ink"
                aria-label="关闭预览"
              >
                <X size={16} />
              </button>
            </div>

            <div className="mt-4 grid gap-2 sm:grid-cols-2">
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[9.5px] text-ink-muted">目标</span>
                <p className="mt-1 text-[11px] font-medium text-ink">
                  {mcp.activePreview.targetSummary}
                </p>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="text-[9.5px] text-ink-muted">内容形态</span>
                <p className="mt-1 text-[11px] font-medium text-ink">
                  {mcp.activePreview.contentKind ?? "不适用"}
                </p>
              </div>
            </div>

            <div className="mt-2 rounded-xl border border-line bg-canvas p-3">
              <span className="text-[9.5px] text-ink-muted">脱敏参数摘要</span>
              <p className="mt-1 break-words font-mono text-[10px] text-ink-secondary">
                {mcp.activePreview.parameterSummary || "无参数"}
              </p>
            </div>

            <div className="mt-2 rounded-xl border border-line bg-canvas p-3">
              <span className="text-[9.5px] text-ink-muted">预期副作用</span>
              <ul className="mt-1 space-y-1 text-[10.5px] text-ink-secondary">
                {mcp.activePreview.effects.map((effect) => (
                  <li key={effect} className="flex gap-1.5">
                    <span className="text-accent">•</span>
                    {effect}
                  </li>
                ))}
              </ul>
            </div>

            <details className="mt-2 rounded-xl border border-line bg-canvas p-3" open>
              <summary className="cursor-pointer text-[9.5px] font-medium text-ink">
                最终目标与内容预览
              </summary>
              <pre className="mt-2 max-h-60 overflow-auto whitespace-pre-wrap rounded-lg bg-surface p-2 font-mono text-[9px] leading-relaxed text-ink-secondary">
                {pretty(mcp.activePreview.details)}
              </pre>
            </details>

            {mcp.activePreview.fieldErrors.length > 0 && (
              <div className="mt-2 rounded-xl border border-coral-text/20 bg-coral-bg p-3">
                <p className="flex items-center gap-1.5 text-[10.5px] font-medium text-coral-text">
                  <AlertTriangle size={11} />
                  载荷校验未通过，执行按钮已禁用
                </p>
                <ul className="mt-1 space-y-1 text-[9.5px] text-coral-text">
                  {mcp.activePreview.fieldErrors.map((error) => (
                    <li key={`${error.path}-${error.message}`}>
                      <code>{error.path || "root"}</code>：{error.message}
                    </li>
                  ))}
                </ul>
              </div>
            )}

            <div className="mt-4 flex items-center gap-2">
              <p className="flex flex-1 items-center gap-1.5 text-[9.5px] text-ink-muted">
                <Info size={10} />
                {mcp.activePreview.requiresConfirmation
                  ? "确认令牌 10 分钟内有效且仅可使用一次。"
                  : "本次操作不需要二次确认，但仍会记录审计。"}
              </p>
              <button
                onClick={mcp.clearPreview}
                className="cursor-pointer rounded-xl border border-line px-3 py-2 text-[11px] text-ink-secondary"
              >
                取消
              </button>
              <button
                onClick={() => void mcp.confirmPreview()}
                disabled={mcp.activePreview.fieldErrors.length > 0 || Boolean(mcp.busyAction)}
                className="inline-flex cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 py-2 text-[11px] font-medium text-accent-on disabled:opacity-50"
              >
                <Check size={11} />
                确认并执行
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
