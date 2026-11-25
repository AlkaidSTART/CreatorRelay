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
    <main className="flex h-full min-w-0 flex-col bg-[#FAFAFB]">
      <PageHeader
        eyebrow="ASSISTANT"
        title="AI 助手"
        description="用自然语言指挥创作工作流：改写标题、生成多平台适配稿、查询跨平台表现，所有发布前都会先征求你确认。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-black/[0.1] bg-white px-3 text-xs font-medium text-[#1D1D1F] hover:bg-[#F5F5F7]"
            onClick={() => navigate("/settings")}
          >
            <Sparkles size={12} className="text-[#2563EB]" />
            配置自带模型 Key
          </button>
        }
      />

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-5 overflow-y-auto px-7 py-6 xl:grid-cols-[minmax(0,1fr)_304px]">
        <section className="flex min-h-[480px] flex-col rounded-xl border border-black/[0.07] bg-white p-5">
          <div className="flex-1 space-y-3 overflow-y-auto pb-4">
            {messages.map((message) => (
              <div
                key={message.id}
                className={`rounded-xl border px-4 py-3 ${
                  message.role === "user"
                    ? "border-[#2563EB] bg-[#2563EB] text-white"
                    : "border-black/[0.06] bg-[#FAFAFB] text-[#1D1D1F]"
                }`}
              >
                <div
                  className={`mb-1.5 flex items-center gap-1.5 text-[11px] font-medium ${
                    message.role === "user" ? "text-white/80" : "text-[#6E6E73]"
                  }`}
                >
                  {message.role === "assistant" ? (
                    <>
                      <Bot size={12} /> CreatorRelay Assistant
                    </>
                  ) : (
                    "创作者"
                  )}
                </div>
                <p className="text-[13px] leading-relaxed">{message.content}</p>
                {message.toolCall && (
                  <div className="mt-2.5 flex flex-wrap items-center gap-2 rounded-lg border border-black/[0.07] bg-white px-3 py-2 text-[11px] text-[#6E6E73]">
                    <Terminal size={11} className="text-[#8E8E93]" />
                    <code className="font-mono text-[#1D1D1F]">{message.toolCall.name}</code>
                    <span className="rounded bg-[#EFF6FF] px-1.5 py-0.5 text-[10px] font-medium text-[#2563EB]">
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
                className="cursor-pointer rounded-lg border border-black/[0.08] bg-[#FAFAFB] px-2.5 py-1.5 text-xs text-[#58585D] transition-colors hover:border-[#2563EB]/40 hover:bg-[#EFF6FF] hover:text-[#2563EB]"
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
              className="h-9 flex-1 rounded-lg border border-black/[0.1] bg-white px-3 text-xs text-[#1D1D1F] placeholder:text-[#AEAEB2] focus:border-[#2563EB] focus:outline-none focus:ring-2 focus:ring-[#2563EB]/15"
            />
            <button
              type="submit"
              disabled={!input.trim()}
              className="inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-lg bg-[#2563EB] px-3.5 text-xs font-medium text-white transition-colors hover:bg-[#1D4ED8] disabled:cursor-not-allowed disabled:bg-[#C7C7CC]"
            >
              <Send size={12} />
              发送
            </button>
          </form>
        </section>

        <aside className="flex flex-col rounded-xl border border-black/[0.07] bg-white p-5">
          <span className="text-[10px] font-semibold tracking-widest text-[#8E8E93] uppercase">
            TOOL REGISTRY
          </span>
          <h2 className="mt-0.5 mb-4 text-sm font-semibold text-[#1D1D1F]">可用创作工具</h2>
          <div className="space-y-2.5">
            {internalTools.map((tool) => (
              <div
                key={tool.name}
                className="rounded-lg border border-black/[0.05] bg-[#FAFAFB] px-3 py-2.5"
              >
                <div className="mb-1 flex items-center justify-between gap-2">
                  <code className="font-mono text-[11px] font-medium text-[#1D1D1F]">
                    {tool.name}
                  </code>
                  <span className="rounded bg-white px-1.5 py-0.5 text-[10px] font-medium text-[#6E6E73] ring-1 ring-black/[0.07]">
                    {tool.level}
                  </span>
                </div>
                <small className="text-[11px] text-[#6E6E73]">{tool.desc}</small>
              </div>
            ))}
          </div>
          <div className="mt-4 flex items-start gap-2 rounded-lg border border-black/[0.06] bg-[#FAFAFB] px-3 py-2.5 text-[11px] leading-relaxed text-[#6E6E73]">
            <ShieldCheck size={13} className="mt-0.5 shrink-0 text-[#1D1D1F]" />
            <span>未经你明确确认，AI 绝不会执行自动发布、删除或修改公开资料。</span>
          </div>
          <button
            className="mt-3 h-8 w-full cursor-pointer rounded-lg border border-black/[0.1] bg-white text-xs font-medium text-[#1D1D1F] hover:bg-[#F5F5F7]"
            onClick={() => showToast("MCP Server 运行状态正常，可在系统设置中管理")}
          >
            查看 MCP 运行状态
          </button>
        </aside>
      </div>
    </main>
  );
}
