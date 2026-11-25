import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import { demoPublishedContents, demoPublishTasks } from "../data/demoData";
import type {
  AnalyticsQuery,
  AnalyticsResult,
  CreatePublishTasksInput,
  DashboardFilter,
  ExternalWork,
  ExternalWorkFilter,
  ManualCompletionInput,
  ManualPublishPackage,
  OperationsDashboard,
  PublishPreview,
  PublishTask,
  ResolveUncertainInput,
  SyncPlatformWorksInput,
  SyncRunResult,
} from "../types/operations";
import { useUiStore } from "./uiStore";

const PLATFORM_IDS: Record<string, string> = {
  掘金: "juejin",
  CSDN: "csdn",
  知乎: "zhihu",
  微信公众号: "wechat",
  Bilibili: "bilibili",
  YouTube: "youtube",
  抖音: "douyin",
  快手: "kuaishou",
  小红书: "xiaohongshu",
  微博: "weibo",
};

function demoTask(task: (typeof demoPublishTasks)[number], index: number): PublishTask {
  const kind = task.articleTitle.includes("视频") ? "video" : "article";
  return {
    id: task.id,
    sourceType: "platformDraft",
    sourceId: `demo-draft-${index}`,
    contentId: `demo-content-${index}`,
    contentVersion: "demo-v1",
    contentKind: kind,
    platformId: PLATFORM_IDS[task.platform] ?? task.platform,
    platformName: task.platform,
    accountId: `demo-account-${index}`,
    accountAlias: task.accountAlias,
    adapterId: null,
    title: task.articleTitle,
    body: task.note,
    tags: ["演示数据"],
    media: [],
    executionMode: "manual",
    status: task.status,
    paused: task.status === "FAILED" && task.note.includes("过期"),
    pauseReason: task.status === "FAILED" && task.note.includes("过期") ? task.note : null,
    scheduledAt: task.scheduledAt ?? null,
    externalId: task.status === "SUCCESS" ? `demo-${task.id}` : null,
    externalUrl: null,
    errorCode: null,
    errorMessage: task.status === "FAILED" ? task.note : null,
    nextAction: task.status === "SUCCESS" ? null : "review_and_confirm",
    attemptCount: task.status === "SUCCESS" || task.status === "FAILED" ? 1 : 0,
    createdAt: new Date(Date.now() - index * 3600_000).toISOString(),
    updatedAt: new Date(Date.now() - index * 3600_000).toISOString(),
  };
}

const initialDemoTasks = demoPublishTasks.map(demoTask);

const initialDemoWorks: ExternalWork[] = demoPublishedContents.map((item) => ({
  id: item.id,
  sourceKey: `demo:${item.id}`,
  accountId: `demo-${item.id}`,
  platformId: PLATFORM_IDS[item.platform] ?? item.platform,
  platformName: item.platform,
  accountAlias: item.accountAlias,
  externalId: item.externalId,
  title: item.title,
  contentKind:
    item.contentType === "视频" ? "video" : item.contentType === "图文" ? "imageText" : "article",
  originalContentType: item.contentType,
  url: null,
  publishedAt: new Date(item.publishedAt).toISOString(),
  source: "publish",
  syncedAt: new Date().toISOString(),
  createdAt: new Date(item.publishedAt).toISOString(),
  updatedAt: new Date().toISOString(),
  metrics: [
    {
      id: `${item.id}-reads`,
      workId: item.id,
      metricName: item.readsMetric,
      metricDefinition: "浏览器预览数据，仅用于界面演示",
      unit: "次",
      value: Number(item.readsLabel.replace(/,/g, "")) || 0,
      capturedAt: new Date().toISOString(),
      snapshotDay: new Date().toISOString().slice(0, 10),
      source: "demo",
      canonicalKey: null,
      comparable: false,
    },
    {
      id: `${item.id}-likes`,
      workId: item.id,
      metricName: "点赞数",
      metricDefinition: "浏览器预览数据，仅用于界面演示",
      unit: "次",
      value: item.likes,
      capturedAt: new Date().toISOString(),
      snapshotDay: new Date().toISOString().slice(0, 10),
      source: "demo",
      canonicalKey: null,
      comparable: false,
    },
  ],
}));

