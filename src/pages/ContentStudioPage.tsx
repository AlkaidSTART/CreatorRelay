import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router";
import {
  AlertTriangle,
  ArrowRight,
  BookOpen,
  Check,
  ChevronRight,
  FileText,
  History,
  Image,
  MoreHorizontal,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  RefreshCw,
  Search,
  Send,
  Sparkles,
  Video,
  X,
} from "../components/Icons";
import { StudioWorkspaceOnboarding } from "../components/StudioWorkspaceEmpty";
import { ContentEditor } from "../components/studio/ContentEditor";
import { PlatformDraftPanel } from "../components/studio/PlatformDraftPanel";
import { AiAssistantPanel } from "../components/studio/AiAssistantPanel";
import { ContentChecksPanel } from "../components/studio/ContentChecksPanel";
import { useStudioStore } from "../stores/studioStore";
import { useUiStore } from "../stores/uiStore";
import { useWorkspaceStore } from "../stores/workspaceStore";
import { CONTENT_KIND_META, contentKindLabel, type ContentKind } from "../types/content";
import { formatRelativeTime } from "../utils/format";

type RightTab = "drafts" | "ai" | "checks" | null;

const KIND_ICONS: Record<ContentKind, typeof Video> = {
  video: Video,
  imageText: Image,
  imageArticle: BookOpen,
  article: FileText,
};

function kindBadgeClass(kind: ContentKind) {
  switch (kind) {
    case "video":
      return "bg-rose-bg text-rose-text";
    case "imageText":
      return "bg-lemon-bg text-lemon-text";
    case "imageArticle":
      return "bg-sky-bg text-sky-text";
    default:
      return "bg-accent-soft text-accent-deep";
  }
}

