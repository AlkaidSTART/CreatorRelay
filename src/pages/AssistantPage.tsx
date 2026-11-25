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
    <main className="flex h-full min-w-0 flex-col bg-[#FAF8F3]">
      <PageHeader
        eyebrow="AI COPILOT"
        title="AI 助手"
        description="用自然语言指挥创作工作流：改写标题、生成多平台适配稿、查询跨平台表现，所有发布前都会先征求你确认。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-[#DCD7CB] bg-white px-3 text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
            onClick={() => navigate("/settings")}
          >
            <Sparkles size={12} className="text-[#C6923A]" />
            配置自带模型 Key
          </button>
        }
      />

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-5 overflow-y-auto px-7 py-6 xl:grid-cols-[minmax(0,1fr)_310px]">
        <section className="flex min-h-[480px] flex-col rounded-2xl border border-[#E6E2D8] bg-white p-5">
          <div className="flex-1 space-y-3 overflow-y-auto pb-4">
            {messages.map((message) => (
              <div
                key={message.id}
                className={`rounded-2xl border px-4 py-3 ${
                  message.role === "user"
                    ? "border-[#2F4A36] bg-[#2F4A36] text-white"
                    : "border-[#EAE6DC] bg-[#FAF8F3] text-[#242A24]"
                }`}
              >
                <div
                  className={`mb-1.5 flex items-center gap-1.5 text-[11px] font-medium ${
                    message.role === "user" ? "text-[#C5D6C8]" : "text-[#4C6450]"
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
                  <div className="mt-2.5 flex flex-wrap items-center gap-2 rounded-xl border border-[#E6E2D8] bg-white px-3 py-2 text-[11px] text-[#646B64]">
                    <Terminal size={11} className="text-[#4C6450]" />
                    <code className="font-mono text-[#233A29]">{message.toolCall.name}</code>
                    <span className="rounded-md bg-[#EAF0EA] px-1.5 py-0.5 text-[10px] font-medium text-[#2F4A36]">
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
                className="cursor-pointer rounded-xl border border-[#E6E2D8] bg-[#FDFCF9] px-3 py-1.5 text-xs text-[#545C54] transition-colors hover:border-[#2F4A36]/40 hover:bg-[#EAF0EA]/60 hover:text-[#233A29]"
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
              className="h-9.5 flex-1 rounded-xl border border-[#DCD7CB] bg-white px-3.5 text-xs text-[#1E241E] placeholder:text-[#9CA19A] focus:border-[#2F4A36] focus:outline-none focus:ring-2 focus:ring-[#2F4A36]/15"
            />
            <button
              type="submit"
              disabled={!input.trim()}
              className="inline-flex h-9.5 cursor-pointer items-center gap-1.5 rounded-xl bg-[#2F4A36] px-4 text-xs font-medium text-white transition-colors hover:bg-[#233A29] disabled:cursor-not-allowed disabled:bg-[#C7CEC8]"
            >
              <Send size={12} />
              发送
            </button>
          </form>
        </section>

        <aside className="flex flex-col rounded-2xl border border-[#E6E2D8] bg-white p-5">
          <span className="text-[10px] font-semibold tracking-widest text-[#4C6450] uppercase">
            TOOL REGISTRY
          </span>
          <h2 className="mt-0.5 mb-4 text-base font-semibold text-[#1E241E]">可用创作工具</h2>
          <div className="space-y-2.5">
            {internalTools.map((tool) => (
              <div
                key={tool.name}
                className="rounded-xl border border-[#EFECE4] bg-[#FCFBF8] px-3 py-2.5"
              >
                <div className="mb-1 flex items-center justify-between gap-2">
                  <code className="font-mono text-[11px] font-medium text-[#233A29]">
                    {tool.name}
                  </code>
                  <span className="rounded-md bg-[#EAF0EA] px-1.5 py-0.5 text-[10px] font-medium text-[#2F4A36]">
                    {tool.level}
                  </span>
                </div>
                <small className="text-[11px] text-[#646B64]">{tool.desc}</small>
              </div>
            ))}
          </div>
          <div className="mt-4 flex items-start gap-2 rounded-xl border border-[#E6E2D8] bg-[#FAF8F3] px-3 py-2.5 text-[11px] leading-relaxed text-[#646B64]">
            <ShieldCheck size={13} className="mt-0.5 shrink-0 text-[#2F4A36]" />
            <span>未经你明确确认，AI 绝不会执行自动发布、删除或修改公开资料。</span>
          </div>
          <button
            className="mt-3 h-8.5 w-full cursor-pointer rounded-xl border border-[#DCD7CB] bg-white text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
            onClick={() => showToast("MCP Server 运行状态正常，可在系统设置中管理")}
          >
            查看 MCP 运行状态
          </button>
        </aside>
      </div>
    </main>
  );
}
