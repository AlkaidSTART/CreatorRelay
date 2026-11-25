import { useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router";
import {
  AlertTriangle,
  BookOpen,
  Check,
  ChevronRight,
  Clock3,
  FileText,
  History,
  Image,
  Info,
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
  const showToast = useUiStore((state) => state.showToast);
  const viewMode = useUiStore((state) => state.viewMode);
  const setViewMode = useUiStore((state) => state.setViewMode);
  const [rightTab, setRightTab] = useState<RightTab>("drafts");
  const [deriveOpen, setDeriveOpen] = useState(false);
  const deriveMenuRef = useRef<HTMLDivElement>(null);

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
        document.querySelector<HTMLInputElement>("[data-studio-search]")?.focus();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [save]);

  useEffect(() => {
    if (!deriveOpen) return;
    function handleOutsideClick(event: MouseEvent) {
      if (deriveMenuRef.current && !deriveMenuRef.current.contains(event.target as Node)) {
        setDeriveOpen(false);
      }
    }
    window.addEventListener("mousedown", handleOutsideClick);
    return () => window.removeEventListener("mousedown", handleOutsideClick);
  }, [deriveOpen]);

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
        <div className="fixed top-4 left-1/2 z-40 flex max-w-[680px] -translate-x-1/2 items-start gap-3 rounded-2xl border border-coral-border bg-coral-bg px-4 py-3 text-xs text-coral-text shadow-lg">
          <AlertTriangle size={14} className="mt-0.5 shrink-0" />
          <div className="min-w-0 flex-1">
            <strong className="block font-medium">保存失败，未保存内容仍在编辑器中</strong>
            <span className="mt-0.5 block break-words text-[11px] opacity-90">{saveError}</span>
          </div>
          <button
            type="button"
            onClick={() => void save()}
            className="shrink-0 cursor-pointer rounded-lg bg-white/80 px-2.5 py-1 text-[11px] font-medium transition-colors hover:bg-white"
          >
            重试保存
          </button>
        </div>
      )}

      {/* Left Column: Content Package List */}
      <aside className="flex w-[264px] shrink-0 flex-col border-r border-line bg-sidebar/85">
        <div className="border-b border-line px-4 pt-4 pb-3.5">
          <div className="flex items-start justify-between gap-2">
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                <h1 className="text-[15px] font-semibold tracking-tight text-ink">内容工作室</h1>
                {packages.length > 0 && (
                  <span className="rounded-full bg-surface px-2 py-0.5 text-[10px] font-medium text-ink-muted ring-1 ring-line">
                    {packages.length}
                  </span>
                )}
              </div>
              <p className="mt-0.5 text-[11px] text-ink-muted">四形态主稿与平台适配</p>
            </div>
            <button
              type="button"
              onClick={() => setIsCreating(true)}
              className="grid h-8 w-8 shrink-0 cursor-pointer place-items-center rounded-xl bg-accent text-accent-on shadow-[0_2px_8px_rgba(123,104,168,0.26)] transition-all hover:bg-accent-hover active:scale-95"
              aria-label="新建内容"
              title="新建内容主稿 (⌘N)"
            >
              <Plus size={13} />
            </button>
          </div>

          <div className="mt-3 flex h-8.5 items-center gap-2 rounded-xl border border-line bg-surface px-2.5 shadow-[0_1px_2px_rgba(51,43,41,0.03)] transition-all focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
            <Search size={12} className="shrink-0 text-ink-muted" />
            <input
              data-studio-search
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索主稿、摘要、标签"
              className="min-w-0 flex-1 bg-transparent text-xs text-ink placeholder:text-ink-muted focus:outline-none"
            />
            {query ? (
              <button
                type="button"
                onClick={() => setQuery("")}
                aria-label="清空搜索"
                className="grid h-4.5 w-4.5 cursor-pointer place-items-center rounded text-ink-muted hover:bg-hover hover:text-ink"
              >
                <X size={10} />
              </button>
            ) : (
              <kbd className="shrink-0 rounded bg-subtle px-1.5 py-0.5 text-[9.5px] font-medium text-ink-muted">
                ⌘K
              </kbd>
            )}
          </div>
        </div>

        {/* Single-row horizontal filter pills */}
        <div className="border-b border-line px-3 py-2">
          <div className="no-scrollbar flex items-center gap-1 overflow-x-auto">
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
                  className={`shrink-0 cursor-pointer whitespace-nowrap rounded-lg px-2.5 py-1 text-[11px] font-medium transition-all ${
                    active
                      ? "bg-surface text-ink shadow-[0_1px_3px_rgba(51,43,41,0.08)] ring-1 ring-line"
                      : "text-ink-muted hover:bg-hover/70 hover:text-ink"
                  }`}
                >
                  {item.label}
                </button>
              );
            })}
          </div>
        </div>

        {/* Package List */}
        <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
          {loading ? (
            <div className="flex flex-col items-center justify-center gap-2 py-10 text-ink-muted">
              <RefreshCw size={14} className="animate-spin opacity-60" />
              <p className="text-xs">正在载入内容包...</p>
            </div>
          ) : filtered.length === 0 ? (
            <div className="rounded-2xl border border-dashed border-line-strong bg-surface/65 p-4 text-center">
              <p className="text-xs font-medium text-ink-secondary">
                {packages.length === 0 ? "还没有内容主稿" : "没有匹配的主稿"}
              </p>
              <p className="mt-1 text-[11px] text-ink-muted">
                {packages.length === 0 ? "选择一种形态开始创作" : "试试更换关键词或筛选条件"}
              </p>
              <div className="mt-3.5 grid grid-cols-2 gap-1.5">
                {CONTENT_KIND_META.map((item) => {
                  const Icon = KIND_ICONS[item.kind];
                  return (
                    <button
                      key={item.kind}
                      type="button"
                      onClick={() => openCreation(item.kind)}
                      className="inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-xl border border-line bg-surface px-2 py-2 text-[11px] font-medium text-ink-secondary shadow-[0_1px_2px_rgba(51,43,41,0.02)] transition-all hover:border-accent hover:bg-accent-soft/30 hover:text-accent-deep"
                    >
                      <Icon size={11} />
                      {item.label}
                    </button>
                  );
                })}
              </div>
            </div>
          ) : (
            <div className="space-y-1.5">
              {filtered.map((item) => {
                const Icon = KIND_ICONS[item.kind];
                const isSelected = selectedId === item.id;
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => void selectPackage(item.id)}
                    className={`group relative flex w-full cursor-pointer flex-col gap-1.5 rounded-xl border px-3.5 py-2.5 text-left transition-all ${
                      isSelected
                        ? "border-line-strong/80 bg-surface shadow-[0_2px_10px_rgba(51,43,41,0.06)]"
                        : "border-transparent bg-transparent hover:border-line/70 hover:bg-surface/65"
                    }`}
                  >
                    {isSelected && (
                      <span className="pointer-events-none absolute top-2.5 bottom-2.5 left-0 w-[3px] rounded-r-full bg-accent" />
                    )}
                    <div className="flex items-baseline justify-between gap-2">
                      <strong className="min-w-0 flex-1 truncate text-[12.5px] font-medium text-ink">
                        {item.title || "未命名主稿"}
                      </strong>
                      <span className="shrink-0 text-[10px] text-ink-muted">
                        {formatRelativeTime(item.updatedAt)}
                      </span>
                    </div>
                    {item.summary && (
                      <p className="truncate text-[11px] text-ink-secondary/90">{item.summary}</p>
                    )}
                    <div className="flex items-center gap-1.5 text-[10.5px] text-ink-muted">
                      <span
                        className={`inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 text-[10px] font-medium ${kindBadgeClass(item.kind)}`}
                      >
                        <Icon size={9} />
                        {contentKindLabel(item.kind)}
                      </span>
                      <span>{item.mediaCount} 素材</span>
                      {item.tags.length > 0 && (
                        <>
                          <span className="text-ink-faint">·</span>
                          <span>{item.tags.length} 标签</span>
                        </>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          )}
        </div>

        <div className="border-t border-line px-3 py-2.5">
          <button
            type="button"
            onClick={() => navigate("/studio/classic")}
            className="flex w-full cursor-pointer items-center justify-between rounded-xl px-2.5 py-2 text-[11px] font-medium text-ink-muted transition-colors hover:bg-hover/80 hover:text-ink"
          >
            <span className="inline-flex items-center gap-2">
              <History size={11} />
              经典 Markdown 工作区
            </span>
            <ChevronRight size={10} />
          </button>
        </div>
      </aside>

      {/* Center Column: Main Editor Workspace */}
      <main className="flex min-w-0 flex-1 flex-col bg-canvas">
        <header className="flex h-13 shrink-0 items-center justify-between gap-4 border-b border-line bg-surface/85 px-6 backdrop-blur-sm">
          <div className="flex min-w-0 items-center gap-2.5">
            <span className="inline-flex shrink-0 items-center gap-1.5 whitespace-nowrap rounded-full border border-line bg-surface px-2.5 py-1 text-[11px] font-medium text-ink-secondary shadow-[0_1px_2px_rgba(51,43,41,0.03)]">
              <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dot}`} />
              {saveMeta.label}
            </span>
            {editor && (
              <>
                <span
                  className={`inline-flex shrink-0 items-center gap-1 whitespace-nowrap rounded-md px-2 py-0.5 text-[10.5px] font-medium ${kindBadgeClass(editor.kind)}`}
                >
                  {contentKindLabel(editor.kind)}
                </span>
                <span className="hidden truncate font-mono text-[10.5px] text-ink-muted 2xl:inline">
                  {editor.path}
                </span>
              </>
            )}
          </div>

          <div className="flex shrink-0 items-center gap-2">
            {(editor?.kind === "article" || editor?.kind === "imageArticle") && (
              <div className="flex items-center rounded-lg bg-subtle p-0.5 ring-1 ring-line/70">
                <button
                  type="button"
                  onClick={() => setViewMode("write")}
                  className={`cursor-pointer rounded-md px-2.5 py-1 text-[11px] font-medium transition-all ${
                    viewMode === "write"
                      ? "bg-surface text-ink shadow-[0_1px_2px_rgba(51,43,41,0.06)]"
                      : "text-ink-muted hover:text-ink"
                  }`}
                >
                  编辑
                </button>
                <button
                  type="button"
                  onClick={() => setViewMode("preview")}
                  className={`cursor-pointer rounded-md px-2.5 py-1 text-[11px] font-medium transition-all ${
                    viewMode === "preview"
                      ? "bg-surface text-ink shadow-[0_1px_2px_rgba(51,43,41,0.06)]"
                      : "text-ink-muted hover:text-ink"
                  }`}
                >
                  预览
                </button>
              </div>
            )}

            {editor && (
              <div ref={deriveMenuRef} className="relative">
                <button
                  type="button"
                  onClick={() => setDeriveOpen((open) => !open)}
                  className="inline-flex h-8 cursor-pointer items-center gap-1.5 whitespace-nowrap rounded-lg border border-line bg-surface px-3 text-xs font-medium text-ink-secondary shadow-[0_1px_2px_rgba(51,43,41,0.03)] transition-colors hover:border-line-strong hover:text-ink"
                >
                  派生形态
                </button>
                {deriveOpen && (
                  <div className="absolute top-full right-0 z-30 mt-1.5 w-48 overflow-hidden rounded-xl border border-line bg-surface p-1 shadow-[0_12px_32px_rgba(51,43,41,0.12)]">
                    <div className="px-2.5 py-1.5 text-[10px] font-medium text-ink-muted">
                      从当前主稿派生新形态
                    </div>
                    {CONTENT_KIND_META.filter((item) => item.kind !== editor.kind).map((item) => {
                      const Icon = KIND_ICONS[item.kind];
                      return (
                        <button
                          key={item.kind}
                          type="button"
                          onClick={() => {
                            setDeriveOpen(false);
                            void derivePackage(item.kind);
                          }}
                          className="flex w-full cursor-pointer items-center gap-2 rounded-lg px-2.5 py-2 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                        >
                          <Icon size={11} className="text-ink-muted" />
                          <span>{item.label}</span>
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>
            )}

            <button
              type="button"
              onClick={() => void save()}
              disabled={saveState === "saving" || saveState === "saved"}
              className="inline-flex h-8 cursor-pointer items-center whitespace-nowrap rounded-lg bg-accent px-3.5 text-xs font-medium text-accent-on shadow-[0_1px_4px_rgba(123,104,168,0.22)] transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-40"
            >
              保存
            </button>

            {!rightTab && (
              <button
                type="button"
                onClick={() => setRightTab("drafts")}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 whitespace-nowrap rounded-lg border border-line bg-surface px-2.5 text-xs font-medium text-ink-secondary transition-colors hover:border-accent-border hover:bg-accent-soft/40 hover:text-accent-deep"
                title="展开右侧工作栏"
              >
                <PanelLeftOpen size={12} />
                工作栏
              </button>
            )}
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto">
          {editor ? (
            <div className="px-7 py-7 lg:px-10 lg:py-8">
              <ContentEditor preview={viewMode === "preview"} />
            </div>
          ) : (
            <div className="grid h-full place-items-center p-8">
              <div className="max-w-[540px] rounded-2xl border border-line bg-surface p-8 text-center shadow-[0_4px_24px_rgba(51,43,41,0.04)]">
                <div className="mx-auto grid h-12 w-12 place-items-center rounded-2xl border border-accent-border bg-accent-soft text-accent-deep">
                  <Plus size={18} />
                </div>
                <h2 className="mt-4 text-base font-semibold tracking-tight text-ink">
                  创建第一份内容主稿
                </h2>
                <p className="mt-1.5 text-xs leading-relaxed text-ink-muted">
                  支持视频、图文、图文文章和纯文章。创建后可自动保存、生成多平台适配稿、检查规则并调用
                  BYOK 模型。
                </p>
                <div className="mt-6 grid grid-cols-2 gap-2.5">
                  {CONTENT_KIND_META.map((item) => {
                    const Icon = KIND_ICONS[item.kind];
                    return (
                      <button
                        key={item.kind}
                        type="button"
                        onClick={() => openCreation(item.kind)}
                        className="group flex cursor-pointer items-start gap-2.5 rounded-xl border border-line bg-canvas/70 p-3.5 text-left transition-all hover:border-accent hover:bg-surface hover:shadow-sm"
                      >
                        <span
                          className={`mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-lg ${kindBadgeClass(item.kind)}`}
                        >
                          <Icon size={12} />
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

        <footer className="flex h-9 shrink-0 items-center justify-between border-t border-line bg-surface/75 px-6 text-[11px] text-ink-muted">
          <div className="flex items-center gap-3">
            <span className="inline-flex items-center gap-1.5 whitespace-nowrap">
              <Clock3 size={11} />
              {editor ? `版本 ${editor.contentVersion.slice(0, 8)}` : "无内容版本"}
            </span>
            {editor && (
              <>
                <span className="text-ink-faint">·</span>
                <span className="whitespace-nowrap">
                  {editor.media.length} 个媒体 · {editor.tags.length} 个标签
                </span>
              </>
            )}
          </div>
          <div className="flex items-center gap-3.5">
            <button
              type="button"
              onClick={() => void handleSimulateFailure()}
              className="cursor-pointer whitespace-nowrap text-[10.5px] text-ink-faint transition-colors hover:text-ink-muted"
              title="M5 验收用：下一次保存将显示失败状态且保留编辑内容"
            >
              故障演练
            </button>
            <button
              type="button"
              onClick={() => void loadPackages()}
              className="inline-flex cursor-pointer items-center gap-1 whitespace-nowrap transition-colors hover:text-ink"
            >
              <RefreshCw size={10} />
              刷新
            </button>
            <span className="inline-flex items-center gap-1 whitespace-nowrap">
              <Info size={10} />
              自动保存
            </span>
          </div>
        </footer>
      </main>

      {/* Right Column: Inspector Panel */}
      {rightTab && (
        <aside className="flex w-[340px] shrink-0 flex-col border-l border-line bg-surface-soft xl:w-[368px]">
          <div className="flex h-13 shrink-0 items-center justify-between gap-2 border-b border-line bg-surface/85 px-3 backdrop-blur-sm">
            <div className="flex items-center gap-0.5 rounded-xl bg-subtle p-1 ring-1 ring-line/60">
              {(
                [
                  ["drafts", "平台稿", Send],
                  ["ai", "AI 辅助", Sparkles],
                  ["checks", "内容检查", Check],
                ] as const
              ).map(([id, label, Icon]) => {
                const active = rightTab === id;
                return (
                  <button
                    key={id}
                    type="button"
                    onClick={() => setRightTab(id)}
                    className={`inline-flex cursor-pointer items-center gap-1.5 whitespace-nowrap rounded-lg px-2.5 py-1 text-[11px] font-medium transition-all ${
                      active
                        ? "bg-surface text-ink shadow-[0_1px_3px_rgba(51,43,41,0.08)]"
                        : "text-ink-muted hover:text-ink"
                    }`}
                  >
                    <Icon size={10} className={active ? "text-accent" : "opacity-70"} />
                    {label}
                  </button>
                );
              })}
            </div>
            <div className="flex items-center gap-0.5">
              <button
                type="button"
                onClick={() => showToast("右侧工作栏可随时收起；主稿内容会继续自动保存")}
                className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-faint transition-colors hover:bg-hover hover:text-ink-muted"
                aria-label="面板帮助"
                title="工作栏说明"
              >
                <Info size={12} />
              </button>
              <button
                type="button"
                onClick={() => setRightTab(null)}
                className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-muted transition-colors hover:bg-hover hover:text-ink"
                aria-label="关闭右侧面板"
                title="收起工作栏"
              >
                <X size={12} />
              </button>
            </div>
          </div>
          <div className="min-h-0 flex-1">
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
