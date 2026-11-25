import { useState } from "react";
import { Bot, Send, ShieldCheck, Sparkles, Terminal } from "lucide-react";
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
  { name: "studio_search_articles", level: "Read", desc: "检索本地工作区主稿与平台稿" },
  { name: "studio_adapt_content", level: "Write", desc: "基于主稿生成目标平台适配稿" },
  {
    name: "publishing_create_task",
    level: "Confirm",
    desc: "创建待确认发布任务（不自动直接发布）",
  },
  { name: "analytics_compare_platforms", level: "Read", desc: "按独立口径对比多平台近期指标" },
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
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="ASSISTANT"
        title="AI 助手"
        description="通过 BYOK 模型与工具注册中心调用 CreatorRelay 内部工具和平台 MCP 工具。"
        priority="P1"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-3 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
            onClick={() => navigate("/settings")}
          >
            <Sparkles size={13} />
            配置 BYOK 模型
          </button>
        }
      />

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-5 overflow-y-auto px-7 py-6 xl:grid-cols-[minmax(0,1fr)_310px]">
        <section className="flex min-h-[480px] flex-col rounded-xl border border-zinc-200/80 bg-white p-5">
          <div className="flex-1 space-y-3 overflow-y-auto pb-4">
            {messages.map((message) => (
              <div
                key={message.id}
                className={`rounded-xl border px-4 py-3 ${
                  message.role === "user"
                    ? "border-zinc-900 bg-zinc-900 text-white"
                    : "border-zinc-200/80 bg-zinc-50/70 text-zinc-800"
                }`}
              >
                <div
                  className={`mb-1.5 flex items-center gap-1.5 text-[11px] font-medium ${
                    message.role === "user" ? "text-zinc-300" : "text-zinc-500"
                  }`}
                >
                  {message.role === "assistant" ? (
                    <>
                      <Bot size={13} /> CreatorRelay Assistant
                    </>
                  ) : (
                    "创作者"
                  )}
                </div>
                <p className="text-[13px] leading-relaxed">{message.content}</p>
                {message.toolCall && (
                  <div className="mt-2.5 flex flex-wrap items-center gap-2 rounded-lg border border-zinc-200/80 bg-white px-3 py-2 text-[11px] text-zinc-500">
                    <Terminal size={12} className="text-zinc-400" />
                    <code className="font-mono text-zinc-800">{message.toolCall.name}</code>
                    <span className="rounded bg-zinc-100 px-1.5 py-0.5 text-[10px] font-medium text-zinc-600">
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
                className="cursor-pointer rounded-lg border border-zinc-200 bg-white px-2.5 py-1.5 text-xs text-zinc-600 transition-colors hover:border-zinc-300 hover:bg-zinc-50 hover:text-zinc-900"
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
              className="h-9 flex-1 rounded-lg border border-zinc-200 bg-white px-3 text-xs text-zinc-900 placeholder:text-zinc-400 focus:border-zinc-900 focus:outline-none"
            />
            <button
              type="submit"
              disabled={!input.trim()}
              className="inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3.5 text-xs font-medium text-white transition-colors hover:bg-zinc-800 disabled:cursor-not-allowed disabled:bg-zinc-300"
            >
              <Send size={13} />
              发送
            </button>
          </form>
        </section>

        <aside className="flex flex-col rounded-xl border border-zinc-200/80 bg-white p-5">
          <span className="text-[10px] font-semibold tracking-widest text-zinc-400 uppercase">
            TOOL REGISTRY
          </span>
          <h2 className="mt-0.5 mb-4 text-base font-semibold text-zinc-900">已注册工具与权限</h2>
          <div className="space-y-2.5">
            {internalTools.map((tool) => (
              <div
                key={tool.name}
                className="rounded-lg border border-zinc-100 bg-zinc-50/70 px-3 py-2.5"
              >
                <div className="mb-1 flex items-center justify-between gap-2">
                  <code className="font-mono text-[11px] font-medium text-zinc-800">
                    {tool.name}
                  </code>
                  <span className="rounded bg-white px-1.5 py-0.5 text-[10px] font-medium text-zinc-500 ring-1 ring-zinc-200">
                    {tool.level}
                  </span>
                </div>
                <small className="text-[11px] text-zinc-500">{tool.desc}</small>
              </div>
            ))}
          </div>
          <div className="mt-4 flex items-start gap-2 rounded-lg border border-zinc-200/80 bg-zinc-50 px-3 py-2.5 text-[11px] leading-relaxed text-zinc-500">
            <ShieldCheck size={14} className="mt-0.5 shrink-0 text-zinc-600" />
            <span>未经用户明确确认，AI 不会执行自动发布、删除或公开资料修改。</span>
          </div>
          <button
            className="mt-3 h-8 w-full cursor-pointer rounded-lg border border-zinc-200 bg-white text-xs font-medium text-zinc-700 hover:bg-zinc-50"
            onClick={() => showToast("MCP Server 生命周期管理可在系统设置中配置")}
          >
            查看 MCP 运行状态
          </button>
        </aside>
      </div>
    </main>
  );
}
