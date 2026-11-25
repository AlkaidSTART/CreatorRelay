import { useEffect, useMemo, useState } from "react";
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
      return "bg-amber-100 text-amber-800";
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
      ? { label: "保存中...", dot: "bg-amber-500" }
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
      <div className="flex h-full items-center justify-center bg-surface-soft p-8">
        <StudioWorkspaceOnboarding onChooseWorkspace={() => void chooseWorkspace()} />
      </div>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden bg-surface-soft">
      {saveState === "error" && (
        <div className="fixed top-3 left-1/2 z-40 flex max-w-[680px] -translate-x-1/2 items-start gap-3 rounded-xl border border-coral-border bg-coral-bg px-4 py-2.5 text-xs text-coral-text shadow-lg">
          <AlertTriangle size={13} className="mt-0.5 shrink-0" />
          <div className="min-w-0 flex-1">
            <strong className="block font-medium">保存失败，未保存内容仍在编辑器中</strong>
            <span className="mt-0.5 block break-words text-[11px] opacity-90">{saveError}</span>
          </div>
          <button
            type="button"
            onClick={() => void save()}
            className="shrink-0 rounded-lg bg-white/70 px-2 py-1 text-[10.5px] font-medium"
          >
            重试保存
          </button>
        </div>
      )}

      <aside className="flex w-[248px] shrink-0 flex-col border-r border-line bg-sidebar">
        <div className="border-b border-line p-3">
          <div className="flex items-center justify-between">
            <div>
              <h1 className="text-sm font-semibold tracking-tight text-ink">内容工作室</h1>
              <p className="mt-0.5 text-[10px] text-ink-muted">四形态主稿与平台适配</p>
            </div>
            <button
              type="button"
              onClick={() => setIsCreating(true)}
              className="grid h-7 w-7 place-items-center rounded-lg bg-accent text-accent-on shadow-sm"
              aria-label="新建内容"
            >
              <Plus size={12} />
            </button>
          </div>
          <div className="mt-2.5 flex items-center gap-1.5 rounded-lg border border-line bg-surface px-2 py-1.5">
            <Search size={11} className="text-ink-muted" />
            <input
              data-studio-search
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="搜索主稿、摘要、标签"
              className="min-w-0 flex-1 bg-transparent text-[11px] text-ink placeholder:text-ink-muted focus:outline-none"
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-1 border-b border-line px-2.5 py-2">
          {[
            { id: "ALL" as const, label: "全部" },
            ...CONTENT_KIND_META.map((item) => ({ id: item.kind, label: item.label })),
          ].map((item) => (
            <button
              key={item.id}
              type="button"
              onClick={() => setFilter(item.id)}
              className={`rounded-md px-1.5 py-1 text-[9.5px] font-medium ${
                filter === item.id
                  ? "bg-accent-soft text-accent-deep"
                  : "text-ink-muted hover:bg-hover hover:text-ink"
              }`}
            >
              {item.label}
            </button>
          ))}
        </div>

        <div className="min-h-0 flex-1 overflow-y-auto p-2">
          {loading ? (
            <p className="px-2 py-6 text-center text-[11px] text-ink-muted">正在载入内容包...</p>
          ) : filtered.length === 0 ? (
            <div className="rounded-xl border border-dashed border-line bg-canvas p-3 text-center">
              <p className="text-[11px] text-ink-muted">
                {packages.length === 0 ? "工作区还没有内容主稿" : "没有匹配的主稿"}
              </p>
              <div className="mt-2 grid grid-cols-2 gap-1">
                {CONTENT_KIND_META.map((item) => (
                  <button
                    key={item.kind}
                    type="button"
                    onClick={() => openCreation(item.kind)}
                    className="rounded-md border border-line bg-surface px-1 py-1 text-[9.5px] text-ink-secondary hover:border-accent"
                  >
                    + {item.label}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <div className="space-y-1">
              {filtered.map((item) => {
                const Icon = KIND_ICONS[item.kind];
                return (
                  <button
                    key={item.id}
                    type="button"
                    onClick={() => void selectPackage(item.id)}
                    className={`flex w-full items-start gap-2 rounded-xl border px-2.5 py-2 text-left ${
                      selectedId === item.id
                        ? "border-accent-border bg-surface shadow-sm"
                        : "border-transparent hover:bg-hover"
                    }`}
                  >
                    <span className="mt-0.5 grid h-6 w-6 shrink-0 place-items-center rounded-lg bg-accent-soft text-accent-deep">
                      <Icon size={10} />
                    </span>
                    <span className="min-w-0 flex-1">
                      <strong className="block truncate text-[11px] font-medium text-ink">
                        {item.title || "未命名主稿"}
                      </strong>
                      <span className="mt-0.5 flex items-center gap-1 text-[9.5px] text-ink-muted">
                        <span className={`rounded px-1 py-0.5 ${kindBadgeClass(item.kind)}`}>
                          {contentKindLabel(item.kind)}
                        </span>
                        <span>{item.mediaCount} 素材</span>
                        <span>·</span>
                        <span>{formatRelativeTime(item.updatedAt)}</span>
                      </span>
                    </span>
                  </button>
                );
              })}
            </div>
          )}
        </div>

        <div className="border-t border-line p-2.5">
          <button
            type="button"
            onClick={() => navigate("/studio/classic")}
            className="flex w-full items-center justify-between rounded-lg px-2 py-1.5 text-[10px] text-ink-muted hover:bg-hover hover:text-ink"
          >
            <span className="inline-flex items-center gap-1.5">
              <History size={10} />
              经典 Markdown 工作区
            </span>
            <ChevronRight size={9} />
          </button>
        </div>
      </aside>

      <main className="flex min-w-0 flex-1 flex-col">
        <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line bg-surface-soft px-4">
          <div className="flex min-w-0 items-center gap-2">
            <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dot}`} />
            <span className="text-[11px] font-medium text-ink-secondary">{saveMeta.label}</span>
            {editor && (
              <>
                <span className="text-ink-faint">·</span>
                <span className="truncate font-mono text-[10px] text-ink-muted">
                  {editor.path}
                </span>
              </>
            )}
          </div>
          <div className="flex shrink-0 items-center gap-1.5">
            {(editor?.kind === "article" || editor?.kind === "imageArticle") && (
              <div className="flex items-center rounded-lg bg-hover p-0.5">
                <button
                  type="button"
                  onClick={() => setViewMode("write")}
                  className={`rounded-md px-2 py-1 text-[10px] font-medium ${
                    viewMode === "write" ? "bg-surface text-ink shadow-sm" : "text-ink-muted"
                  }`}
                >
                  编辑
                </button>
                <button
                  type="button"
                  onClick={() => setViewMode("preview")}
                  className={`rounded-md px-2 py-1 text-[10px] font-medium ${
                    viewMode === "preview" ? "bg-surface text-ink shadow-sm" : "text-ink-muted"
                  }`}
                >
                  预览
                </button>
              </div>
            )}
            {editor && (
              <div className="relative">
                <button
                  type="button"
                  onClick={() => setDeriveOpen((open) => !open)}
                  className="rounded-lg border border-line bg-surface px-2 py-1.5 text-[10px] font-medium text-ink-secondary hover:border-accent"
                >
                  派生形态
                </button>
                {deriveOpen && (
                  <div className="absolute top-full right-0 z-30 mt-1 w-40 overflow-hidden rounded-xl border border-line bg-surface shadow-xl">
                    {CONTENT_KIND_META.filter((item) => item.kind !== editor.kind).map((item) => (
                      <button
                        key={item.kind}
                        type="button"
                        onClick={() => {
                          setDeriveOpen(false);
                          void derivePackage(item.kind);
                        }}
                        className="block w-full px-3 py-2 text-left text-[10.5px] text-ink-secondary hover:bg-hover hover:text-ink"
                      >
                        {item.label}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            )}
            <button
              type="button"
              onClick={() => {
                setRightTab("drafts");
              }}
              className={`inline-flex items-center gap-1 rounded-lg px-2 py-1.5 text-[10px] font-medium ${
                rightTab === "drafts"
                  ? "bg-accent-soft text-accent-deep"
                  : "text-ink-secondary hover:bg-hover"
              }`}
            >
              <Send size={10} /> 平台稿
            </button>
            <button
              type="button"
              onClick={() => setRightTab("ai")}
              className={`inline-flex items-center gap-1 rounded-lg px-2 py-1.5 text-[10px] font-medium ${
                rightTab === "ai" ? "bg-accent-soft text-accent-deep" : "text-ink-secondary hover:bg-hover"
              }`}
            >
              <Sparkles size={10} /> AI
            </button>
            <button
              type="button"
              onClick={() => setRightTab("checks")}
              className={`inline-flex items-center gap-1 rounded-lg px-2 py-1.5 text-[10px] font-medium ${
                rightTab === "checks" ? "bg-accent-soft text-accent-deep" : "text-ink-secondary hover:bg-hover"
              }`}
            >
              <Check size={10} /> 检查
            </button>
            <button
              type="button"
              onClick={() => void save()}
              disabled={saveState === "saving" || saveState === "saved"}
              className="rounded-lg bg-accent px-2.5 py-1.5 text-[10px] font-medium text-accent-on disabled:opacity-40"
            >
              保存
            </button>
            {rightTab && (
              <button
                type="button"
                onClick={() => setRightTab(null)}
                className="grid h-7 w-7 place-items-center rounded text-ink-muted hover:bg-hover"
                aria-label="关闭右侧面板"
              >
                <X size={11} />
              </button>
            )}
          </div>
        </header>

        <div className="min-h-0 flex-1 overflow-y-auto">
          {editor ? (
            <div className="px-6 py-6 sm:px-10">
              <ContentEditor preview={viewMode === "preview"} />
            </div>
          ) : (
            <div className="grid h-full place-items-center p-8">
              <div className="max-w-[520px] rounded-2xl border border-dashed border-line bg-canvas p-8 text-center">
                <div className="mx-auto grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-accent-deep">
                  <Plus size={18} />
                </div>
                <h2 className="mt-3 text-sm font-semibold text-ink">创建第一份内容主稿</h2>
                <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
                  支持视频、图文、图文文章和纯文章。创建后可自动保存、生成平台稿、检查规则并调用
                  BYOK 模型。
                </p>
                <div className="mt-4 grid grid-cols-2 gap-2">
                  {CONTENT_KIND_META.map((item) => (
                    <button
                      key={item.kind}
                      type="button"
                      onClick={() => openCreation(item.kind)}
                      className="rounded-xl border border-line bg-surface px-3 py-2 text-left hover:border-accent"
                    >
                      <strong className="block text-[11px] font-medium text-ink">
                        {item.label}
                      </strong>
                      <span className="mt-0.5 block text-[9.5px] leading-snug text-ink-muted">
                        {item.description}
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>

        <footer className="flex h-8 shrink-0 items-center justify-between border-t border-line bg-surface-soft px-4 text-[10px] text-ink-muted">
          <div className="flex items-center gap-3">
            <span className="inline-flex items-center gap-1">
              <Clock3 size={10} />
              {editor ? `内容版本 ${editor.contentVersion.slice(0, 8)}` : "无内容版本"}
            </span>
            {editor && <span>{editor.media.length} 个媒体 · {editor.tags.length} 个标签</span>}
          </div>
          <div className="flex items-center gap-3">
            <button
              type="button"
              onClick={() => void handleSimulateFailure()}
              className="text-[9.5px] text-ink-faint hover:text-ink-muted"
              title="M5 验收用：下一次保存将显示失败状态且保留编辑内容"
            >
              故障演练
            </button>
            <button
              type="button"
              onClick={() => void loadPackages()}
              className="inline-flex items-center gap-1 hover:text-ink"
            >
              <RefreshCw size={9} />
              刷新
            </button>
            <span className="inline-flex items-center gap-1">
              <Info size={9} />
              自动保存
            </span>
          </div>
        </footer>
      </main>

      {rightTab && (
        <aside className="flex w-[372px] shrink-0 flex-col border-l border-line bg-canvas">
          <div className="flex h-10 items-center gap-1 border-b border-line px-2">
            {[
              ["drafts", "平台稿"],
              ["ai", "AI 辅助"],
              ["checks", "内容检查"],
            ].map(([id, label]) => (
              <button
                key={id}
                type="button"
                onClick={() => setRightTab(id as RightTab)}
                className={`rounded-lg px-2.5 py-1 text-[10.5px] font-medium ${
                  rightTab === id
                    ? "bg-surface text-ink shadow-sm"
                    : "text-ink-muted hover:text-ink"
                }`}
              >
                {label}
              </button>
            ))}
            <button
              type="button"
              onClick={() => showToast("右侧面板可随时收起；内容会继续自动保存")}
              className="ml-auto text-ink-faint hover:text-ink-muted"
              aria-label="面板帮助"
            >
              <Info size={11} />
            </button>
          </div>
          <div className="min-h-0 flex-1">
            {rightTab === "drafts" && <PlatformDraftPanel onShowChecks={() => setRightTab("checks")} />}
            {rightTab === "ai" && <AiAssistantPanel />}
            {rightTab === "checks" && <ContentChecksPanel />}
          </div>
        </aside>
      )}
    </div>
  );
}
