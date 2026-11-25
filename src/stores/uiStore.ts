import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import { applyThemeToDocument, getInitialTheme, type ThemeId } from "../utils/themes";

export type ViewMode = "write" | "preview";
export type StudioRightPanel = "none" | "info" | "history";

export interface UiStore {
  theme: ThemeId;
  sidebarOpen: boolean;
  docListOpen: boolean;
  showDemoBanner: boolean;
  isCreating: boolean;
  newTitle: string;
  error: string;
  notice: string;
  viewMode: ViewMode;
  rightPanel: StudioRightPanel;
  setTheme: (theme: ThemeId) => void;
  setSidebarOpen: (open: boolean) => void;
  toggleSidebar: () => void;
  setDocListOpen: (open: boolean) => void;
  toggleDocList: () => void;
  setShowDemoBanner: (show: boolean) => void;
  setIsCreating: (open: boolean) => void;
  setNewTitle: (title: string) => void;
  setError: (error: string) => void;
  clearError: () => void;
  setNotice: (notice: string) => void;
  showToast: (message: string, durationMs?: number) => void;
  setViewMode: (mode: ViewMode) => void;
  setRightPanel: (panel: StudioRightPanel) => void;
  toggleRightPanel: (panel: Exclude<StudioRightPanel, "none">) => void;
}

let toastTimer: ReturnType<typeof setTimeout> | null = null;

const createUiStoreState: StateCreator<UiStore> = (set) => ({
  theme: getInitialTheme(),
  sidebarOpen: true,
  docListOpen: true,
  showDemoBanner: false,
  isCreating: false,
  newTitle: "",
  error: "",
  notice: "",
  viewMode: "write",
  rightPanel: "none",
  setTheme: (theme) => {
    applyThemeToDocument(theme);
    set({ theme });
  },
  setSidebarOpen: (sidebarOpen) => set({ sidebarOpen }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setDocListOpen: (docListOpen) => set({ docListOpen }),
  toggleDocList: () => set((state) => ({ docListOpen: !state.docListOpen })),
  setShowDemoBanner: (showDemoBanner) => set({ showDemoBanner }),
  setIsCreating: (isCreating) => set({ isCreating }),
  setNewTitle: (newTitle) => set({ newTitle }),
  setError: (error) => set({ error }),
  clearError: () => set({ error: "" }),
  setNotice: (notice) => set({ notice }),
  showToast: (message, durationMs = 2200) => {
    if (toastTimer) clearTimeout(toastTimer);
    set({ notice: message });
    toastTimer = setTimeout(() => {
      set({ notice: "" });
      toastTimer = null;
    }, durationMs);
  },
  setViewMode: (viewMode) => set({ viewMode }),
  setRightPanel: (rightPanel) => set({ rightPanel }),
  toggleRightPanel: (panel) =>
    set((state) => ({ rightPanel: state.rightPanel === panel ? "none" : panel })),
});

export const useUiStore = create<UiStore>()(createUiStoreState);

/** Creates an isolated vanilla UI store for unit tests. */
export const createUiStore = () => createStore<UiStore>()(createUiStoreState);