const emptyAnalytics = (query: AnalyticsQuery): AnalyticsResult => ({
  range: query.range,
  granularity: query.granularity,
  from: new Date(Date.now() - 7 * 86400_000).toISOString(),
  to: new Date().toISOString(),
  series: [],
  warnings: ["浏览器预览没有真实平台采样；请在桌面应用同步账号作品后查看趋势"],
});

export interface OperationsStore {
  tasks: PublishTask[];
  works: ExternalWork[];
  syncRuns: SyncRunResult[];
  preview: PublishPreview | null;
  selectedWork: ExternalWork | null;
  analytics: AnalyticsResult | null;
  dashboard: OperationsDashboard | null;
  loading: boolean;
  initialized: boolean;
  error: string;
  initialize: (force?: boolean) => Promise<void>;
  loadTasks: () => Promise<void>;
  loadWorks: (filter?: ExternalWorkFilter) => Promise<void>;
  loadSyncRuns: (limit?: number) => Promise<void>;
  loadAnalytics: (query: AnalyticsQuery) => Promise<void>;
  loadDashboard: (filter: DashboardFilter) => Promise<void>;
  createTasks: (input: CreatePublishTasksInput) => Promise<PublishTask[]>;
  prepareTask: (taskId: string) => Promise<PublishPreview | null>;
  confirmTask: (taskId: string, confirmationToken: string) => Promise<PublishTask | null>;
  cancelTask: (taskId: string, reason?: string) => Promise<PublishTask | null>;
  retryTask: (taskId: string) => Promise<PublishTask | null>;
  runDueTasks: () => Promise<PublishTask[]>;
  exportManualPackage: (taskId: string) => Promise<ManualPublishPackage | null>;
  completeManualTask: (taskId: string, input: ManualCompletionInput) => Promise<PublishTask | null>;
  resolveUncertainTask: (
    taskId: string,
    input: ResolveUncertainInput,
  ) => Promise<PublishTask | null>;
  syncWorks: (input: SyncPlatformWorksInput) => Promise<SyncRunResult | null>;
  resumeAccountTasks: (accountId: string) => Promise<PublishTask[]>;
  selectWork: (work: ExternalWork | null) => void;
  clearPreview: () => void;
}

function upsertTask(tasks: PublishTask[], task: PublishTask): PublishTask[] {
  const exists = tasks.some((item) => item.id === task.id);
  return exists ? tasks.map((item) => (item.id === task.id ? task : item)) : [task, ...tasks];
}