export function ContentStudioPage() {
  const navigate = useNavigate();
  const workspace = useWorkspaceStore((state) => state.workspace);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const packages = useStudioStore((state) => state.packages);
  const drafts = useStudioStore((state) => state.drafts);
  const filter = useStudioStore((state) => state.filter);
  const setFilter = useStudioStore((state) => state.setFilter);
  const query = useStudioStore((state) => state.query);
  const setQuery = useStudioStore((state) => state.setQuery);
  const selectedId = useStudioStore((state) => state.selectedId);
  const editor = useStudioStore((state) => state.editor);
  const loading = useStudioStore((state) => state.loading);
  const saveState = useStudioStore((state) => state.saveState);
  const saveError = useStudioStore((state) => state.saveError);
  const initialize = useStudioStore((state) => state.initialize);
  const loadPackages = useStudioStore((state) => state.loadPackages);
  const selectPackage = useStudioStore((state) => state.selectPackage);
  const save = useStudioStore((state) => state.save);
  const derivePackage = useStudioStore((state) => state.derivePackage);
  const setIsCreating = useUiStore((state) => state.setIsCreating);
  const setNewKind = useUiStore((state) => state.setNewKind);
  const viewMode = useUiStore((state) => state.viewMode);
  const setViewMode = useUiStore((state) => state.setViewMode);
  const docListOpen = useUiStore((state) => state.docListOpen);
  const setDocListOpen = useUiStore((state) => state.setDocListOpen);

  // Default to null so the writing canvas stays serene and uncluttered until an inspector is requested.
  const [rightTab, setRightTab] = useState<RightTab>(null);
  const [moreMenuOpen, setMoreMenuOpen] = useState(false);
  const moreMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function syncResponsivePanels() {
      if (window.innerWidth < 1180) {
        useUiStore.getState().setDocListOpen(false);
      }
    }
    syncResponsivePanels();
    window.addEventListener("resize", syncResponsivePanels);
    return () => window.removeEventListener("resize", syncResponsivePanels);
  }, []);

  useEffect(() => {
    if (!workspace) return;
    void initialize();
  }, [workspace?.path, initialize]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
        event.preventDefault();
        void save();
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        if (!useUiStore.getState().docListOpen) {
          useUiStore.getState().setDocListOpen(true);
        }
        window.setTimeout(() => {
          document.querySelector<HTMLInputElement>("[data-studio-search]")?.focus();
        }, 0);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [save]);

  useEffect(() => {
    if (!moreMenuOpen) return;
    function handleOutsideClick(event: MouseEvent) {
      if (moreMenuRef.current && !moreMenuRef.current.contains(event.target as Node)) {
        setMoreMenuOpen(false);
      }
    }
    window.addEventListener("mousedown", handleOutsideClick);
    return () => window.removeEventListener("mousedown", handleOutsideClick);
  }, [moreMenuOpen]);

  const filtered = useMemo(() => {
    const term = query.trim().toLocaleLowerCase();
    return packages.filter((item) => {
      if (filter !== "ALL" && item.kind !== filter) return false;
      if (!term) return true;
      return `${item.title} ${item.summary} ${item.tags.join(" ")}`
        .toLocaleLowerCase()
        .includes(term);
    });
  }, [packages, filter, query]);

  const saveMeta =
    saveState === "saving"
      ? { label: "保存中...", dot: "bg-amber-500 animate-pulse" }
      : saveState === "unsaved"
        ? { label: "未保存", dot: "bg-amber-500" }
        : saveState === "error"
          ? { label: "保存失败", dot: "bg-coral-text" }
          : { label: "已保存", dot: "bg-mint-text" };

  function openCreation(kind: ContentKind) {
    setNewKind(kind);
    setIsCreating(true);
  }

  function toggleRightTab(tab: Exclude<RightTab, null>) {
    setRightTab((current) => (current === tab ? null : tab));
  }

  async function handleSimulateFailure() {
    useStudioStore.setState({ simulateNextSaveFailure: true });
    useStudioStore.getState().updateEditor({});
    await save();
  }

  if (!workspace) {
    return (
      <div className="flex h-full items-center justify-center bg-canvas p-8">
        <StudioWorkspaceOnboarding onChooseWorkspace={() => void chooseWorkspace()} />
      </div>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden bg-canvas">
      {saveState === "error" && (
        <div className="fixed top-4 left-1/2 z-40 flex max-w-[640px] -translate-x-1/2 items-start gap-3 rounded-lg border border-coral-border bg-coral-bg px-4 py-3 text-xs text-coral-text shadow-lg">
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <div className="min-w-0 flex-1">
            <strong className="block font-medium">保存失败，未保存内容仍在编辑器中</strong>
            <span className="mt-0.5 block break-words text-[11px] opacity-90">{saveError}</span>
          </div>
          <button
            type="button"
            onClick={() => void save()}
            className="shrink-0 cursor-pointer rounded-md bg-white/85 px-2.5 py-1 text-[11px] font-medium transition-colors hover:bg-white"
          >
            重试保存
          </button>
        </div>
      )}

      {/* Left Pane: Clean Collapsible Package List */}
      {docListOpen && (
        <aside className="flex w-[224px] shrink-0 flex-col border-r border-line bg-surface-soft">
          <div className="flex h-11 shrink-0 items-center justify-between border-b border-line px-3">
            <div className="flex min-w-0 items-center gap-1.5">
              <span className="text-[12.5px] font-semibold tracking-tight text-ink">主稿</span>
              <span className="font-mono text-[10.5px] text-ink-muted">{packages.length}</span>
            </div>
            <div className="flex items-center gap-1">
              <button
                type="button"
                onClick={() => setIsCreating(true)}
                className="grid h-6.5 w-6.5 shrink-0 cursor-pointer place-items-center rounded-md text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                aria-label="新建内容"
                title="新建内容主稿 (⌘N)"
              >
                <Plus size={13} />
              </button>
              <button
                type="button"
                onClick={() => setDocListOpen(false)}
                className="grid h-6.5 w-6.5 shrink-0 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink"
                aria-label="收起主稿列表"
                title="收起主稿列表"
              >
                <PanelLeftClose size={13} />
              </button>
            </div>
          </div>

          <div className="space-y-2 border-b border-line px-2.5 py-2">
            <div className="flex h-7 items-center gap-1.5 rounded-md border border-line bg-surface px-2 transition-colors focus-within:border-accent">
              <Search size={11} className="shrink-0 text-ink-muted" />
              <input
                data-studio-search
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索主稿..."
                className="min-w-0 flex-1 bg-transparent text-[11.5px] text-ink placeholder:text-ink-muted focus:outline-none"
              />
              {query ? (
                <button
                  type="button"
                  onClick={() => setQuery("")}
                  aria-label="清空搜索"
                  className="grid h-4 w-4 cursor-pointer place-items-center rounded-xs text-ink-muted hover:bg-hover hover:text-ink"
                >
                  <X size={9} />
                </button>
              ) : (
                <kbd className="shrink-0 font-mono text-[9.5px] text-ink-faint">⌘K</kbd>
              )}
            </div>

            <div className="no-scrollbar flex items-center gap-0.5 overflow-x-auto">
              {[
                { id: "ALL" as const, label: "全部" },
                ...CONTENT_KIND_META.map((item) => ({ id: item.kind, label: item.label })),
              ].map((item) => {
                const active = filter === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => setFilter(item.id)}
                    className={`shrink-0 cursor-pointer whitespace-nowrap rounded-sm px-1.5 py-0.5 text-[10.5px] font-medium transition-colors ${
                      active
                        ? "bg-hover text-ink"
                        : "text-ink-muted hover:bg-hover/60 hover:text-ink"
                    }`}
                  >
                    {item.label}
                  </button>
                );
              })}
            </div>
          </div>

          {/* Hairline-divided Package List */}
          <div className="min-h-0 flex-1 overflow-y-auto">
            {loading ? (
              <div className="flex flex-col items-center justify-center gap-2 py-10 text-ink-muted">
                <RefreshCw size={13} className="animate-spin opacity-60" />
                <p className="text-[11.5px]">正在载入内容包...</p>
              </div>
            ) : filtered.length === 0 ? (
              <div className="p-3 text-center">
                <p className="text-xs text-ink-muted">
                  {packages.length === 0 ? "还没有内容主稿" : "没有匹配的主稿"}
                </p>
                <div className="mt-2.5 grid grid-cols-2 gap-1.5">
                  {CONTENT_KIND_META.map((item) => {
                    const Icon = KIND_ICONS[item.kind];
                    return (
                      <button
                        key={item.kind}
                        type="button"
                        onClick={() => openCreation(item.kind)}
                        className="inline-flex cursor-pointer items-center justify-center gap-1 rounded-md border border-line bg-surface px-1.5 py-1.5 text-[10.5px] font-medium text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
                      >
                        <Icon size={10} />
                        {item.label}
                      </button>
                    );
                  })}
                </div>
              </div>
            ) : (
              <div className="divide-y divide-line/60">
                {filtered.map((item) => {
                  const Icon = KIND_ICONS[item.kind];
                  const isSelected = selectedId === item.id;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => void selectPackage(item.id)}
                      className={`group relative flex w-full cursor-pointer flex-col gap-1 px-3 py-2.5 text-left transition-colors ${
                        isSelected ? "bg-surface" : "hover:bg-hover/50"
                      }`}
                    >
                      {isSelected && (
                        <span className="pointer-events-none absolute top-0 bottom-0 left-0 w-[2.5px] bg-accent" />
                      )}
                      <div className="flex items-baseline justify-between gap-2">
                        <strong
                          className={`min-w-0 flex-1 truncate text-[12px] ${
                            isSelected ? "font-semibold text-ink" : "font-medium text-ink-secondary"
                          }`}
                        >
                          {item.title || "未命名主稿"}
                        </strong>
                        <span className="shrink-0 font-mono text-[9.5px] text-ink-muted">
                          {formatRelativeTime(item.updatedAt)}
                        </span>
                      </div>
                      <div className="flex items-center gap-1.5 text-[10px] text-ink-muted">
                        <span
                          className={`inline-flex items-center gap-1 rounded-xs px-1.5 py-px text-[9.5px] font-medium ${kindBadgeClass(item.kind)}`}
                        >
                          <Icon size={8} />
                          {contentKindLabel(item.kind)}
                        </span>
                        {item.mediaCount > 0 && <span>{item.mediaCount} 媒体</span>}
                      </div>
                    </button>
                  );
                })}
              </div>
            )}
          </div>
        </aside>
      )}

      {/* Center Stage: Unified Document Editor */}
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-surface">
        {/* Single Clean Studio Toolbar */}
        <header className="flex h-11 shrink-0 items-center justify-between gap-3 border-b border-line bg-surface px-4">
          {/* Left: List Toggle + Kind + Save Status */}
          <div className="flex min-w-0 items-center gap-2">
            {!docListOpen && (
              <button
                type="button"
                onClick={() => setDocListOpen(true)}
                className="inline-flex h-7 shrink-0 cursor-pointer items-center gap-1.5 rounded-md border border-line bg-surface px-2 text-[11.5px] font-medium text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
                title="展开主稿列表 (⌘K)"
              >
                <PanelLeftOpen size={12} />
                <span className="hidden sm:inline">主稿 ({packages.length})</span>
              </button>
            )}

            {editor && (
              <span
                className={`inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-xs px-1.5 py-0.5 text-[10.5px] font-medium ${kindBadgeClass(editor.kind)}`}
              >
                {contentKindLabel(editor.kind)}
              </span>
            )}

            <span className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap text-[11px] text-ink-muted">
              <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dot}`} />
              {saveMeta.label}
            </span>
          </div>

          {/* Right: Mode Toggle + On-Demand Inspector Tabs + Save + Overflow */}
          <div className="flex shrink-0 items-center gap-1.5">
            {(editor?.kind === "article" || editor?.kind === "imageArticle") && (
              <div className="flex items-center rounded-md border border-line bg-subtle p-0.5">
                <button
                  type="button"
                  onClick={() => setViewMode("write")}
                  className={`cursor-pointer rounded-xs px-2 py-0.5 text-[11px] font-medium transition-colors ${
                    viewMode === "write"
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-muted hover:text-ink"
                  }`}
                >
                  编辑
                </button>
                <button
                  type="button"
                  onClick={() => setViewMode("preview")}
                  className={`cursor-pointer rounded-xs px-2 py-0.5 text-[11px] font-medium transition-colors ${
                    viewMode === "preview"
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-muted hover:text-ink"
                  }`}
                >
                  预览
                </button>
              </div>
            )}

            {editor && (
              <>
                <span className="mx-0.5 hidden h-4 w-px bg-line sm:inline-block" aria-hidden="true" />
                <div className="flex items-center gap-1">
                  <button
                    type="button"
                    onClick={() => toggleRightTab("drafts")}
                    className={`inline-flex h-7 cursor-pointer items-center gap-1 whitespace-nowrap rounded-md border px-2 text-[11.5px] font-medium transition-colors ${
                      rightTab === "drafts"
                        ? "border-accent-border bg-accent-soft text-accent-deep"
                        : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                    }`}
                  >
                    <Send size={11} />
                    <span>平台稿</span>
                    {drafts.length > 0 && (
                      <span className="font-mono text-[10px] opacity-80">{drafts.length}</span>
                    )}
                  </button>
                  <button
                    type="button"
                    onClick={() => toggleRightTab("ai")}
                    className={`inline-flex h-7 cursor-pointer items-center gap-1 whitespace-nowrap rounded-md border px-2 text-[11.5px] font-medium transition-colors ${
                      rightTab === "ai"
                        ? "border-accent-border bg-accent-soft text-accent-deep"
                        : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                    }`}
                  >
                    <Sparkles size={11} />
                    <span className="hidden sm:inline">AI</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => toggleRightTab("checks")}
                    className={`inline-flex h-7 cursor-pointer items-center gap-1 whitespace-nowrap rounded-md border px-2 text-[11.5px] font-medium transition-colors ${
                      rightTab === "checks"
                        ? "border-accent-border bg-accent-soft text-accent-deep"
                        : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                    }`}
                  >
                    <Check size={11} />
                    <span className="hidden sm:inline">检查</span>
                  </button>
                </div>
              </>
            )}

            <button
              type="button"
              onClick={() => void save()}
              disabled={saveState === "saving" || saveState === "saved"}
              className="inline-flex h-7 cursor-pointer items-center whitespace-nowrap rounded-md bg-accent px-2.5 text-[11.5px] font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-35"
            >
              保存
            </button>

            {/* Secondary Actions Overflow Menu */}
            <div ref={moreMenuRef} className="relative">
              <button
                type="button"
                onClick={() => setMoreMenuOpen((open) => !open)}
                aria-label="更多操作"
                title="更多操作"
                className="grid h-7 w-7 cursor-pointer place-items-center rounded-md border border-line bg-surface text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
              >
                <MoreHorizontal size={13} />
              </button>
              {moreMenuOpen && (
                <div className="absolute top-full right-0 z-30 mt-1 w-52 overflow-hidden rounded-md border border-line-strong bg-surface p-1 shadow-lg">
                  {editor && (
                    <>
                      <div className="px-2 py-1 text-[10px] font-medium tracking-wider text-ink-muted uppercase">
                        派生其他形态
                      </div>
                      {CONTENT_KIND_META.filter((item) => item.kind !== editor.kind).map((item) => {
                        const Icon = KIND_ICONS[item.kind];
                        return (
                          <button
                            key={item.kind}
                            type="button"
                            onClick={() => {
                              setMoreMenuOpen(false);
                              void derivePackage(item.kind);
                            }}
                            className="flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                          >
                            <Icon size={11} className="text-ink-muted" />
                            <span>派生为 {item.label}</span>
                          </button>
                        );
                      })}
                      <div className="my-1 border-t border-line" />
                    </>
                  )}
                  <button
                    type="button"
                    onClick={() => {
                      setMoreMenuOpen(false);
                      navigate("/publishing");
                    }}
                    className="flex w-full cursor-pointer items-center justify-between rounded-sm px-2 py-1.5 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                  >
                    <span className="inline-flex items-center gap-2">
                      <Send size={11} className="text-ink-muted" />
                      前往发布中心
                    </span>
                    <ArrowRight size={10} className="text-ink-faint" />
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setMoreMenuOpen(false);
                      void loadPackages();
                    }}
                    className="flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                  >
                    <RefreshCw size={11} className="text-ink-muted" />
                    <span>刷新主稿列表</span>
                  </button>
                  <button
                    type="button"
                    onClick={() => {
                      setMoreMenuOpen(false);
                      navigate("/studio/classic");
                    }}
                    className="flex w-full cursor-pointer items-center justify-between rounded-sm px-2 py-1.5 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                  >
                    <span className="inline-flex items-center gap-2">
                      <History size={11} className="text-ink-muted" />
                      经典 Markdown 工作区
                    </span>
                    <ChevronRight size={10} className="text-ink-faint" />
                  </button>
                  {editor && (
                    <>
                      <div className="my-1 border-t border-line" />
                      <div className="px-2 py-1 font-mono text-[10px] text-ink-muted">
                        版本 {editor.contentVersion.slice(0, 8)} · {editor.media.length} 媒体
                      </div>
                      <button
                        type="button"
                        onClick={() => {
                          setMoreMenuOpen(false);
                          void handleSimulateFailure();
                        }}
                        className="flex w-full cursor-pointer items-center gap-2 rounded-sm px-2 py-1.5 text-left text-[11px] text-ink-muted transition-colors hover:bg-hover hover:text-ink"
                        title="M5 验收用：下一次保存将显示失败状态且保留编辑内容"
                      >
                        <AlertTriangle size={11} />
                        <span>保存故障演练</span>
                      </button>
                    </>
                  )}
                </div>
              )}
            </div>
          </div>
        </header>

        {/* Clean Single-Canvas Document Area */}
        <div className="min-h-0 flex-1 overflow-hidden">
          {editor ? (
            <ContentEditor preview={viewMode === "preview"} />
          ) : (
            <div className="grid h-full place-items-center bg-canvas p-6">
              <div className="w-full max-w-[480px] rounded-lg border border-line bg-surface p-6">
                <h2 className="text-[15px] font-semibold tracking-tight text-ink">
                  创建第一份内容主稿
                </h2>
                <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
                  选择内容形态开始创作；右侧平台适配、AI 辅助与内容检查可按需展开。
                </p>
                <div className="mt-4 grid grid-cols-2 gap-2">
                  {CONTENT_KIND_META.map((item) => {
                    const Icon = KIND_ICONS[item.kind];
                    return (
                      <button
                        key={item.kind}
                        type="button"
                        onClick={() => openCreation(item.kind)}
                        className="group flex cursor-pointer items-start gap-2.5 rounded-md border border-line bg-canvas/60 p-3 text-left transition-colors hover:border-line-strong hover:bg-subtle"
                      >
                        <span
                          className={`mt-0.5 grid h-6 w-6 shrink-0 place-items-center rounded-sm ${kindBadgeClass(item.kind)}`}
                        >
                          <Icon size={11} />
                        </span>
                        <span className="min-w-0">
                          <strong className="block text-xs font-medium text-ink">
                            {item.label}
                          </strong>
                          <span className="mt-0.5 block text-[10.5px] leading-snug text-ink-muted">
                            {item.description}
                          </span>
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>
          )}
        </div>
      </main>

      {/* Right On-Demand Inspector Drawer (Only rendered when user opens a tab) */}
      {rightTab && (
        <aside className="flex w-[320px] shrink-0 flex-col border-l border-line bg-surface-soft xl:w-[348px]">
          <div className="flex h-11 shrink-0 items-center justify-between border-b border-line bg-surface px-3">
            <span className="text-xs font-semibold text-ink">
              {rightTab === "drafts"
                ? "平台适配稿"
                : rightTab === "ai"
                  ? "AI 创作辅助"
                  : "发布前内容检查"}
            </span>
            <button
              type="button"
              onClick={() => setRightTab(null)}
              className="grid h-6.5 w-6.5 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink"
              aria-label="关闭右侧面板"
              title="收起工作栏"
            >
              <X size={12} />
            </button>
          </div>
          <div className="min-h-0 flex-1 overflow-hidden">
            {rightTab === "drafts" && (
              <PlatformDraftPanel onShowChecks={() => setRightTab("checks")} />
            )}
            {rightTab === "ai" && <AiAssistantPanel />}
            {rightTab === "checks" && <ContentChecksPanel />}
          </div>
        </aside>
      )}
    </div>
  );
}
