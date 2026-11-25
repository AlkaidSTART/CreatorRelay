import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import type {
  ContentAsset,
  ContentKind,
  ContentPackage,
  ContentPackageSummary,
  ContentSaveInput,
  DraftSaveInput,
  DraftTarget,
  PlatformDraft,
} from "../types/content";
import { useUiStore } from "./uiStore";

export type StudioSaveState = "saved" | "saving" | "unsaved" | "error";

export interface ContentEditorState {
  id: string;
  kind: ContentKind;
  title: string;
  summary: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  body: string;
  contentHash: string;
  contentVersion: string;
  createdAt: string;
  updatedAt: string;
  path: string;
  source?: { contentId: string; version: string } | null;
}

export interface DraftEditorState {
  id: string;
  contentId: string;
  platformId: string;
  platformName: string;
  accountId: string;
  accountAlias: string;
  adapterId?: string | null;
  title: string;
  summary: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  body: string;
  sourceVersion: string;
  generatedAt: string;
  updatedAt: string;
  manuallyEdited: boolean;
  hash: string;
  sourceChanged: boolean;
}

export interface StudioStore {
  packages: ContentPackageSummary[];
  selectedId: string;
  current: ContentPackage | null;
  editor: ContentEditorState | null;
  filter: "ALL" | ContentKind;
  query: string;
  loading: boolean;
  initialized: boolean;
  saveState: StudioSaveState;
  saveError: string;
  simulateNextSaveFailure: boolean;
  drafts: PlatformDraft[];
  selectedDraftId: string;
  draftEditor: DraftEditorState | null;
  draftSaveState: StudioSaveState;
  draftError: string;
  initialize: (force?: boolean) => Promise<void>;
  loadPackages: () => Promise<ContentPackageSummary[]>;
  setFilter: (filter: "ALL" | ContentKind) => void;
  setQuery: (query: string) => void;
  createPackage: (kind: ContentKind, title: string) => Promise<ContentPackage | null>;
  derivePackage: (targetKind: ContentKind, title?: string) => Promise<ContentPackage | null>;
  selectPackage: (id: string) => Promise<boolean>;
  updateEditor: (patch: Partial<ContentEditorState>) => void;
  addAsset: (asset: ContentAsset, role: "media" | "cover") => void;
  updateMedia: (assetId: string, patch: Partial<ContentAsset>) => void;
  moveMedia: (assetId: string, direction: -1 | 1) => void;
  reorderMedia: (sourceId: string, targetId: string) => void;
  removeMedia: (assetId: string) => void;
  setCover: (asset: ContentAsset) => void;
  clearCover: () => void;
  save: (options?: { silent?: boolean }) => Promise<boolean>;
  loadDrafts: (contentId?: string) => Promise<void>;
  createDrafts: (targets: DraftTarget[]) => Promise<PlatformDraft[]>;
  selectDraft: (draftId: string) => Promise<void>;
  updateDraftEditor: (patch: Partial<DraftEditorState>) => void;
  saveDraft: () => Promise<boolean>;
  acknowledgeDraftSource: (draftId: string) => Promise<void>;
  regenerateDraft: (draftId: string) => Promise<void>;
}

const STORAGE_PACKAGES = "creatorrelay.m5.packages";
const STORAGE_DRAFTS = "creatorrelay.m5.drafts";

function packageToEditor(packageData: ContentPackage): ContentEditorState {
  return {
    id: packageData.id,
    kind: packageData.kind,
    title: packageData.title,
    summary: packageData.summary,
    tags: packageData.tags,
    cover: packageData.cover ?? null,
    media: packageData.media.map((asset) => ({ ...asset })),
    body: packageData.body,
    contentHash: packageData.contentHash,
    contentVersion: packageData.contentVersion,
    createdAt: packageData.createdAt,
    updatedAt: packageData.updatedAt,
    path: packageData.path,
    source: packageData.source ?? null,
  };
}

function summaryFromEditor(editor: ContentEditorState): ContentPackageSummary {
  return {
    id: editor.id,
    kind: editor.kind,
    title: editor.title,
    summary: editor.summary,
    tags: editor.tags,
    updatedAt: editor.updatedAt,
    hasCover: Boolean(editor.cover),
    mediaCount: editor.media.length,
    path: editor.path,
    contentVersion: editor.contentVersion,
  };
}

