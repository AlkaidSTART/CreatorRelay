import { useEffect, useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readFile, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  AlignLeft,
  ArrowDownToLine,
  ArrowRight,
  ArrowUpRight,
  BookOpen,
  Check,
  ChevronRight,
  Clock3,
  FilePlus2,
  FolderOpen,
  History,
  Image,
  Info,
  Plus,
  Search,
  Send,
  Sparkles,
  Tags,
  X,
} from "../components/Icons";
import { PlatformLogo } from "../components/PlatformLogo";
import {
  StudioNoDocumentState,
  StudioWorkspaceOnboarding,
} from "../components/StudioWorkspaceEmpty";
import { useNavigate } from "react-router";
import { PLATFORM_CATEGORIES } from "../data/demoData";
import { useContentStore, usePlatformStore, useUiStore, useWorkspaceStore } from "../stores";
import { formatRelativeTime } from "../utils/format";
import { bodyFromMarkdown } from "../utils/markdown";
import { getPlatformVisual } from "../utils/platformStyle";
import "./StudioPage.css";

function extractFrontmatterBlock(markdown: string): { frontmatter: string; body: string } {
  if (markdown.startsWith("---\n")) {
    const end = markdown.indexOf("\n---", 4);
    if (end >= 0) {
      const prefix = markdown.slice(0, end + 5);
      const rest = markdown.slice(end + 5).replace(/^\n/, "");
      return { frontmatter: prefix, body: rest };
    }
  }
  return { frontmatter: "", body: markdown };
}

