import { useState } from "react";
import { Bot, Send, ShieldCheck, Sparkles, Terminal } from "../components/Icons";
import { useNavigate } from "react-router";
import { PageHeader } from "../components/PageHeader";
import { useContentStore, useUiStore } from "../stores";

const quickPrompts = [
  "查询掘金与 Bilibili 最近 7 天的核心指标表现",
  "把当前主稿适配为掘金和 CSDN 两个平台版本",
  "检查所有已绑定账号的登录会话状态",
  "为当前草稿生成 3 个更具吸引力的技术标题",
];

const internalTools = [
  { name: "studio_search_articles", level: "只读", desc: "检索本地工作区主稿与平台稿" },
  { name: "studio_adapt_content", level: "写入", desc: "基于主稿生成目标平台适配稿" },
  {
    name: "publishing_create_task",
    level: "需确认",
    desc: "创建待确认发布任务（不自动直接发布）",
  },
  { name: "analytics_compare_platforms", level: "只读", desc: "按独立口径对比多平台近期指标" },
];

export function AssistantPage() {
  const navigate = useNavigate();
  const showToast = useUiStore((state) => state.showToast);
  const messages = useContentStore((state) => state.assistantMessages);
  const sendAssistantPrompt = useContentStore((state) => state.sendAssistantPrompt);
  const [input, setInput] = useState("");

  function handleSend(text: string) {
    if (!text.trim()) return;
    sendAssistantPrompt(text);
    setInput("");
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="ASSISTANT"
        title="AI 助手"
        description="用自然语言指挥创作工作流：改写标题、生成多平台适配稿、查询跨平台表现，所有发布前都会先征求你确认。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-accent-border bg-accent-soft px-3 text-xs font-medium text-accent-deep transition-colors hover:bg-accent-border"
            onClick={() => navigate("/settings")}
          >
            <Sparkles size={12} className="text-accent" />
            配置自带模型 Key
          </button>
        }
      />

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-5 overflow-y-auto px-7 py-6 xl:grid-cols-[minmax(0,1fr)_304px]">
        <section className="flex min-h-[480px] flex-col rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="flex-1 space-y-3 overflow-y-auto pb-4">
            {messages.map((message) => (
              <div
                key={message.id}
                className={`rounded-2xl border px-4 py-3 ${
                  message.role === "user"
                    ? "border-accent bg-accent text-accent-on shadow-[0_2px_8px_rgba(123,104,168,0.2)]"
                    : "border-line bg-canvas text-ink"
                }`}
              >
                <div
                  className={`mb-1.5 flex items-center gap-1.5 text-[11px] font-medium ${
                    message.role === "user" ? "text-white/85" : "text-ink-secondary"
                  }`}
                >
                  {message.role === "assistant" ? (
                    <>
                      <Bot size={12} className="text-accent" /> CreatorRelay Assistant
                    </>
                  ) : (
                    "创作者"
                  )}
                </div>
                <p className="text-[13px] leading-relaxed">{message.content}</p>
                {message.toolCall && (
                  <div className="mt-2.5 flex flex-wrap items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2 text-[11px] text-ink-secondary">
                    <Terminal size={11} className="text-ink-muted" />
                    <code className="font-mono text-ink">{message.toolCall.name}</code>
                    <span className="rounded-md bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent-deep">
                      {message.toolCall.permission}
                    </span>
                    <span>{message.toolCall.summary}</span>
                  </div>
                )}
              </div>
            ))}
          </div>

          <div className="mb-3 flex flex-wrap gap-1.5">
            {quickPrompts.map((prompt) => (
              <button
                key={prompt}
                onClick={() => handleSend(prompt)}
                className="cursor-pointer rounded-xl border border-line bg-canvas px-2.5 py-1.5 text-xs text-ink-secondary transition-colors hover:border-accent-border hover:bg-accent-soft hover:text-accent-deep"
              >
                {prompt}
              </button>
            ))}
          </div>

          <form
            className="flex gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              handleSend(input);
            }}
          >
            <input
              value={input}
              onChange={(event) => setInput(event.target.value)}
              placeholder="输入指令，例如：对比掘金与知乎最近 7 天的互动表现…"
              aria-label="向 AI 助手发送指令"
              className="h-9 flex-1 rounded-xl border border-line-strong bg-surface px-3 text-xs text-ink placeholder:text-ink-muted focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
            />
            <button
              type="submit"
              disabled={!input.trim()}
              className="inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)] transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
            >
              <Send size={12} />
              发送
            </button>
          </form>
        </section>

        <aside className="flex flex-col rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <span className="text-[10px] font-semibold tracking-widest text-ink-muted uppercase">
            TOOL REGISTRY
          </span>
          <h2 className="mt-0.5 mb-4 text-sm font-semibold text-ink">可用创作工具</h2>
          <div className="space-y-2.5">
            {internalTools.map((tool) => (
              <div
                key={tool.name}
                className="rounded-xl border border-line bg-canvas px-3 py-2.5"
              >
                <div className="mb-1 flex items-center justify-between gap-2">
                  <code className="font-mono text-[11px] font-medium text-ink">
                    {tool.name}
                  </code>
                  <span
                    className={`rounded-md px-1.5 py-0.5 text-[10px] font-medium ${
                      tool.level === "需确认"
                        ? "bg-lemon-bg text-lemon-text"
                        : tool.level === "写入"
                          ? "bg-accent-soft text-accent-deep"
                          : "bg-mint-bg text-mint-text"
                    }`}
                  >
                    {tool.level}
                  </span>
                </div>
                <small className="text-[11px] text-ink-secondary">{tool.desc}</small>
              </div>
            ))}
          </div>
          <div className="mt-4 flex items-start gap-2 rounded-xl border border-mint-border bg-mint-bg/70 px-3 py-2.5 text-[11px] leading-relaxed text-mint-text">
            <ShieldCheck size={13} className="mt-0.5 shrink-0 text-mint-text" />
            <span>未经你明确确认，AI 绝不会执行自动发布、删除或修改公开资料。</span>
          </div>
          <button
            className="mt-3 h-8 w-full cursor-pointer rounded-xl border border-line-strong bg-surface text-xs font-medium text-ink hover:bg-subtle"
            onClick={() => showToast("MCP Server 运行状态正常，可在系统设置中管理")}
          >
            查看 MCP 运行状态
          </button>
        </aside>
      </div>
    </main>
  );
}
