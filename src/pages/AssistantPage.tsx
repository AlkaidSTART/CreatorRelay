import { useState } from "react";
import { Bot, Send, ShieldCheck, Sparkles, Terminal } from "lucide-react";
import { useNavigate, useOutletContext } from "react-router";
import { PageHeader } from "../components/PageHeader";
import type { AppOutletContext } from "../layouts/AppLayout";

interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolCall?: {
    name: string;
    permission: "Read" | "Write";
    summary: string;
  };
}

const quickPrompts = [
  "查询掘金与 Bilibili 最近 7 天的核心指标表现",
  "把当前主稿适配为掘金和 CSDN 两个平台版本",
  "检查所有已绑定账号的登录会话状态",
  "为当前草稿生成 3 个更具吸引力的技术标题",
];

const initialMessages: ChatMessage[] = [
  {
    id: "msg-1",
    role: "assistant",
    content:
      "你好，我是 CreatorRelay AI 助手。我可以在你的授权下调用本地 Studio 工具与平台 MCP 工具，帮助你查询跨平台数据、适配平台稿或发起发布准备。写操作与发布操作在执行前都会向你明确确认。",
  },
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
  const { showToast } = useOutletContext<AppOutletContext>();
  const navigate = useNavigate();
  const [input, setInput] = useState("");
  const [messages, setMessages] = useState<ChatMessage[]>(initialMessages);

  function sendPrompt(text: string) {
    const trimmed = text.trim();
    if (!trimmed) return;
    const userMsg: ChatMessage = {
      id: `u-${Date.now()}`,
      role: "user",
      content: trimmed,
    };
    const replyMsg: ChatMessage = {
      id: `a-${Date.now() + 1}`,
      role: "assistant",
      content:
        "当前为路由视图演示模式，尚未绑定 BYOK API Key。请前往「系统设置 → AI 模型供应商」配置 OpenAI / Anthropic / Gemini / DeepSeek 或 OpenAI Compatible 端点后启用实时 Agent 执行。",
      toolCall: {
        name: trimmed.includes("适配")
          ? "studio_adapt_content"
          : trimmed.includes("登录")
            ? "account_get_info"
            : "analytics_compare_platforms",
        permission: trimmed.includes("适配") ? "Write" : "Read",
        summary: "本地沙箱演练 · 未触发外部网络调用",
      },
    };
    setMessages((prev) => [...prev, userMsg, replyMsg]);
    setInput("");
  }

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="ASSISTANT"
        title="AI 助手"
        description="通过 BYOK 模型与工具注册中心调用 CreatorRelay 内部工具和平台 MCP 工具。"
        priority="P1"
        actions={
          <button className="secondary-button" onClick={() => navigate("/settings")}>
            <Sparkles size={13} />
            配置 BYOK 模型
          </button>
        }
      />

      <div className="page-body assistant-grid">
        <section className="surface-card assistant-chat">
          <div className="assistant-messages">
            {messages.map((message) => (
              <div className={`chat-bubble ${message.role}`} key={message.id}>
                <div className="chat-role">
                  {message.role === "assistant" ? (
                    <>
                      <Bot size={13} /> CreatorRelay Assistant
                    </>
                  ) : (
                    "创作者"
                  )}
                </div>
                <p>{message.content}</p>
                {message.toolCall && (
                  <div className="tool-trace">
                    <Terminal size={12} />
                    <code>{message.toolCall.name}</code>
                    <span className="tag-chip">{message.toolCall.permission}</span>
                    <span>{message.toolCall.summary}</span>
                  </div>
                )}
              </div>
            ))}
          </div>

          <div className="quick-prompts">
            {quickPrompts.map((prompt) => (
              <button key={prompt} className="prompt-chip" onClick={() => sendPrompt(prompt)}>
                {prompt}
              </button>
            ))}
          </div>

          <form
            className="chat-composer"
            onSubmit={(event) => {
              event.preventDefault();
              sendPrompt(input);
            }}
          >
            <input
              value={input}
              onChange={(event) => setInput(event.target.value)}
              placeholder="输入指令，例如：对比掘金与知乎最近 7 天的互动表现…"
              aria-label="向 AI 助手发送指令"
            />
            <button type="submit" className="primary-button" disabled={!input.trim()}>
              <Send size={13} />
              发送
            </button>
          </form>
        </section>

        <aside className="surface-card assistant-sidebar">
          <div className="card-heading">
            <div>
              <span className="context-eyebrow">TOOL REGISTRY</span>
              <h2>已注册工具与权限</h2>
            </div>
          </div>
          <div className="stack-list">
            {internalTools.map((tool) => (
              <div className="tool-item" key={tool.name}>
                <div className="tool-item-top">
                  <code>{tool.name}</code>
                  <span className="tag-chip">{tool.level}</span>
                </div>
                <small>{tool.desc}</small>
              </div>
            ))}
          </div>
          <div className="callout-bar compact">
            <ShieldCheck size={14} />
            <span>未经用户明确确认，AI 不会执行自动发布、删除或公开资料修改。</span>
          </div>
          <button
            className="secondary-button full-width"
            onClick={() => showToast("MCP Server 生命周期管理可在系统设置中配置")}
          >
            查看 MCP 运行状态
          </button>
        </aside>
      </div>
    </main>
  );
}
