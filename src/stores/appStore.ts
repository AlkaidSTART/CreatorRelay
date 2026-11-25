import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";

export type Document = {
  id: string;
  title: string;
  path: string;
  updatedAt: string;
  preview: string;
  wordCount: number;
  charCount: number;
  tags: string[];
  contentHash?: string;
};

export type Revision = { id: number; createdAt: string; label: string; preview: string };
export type WorkspaceInfo = { path: string; name: string };
export type SaveState = "saved" | "saving" | "unsaved" | "error";
type ValueOrUpdater<T> = T | ((current: T) => T);

export const starterRevisions: Revision[] = [
  { id: 3, createdAt: "今天 09:41", label: "当前版本", preview: "当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘……" },
  { id: 2, createdAt: "今天 09:18", label: "自动保存", preview: "最近我把一个数据转换任务迁移到了 WebAssembly 运行时。它并没有让系统架构……" },
  { id: 1, createdAt: "昨天 16:32", label: "初始版本", preview: "当服务离用户更近，延迟自然会低一些。真正的挑战是如何在有限资源里运行代码。" },
];

export type AppStore = {
  workspace: WorkspaceInfo | null;
  documents: Document[];
  selectedId: string;
  content: string;
  baseHash: string;
  query: string;
  saveState: SaveState;
  viewMode: "write" | "preview";
  showHistory: boolean;
  revisions: Revision[];
  isCreating: boolean;
  newTitle: string;
  loading: boolean;
  error: string;
  notice: string;
  sidebarOpen: boolean;
  showDemoBanner: boolean;
  setWorkspace: (value: ValueOrUpdater<WorkspaceInfo | null>) => void;
  setDocuments: (value: ValueOrUpdater<Document[]>) => void;
  setSelectedId: (value: ValueOrUpdater<string>) => void;
  setContent: (value: ValueOrUpdater<string>) => void;
  updateContent: (content: string) => void;
  setBaseHash: (value: ValueOrUpdater<string>) => void;
  setQuery: (value: ValueOrUpdater<string>) => void;
  setSaveState: (value: ValueOrUpdater<SaveState>) => void;
  setViewMode: (value: ValueOrUpdater<"write" | "preview">) => void;
  setShowHistory: (value: ValueOrUpdater<boolean>) => void;
  setRevisions: (value: ValueOrUpdater<Revision[]>) => void;
  setIsCreating: (value: ValueOrUpdater<boolean>) => void;
  setNewTitle: (value: ValueOrUpdater<string>) => void;
  setLoading: (value: ValueOrUpdater<boolean>) => void;
  setError: (value: ValueOrUpdater<string>) => void;
  setNotice: (value: ValueOrUpdater<string>) => void;
  setSidebarOpen: (value: ValueOrUpdater<boolean>) => void;
  setShowDemoBanner: (value: ValueOrUpdater<boolean>) => void;
};

function resolveValue<T>(value: ValueOrUpdater<T>, current: T): T {
  return typeof value === "function" ? (value as (current: T) => T)(current) : value;
}

const createAppStoreState: StateCreator<AppStore> = (set) => ({
  workspace: null,
  documents: [],
  selectedId: "",
  content: "",
  baseHash: "",
  query: "",
  saveState: "saved",
  viewMode: "write",
  showHistory: false,
  revisions: starterRevisions.map((revision) => ({ ...revision })),
  isCreating: false,
  newTitle: "",
  loading: true,
  error: "",
  notice: "",
  sidebarOpen: true,
  showDemoBanner: false,
  setWorkspace: (value) => set((state) => ({ workspace: resolveValue(value, state.workspace) })),
  setDocuments: (value) => set((state) => ({ documents: resolveValue(value, state.documents) })),
  setSelectedId: (value) => set((state) => ({ selectedId: resolveValue(value, state.selectedId) })),
  setContent: (value) => set((state) => ({ content: resolveValue(value, state.content) })),
  updateContent: (content) => set({ content, saveState: "unsaved" }),
  setBaseHash: (value) => set((state) => ({ baseHash: resolveValue(value, state.baseHash) })),
  setQuery: (value) => set((state) => ({ query: resolveValue(value, state.query) })),
  setSaveState: (value) => set((state) => ({ saveState: resolveValue(value, state.saveState) })),
  setViewMode: (value) => set((state) => ({ viewMode: resolveValue(value, state.viewMode) })),
  setShowHistory: (value) => set((state) => ({ showHistory: resolveValue(value, state.showHistory) })),
  setRevisions: (value) => set((state) => ({ revisions: resolveValue(value, state.revisions) })),
  setIsCreating: (value) => set((state) => ({ isCreating: resolveValue(value, state.isCreating) })),
  setNewTitle: (value) => set((state) => ({ newTitle: resolveValue(value, state.newTitle) })),
  setLoading: (value) => set((state) => ({ loading: resolveValue(value, state.loading) })),
  setError: (value) => set((state) => ({ error: resolveValue(value, state.error) })),
  setNotice: (value) => set((state) => ({ notice: resolveValue(value, state.notice) })),
  setSidebarOpen: (value) => set((state) => ({ sidebarOpen: resolveValue(value, state.sidebarOpen) })),
  setShowDemoBanner: (value) => set((state) => ({ showDemoBanner: resolveValue(value, state.showDemoBanner) })),
});

export const useAppStore = create<AppStore>()(createAppStoreState);

/** Creates an isolated vanilla store for unit tests and non-React consumers. */
export const createAppStore = () => createStore<AppStore>()(createAppStoreState);
