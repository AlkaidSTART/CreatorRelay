import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import {
  demoPlatformAccounts,
  demoPublishedContents,
  demoPublishTasks,
  type PlatformAccountItem,
  type PublishedContentItem,
  type PublishTaskItem,
  type PublishTaskStatus,
} from "../data/demoData";
import { useUiStore } from "./uiStore";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  toolCall?: {
    name: string;
    permission: "Read" | "Write";
    summary: string;
  };
}

export type AnalyticsRange = "7d" | "30d" | "90d";

const initialAssistantMessages: ChatMessage[] = [
  {
    id: "msg-1",
    role: "assistant",
    content:
      "你好，我是 CreatorRelay AI 助手。我可以在你的授权下调用本地 Studio 工具与平台 MCP 工具，帮助你查询跨平台数据、适配平台稿或发起发布准备。写操作与发布操作在执行前都会向你明确确认。",
  },
];

export interface ContentStore {
  publishTasks: PublishTaskItem[];
  publishedContents: PublishedContentItem[];
  platformAccounts: PlatformAccountItem[];
  publishingFilter: "ALL" | PublishTaskStatus;
  contentKeyword: string;
  contentPlatformFilter: string;
  analyticsRange: AnalyticsRange;
  assistantMessages: ChatMessage[];
  setPublishingFilter: (filter: "ALL" | PublishTaskStatus) => void;
  setContentKeyword: (keyword: string) => void;
  setContentPlatformFilter: (platform: string) => void;
  setAnalyticsRange: (range: AnalyticsRange) => void;
  sendAssistantPrompt: (text: string) => void;
  checkPlatformAccountStatus: (accountId: string) => void;
}

const createContentStoreState: StateCreator<ContentStore> = (set, get) => ({
  publishTasks: demoPublishTasks,
  publishedContents: demoPublishedContents,
  platformAccounts: demoPlatformAccounts,
  publishingFilter: "ALL",
  contentKeyword: "",
  contentPlatformFilter: "全部平台",
  analyticsRange: "7d",
  assistantMessages: initialAssistantMessages,

  setPublishingFilter: (publishingFilter) => set({ publishingFilter }),
  setContentKeyword: (contentKeyword) => set({ contentKeyword }),
  setContentPlatformFilter: (contentPlatformFilter) => set({ contentPlatformFilter }),
  setAnalyticsRange: (analyticsRange) => set({ analyticsRange }),

  sendAssistantPrompt: (text) => {
    const trimmed = text.trim();
    if (!trimmed) return;
    const now = Date.now();
    const userMsg: ChatMessage = {
      id: `u-${now}`,
      role: "user",
      content: trimmed,
    };
    const replyMsg: ChatMessage = {
      id: `a-${now + 1}`,
      role: "assistant",
      content:
        "当前为本地演示模式，尚未绑定 BYOK API Key。请前往「系统设置 → AI 模型供应商」配置 OpenAI / Anthropic / Gemini / DeepSeek 或 OpenAI Compatible 端点后启用实时 Agent 执行。",
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
    set((state) => ({
      assistantMessages: [...state.assistantMessages, userMsg, replyMsg],
    }));
  },

  checkPlatformAccountStatus: (accountId) => {
    const target = get().platformAccounts.find((item) => item.id === accountId);
    if (!target) return;
    set((state) => ({
      platformAccounts: state.platformAccounts.map((item) =>
        item.id === accountId ? { ...item, lastSyncedAt: "刚刚" } : item,
      ),
    }));
    useUiStore
      .getState()
      .showToast(`已检测 ${target.platformName} (${target.alias}) 会话状态`);
  },
});

export const useContentStore = create<ContentStore>()(createContentStoreState);

/** Creates an isolated vanilla content store for unit tests. */
export const createContentStore = () => createStore<ContentStore>()(createContentStoreState);