export function StudioPage() {
  const navigate = useNavigate();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const searchBoxRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [showFrontmatter, setShowFrontmatter] = useState(false);
  const [searchOpen, setSearchOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const [selectedRevisionId, setSelectedRevisionId] = useState("");
  const [revisionLabel, setRevisionLabel] = useState("");
  const [assetPreviewUrl, setAssetPreviewUrl] = useState("");
  const [assetPreviewMime, setAssetPreviewMime] = useState("");
  const [documentAssets, setDocumentAssets] = useState<
    Array<{
      path: string;
      mimeType: string;
      size: number;
      available: boolean;
      warning: string | null;
    }>
  >([]);

  const workspace = useWorkspaceStore((state) => state.workspace);
  const documents = useWorkspaceStore((state) => state.documents);
  const selectedId = useWorkspaceStore((state) => state.selectedId);
  const content = useWorkspaceStore((state) => state.content);
  const updateContent = useWorkspaceStore((state) => state.updateContent);
  const query = useWorkspaceStore((state) => state.query);
  const setQuery = useWorkspaceStore((state) => state.setQuery);
  const saveState = useWorkspaceStore((state) => state.saveState);
  const revisions = useWorkspaceStore((state) => state.revisions);
  const recentExternalChange = useWorkspaceStore((state) => state.recentExternalChange);
  const resolveExternalChange = useWorkspaceStore((state) => state.resolveExternalChange);
  const checkExternalChange = useWorkspaceStore((state) => state.checkExternalChange);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const selectDocumentSafely = useWorkspaceStore((state) => state.selectDocumentSafely);
  const restoreRevision = useWorkspaceStore((state) => state.restoreRevision);
  const createRevision = useWorkspaceStore((state) => state.createRevision);
  const loadDocumentAssets = useWorkspaceStore((state) => state.loadDocumentAssets);

  const allPlatformAccounts = usePlatformStore((state) => state.accounts);
  const platformAccounts = allPlatformAccounts.filter(
    (account) => account.status === "ACTIVE" && account.enabled !== false,
  );
  const publishTasks = useContentStore((state) => state.publishTasks);
  const createPublishTaskFromDraft = useContentStore((state) => state.createPublishTaskFromDraft);

  const viewMode = useUiStore((state) => state.viewMode);
  const setViewMode = useUiStore((state) => state.setViewMode);
  const rightPanel = useUiStore((state) => state.rightPanel);
  const setRightPanel = useUiStore((state) => state.setRightPanel);
  const toggleRightPanel = useUiStore((state) => state.toggleRightPanel);
  const setIsCreating = useUiStore((state) => state.setIsCreating);
  const showToast = useUiStore((state) => state.showToast);

  useEffect(() => {
    function handlePointerDown(event: MouseEvent) {
      if (searchBoxRef.current && !searchBoxRef.current.contains(event.target as Node)) {
        setSearchOpen(false);
      }
    }
    function handleKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setSearchOpen(true);
        searchInputRef.current?.focus();
      } else if (event.key === "Escape") {
        setSearchOpen(false);
        searchInputRef.current?.blur();
      }
    }
    document.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  useEffect(() => {
    if (!isTauri() || !workspace || !selectedId) return;
    void checkExternalChange();
    const timer = window.setInterval(() => void checkExternalChange(), 2000);
    return () => window.clearInterval(timer);
  }, [workspace?.path, selectedId, checkExternalChange]);

  const clampedActiveIndex = documents.length > 0 ? Math.min(activeIndex, documents.length - 1) : 0;

  const isMissingWorkspace = isTauri() && !workspace;

  const selected = useMemo(
    () => documents.find((doc) => doc.id === selectedId) ?? null,
    [documents, selectedId],
  );

  useEffect(() => {
    if (!selected || !isTauri() || saveState !== "saved") {
      if (!selected) setDocumentAssets([]);
      return;
    }
    void loadDocumentAssets(selected.path).then(setDocumentAssets);
  }, [selected?.path, saveState, loadDocumentAssets]);

  const { frontmatter, body: editableBody } = useMemo(
    () => extractFrontmatterBlock(content),
    [content],
  );
  const body = useMemo(() => bodyFromMarkdown(content), [content]);
  const chars = useMemo(() => body.replace(/\s/gu, "").length, [body]);
  const readingMinutes = Math.max(1, Math.ceil(chars / 400));

  function handleBodyChange(nextBody: string) {
    if (frontmatter) {
      updateContent(`${frontmatter}\n${nextBody}`);
    } else {
      updateContent(nextBody);
    }
  }

  async function ensureCurrentDraftSaved() {
    const store = useWorkspaceStore.getState();
    if (store.saveState === "saving" || store.saveState === "error") {
      showToast("请先完成当前文章的保存，再继续此操作", 2400);
      return false;
    }
    if (store.saveState === "unsaved") {
      const saved = await store.saveCurrentDocument({ silent: true });
      if (!saved) return false;
    }
    return true;
  }

  async function handleExportMarkdown() {
    if (!selected) return;
    if (!isTauri()) {
      showToast("请在桌面端导出 Markdown 文件");
      return;
    }
    if (!(await ensureCurrentDraftSaved())) return;
    try {
      const target = await save({
        defaultPath: `${selected.title.replace(/[\\/:*?"<>|]/gu, "-")}.md`,
        filters: [{ name: "Markdown", extensions: ["md"] }],
      });
      if (!target) return;
      await writeTextFile(target, content);
      showToast("Markdown 已导出");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleExportBundle() {
    if (!selected || !isTauri()) {
      showToast("请在桌面端选择文章后导出内容包");
      return;
    }
    if (!(await ensureCurrentDraftSaved())) return;
    try {
      const output = await open({
        directory: true,
        multiple: false,
        title: "选择内容包导出父目录",
      });
      if (typeof output !== "string") return;
      const folderName = selected.title.replace(/[\\/:*?"<>|]/gu, "-").slice(0, 80) || "article";
      await invoke("export_document_bundle", {
        documentPath: selected.path,
        destinationPath: `${output}/${folderName}-content-package`,
      });
      showToast("Markdown 与引用素材已导出为内容包");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleImportBundle() {
    if (!isTauri()) {
      showToast("请在桌面端导入内容包");
      return;
    }
    if (!(await ensureCurrentDraftSaved())) return;
    try {
      const source = await open({
        directory: true,
        multiple: false,
        title: "选择 CreatorRelay 导出的文章内容包",
      });
      if (typeof source !== "string") return;
      const imported = await invoke<{
        id: string;
        title: string;
        path: string;
        updatedAt: string;
        preview: string;
        wordCount: number;
        charCount: number;
        tags: string[];
        contentHash: string;
        assets: Array<{
          path: string;
          mimeType: string;
          size: number;
          available: boolean;
          warning: string | null;
        }>;
      }>("import_document_bundle", { sourcePath: source, title: null });
      await useWorkspaceStore.getState().loadDocuments();
      useWorkspaceStore.setState({ selectedId: imported.id });
      await useWorkspaceStore.getState().loadDocument(imported);
      await useWorkspaceStore.getState().loadRevisions(imported.path);
      showToast("内容包及素材已导入工作区");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleImportMarkdown() {
    if (!isTauri()) {
      showToast("请在桌面端导入 Markdown 文件");
      return;
    }
    if (!(await ensureCurrentDraftSaved())) return;
    try {
      const source = await open({
        multiple: false,
        filters: [{ name: "Markdown", extensions: ["md", "markdown"] }],
      });
      if (typeof source !== "string") return;
      const imported = await invoke<{
        id: string;
        title: string;
        path: string;
        updatedAt: string;
        preview: string;
        wordCount: number;
        charCount: number;
        tags: string[];
        contentHash: string;
      }>("import_markdown", {
        sourceName: source.split(/[\\/]/u).pop() || "import.md",
        content: await readTextFile(source),
      });
      const store = useWorkspaceStore.getState();
      await store.loadDocuments();
      useWorkspaceStore.setState({ selectedId: imported.id });
      await useWorkspaceStore.getState().loadDocument(imported);
      await useWorkspaceStore.getState().loadRevisions(imported.path);
      showToast("Markdown 已导入到工作区");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleRevealAsset(path: string) {
    if (!selected || !isTauri()) return;
    try {
      const absolutePath = await invoke<string>("reveal_workspace_asset", {
        documentPath: selected.path,
        assetPath: path,
      });
      await revealItemInDir(absolutePath);
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleImportAsset() {
    if (!selected || !isTauri()) {
      showToast("请在桌面工作区选择文章后导入素材");
      return;
    }
    try {
      const source = await open({
        multiple: false,
        filters: [
          {
            name: "图片、视频与附件",
            extensions: [
              "png",
              "jpg",
              "jpeg",
              "gif",
              "webp",
              "svg",
              "mp4",
              "mov",
              "webm",
              "pdf",
              "txt",
            ],
          },
        ],
      });
      if (typeof source !== "string") return;
      const file = await readFile(source);
      if (file.byteLength > 256 * 1024 * 1024) {
        showToast("素材不能超过 256 MiB", 2600);
        return;
      }
      const currentMarkdown = useWorkspaceStore.getState().content;
      const parts = extractFrontmatterBlock(currentMarkdown);
      const currentBody = parts.body;
      const result = await invoke<{ markdown: string; path: string; mimeType: string }>(
        "import_workspace_asset",
        {
          documentPath: selected.path,
          sourceName: source.split(/[\\/]/u).pop() || "asset.bin",
          bytes: Array.from(file),
        },
      );
      const separator = currentBody.trim() ? "\n\n" : "";
      useWorkspaceStore
        .getState()
        .updateContent(
          `${parts.frontmatter}${separator}${currentBody}${separator}${result.markdown}\n`,
        );
      setDocumentAssets(await loadDocumentAssets(selected.path));
      showToast("素材已复制到文章 assets/ 并插入相对引用");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handlePreviewAsset(path: string) {
    if (!selected || !isTauri()) return;
    try {
      const asset = await invoke<{ bytes: number[]; mimeType: string }>("read_workspace_asset", {
        documentPath: selected.path,
        assetPath: path,
      });
      if (asset.mimeType === "image/svg+xml") {
        setAssetPreviewMime(asset.mimeType);
        setAssetPreviewUrl("svg-asset-located");
        return;
      }
      if (!asset.mimeType.startsWith("image/")) {
        showToast(`已定位素材（${asset.mimeType}）`);
        return;
      }
      const blob = new Blob([new Uint8Array(asset.bytes)], { type: asset.mimeType });
      setAssetPreviewUrl((current) => {
        if (current) URL.revokeObjectURL(current);
        return URL.createObjectURL(blob);
      });
      setAssetPreviewMime(asset.mimeType);
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    }
  }

  async function handleCreateDocument() {
    if (!selected) {
      setIsCreating(true);
      return;
    }
    if (!(await ensureCurrentDraftSaved())) return;
    setIsCreating(true);
  }

  const saveMeta =
    saveState === "saving"
      ? { label: "保存中...", dotClass: "bg-lemon-text" }
      : saveState === "unsaved"
        ? { label: "未保存", dotClass: "bg-lemon-text" }
        : saveState === "error"
          ? { label: "保存失败", dotClass: "bg-coral-text" }
          : { label: "已保存", dotClass: "bg-mint-text" };

  return (
    <div className="flex h-full w-full overflow-hidden bg-surface-soft">
      {recentExternalChange && (
        <div className="fixed top-3 left-1/2 z-40 flex -translate-x-1/2 items-center gap-3 rounded-xl border border-amber-300 bg-amber-50 px-4 py-2.5 text-xs text-amber-950 shadow-lg">
          <span>检测到外部修改：{recentExternalChange}</span>
          <button
            type="button"
            onClick={() => void resolveExternalChange("reload")}
            className="rounded-lg bg-amber-200 px-2.5 py-1 font-medium hover:bg-amber-300"
          >
            载入磁盘版本
          </button>
          <button
            type="button"
            disabled={saveState === "saving" || saveState === "error"}
            onClick={() => void resolveExternalChange("keep")}
            className="rounded-lg border border-amber-400 px-2.5 py-1 font-medium hover:bg-amber-100 disabled:opacity-50"
          >
            保留我的草稿
          </button>
        </div>
      )}
      {/* Center Distraction-Free Writing Area */}
      <main className="flex min-w-0 flex-1 flex-col bg-surface-soft">
        {/* Single Unified Header Toolbar */}
        <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-line bg-surface-soft px-4">
          <div className="flex min-w-0 items-center gap-3">
            {/* Consolidated Local Drafts Search Box & Popover */}
            <div ref={searchBoxRef} className="relative shrink-0">
              <div
                onClick={() => {
                  setSearchOpen(true);
                  if (!isMissingWorkspace) {
                    searchInputRef.current?.focus();
                  }
                }}
                className={`flex h-8 w-[200px] items-center gap-2 rounded-xl border bg-surface px-2.5 text-ink-muted transition-colors sm:w-[232px] ${
                  searchOpen
                    ? "border-accent shadow-2xs"
                    : "border-line hover:border-line-strong"
                } ${isMissingWorkspace ? "cursor-pointer opacity-75" : ""}`}
              >
                <Search size={12} className="shrink-0" />
                <input
                  ref={searchInputRef}
                  value={query}
                  onFocus={() => setSearchOpen(true)}
                  onChange={(event) => {
                    setQuery(event.target.value);
                    setActiveIndex(0);
                    setSearchOpen(true);
                  }}
                  onKeyDown={(event) => {
                    if (!searchOpen || documents.length === 0) return;
                    if (event.key === "ArrowDown") {
                      event.preventDefault();
                      setActiveIndex((prev) => (prev + 1) % documents.length);
                    } else if (event.key === "ArrowUp") {
                      event.preventDefault();
                      setActiveIndex((prev) => (prev - 1 + documents.length) % documents.length);
                    } else if (event.key === "Enter") {
                      event.preventDefault();
                      const target = documents[clampedActiveIndex];
                      if (target) {
                        void selectDocumentSafely(target);
                        setSearchOpen(false);
                        searchInputRef.current?.blur();
                      }
                    }
                  }}
                  disabled={isMissingWorkspace}
                  placeholder={isMissingWorkspace ? "连接工作区后可搜索..." : "搜索主稿..."}
                  aria-label="搜索文章"
                  aria-expanded={searchOpen}
                  className="min-w-0 flex-1 bg-transparent text-xs text-ink placeholder:text-ink-muted focus:outline-none disabled:cursor-pointer"
                />
                {query ? (
                  <button
                    type="button"
                    onClick={(event) => {
                      event.stopPropagation();
                      setQuery("");
                      setActiveIndex(0);
                      searchInputRef.current?.focus();
                    }}
                    aria-label="清空搜索"
                    className="cursor-pointer text-ink-muted hover:text-ink"
                  >
                    <X size={12} />
                  </button>
                ) : (
                  <span className="shrink-0 rounded-md bg-sidebar px-1.5 py-0.5 font-mono text-[10px] text-ink-muted">
                    ⌘K
                  </span>
                )}
              </div>

              {searchOpen && (
                <div className="absolute top-full left-0 z-30 mt-1.5 flex max-h-[min(70vh,520px)] w-[320px] flex-col overflow-hidden rounded-2xl border border-line bg-canvas shadow-lg">
                  <div className="flex h-10 shrink-0 items-center justify-between border-b border-line px-3.5">
                    <div className="flex items-center gap-2">
                      <h2 className="text-xs font-semibold text-ink">本地主稿</h2>
                      <span className="rounded-full bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent-deep">
                        {documents.length}
                      </span>
                    </div>
                    <button
                      type="button"
                      className={`grid h-7 w-7 place-items-center rounded-md transition-colors ${
                        isMissingWorkspace
                          ? "cursor-not-allowed text-ink-faint"
                          : "cursor-pointer text-ink-secondary hover:bg-hover hover:text-ink"
                      }`}
                      aria-label="新建文章"
                      title={isMissingWorkspace ? "请先选择本地工作区" : "新建文章"}
                      disabled={isMissingWorkspace}
                      onClick={() => {
                        setSearchOpen(false);
                        setIsCreating(true);
                      }}
                    >
                      <FilePlus2 size={14} />
                    </button>
                  </div>

                  <div className="flex-1 space-y-1 overflow-y-auto px-2.5 py-2" role="list">
                    {documents.map((doc, index) => {
                      const isSelected = doc.id === selectedId;
                      const isActive = index === clampedActiveIndex;
                      return (
                        <button
                          key={doc.id}
                          type="button"
                          role="listitem"
                          onMouseEnter={() => setActiveIndex(index)}
                          onClick={() => {
                            void selectDocumentSafely(doc);
                            setSearchOpen(false);
                          }}
                          className={`relative block w-full cursor-pointer rounded-xl border px-3 py-2.5 text-left transition-all ${
                            isSelected
                              ? "border-accent-border bg-surface shadow-2xs"
                              : isActive
                                ? "border-transparent bg-hover/70"
                                : "border-transparent bg-transparent hover:bg-hover/60"
                          }`}
                        >
                          {isSelected && (
                            <span className="absolute top-2.5 bottom-2.5 left-0 w-[3px] rounded-r-full bg-accent" />
                          )}
                          <div className="flex items-center justify-between gap-2 text-[11px] text-ink-muted">
                            <span>{formatRelativeTime(doc.updatedAt)}</span>
                            <span>{doc.charCount || doc.wordCount} 字</span>
                          </div>
                          <strong
                            className={`mt-1 line-clamp-2 block text-[13px] leading-snug ${
                              isSelected ? "font-semibold text-ink" : "font-medium text-ink"
                            }`}
                          >
                            {doc.title}
                          </strong>
                          <p className="mt-1 line-clamp-1 text-[11.5px] text-ink-muted">
                            {doc.preview}
                          </p>
                          {doc.tags.length > 0 && (
                            <div className="mt-2 flex flex-wrap gap-1">
                              {doc.tags.slice(0, 2).map((tag) => (
                                <span
                                  key={tag}
                                  className={`rounded-md px-1.5 py-0.5 text-[10px] ${
                                    isSelected
                                      ? "bg-accent-soft text-accent-deep"
                                      : "bg-subtle text-ink-secondary"
                                  }`}
                                >
                                  {tag}
                                </span>
                              ))}
                            </div>
                          )}
                        </button>
                      );
                    })}
                    {!documents.length && (
                      <div className="mx-1 my-3 rounded-2xl border border-dashed border-line-strong/75 bg-surface-soft/75 px-4 py-6 text-center">
                        {isMissingWorkspace ? (
                          <>
                            <div className="mx-auto grid h-8 w-8 place-items-center rounded-xl bg-accent-soft text-accent-deep">
                              <FolderOpen size={13} />
                            </div>
                            <p className="mt-2.5 text-xs font-medium text-ink">尚未连接工作区</p>
                            <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
                              选择本地文件夹后，主稿会自动显示在这里
                            </p>
                          </>
                        ) : query.trim() ? (
                          <>
                            <p className="text-xs font-medium text-ink">没有找到匹配的主稿</p>
                            <button
                              type="button"
                              onClick={() => setQuery("")}
                              className="mt-2.5 inline-flex cursor-pointer items-center gap-1 rounded-lg bg-accent-soft px-2.5 py-1 text-[11px] font-medium text-accent-deep transition-colors hover:bg-accent-border"
                            >
                              清空搜索
                            </button>
                          </>
                        ) : (
                          <>
                            <p className="text-xs font-medium text-ink">工作区还是空的</p>
                            <p className="mt-1 text-[11px] text-ink-muted">
                              写下你的第一篇 Markdown 主稿
                            </p>
                            <button
                              type="button"
                              onClick={() => {
                                setSearchOpen(false);
                                setIsCreating(true);
                              }}
                              className="mt-3 inline-flex cursor-pointer items-center gap-1 rounded-lg bg-accent-soft px-2.5 py-1 text-[11px] font-medium text-accent-deep transition-colors hover:bg-accent-border"
                            >
                              <Plus size={11} />
                              新建文章
                            </button>
                          </>
                        )}
                      </div>
                    )}
                  </div>

                  {/* Bottom Local Workspace Info */}
                  <div className="shrink-0 border-t border-line p-2.5">
                    <button
                      type="button"
                      onClick={() => {
                        setSearchOpen(false);
                        void chooseWorkspace();
                      }}
                      aria-label={workspace ? "更换工作区" : "选择本地工作区"}
                      title={workspace ? "更换工作区" : "选择本地工作区"}
                      className="group flex w-full cursor-pointer items-center gap-2.5 rounded-xl border border-line bg-surface px-2.5 py-2 text-left transition-colors hover:border-line-strong hover:bg-surface-soft"
                    >
                      <div
                        className={`grid h-7 w-7 shrink-0 place-items-center rounded-lg ${
                          workspace
                            ? "bg-mint-bg text-mint-text"
                            : "bg-accent-soft text-accent-deep"
                        }`}
                      >
                        <FolderOpen size={12} />
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-1.5">
                          <span
                            className={`h-1.5 w-1.5 rounded-full ${
                              workspace ? "bg-mint-text" : "bg-ink-faint"
                            }`}
                          />
                          <span className="text-[11px] font-medium text-ink">
                            {workspace ? workspace.name || "本地工作区" : "本地工作区"}
                          </span>
                        </div>
                        <p className="truncate text-[10px] text-ink-muted">
                          {workspace?.path ?? "点击选择本地文件夹"}
                        </p>
                      </div>
                      <span className="grid h-6 w-6 shrink-0 place-items-center rounded text-ink-muted transition-colors group-hover:bg-sidebar group-hover:text-ink">
                        <ArrowUpRight size={12} />
                      </span>
                    </button>
                  </div>
                </div>
              )}
            </div>

            <div className="flex min-w-0 items-center gap-2">
              <h1
                className={`truncate text-[13px] font-medium ${
                  isMissingWorkspace ? "text-ink-muted" : "text-ink"
                }`}
              >
                {isMissingWorkspace ? "等待连接本地工作区" : (selected?.title ?? "未选择文章")}
              </h1>
              {selected && (
                <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full bg-sidebar px-2 py-0.5 text-[11px] text-ink-secondary">
                  <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dotClass}`} />
                  {saveMeta.label}
                </span>
              )}
            </div>
          </div>

          {!isMissingWorkspace && (
            <>
              {/* Segmented Write / Preview Switch */}
              <div className="flex shrink-0 items-center rounded-xl bg-sidebar p-0.5">
                <button
                  onClick={() => setViewMode("write")}
                  className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-xs font-medium transition-all ${
                    viewMode === "write"
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-secondary hover:text-ink"
                  }`}
                >
                  <AlignLeft size={12} />
                  编辑
                </button>
                <button
                  onClick={() => setViewMode("preview")}
                  className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-xs font-medium transition-all ${
                    viewMode === "preview"
                      ? "bg-surface text-ink shadow-2xs"
                      : "text-ink-secondary hover:text-ink"
                  }`}
                >
                  <BookOpen size={12} />
                  预览
                </button>
              </div>

              {/* Right Toolbar Actions */}
              <div className="flex shrink-0 items-center gap-1">
                <button
                  onClick={() => toggleRightPanel("info")}
                  aria-label="文章信息"
                  title="文章信息"
                  className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg px-2 text-xs transition-colors ${
                    rightPanel === "info"
                      ? "bg-accent-soft font-medium text-accent-deep"
                      : "text-ink-secondary hover:bg-sidebar hover:text-ink"
                  }`}
                >
                  <Info size={13} />
                  <span className="hidden xl:inline">信息</span>
                </button>
                <button
                  onClick={() => toggleRightPanel("history")}
                  aria-label="版本历史"
                  title="版本历史"
                  className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-lg px-2 text-xs transition-colors ${
                    rightPanel === "history"
                      ? "bg-accent-soft font-medium text-accent-deep"
                      : "text-ink-secondary hover:bg-sidebar hover:text-ink"
                  }`}
                >
                  <History size={13} />
                  <span className="hidden xl:inline">历史</span>
                </button>
                <button
                  onClick={() => void handleImportMarkdown()}
                  aria-label="导入 Markdown"
                  title="导入 Markdown"
                  className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-secondary transition-colors hover:bg-sidebar hover:text-ink"
                >
                  <FilePlus2 size={13} />
                </button>
                <button
                  onClick={() => void handleImportBundle()}
                  aria-label="导入内容包"
                  title="导入文章内容包"
                  className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-secondary transition-colors hover:bg-sidebar hover:text-ink"
                >
                  <FilePlus2 size={13} />
                </button>
                <button
                  onClick={() => void handleExportBundle()}
                  aria-label="导出内容包"
                  title="导出文章与引用素材"
                  className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-secondary transition-colors hover:bg-sidebar hover:text-ink"
                >
                  <FolderOpen size={13} />
                </button>
                <button
                  onClick={() => void handleExportMarkdown()}
                  aria-label="导出"
                  title="导出 Markdown"
                  className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-secondary transition-colors hover:bg-sidebar hover:text-ink"
                >
                  <ArrowDownToLine size={13} />
                </button>
                <div className="mx-1 h-4 w-px bg-line" />
                <button
                  onClick={() => navigate("/publishing")}
                  className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover"
                >
                  发布准备
                  <ArrowRight size={12} />
                </button>
              </div>
            </>
          )}
        </header>

        {/* Subtle Multi-Platform Relay Strip */}
        {selected && (
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-line bg-canvas px-5 py-2">
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="mr-1 inline-flex items-center gap-1.5 text-[11px] font-medium text-ink-secondary">
                <Send size={11} className="text-ink-muted" />
                分发到平台：
              </span>
              {platformAccounts.map((account) => {
                const visual = getPlatformVisual(account.platformName);
                const existingTask = publishTasks.find(
                  (task) =>
                    task.articleTitle === selected.title && task.platform === account.platformName,
                );
                return (
                  <button
                    key={account.id}
                    type="button"
                    onClick={() => {
                      if (existingTask) {
                        navigate("/publishing");
                      } else {
                        createPublishTaskFromDraft({
                          articleTitle: selected.title,
                          platform: account.platformName,
                          platformMark: visual.mark,
                          accountAlias: account.alias,
                        });
                      }
                    }}
                    className={`inline-flex cursor-pointer items-center gap-1.5 rounded-lg border px-2 py-0.5 text-[11px] font-medium transition-all ${
                      existingTask
                        ? "border-accent-border bg-accent-soft text-accent-deep hover:bg-accent-border"
                        : "border-line bg-surface text-ink-secondary hover:border-line-strong hover:text-ink"
                    }`}
                    title={
                      existingTask
                        ? `已在 ${account.platformName} 发布队列中，点击前往查看`
                        : `为 ${account.platformName} 生成独立适配稿`
                    }
                  >
                    <PlatformLogo platformName={account.platformName} size={14} />
                    <span>{account.platformName}</span>
                    {existingTask ? (
                      <span className="text-[10px] opacity-80">
                        {existingTask.status === "SUCCESS" ? "已发" : "队列中"}
                      </span>
                    ) : (
                      <Plus size={10} className="text-ink-muted" />
                    )}
                  </button>
                );
              })}
            </div>
            <button
              type="button"
              onClick={() => navigate("/assistant")}
              className="inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-accent hover:text-accent-deep"
            >
              <Sparkles size={11} />
              AI 润色标题
            </button>
          </div>
        )}

        {/* Writing Canvas */}
        <div
          className={`min-h-0 flex-1 overflow-y-auto ${
            isMissingWorkspace || !selected ? "" : "px-8 py-9 sm:px-14"
          }`}
        >
          {isMissingWorkspace ? (
            <StudioWorkspaceOnboarding onChooseWorkspace={() => void chooseWorkspace()} />
          ) : selected ? (
            <div className="mx-auto flex min-h-full w-full max-w-[740px] flex-col">
              {/* Clean Document Meta Strip */}
              <div className="mb-6 flex flex-wrap items-center gap-2 border-b border-line pb-3 text-[11px] text-ink-muted">
                <span className="font-mono text-[10.5px] text-ink-muted">{selected.path}</span>
                {selected.tags.length > 0 && (
                  <>
                    <span className="text-ink-faint">·</span>
                    <div className="flex items-center gap-1.5">
                      {selected.tags.map((tag) => (
                        <span
                          key={tag}
                          className="rounded-md bg-accent-soft px-2 py-0.5 text-[10.5px] text-accent-deep"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </>
                )}
                {frontmatter && (
                  <button
                    type="button"
                    onClick={() => setShowFrontmatter((prev) => !prev)}
                    className="ml-auto cursor-pointer rounded-md border border-line bg-canvas px-2 py-0.5 text-[10.5px] text-ink-secondary hover:text-ink"
                  >
                    {showFrontmatter ? "隐藏 Frontmatter" : "查看 Frontmatter"}
                  </button>
                )}
              </div>

              {showFrontmatter && frontmatter && (
                <pre className="mb-5 overflow-x-auto rounded-xl border border-line bg-canvas px-4 py-3 font-mono text-[11.5px] leading-relaxed text-ink-secondary">
                  {frontmatter}
                </pre>
              )}

              {viewMode === "write" ? (
                <textarea
                  ref={textareaRef}
                  aria-label="Markdown 正文"
                  spellCheck={false}
                  value={editableBody}
                  onChange={(event) => handleBodyChange(event.target.value)}
                  placeholder="开始写下你的想法..."
                  className="min-h-[calc(100vh-280px)] w-full flex-1 resize-none border-0 bg-transparent text-[15px] leading-[1.85] text-ink placeholder:text-ink-faint focus:outline-none"
                />
              ) : (
                <article className="studio-markdown-preview pb-12">
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
                </article>
              )}
            </div>
          ) : (
            <StudioNoDocumentState onCreateDocument={() => void handleCreateDocument()} />
          )}
        </div>

        {/* Minimal Footer Status Bar */}
        <footer className="flex h-8 shrink-0 items-center justify-between border-t border-line bg-surface-soft px-5 text-[11px] text-ink-muted">
          {isMissingWorkspace ? (
            <>
              <div className="flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-ink-faint" />
                <span>尚未连接本地工作区</span>
              </div>
              <div className="flex items-center gap-3">
                <span>Markdown</span>
                <span>UTF-8</span>
              </div>
            </>
          ) : (
            <>
              <div className="flex items-center gap-3">
                <span className="inline-flex items-center gap-1">
                  <AlignLeft size={11} />
                  Markdown
                </span>
                <span>{chars.toLocaleString()} 字</span>
                <span>预计阅读 {readingMinutes} 分钟</span>
              </div>
              <div className="flex items-center gap-3">
                <span className="font-mono text-[10.5px]">⌘S 自动保存</span>
                <span>UTF-8</span>
              </div>
            </>
          )}
        </footer>
      </main>

      {/* Right On-Demand Slide-Over Drawer (Info / History) */}
      {rightPanel !== "none" && (
        <aside className="flex h-full w-[296px] shrink-0 flex-col border-l border-line bg-canvas">
          <div className="flex h-12 items-center justify-between border-b border-line px-4">
            <div className="flex items-center gap-1 rounded-xl bg-hover p-0.5">
              <button
                onClick={() => setRightPanel("info")}
                className={`cursor-pointer rounded-lg px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "info"
                    ? "bg-surface text-ink shadow-2xs"
                    : "text-ink-secondary hover:text-ink"
                }`}
              >
                文章信息
              </button>
              <button
                onClick={() => setRightPanel("history")}
                className={`cursor-pointer rounded-lg px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "history"
                    ? "bg-surface text-ink shadow-2xs"
                    : "text-ink-secondary hover:text-ink"
                }`}
              >
                版本历史
              </button>
            </div>
            <button
              onClick={() => setRightPanel("none")}
              aria-label="关闭侧边抽屉"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-ink-muted hover:bg-hover hover:text-ink"
            >
              <X size={14} />
            </button>
          </div>

          {rightPanel === "info" ? (
            <div className="flex flex-1 flex-col gap-5 overflow-y-auto p-4">
              {selected ? (
                <>
                  {/* Publish Platforms */}
                  <section>
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-ink">发布平台适配</span>
                      <button
                        onClick={() => navigate("/platforms")}
                        className="cursor-pointer text-[11px] font-medium text-accent hover:text-accent-deep"
                      >
                        管理
                      </button>
                    </div>
                    <div className="space-y-3">
                      {PLATFORM_CATEGORIES.map((category) => {
                        const groupAccounts = platformAccounts.filter(
                          (account) => account.category === category.id,
                        );
                        if (groupAccounts.length === 0) return null;
                        return (
                          <div key={category.id} className="space-y-1.5">
                            <span className="block text-[10.5px] font-medium text-ink-muted">
                              {category.label}
                            </span>
                            {groupAccounts.map((account) => {
                              const visual = getPlatformVisual(account.platformName);
                              const existing = publishTasks.find(
                                (task) =>
                                  task.articleTitle === selected.title &&
                                  task.platform === account.platformName,
                              );
                              return (
                                <div
                                  key={account.id}
                                  className="flex items-center gap-2.5 rounded-xl border border-line bg-surface px-3 py-2"
                                >
                                  <PlatformLogo platformName={account.platformName} size={20} />
                                  <div className="min-w-0 flex-1">
                                    <strong className="block text-xs font-medium text-ink">
                                      {account.platformName}
                                    </strong>
                                    <small className="text-[11px] text-ink-muted">
                                      {existing ? "已加入发布队列" : account.contentFormats}
                                    </small>
                                  </div>
                                  <button
                                    onClick={() => {
                                      if (existing) {
                                        navigate("/publishing");
                                      } else {
                                        createPublishTaskFromDraft({
                                          articleTitle: selected.title,
                                          platform: account.platformName,
                                          platformMark: visual.mark,
                                          accountAlias: account.alias,
                                        });
                                      }
                                    }}
                                    aria-label={`为${account.platformName}创建平台稿`}
                                    className="grid h-6 w-6 cursor-pointer place-items-center rounded text-ink-secondary hover:bg-accent-soft hover:text-accent-deep"
                                  >
                                    {existing ? (
                                      <Check size={13} className="text-mint-text" />
                                    ) : (
                                      <Plus size={13} />
                                    )}
                                  </button>
                                </div>
                              );
                            })}
                          </div>
                        );
                      })}
                    </div>
                    <p className="mt-2.5 rounded-xl bg-sidebar px-3 py-2 text-[11px] leading-relaxed text-ink-secondary">
                      平台稿独立保存，主稿更新不会覆盖你在各平台的定制修改。
                    </p>
                  </section>

                  {/* Tags */}
                  <section className="border-t border-line pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-ink">标签</span>
                      <button
                        onClick={() => showToast("标签编辑正在准备中")}
                        aria-label="编辑标签"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-ink-muted hover:bg-hover hover:text-ink"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {selected.tags.length > 0 ? (
                        selected.tags.map((tag) => (
                          <span
                            key={tag}
                            className="rounded-lg border border-accent-border bg-accent-soft px-2 py-1 text-[11px] text-accent-deep"
                          >
                            {tag}
                          </span>
                        ))
                      ) : (
                        <button
                          onClick={() => showToast("标签编辑正在准备中")}
                          className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-dashed border-line-strong px-2.5 py-1 text-[11px] text-ink-secondary hover:border-accent hover:text-accent-deep"
                        >
                          <Tags size={12} />
                          添加标签
                        </button>
                      )}
                    </div>
                  </section>

                  {/* Assets */}
                  <section className="border-t border-line pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-ink">素材</span>
                      <button
                        onClick={() => void handleImportAsset()}
                        aria-label="导入素材"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-ink-muted hover:bg-hover hover:text-ink"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    {selected && isTauri() ? (
                      <div className="space-y-2">
                        {documentAssets.map((asset) => (
                          <div key={asset.path} className="flex gap-1.5">
                            <button
                              type="button"
                              onClick={() => void handlePreviewAsset(asset.path)}
                              disabled={!asset.available}
                              title={asset.warning ?? asset.path}
                              className={`flex min-w-0 flex-1 items-center gap-2 rounded-lg border px-2.5 py-2 text-left text-[11px] ${
                                asset.available
                                  ? "border-line bg-surface text-ink-secondary hover:border-accent"
                                  : "border-amber-300 bg-amber-50 text-amber-900"
                              }`}
                            >
                              <Image size={13} className="shrink-0 text-accent" />
                              <span className="truncate">{asset.path}</span>
                              <span className="ml-auto shrink-0 text-[10px]">
                                {asset.available
                                  ? `${Math.ceil(asset.size / 1024)} KB`
                                  : "无法定位"}
                              </span>
                            </button>
                            <button
                              type="button"
                              onClick={() => void handleRevealAsset(asset.path)}
                              disabled={!asset.available}
                              aria-label={`在文件管理器中定位 ${asset.path}`}
                              title="在文件管理器中定位"
                              className="grid w-8 shrink-0 place-items-center rounded-lg border border-line bg-surface text-ink-muted hover:border-accent hover:text-accent disabled:cursor-not-allowed disabled:opacity-40"
                            >
                              <ArrowUpRight size={11} />
                            </button>
                          </div>
                        ))}
                        {assetPreviewUrl && (
                          <div className="rounded-lg border border-line bg-surface p-2">
                            {assetPreviewMime === "image/svg+xml" ? (
                              <div className="text-[11px] text-ink-secondary">
                                SVG 素材已定位；为降低脚本注入风险，不在 WebView 中内联预览。
                              </div>
                            ) : (
                              <img
                                src={assetPreviewUrl}
                                alt="工作区素材预览"
                                className="max-h-36 w-full rounded object-contain"
                              />
                            )}
                            <button
                              type="button"
                              onClick={() => {
                                if (assetPreviewUrl !== "svg-asset-located")
                                  URL.revokeObjectURL(assetPreviewUrl);
                                setAssetPreviewUrl("");
                              }}
                              className="mt-2 text-[10px] text-accent"
                            >
                              关闭预览
                            </button>
                          </div>
                        )}
                        <button
                          type="button"
                          onClick={() => void handleImportAsset()}
                          className="flex w-full cursor-pointer items-center gap-2 rounded-xl border border-dashed border-line-strong bg-surface px-3 py-2.5 text-left text-[11.5px] text-ink-secondary transition-colors hover:border-accent hover:text-ink"
                        >
                          <Image size={14} className="shrink-0 text-accent" />
                          <span className="flex-1">添加素材并插入相对引用</span>
                          <ChevronRight size={12} className="shrink-0 text-ink-muted" />
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => showToast("选择桌面工作区及文章后即可导入素材")}
                        className="flex w-full cursor-pointer items-center gap-2 rounded-xl border border-dashed border-line-strong bg-surface px-3 py-2.5 text-left text-[11.5px] text-ink-secondary transition-colors hover:border-accent hover:text-ink"
                      >
                        <Image size={14} className="shrink-0 text-accent" />
                        <span className="flex-1">选择文章后从本地添加素材</span>
                        <ChevronRight size={12} className="shrink-0 text-ink-muted" />
                      </button>
                    )}
                  </section>

                  {/* Source Version */}
                  <section className="border-t border-line pt-4">
                    <span className="mb-2.5 block text-xs font-semibold text-ink">当前版本</span>
                    <div className="flex items-center gap-2.5 rounded-xl border border-line bg-surface px-3 py-2.5">
                      <div className="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-sky-bg text-sky-text">
                        <Clock3 size={13} />
                      </div>
                      <div className="min-w-0 flex-1">
                        <strong className="block text-xs font-medium text-ink">主稿版本 03</strong>
                        <small className="text-[11px] text-ink-muted">
                          {formatRelativeTime(selected.updatedAt)} · 最新
                        </small>
                      </div>
                      <Check size={13} className="shrink-0 text-mint-text" />
                    </div>
                  </section>
                </>
              ) : (
                <div className="py-12 text-center text-xs text-ink-muted">选择文章后查看详情</div>
              )}
            </div>
          ) : (
            <div className="flex flex-1 flex-col overflow-y-auto p-4">
              <p className="mb-3 text-[11.5px] leading-relaxed text-ink-secondary">
                版本存储在应用数据目录中的独立 Git 仓库，不会在工作区创建
                .git；自动保存不会创建版本。
              </p>
              <div className="mb-4 flex gap-2">
                <input
                  value={revisionLabel}
                  onChange={(event) => setRevisionLabel(event.target.value)}
                  aria-label="版本说明"
                  placeholder="版本说明（可选）"
                  maxLength={120}
                  className="min-w-0 flex-1 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs text-ink outline-none focus:border-accent"
                />
                <button
                  type="button"
                  onClick={() => {
                    void createRevision(revisionLabel.trim() || undefined);
                    setRevisionLabel("");
                  }}
                  disabled={!selected || saveState === "saving" || saveState === "error"}
                  className="shrink-0 rounded-lg bg-accent px-2.5 py-1.5 text-xs font-medium text-accent-on disabled:cursor-not-allowed disabled:opacity-50"
                >
                  保存版本
                </button>
              </div>
              <button
                type="button"
                onClick={() => void handleExportBundle()}
                disabled={!selected}
                className="mb-3 inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-line bg-surface px-3 py-2 text-xs font-medium text-ink-secondary hover:border-accent disabled:opacity-50"
              >
                <FolderOpen size={12} />
                导出文章与引用素材
              </button>
              <div className="space-y-4">
                {revisions.map((revision, index) => (
                  <div key={revision.id} className="relative flex gap-3">
                    <div className="flex flex-col items-center">
                      <span
                        className={`mt-1 h-2.5 w-2.5 shrink-0 rounded-full border-2 ${
                          index === 0 ? "border-accent bg-accent" : "border-line-strong bg-surface"
                        }`}
                      />
                      {index < revisions.length - 1 && (
                        <span className="mt-1 w-px flex-1 bg-line" />
                      )}
                    </div>
                    <div className="min-w-0 flex-1 pb-2">
                      <div className="flex items-center gap-2">
                        <strong className="text-xs font-medium text-ink">{revision.label}</strong>
                        {index === 0 && (
                          <span className="rounded-md bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent-deep">
                            当前
                          </span>
                        )}
                      </div>
                      <small className="mt-0.5 block text-[11px] text-ink-muted">
                        {formatRelativeTime(revision.createdAt)}
                      </small>
                      <p className="mt-1.5 line-clamp-2 text-xs leading-relaxed text-ink-secondary">
                        {revision.preview}
                      </p>
                      {selectedRevisionId === revision.id && revision.diff && (
                        <pre className="mt-2 max-h-48 overflow-auto rounded-lg bg-hover p-2 font-mono text-[10px] leading-relaxed text-ink-secondary">
                          {revision.diff}
                        </pre>
                      )}
                      <button
                        type="button"
                        onClick={() =>
                          setSelectedRevisionId((current) =>
                            current === revision.id ? "" : revision.id,
                          )
                        }
                        className="mt-2 mr-3 text-[11px] font-medium text-accent hover:text-accent-deep"
                      >
                        {selectedRevisionId === revision.id ? "隐藏差异" : "查看差异"}
                      </button>
                      {index > 0 && (
                        <button
                          onClick={() => void restoreRevision(revision)}
                          className="mt-2 inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-accent hover:text-accent-deep"
                        >
                          恢复此版本
                          <ArrowRight size={11} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-auto flex items-center gap-2 border-t border-line pt-3 text-[11px] text-ink-muted">
                <History size={12} />
                <span>Git 历史保存在应用数据目录</span>
              </div>
            </div>
          )}
        </aside>
      )}
    </div>
  );
}
