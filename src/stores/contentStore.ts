import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import {
  demoPlatformAccounts,
  demoPublishedContents,
  demoPublishTasks,
  type PlatformAccountItem,
  type PlatformCategory,
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
  platformCategoryFilter: "ALL" | PlatformCategory;
  publishingFilter: "ALL" | PublishTaskStatus;
  contentKeyword: string;
  contentPlatformFilter: string;
  analyticsRange: AnalyticsRange;
  assistantMessages: ChatMessage[];
  setPlatformCategoryFilter: (category: "ALL" | PlatformCategory) => void;
  setPublishingFilter: (filter: "ALL" | PublishTaskStatus) => void;
  setContentKeyword: (keyword: string) => void;
  setContentPlatformFilter: (platform: string) => void;
  setAnalyticsRange: (range: AnalyticsRange) => void;
  sendAssistantPrompt: (text: string) => void;
  checkPlatformAccountStatus: (accountId: string) => void;
  reauthPlatformAccount: (accountId: string) => void;
  createPublishTaskFromDraft: (payload: {
    articleTitle: string;
    platform: string;
    platformMark: string;
    accountAlias: string;
  }) => void;
  completePublishTask: (taskId: string) => void;
  retryPublishTask: (taskId: string) => void;
}

const createContentStoreState: StateCreator<ContentStore> = (set, get) => ({
  publishTasks: demoPublishTasks,
  publishedContents: demoPublishedContents,
  platformAccounts: demoPlatformAccounts,
  platformCategoryFilter: "ALL",
  publishingFilter: "ALL",
  contentKeyword: "",
  contentPlatformFilter: "全部平台",
  analyticsRange: "7d",
  assistantMessages: initialAssistantMessages,

  setPlatformCategoryFilter: (platformCategoryFilter) => set({ platformCategoryFilter }),
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

  reauthPlatformAccount: (accountId) => {
    const target = get().platformAccounts.find((item) => item.id === accountId);
    if (!target) return;
    set((state) => ({
      platformAccounts: state.platformAccounts.map((item) =>
        item.id === accountId ? { ...item, status: "ACTIVE", lastSyncedAt: "刚刚" } : item,
      ),
    }));
    useUiStore
      .getState()
      .showToast(`${target.platformName} (${target.alias}) 已重新连接，会话恢复正常`);
  },

  createPublishTaskFromDraft: ({ articleTitle, platform, platformMark, accountAlias }) => {
    const existing = get().publishTasks.find(
      (task) => task.articleTitle === articleTitle && task.platform === platform,
    );
    if (existing) {
      useUiStore
        .getState()
        .showToast(`《${articleTitle}》已在 ${platform} 接力队列中`);
      return;
    }
    const newTask: PublishTaskItem = {
      id: `task-${Date.now()}`,
      articleTitle,
      draftType: "Platform Draft",
      platform,
      platformMark,
      accountAlias,
      status: "NEEDS_MANUAL_ACTION",
      updatedAt: "刚刚",
      note: `已从主稿自动生成 ${platform} 适配稿，确认封面与标签后即可一键发布`,
    };
    set((state) => ({
      publishTasks: [newTask, ...state.publishTasks],
    }));
    useUiStore
      .getState()
      .showToast(`已为 ${platform} 生成适配稿并加入发布接力队列`);
  },

  completePublishTask: (taskId) => {
    const target = get().publishTasks.find((task) => task.id === taskId);
    if (!target) return;
    set((state) => ({
      publishTasks: state.publishTasks.map((task) =>
        task.id === taskId
          ? {
              ...task,
              status: "SUCCESS",
              updatedAt: "刚刚",
              note: `已完成 ${task.platform} 发布确认，外部作品链接已自动归档`,
            }
          : task,
      ),
    }));
    useUiStore
      .getState()
      .showToast(`《${target.articleTitle}》已成功接力发布至 ${target.platform}`);
  },

  retryPublishTask: (taskId) => {
    const target = get().publishTasks.find((task) => task.id === taskId);
    if (!target) return;
    set((state) => ({
      publishTasks: state.publishTasks.map((task) =>
        task.id === taskId
          ? {
              ...task,
              status: "NEEDS_MANUAL_ACTION",
              updatedAt: "刚刚",
              note: "已重新核对平台会话状态，确认封面与标签后即可继续发布",
            }
          : task,
      ),
    }));
    useUiStore
      .getState()
      .showToast(`已重新检查 ${target.platform} 发布任务，可立即确认发布`);
  },
});

export const useContentStore = create<ContentStore>()(createContentStoreState);

/** Creates an isolated vanilla content store for unit tests. */
export const createContentStore = () => createStore<ContentStore>()(createContentStoreState);