function editorFingerprint(editor: ContentEditorState): string {
  return JSON.stringify({
    title: editor.title,
    summary: editor.summary,
    tags: editor.tags,
    cover: editor.cover,
    media: editor.media,
    body: editor.body,
  });
}

async function digest(value: string): Promise<string> {
  if (globalThis.crypto?.subtle) {
    const bytes = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value));
    return Array.from(new Uint8Array(bytes))
      .map((byte) => byte.toString(16).padStart(2, "0"))
      .join("");
  }
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = (hash * 31 + value.charCodeAt(index)) | 0;
  }
  return `local-${Math.abs(hash)}`;
}

function readLocal<T>(key: string): T[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(key) ?? "[]") as unknown;
    return Array.isArray(parsed) ? (parsed as T[]) : [];
  } catch {
    return [];
  }
}

function writeLocal<T>(key: string, values: T[]) {
  localStorage.setItem(key, JSON.stringify(values));
}

function createDemoPackage(kind: ContentKind, title: string): ContentPackage {
  const id = crypto.randomUUID();
  const now = new Date().toISOString();
  const body = kind === "article" || kind === "imageArticle" ? `# ${title}\n\n` : "";
  const version = `local-${crypto.randomUUID()}`;
  return {
    schemaVersion: 1,
    id,
    kind,
    title,
    summary: "",
    tags: [],
    cover: null,
    media: [],
    createdAt: now,
    updatedAt: now,
    source: null,
    body,
    path: `contents/${id}`,
    contentHash: version,
    contentVersion: version,
  };
}

function draftToEditor(draft: PlatformDraft): DraftEditorState {
  return {
    id: draft.id,
    contentId: draft.contentId,
    platformId: draft.platformId,
    platformName: draft.platformName,
    accountId: draft.accountId,
    accountAlias: draft.accountAlias,
    adapterId: draft.adapterId,
    title: draft.title,
    summary: draft.summary,
    tags: draft.tags,
    cover: draft.cover ?? null,
    media: draft.media.map((asset) => ({ ...asset })),
    body: draft.body,
    sourceVersion: draft.sourceVersion,
    generatedAt: draft.generatedAt,
    updatedAt: draft.updatedAt,
    manuallyEdited: draft.manuallyEdited,
    hash: draft.hash,
    sourceChanged: draft.sourceChanged,
  };
}

function withLocalSourceState(draft: PlatformDraft, current: ContentPackage | null): PlatformDraft {
  return {
    ...draft,
    sourceChanged:
      Boolean(current && current.id === draft.contentId) &&
      current?.contentVersion !== draft.sourceVersion,
  };
}