const createOperationsStoreState: StateCreator<OperationsStore> = (set, get) => ({
  tasks: initialDemoTasks,
  works: initialDemoWorks,
  syncRuns: [],
  preview: null,
  selectedWork: null,
  analytics: null,
  dashboard: null,
  loading: false,
  initialized: false,
  error: "",

  initialize: async (force = false) => {
    if (get().initialized && !force) return;
    set({ loading: true, error: "" });
    try {
      if (isTauri()) {
        const [tasks, works, syncRuns] = await Promise.all([
          invoke<PublishTask[]>("list_publish_tasks"),
          invoke<ExternalWork[]>("list_external_works", { filter: null }),
          invoke<SyncRunResult[]>("list_sync_runs", { limit: 20 }),
        ]);
        set({ tasks, works, syncRuns });
      }
      set({ initialized: true });
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause);
      set({ error: message });
      useUiStore.getState().setError(message);
    } finally {
      set({ loading: false });
    }
  },

  loadTasks: async () => {
    if (!isTauri()) return;
    try {
      const tasks = await invoke<PublishTask[]>("list_publish_tasks");
      set({ tasks });
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  },

  loadWorks: async (filter) => {
    if (!isTauri()) return;
    try {
      const works = await invoke<ExternalWork[]>("list_external_works", { filter: filter ?? null });
      set({ works });
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  },

  loadSyncRuns: async (limit = 20) => {
    if (!isTauri()) return;
    try {
      const syncRuns = await invoke<SyncRunResult[]>("list_sync_runs", { limit });
      set({ syncRuns });
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  },

  loadAnalytics: async (query) => {
    if (!isTauri()) {
      set({ analytics: emptyAnalytics(query) });
      return;
    }
    try {
      const analytics = await invoke<AnalyticsResult>("get_analytics_trend", { query });
      set({ analytics });
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  },

  loadDashboard: async (filter) => {
    if (!isTauri()) {
      set({ dashboard: null });
      return;
    }
    try {
      const dashboard = await invoke<OperationsDashboard>("get_operations_dashboard", {
        filter,
      });
      set({ dashboard });
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  },

  createTasks: async (input) => {
    try {
      let tasks: PublishTask[];
      if (isTauri()) {
        const result = await invoke<{ tasks: PublishTask[]; deduplicatedCount: number }>(
          "create_publish_tasks",
          { input },
        );
        tasks = result.tasks;
        if (result.deduplicatedCount > 0) {
          useUiStore.getState().showToast(`已复用 ${result.deduplicatedCount} 个相同目标发布任务`);
        }
      } else {
        const now = new Date().toISOString();
        tasks = input.targets.map((target) => ({
          id: crypto.randomUUID(),
          sourceType: input.sourceType,
          sourceId: input.sourceId,
          contentId: input.sourceId,
          contentVersion: "browser-preview",
          contentKind: "article",
          platformId: target.platformId,
          platformName: target.platformName,
          accountId: target.accountId,
          accountAlias: target.accountAlias,
          adapterId: target.adapterId,
          title: "浏览器预览发布任务",
          body: "桌面应用会使用最终平台稿快照。",
          tags: [],
          media: [],
          executionMode: "manual",
          status: "CREATED",
          paused: false,
          scheduledAt: input.scheduledAt ?? null,
          externalId: null,
          externalUrl: null,
          errorCode: null,
          errorMessage: null,
          nextAction: "review_and_confirm",
          attemptCount: 0,
          createdAt: now,
          updatedAt: now,
        }));
      }
      set((state) => ({
        tasks: tasks.reduce((all, task) => upsertTask(all, task), state.tasks),
      }));
      return tasks;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return [];
    }
  },

  prepareTask: async (taskId) => {
    try {
      if (isTauri()) {
        const preview = await invoke<PublishPreview>("prepare_publish_task", { taskId });
        set((state) => ({
          preview,
          tasks: upsertTask(state.tasks, preview.task),
        }));
        return preview;
      }
      const task = get().tasks.find((item) => item.id === taskId);
      if (!task) throw new Error("未找到发布任务");
      const preview: PublishPreview = {
        task,
        taskId,
        confirmationToken: crypto.randomUUID(),
        confirmationExpiresAt: new Date(Date.now() + 10 * 60_000).toISOString(),
        executionMode: "manual",
        capability: "unavailable",
        capabilityReason: "浏览器预览不调用真实平台适配器",
        validation: {
          ok: true,
          contentKind: task.contentKind,
          issues: [],
          mediaOrder: task.media.map((asset) => asset.order),
          coverAssetId: task.cover?.id ?? null,
        },
        actions: [
          { id: "auto_publish", label: "自动发布", available: false, reason: "需要桌面适配器" },
          {
            id: "manual_package",
            label: "导出人工发布包",
            available: false,
            reason: "需要桌面工作区",
          },
          { id: "cancel", label: "取消任务", available: true },
        ],
      };
      set({ preview });
      return preview;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  confirmTask: async (taskId, confirmationToken) => {
    try {
      let task: PublishTask;
      if (isTauri()) {
        task = await invoke<PublishTask>("confirm_publish_task", {
          taskId,
          confirmationToken,
        });
      } else {
        const existing = get().tasks.find((item) => item.id === taskId);
        if (!existing) throw new Error("未找到发布任务");
        task = {
          ...existing,
          status: "NEEDS_MANUAL_ACTION",
          nextAction: "export_manual_package_or_open_platform",
          updatedAt: new Date().toISOString(),
        };
      }
      set((state) => ({
        tasks: upsertTask(state.tasks, task),
        preview: null,
      }));
      return task;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  cancelTask: async (taskId, reason) => {
    try {
      const task = isTauri()
        ? await invoke<PublishTask>("cancel_publish_task", { taskId, reason: reason ?? null })
        : (() => {
            const existing = get().tasks.find((item) => item.id === taskId);
            if (!existing) throw new Error("未找到发布任务");
            return { ...existing, status: "CANCELLED" as const };
          })();
      set((state) => ({ tasks: upsertTask(state.tasks, task), preview: null }));
      return task;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  retryTask: async (taskId) => {
    try {
      const task = isTauri()
        ? await invoke<PublishTask>("retry_publish_task", { taskId })
        : (() => {
            const existing = get().tasks.find((item) => item.id === taskId);
            if (!existing) throw new Error("未找到发布任务");
            return { ...existing, status: "CREATED" as const, errorMessage: null };
          })();
      set((state) => ({ tasks: upsertTask(state.tasks, task) }));
      return task;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  runDueTasks: async () => {
    if (!isTauri()) {
      useUiStore.getState().showToast("浏览器预览不执行定时发布");
      return [];
    }
    try {
      const tasks = await invoke<PublishTask[]>("run_due_publish_tasks");
      set((state) => ({
        tasks: tasks.reduce((all, task) => upsertTask(all, task), state.tasks),
      }));
      return tasks;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return [];
    }
  },

  exportManualPackage: async (taskId) => {
    if (!isTauri()) {
      useUiStore.getState().showToast("浏览器预览不导出工作区文件；请在桌面应用操作");
      return null;
    }
    try {
      return await invoke<ManualPublishPackage>("export_manual_publish_package", { taskId });
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  completeManualTask: async (taskId, input) => {
    try {
      const task = isTauri()
        ? await invoke<PublishTask>("complete_manual_publish_task", { taskId, input })
        : (() => {
            const existing = get().tasks.find((item) => item.id === taskId);
            if (!existing) throw new Error("未找到发布任务");
            return {
              ...existing,
              status: "SUCCESS" as const,
              externalId: input.externalId ?? `manual-${taskId}`,
              externalUrl: input.externalUrl ?? null,
              errorMessage: null,
            };
          })();
      set((state) => ({ tasks: upsertTask(state.tasks, task) }));
      return task;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  resolveUncertainTask: async (taskId, input) => {
    try {
      const task = isTauri()
        ? await invoke<PublishTask>("resolve_uncertain_publish_task", { taskId, input })
        : (() => {
            const existing = get().tasks.find((item) => item.id === taskId);
            if (!existing) throw new Error("未找到发布任务");
            const status =
              input.outcome === "success"
                ? "SUCCESS"
                : input.outcome === "failed"
                  ? "FAILED"
                  : "CANCELLED";
            return { ...existing, status } as PublishTask;
          })();
      set((state) => ({ tasks: upsertTask(state.tasks, task) }));
      return task;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  syncWorks: async (input) => {
    if (!isTauri()) {
      const result: SyncRunResult = {
        id: crypto.randomUUID(),
        accountId: input.accountId,
        platformId: input.platformId,
        adapterId: input.adapterId,
        status: "SUCCESS",
        message: "浏览器预览未调用真实平台；请在桌面应用同步",
        importedCount: 0,
        updatedCount: 0,
        skippedCount: 0,
        errorCount: 0,
        accountInvalid: false,
        errors: [],
        startedAt: new Date().toISOString(),
        completedAt: new Date().toISOString(),
      };
      set((state) => ({ syncRuns: [result, ...state.syncRuns] }));
      return result;
    }
    try {
      const result = await invoke<SyncRunResult>("sync_platform_works", { input });
      set((state) => ({
        syncRuns: [result, ...state.syncRuns.filter((run) => run.id !== result.id)],
      }));
      await get().loadWorks();
      return result;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    }
  },

  resumeAccountTasks: async (accountId) => {
    if (!isTauri()) return get().tasks.filter((task) => task.accountId === accountId);
    try {
      const tasks = await invoke<PublishTask[]>("resume_account_publish_tasks", { accountId });
      set((state) => ({
        tasks: tasks.reduce((all, task) => upsertTask(all, task), state.tasks),
      }));
      return tasks;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return [];
    }
  },

  selectWork: (selectedWork) => set({ selectedWork }),
  clearPreview: () => set({ preview: null }),
});

export const useOperationsStore = create<OperationsStore>()(createOperationsStoreState);

export const createOperationsStore = () =>
  createStore<OperationsStore>()(createOperationsStoreState);
