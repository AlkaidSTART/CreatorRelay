import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import { demoDocuments, demoWorkspace, sampleContent } from "../data/demoData";
import { bodyFromMarkdown, countWords, titleFromMarkdown } from "../utils/markdown";
import { useUiStore } from "./uiStore";

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

export type Revision = { id: string; createdAt: string; label: string; preview: string; diff: string };
export type WorkspaceInfo = { path: string; name: string };
export type SaveState = "saved" | "saving" | "unsaved" | "error";

export const starterRevisions: Revision[] = [
  {
    id: "demo-3",
    diff: "",
    createdAt: "今天 09:41",
    label: "当前版本",
    preview: "当服务离用户更近，延迟自然会低一些。但真正的挑战并不是把容器搬到边缘……",
  },
  {
    id: "demo-2",
    diff: "",
    createdAt: "今天 09:18",
    label: "自动保存",
    preview: "最近我把一个数据转换任务迁移到了 WebAssembly 运行时。它并没有让系统架构……",
  },
  {
    id: "demo-1",
    diff: "",
    createdAt: "昨天 16:32",
    label: "初始版本",
    preview: "当服务离用户更近，延迟自然会低一些。真正的挑战是如何在有限资源里运行代码。",
  },
];

async function digest(value: string) {
  if (!crypto.subtle) return "";
  const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
  return Array.from(new Uint8Array(bytes))
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

function getStoredDocuments(): Document[] | null {
  try {
    const parsed = JSON.parse(localStorage.getItem("creatorrelay.demo.documents") ?? "null");
    return Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function defaultDemoMarkdown(document: Document): string {
  return document.id === demoDocuments[0].id
    ? sampleContent
    : `---\nid: ${document.id}\ntitle: "${document.title}"\ncreated_at: 2026-11-24T10:00:00+08:00\nupdated_at: 2026-11-25T09:00:00+08:00\ntags: []\nstatus: draft\n---\n\n# ${document.title}\n\n${document.preview.replace("……", "。")}`;
}

export interface WorkspaceStore {
  workspace: WorkspaceInfo | null;
  documents: Document[];
  selectedId: string;
  content: string;
  baseHash: string;
  query: string;
  saveState: SaveState;
  revisions: Revision[];
  loading: boolean;
  initialized: boolean;
  setQuery: (query: string) => void;
  updateContent: (content: string) => void;
  initialize: () => Promise<void>;
  loadDocuments: (filter?: string) => Promise<Document[]>;
  loadDocument: (document: Document) => Promise<void>;
  loadRevisions: (path: string) => Promise<void>;
  saveCurrentDocument: (options?: { silent?: boolean }) => Promise<boolean>;
  chooseWorkspace: () => Promise<void>;
  selectDocumentSafely: (document: Document) => Promise<boolean>;
  createDocument: () => Promise<Document | null>;
  createRevision: (label?: string) => Promise<void>;
  restoreRevision: (revision: Revision) => Promise<void>;
}

const createWorkspaceStoreState: StateCreator<WorkspaceStore> = (set, get) => {
  let documentLoadSeq = 0;
  let saveInFlight = false;
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;

  const scheduleAutoSave = () => {
    if (autoSaveTimer) clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(() => {
      autoSaveTimer = null;
      void get().saveCurrentDocument();
    }, 650);
  };

  return {
    workspace: null,
    documents: [],
    selectedId: "",
    content: "",
    baseHash: "",
    query: "",
    saveState: "saved",
    revisions: starterRevisions.map((revision) => ({ ...revision })),
    loading: true,
    initialized: false,

    setQuery: (query) => {
      set({ query });
      void get().loadDocuments(query);
    },

    updateContent: (content) => {
      set({ content, saveState: "unsaved" });
      scheduleAutoSave();
    },

    initialize: async () => {
      if (get().initialized) return;
      set({ initialized: true, loading: true });
      const ui = useUiStore.getState();
      try {
        if (isTauri()) {
          const active = await invoke<WorkspaceInfo | null>("get_workspace");
          set({ workspace: active });
          if (active) {
            const items = await invoke<Document[]>("list_documents", { query: null });
            set({ documents: items, selectedId: items[0]?.id ?? "" });
            if (items[0]) {
              await get().loadDocument(items[0]);
              await get().loadRevisions(items[0].path);
            }
          }
        } else {
          const items = getStoredDocuments() ?? demoDocuments;
          ui.setShowDemoBanner(true);
          set({ workspace: demoWorkspace, documents: items, selectedId: items[0]?.id ?? "" });
          if (items[0]) {
            await get().loadDocument(items[0]);
          }
        }
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
      } finally {
        set({ loading: false });
      }
    },

    loadDocuments: async (filter = "") => {
      if (isTauri()) {
        const items = await invoke<Document[]>("list_documents", { query: filter || null });
        set({ documents: items });
        return items;
      }
      const items = getStoredDocuments() ?? demoDocuments;
      const term = filter.trim().toLocaleLowerCase();
      const filtered = term
        ? items.filter((item) =>
            `${item.title} ${item.preview} ${item.tags.join(" ")}`
              .toLocaleLowerCase()
              .includes(term),
          )
        : items;
      set({ documents: filtered });
      return filtered;
    },

    loadDocument: async (document) => {
      const loadId = ++documentLoadSeq;
      const ui = useUiStore.getState();
      ui.clearError();
      try {
        let markdown: string;
        let contentHash = "";
        if (isTauri()) {
          const result = await invoke<{ content: string; contentHash: string }>("read_document", {
            path: document.path,
          });
          markdown = result.content;
          contentHash = result.contentHash;
        } else {
          const key = `creatorrelay.demo.content.${document.id}`;
          const existing = localStorage.getItem(key);
          markdown = existing ?? defaultDemoMarkdown(document);
          if (!existing) {
            localStorage.setItem(key, markdown);
          }
        }
        if (loadId !== documentLoadSeq) return;
        const hash = contentHash || (await digest(markdown));
        if (loadId !== documentLoadSeq) return;
        set({ content: markdown, baseHash: hash, saveState: "saved" });
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
        set({ saveState: "error" });
      }
    },

    loadRevisions: async (path) => {
      if (!isTauri()) return;
      try {
        const items = await invoke<Revision[]>("list_revisions", { path });
        set({ revisions: items });
      } catch {
        set({ revisions: [] });
      }
    },

    saveCurrentDocument: async ({ silent = false } = {}) => {
      const state = get();
      const selected = state.documents.find((doc) => doc.id === state.selectedId);
      if (!selected || state.saveState !== "unsaved") return true;
      if (saveInFlight) return false;

      const snapshot = state.content;
      const ui = useUiStore.getState();
      saveInFlight = true;
      set({ saveState: "saving" });

      try {
        if (isTauri()) {
          const updated = await invoke<Document>("save_document", {
            path: selected.path,
            content: snapshot,
            baseHash: state.baseHash,
          });
          set((current) => ({
            documents: current.documents.map((doc) => (doc.id === updated.id ? updated : doc)),
          }));
          void get().loadRevisions(selected.path);
        } else {
          localStorage.setItem(`creatorrelay.demo.content.${selected.id}`, snapshot);
          const updated: Document = {
            ...selected,
            title: titleFromMarkdown(snapshot, selected.title),
            preview:
              bodyFromMarkdown(snapshot)
                .split("\n")
                .find((line) => line.trim() && !line.startsWith("#"))
                ?.slice(0, 120) ?? "",
            updatedAt: new Date().toISOString(),
            wordCount: countWords(snapshot),
            charCount: bodyFromMarkdown(snapshot).replace(/\s/gu, "").length,
          };
          const all = getStoredDocuments() ?? demoDocuments;
          const next = all.map((doc) => (doc.id === selected.id ? updated : doc));
          localStorage.setItem("creatorrelay.demo.documents", JSON.stringify(next));
          set((current) => ({
            documents: current.documents.map((doc) => (doc.id === selected.id ? updated : doc)),
          }));
        }
        const savedHash = await digest(snapshot);
        const stillSame = get().content === snapshot;
        set({ baseHash: savedHash, saveState: stillSame ? "saved" : "unsaved" });
        if (!silent && stillSame) {
          ui.showToast("已保存到工作区", 1600);
        }
        return stillSame;
      } catch (cause) {
        set({ saveState: "error" });
        ui.setError(cause instanceof Error ? cause.message : String(cause));
        return false;
      } finally {
        saveInFlight = false;
        if (get().content !== snapshot) {
          set({ saveState: "unsaved" });
          scheduleAutoSave();
        }
      }
    },

    chooseWorkspace: async () => {
      const ui = useUiStore.getState();
      if (get().saveState !== "saved" || saveInFlight) {
        ui.showToast("请等待当前文章保存完成后再切换工作区", 2200);
        return;
      }
      ui.clearError();
      try {
        if (!isTauri()) {
          ui.showToast("请在桌面应用中选择真实工作区", 2200);
          return;
        }
        const path = await open({
          directory: true,
          multiple: false,
          title: "选择 CreatorRelay 工作区",
        });
        if (typeof path !== "string") return;
        const next = await invoke<WorkspaceInfo>("set_workspace_path", { path });
        const items = await invoke<Document[]>("list_documents", { query: null });
        set({ workspace: next, documents: items, selectedId: items[0]?.id ?? "" });
        if (items[0]) {
          await get().loadDocument(items[0]);
          await get().loadRevisions(items[0].path);
        }
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
      }
    },

    selectDocumentSafely: async (document) => {
      const state = get();
      const ui = useUiStore.getState();
      if (document.id === state.selectedId) return true;
      if (state.saveState === "error") {
        ui.showToast("当前文章未能保存，请先处理保存错误再切换", 2200);
        return false;
      }
      if (state.saveState === "saving" || saveInFlight) {
        ui.showToast("正在保存当前文章，请稍后再切换", 1800);
        return false;
      }
      if (state.saveState === "unsaved") {
        if (autoSaveTimer) {
          clearTimeout(autoSaveTimer);
          autoSaveTimer = null;
        }
        const ok = await get().saveCurrentDocument({ silent: true });
        if (!ok) {
          ui.setError("文章在保存期间又有新修改，请等待保存完成后再切换。");
          return false;
        }
      }
      set({ selectedId: document.id });
      await get().loadDocument(document);
      await get().loadRevisions(document.path);
      return true;
    },

    createDocument: async () => {
      const state = get();
      const ui = useUiStore.getState();
      if (!state.workspace) {
        ui.setError("请先选择一个工作区");
        ui.setIsCreating(false);
        return null;
      }
      if (state.saveState !== "saved" || saveInFlight) {
        ui.showToast("请等待当前文章保存完成后再新建文档", 2200);
        return null;
      }
      const title = ui.newTitle.trim();
      if (!title) return null;
      try {
        let created: Document;
        if (isTauri()) {
          created = await invoke<Document>("create_document", { title });
        } else {
          const id = `article-${crypto.randomUUID()}`;
          created = {
            id,
            title,
            path: `articles/${id}/article.md`,
            updatedAt: new Date().toISOString(),
            preview: "开始写下你的想法……",
            wordCount: 0,
            charCount: 0,
            tags: [],
          };
          const all = getStoredDocuments() ?? demoDocuments;
          localStorage.setItem("creatorrelay.demo.documents", JSON.stringify([created, ...all]));
          localStorage.setItem(
            `creatorrelay.demo.content.${id}`,
            `---\nid: ${id}\ntitle: "${title}"\ncreated_at: ${new Date().toISOString()}\nupdated_at: ${new Date().toISOString()}\ntags: []\nstatus: draft\n---\n\n# ${title}\n\n`,
          );
        }
        ui.setNewTitle("");
        ui.setIsCreating(false);
        await get().loadDocuments();
        set({ selectedId: created.id });
        ui.setViewMode("write");
        await get().loadDocument(created);
        return created;
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
        return null;
      }
    },

    createRevision: async (label = "手动保存版本") => {
      const state = get();
      const ui = useUiStore.getState();
      const selected = state.documents.find((doc) => doc.id === state.selectedId);
      if (!selected) return;
      if (saveInFlight || state.saveState === "saving") {
        ui.showToast("正在保存文章，请稍后再创建版本", 2200);
        return;
      }
      if (state.saveState === "error") {
        ui.showToast("请先解决当前文章的保存错误", 2200);
        return;
      }
      if (!isTauri()) {
        ui.showToast("演示模式不保存真实版本历史", 2200);
        return;
      }
      try {
        if (get().saveState === "unsaved") {
          const saved = await get().saveCurrentDocument({ silent: true });
          if (!saved) return;
        }
        const items = await invoke<Revision[]>("create_document_revision", {
          path: selected.path,
          label,
        });
        set({ revisions: items });
        ui.showToast(items[0]?.label === label ? "已保存当前文章版本" : "当前内容与最近版本相同");
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
      }
    },

    restoreRevision: async (revision) => {
      const state = get();
      const ui = useUiStore.getState();
      const selected = state.documents.find((doc) => doc.id === state.selectedId);
      if (!selected) return;
      if (state.saveState !== "saved" || saveInFlight) {
        ui.showToast("请等待当前文章保存完成后再恢复历史版本", 2200);
        return;
      }
      try {
        if (isTauri()) {
          const updated = await invoke<Document>("restore_revision", {
            path: selected.path,
            revisionId: revision.id,
            baseHash: state.baseHash,
          });
          await get().loadDocuments();
          await get().loadDocument(updated);
          await get().loadRevisions(updated.path);
        } else {
          ui.showToast("演示模式下仅展示版本记录，桌面端可恢复完整历史版本", 2500);
        }
        ui.setRightPanel("none");
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
      }
    },
  };
};

export const useWorkspaceStore = create<WorkspaceStore>()(createWorkspaceStoreState);

/** Creates an isolated vanilla workspace store for unit tests. */
export const createWorkspaceStore = () => createStore<WorkspaceStore>()(createWorkspaceStoreState);