const createStudioStoreState: StateCreator<StudioStore> = (set, get) => {
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;
  let saveInFlight = false;
  let savedFingerprint = "";
  let loadSequence = 0;

  const scheduleAutoSave = () => {
    if (autoSaveTimer) clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(() => {
      autoSaveTimer = null;
      void get().save();
    }, 700);
  };

  const clearAutoSave = () => {
    if (autoSaveTimer) clearTimeout(autoSaveTimer);
    autoSaveTimer = null;
  };

  return {
    packages: [],
    selectedId: "",
    current: null,
    editor: null,
    filter: "ALL",
    query: "",
    loading: false,
    initialized: false,
    saveState: "saved",
    saveError: "",
    simulateNextSaveFailure: false,
    drafts: [],
    selectedDraftId: "",
    draftEditor: null,
    draftSaveState: "saved",
    draftError: "",

    initialize: async (force = false) => {
      if (get().initialized && !force) return;
      set({ initialized: true, loading: true });
      await get().loadPackages();
      const state = get();
      if (!state.selectedId && state.packages[0]) {
        await get().selectPackage(state.packages[0].id);
      }
      set({ loading: false });
    },

    loadPackages: async () => {
      try {
        const packages = isTauri()
          ? await invoke<ContentPackageSummary[]>("list_content_packages")
          : readLocal<ContentPackage>(STORAGE_PACKAGES).map((item) => summaryFromEditor(packageToEditor(item)));
        set({ packages });
        return packages;
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
        return [];
      }
    },

    setFilter: (filter) => set({ filter }),
    setQuery: (query) => set({ query }),

    createPackage: async (kind, title) => {
      const ui = useUiStore.getState();
      if (!title.trim()) return null;
      try {
        if (get().editor && get().saveState === "unsaved") {
          const saved = await get().save({ silent: true });
          if (!saved) return null;
        }
        let packageData: ContentPackage;
        if (isTauri()) {
          packageData = await invoke<ContentPackage>("create_content_package", {
            kind,
            title: title.trim(),
          });
        } else {
          packageData = createDemoPackage(kind, title.trim());
          const packages = readLocal<ContentPackage>(STORAGE_PACKAGES);
          writeLocal(STORAGE_PACKAGES, [packageData, ...packages]);
        }
        await get().loadPackages();
        set({
          selectedId: packageData.id,
          current: packageData,
          editor: packageToEditor(packageData),
          saveState: "saved",
          saveError: "",
          drafts: [],
          selectedDraftId: "",
          draftEditor: null,
        });
        savedFingerprint = editorFingerprint(packageToEditor(packageData));
        ui.showToast(`已创建${kind === "video" ? "视频" : kind === "imageText" ? "图文" : kind === "imageArticle" ? "图文文章" : "纯文章"}主稿`);
        return packageData;
      } catch (cause) {
        ui.setError(cause instanceof Error ? cause.message : String(cause));
        return null;
      }
    },

    derivePackage: async (targetKind, title) => {
      const current = get().current;
      if (!current) return null;
      try {
        if (get().saveState === "unsaved") {
          const saved = await get().save({ silent: true });
          if (!saved) return null;
        }
        let derived: ContentPackage;
        if (isTauri()) {
          derived = await invoke<ContentPackage>("derive_content_package", {
            sourceId: current.id,
            targetKind,
            title: title ?? null,
          });
        } else {
          const base = createDemoPackage(targetKind, title ?? `${current.title}（派生稿）`);
          derived = {
            ...base,
            summary: current.summary,
            tags: current.tags,
            cover: current.cover,
            media: current.media.map((asset) => ({ ...asset })),
            source: { contentId: current.id, version: current.contentVersion },
          };
          const all = readLocal<ContentPackage>(STORAGE_PACKAGES);
          writeLocal(STORAGE_PACKAGES, [derived, ...all]);
        }
        await get().loadPackages();
        await get().selectPackage(derived.id);
        useUiStore.getState().showToast("已创建可追溯的派生内容包");
        return derived;
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
        return null;
      }
    },

    selectPackage: async (id) => {
      const state = get();
      if (state.selectedId === id && state.current) {
        await get().loadDrafts(id);
        return true;
      }
      if (state.editor && state.saveState !== "saved") {
        const saved = await get().save({ silent: true });
        if (!saved && get().saveState === "error") return false;
      }
      const loadId = ++loadSequence;
      try {
        let packageData: ContentPackage;
        if (isTauri()) {
          packageData = await invoke<ContentPackage>("read_content_package", { packageId: id });
        } else {
          const packageRecord = readLocal<ContentPackage>(STORAGE_PACKAGES).find(
            (item) => item.id === id,
          );
          if (!packageRecord) throw new Error("未找到内容包");
          packageData = packageRecord;
        }
        if (loadId !== loadSequence) return false;
        const editor = packageToEditor(packageData);
        savedFingerprint = editorFingerprint(editor);
        set({
          selectedId: id,
          current: packageData,
          editor,
          saveState: "saved",
          saveError: "",
          selectedDraftId: "",
          draftEditor: null,
        });
        await get().loadDrafts(id);
        return true;
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
        return false;
      }
    },

    updateEditor: (patch) => {
      const editor = get().editor;
      if (!editor) return;
      const next = { ...editor, ...patch };
      set({ editor: next, saveState: "unsaved" });
      scheduleAutoSave();
    },

    addAsset: (asset, role) => {
      if (role === "cover") {
        get().setCover(asset);
        return;
      }
      const editor = get().editor;
      if (!editor) return;
      const media = [...editor.media, { ...asset, order: editor.media.length + 1 }];
      set({ editor: { ...editor, media }, saveState: "unsaved" });
      scheduleAutoSave();
    },

    updateMedia: (assetId, patch) => {
      const editor = get().editor;
      if (!editor) return;
      const media = editor.media.map((asset) =>
        asset.id === assetId ? { ...asset, ...patch } : asset,
      );
      const cover =
        editor.cover?.id === assetId ? { ...editor.cover, ...patch } : editor.cover;
      set({ editor: { ...editor, media, cover }, saveState: "unsaved" });
      scheduleAutoSave();
    },

    moveMedia: (assetId, direction) => {
      const editor = get().editor;
      if (!editor) return;
      const index = editor.media.findIndex((asset) => asset.id === assetId);
      const target = index + direction;
      if (index < 0 || target < 0 || target >= editor.media.length) return;
      const media = editor.media.map((asset) => ({ ...asset }));
      [media[index], media[target]] = [media[target], media[index]];
      media.forEach((asset, position) => {
        asset.order = position + 1;
      });
      set({ editor: { ...editor, media }, saveState: "unsaved" });
      scheduleAutoSave();
    },

    reorderMedia: (sourceId, targetId) => {
      const editor = get().editor;
      if (!editor || sourceId === targetId) return;
      const media = editor.media.map((asset) => ({ ...asset }));
      const sourceIndex = media.findIndex((asset) => asset.id === sourceId);
      const targetIndex = media.findIndex((asset) => asset.id === targetId);
      if (sourceIndex < 0 || targetIndex < 0) return;
      const [moved] = media.splice(sourceIndex, 1);
      media.splice(targetIndex, 0, moved);
      media.forEach((asset, position) => {
        asset.order = position + 1;
      });
      set({ editor: { ...editor, media }, saveState: "unsaved" });
      scheduleAutoSave();
    },

    removeMedia: (assetId) => {
      const editor = get().editor;
      if (!editor) return;
      const media = editor.media
        .filter((asset) => asset.id !== assetId)
        .map((asset, index) => ({ ...asset, order: index + 1 }));
      set({
        editor: {
          ...editor,
          media,
          cover: editor.cover?.id === assetId ? null : editor.cover,
        },
        saveState: "unsaved",
      });
      scheduleAutoSave();
    },

    setCover: (asset) => {
      const editor = get().editor;
      if (!editor) return;
      set({ editor: { ...editor, cover: { ...asset, kind: "image" } }, saveState: "unsaved" });
      scheduleAutoSave();
    },

    clearCover: () => {
      const editor = get().editor;
      if (!editor) return;
      set({ editor: { ...editor, cover: null }, saveState: "unsaved" });
      scheduleAutoSave();
    },

    save: async ({ silent = false } = {}) => {
      const state = get();
      const editor = state.editor;
      if (!editor || state.saveState === "saved") return true;
      if (saveInFlight) return false;
      clearAutoSave();
      const snapshot = editorFingerprint(editor);
      if (state.simulateNextSaveFailure) {
        set({
          simulateNextSaveFailure: false,
          saveState: "error",
          saveError: "模拟磁盘写入失败：未保存内容仍保留在编辑器中，可修复后重试。",
        });
        return false;
      }
      saveInFlight = true;
      set({ saveState: "saving", saveError: "" });
      try {
        let saved: ContentPackage;
        if (isTauri()) {
          const input: ContentSaveInput = {
            title: editor.title,
            summary: editor.summary,
            tags: editor.tags,
            cover: editor.cover,
            media: editor.media,
            body: editor.body,
            baseHash: editor.contentHash,
          };
          saved = await invoke<ContentPackage>("save_content_package", {
            packageId: editor.id,
            input,
          });
        } else {
          const all = readLocal<ContentPackage>(STORAGE_PACKAGES);
          const existing = all.find((item) => item.id === editor.id);
          if (!existing) throw new Error("未找到内容包");
          const now = new Date().toISOString();
          const media = editor.media.map((asset, index) => ({ ...asset, order: index + 1 }));
          const contentVersion = await digest(
            JSON.stringify({
              title: editor.title,
              summary: editor.summary,
              tags: editor.tags,
              cover: editor.cover,
              media,
              body: editor.body,
            }),
          );
          saved = {
            ...existing,
            title: editor.title,
            summary: editor.summary,
            tags: editor.tags,
            cover: editor.cover,
            media,
            body: editor.body,
            updatedAt: now,
            contentHash: contentVersion,
            contentVersion,
          };
          writeLocal(
            STORAGE_PACKAGES,
            all.map((item) => (item.id === saved.id ? saved : item)),
          );
        }
        const nextEditor = packageToEditor(saved);
        const currentFingerprint = get().editor ? editorFingerprint(get().editor!) : "";
        const stillSame = currentFingerprint === snapshot;
        savedFingerprint = editorFingerprint(nextEditor);
        set({
          current: saved,
          editor: stillSame ? nextEditor : get().editor,
          packages: [
            summaryFromEditor(packageToEditor(saved)),
            ...get().packages.filter((item) => item.id !== saved.id),
          ],
          saveState: stillSame ? "saved" : "unsaved",
          saveError: "",
        });
        if (!silent && stillSame) useUiStore.getState().showToast("内容已保存到工作区", 1600);
        return stillSame;
      } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        set({ saveState: "error", saveError: message });
        useUiStore.getState().setError(message);
        return false;
      } finally {
        saveInFlight = false;
        window.setTimeout(() => {
          if (get().editor && editorFingerprint(get().editor!) !== savedFingerprint) {
            set({ saveState: "unsaved" });
            scheduleAutoSave();
          }
        }, 0);
      }
    },

    loadDrafts: async (contentId) => {
      try {
        const drafts = isTauri()
          ? await invoke<PlatformDraft[]>("list_platform_drafts", {
              contentId: contentId ?? null,
            })
          : readLocal<PlatformDraft>(STORAGE_DRAFTS)
              .filter((draft) => !contentId || draft.contentId === contentId)
              .map((draft) => withLocalSourceState(draft, get().current));
        set({ drafts });
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      }
    },

    createDrafts: async (targets) => {
      const state = get();
      if (!state.current || targets.length === 0) return [];
      try {
        let drafts: PlatformDraft[];
        if (isTauri()) {
          drafts = await invoke<PlatformDraft[]>("create_platform_drafts", {
            contentId: state.current.id,
            targets,
          });
        } else {
          const all = readLocal<PlatformDraft>(STORAGE_DRAFTS);
          const created = targets.map((target) => {
            const id = crypto.randomUUID();
            const now = new Date().toISOString();
            return {
              id,
              contentId: state.current!.id,
              platformId: target.platformId,
              platformName: target.platformName,
              accountId: target.accountId,
              accountAlias: target.accountAlias,
              adapterId: target.adapterId,
              title: state.editor?.title ?? state.current!.title,
              summary: state.editor?.summary ?? state.current!.summary,
              body:
                state.current!.kind === "video" || state.current!.kind === "imageText"
                  ? state.editor?.summary ?? ""
                  : state.editor?.body ?? state.current!.body,
              tags: state.editor?.tags ?? [],
              cover: state.editor?.cover ?? null,
              media: state.editor?.media ?? [],
              sourceVersion: state.current!.contentVersion,
              generatedAt: now,
              updatedAt: now,
              manuallyEdited: false,
              hash: `local-${crypto.randomUUID()}`,
              sourceChanged: false,
            } satisfies PlatformDraft;
          });
          writeLocal(STORAGE_DRAFTS, [...created, ...all]);
          drafts = created;
        }
        set((current) => ({ drafts: [...drafts, ...current.drafts] }));
        if (drafts[0]) {
          set({ selectedDraftId: drafts[0].id, draftEditor: draftToEditor(drafts[0]) });
        }
        useUiStore.getState().showToast(`已生成 ${drafts.length} 份独立平台稿`);
        return drafts;
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
        return [];
      }
    },

    selectDraft: async (draftId) => {
      try {
        let draft: PlatformDraft | undefined;
        if (isTauri()) {
          draft = await invoke<PlatformDraft>("read_platform_draft", { draftId });
        } else {
          const stored = readLocal<PlatformDraft>(STORAGE_DRAFTS).find(
            (item) => item.id === draftId,
          );
          draft = stored ? withLocalSourceState(stored, get().current) : undefined;
        }
        if (!draft) throw new Error("未找到平台稿");
        set({
          selectedDraftId: draftId,
          draftEditor: draftToEditor(draft),
          draftSaveState: "saved",
          draftError: "",
        });
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      }
    },

    updateDraftEditor: (patch) => {
      const editor = get().draftEditor;
      if (!editor) return;
      set({ draftEditor: { ...editor, ...patch }, draftSaveState: "unsaved" });
    },

    saveDraft: async () => {
      const editor = get().draftEditor;
      if (!editor || get().draftSaveState === "saved") return true;
      set({ draftSaveState: "saving", draftError: "" });
      try {
        let saved: PlatformDraft;
        if (isTauri()) {
          const input: DraftSaveInput = {
            title: editor.title,
            summary: editor.summary,
            tags: editor.tags,
            cover: editor.cover,
            media: editor.media,
            body: editor.body,
            baseHash: editor.hash,
          };
          saved = await invoke<PlatformDraft>("save_platform_draft", {
            draftId: editor.id,
            input,
          });
        } else {
          const all = readLocal<PlatformDraft>(STORAGE_DRAFTS);
          const existing = all.find((item) => item.id === editor.id);
          if (!existing) throw new Error("未找到平台稿");
          const hash = `local-${crypto.randomUUID()}`;
          saved = {
            ...existing,
            title: editor.title,
            summary: editor.summary,
            tags: editor.tags,
            cover: editor.cover,
            media: editor.media,
            body: editor.body,
            manuallyEdited: true,
            updatedAt: new Date().toISOString(),
            hash,
            sourceChanged: existing.sourceVersion !== get().current?.contentVersion,
          };
          writeLocal(
            STORAGE_DRAFTS,
            all.map((item) => (item.id === saved.id ? saved : item)),
          );
        }
        set((state) => ({
          drafts: state.drafts.map((item) => (item.id === saved.id ? saved : item)),
          draftEditor: draftToEditor(saved),
          draftSaveState: "saved",
        }));
        useUiStore.getState().showToast("平台稿已独立保存");
        return true;
      } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        set({ draftSaveState: "error", draftError: message });
        useUiStore.getState().setError(message);
        return false;
      }
    },

    acknowledgeDraftSource: async (draftId) => {
      try {
        let updated: PlatformDraft;
        if (isTauri()) {
          updated = await invoke<PlatformDraft>("acknowledge_platform_draft", { draftId });
        } else {
          const all = readLocal<PlatformDraft>(STORAGE_DRAFTS);
          const existing = all.find((item) => item.id === draftId);
          if (!existing) throw new Error("未找到平台稿");
          updated = {
            ...existing,
            sourceVersion: get().current?.contentVersion ?? existing.sourceVersion,
            sourceChanged: false,
          };
          writeLocal(
            STORAGE_DRAFTS,
            all.map((item) => (item.id === draftId ? updated : item)),
          );
        }
        set((state) => ({
          drafts: state.drafts.map((item) => (item.id === draftId ? updated : item)),
          draftEditor:
            state.draftEditor?.id === draftId ? draftToEditor(updated) : state.draftEditor,
        }));
        useUiStore.getState().showToast("已保留当前平台稿并确认源内容变化");
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      }
    },

    regenerateDraft: async (draftId) => {
      try {
        let updated: PlatformDraft;
        if (isTauri()) {
          updated = await invoke<PlatformDraft>("regenerate_platform_draft", { draftId });
        } else {
          const existing = readLocal<PlatformDraft>(STORAGE_DRAFTS).find(
            (item) => item.id === draftId,
          );
          const current = get().current;
          if (!existing || !current) throw new Error("未找到平台稿或主稿");
          updated = {
            ...existing,
            title: current.title,
            summary: current.summary,
            body: current.body,
            tags: current.tags,
            cover: current.cover,
            media: current.media,
            sourceVersion: current.contentVersion,
            sourceChanged: false,
            manuallyEdited: false,
            updatedAt: new Date().toISOString(),
            hash: `local-${crypto.randomUUID()}`,
          };
          const all = readLocal<PlatformDraft>(STORAGE_DRAFTS);
          writeLocal(
            STORAGE_DRAFTS,
            all.map((item) => (item.id === draftId ? updated : item)),
          );
        }
        set((state) => ({
          drafts: state.drafts.map((item) => (item.id === draftId ? updated : item)),
          draftEditor:
            state.draftEditor?.id === draftId ? draftToEditor(updated) : state.draftEditor,
        }));
        useUiStore.getState().showToast("已按最新主稿重新生成平台稿");
      } catch (cause) {
        useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      }
    },
  };
};

export const useStudioStore = create<StudioStore>()(createStudioStoreState);

export const createStudioStore = () => createStore<StudioStore>()(createStudioStoreState);
