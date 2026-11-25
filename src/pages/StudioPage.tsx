import { useMemo, useRef, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
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
  FileText,
  FolderOpen,
  History,
  Image,
  Info,
  PanelLeft,
  Plus,
  Search,
  Send,
  Sparkles,
  Tags,
  X,
} from "../components/Icons";
import { useNavigate } from "react-router";
import { useContentStore, useUiStore, useWorkspaceStore } from "../stores";
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

const RELAY_TARGET_PLATFORMS = [
  { platform: "掘金", mark: "稀", accountAlias: "技术主号" },
  { platform: "CSDN", mark: "C", accountAlias: "Rust 专栏号" },
  { platform: "知乎", mark: "知", accountAlias: "林工笔记" },
  { platform: "微信公众号", mark: "微", accountAlias: "工程手记" },
];

export function StudioPage() {
  const navigate = useNavigate();
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [showFrontmatter, setShowFrontmatter] = useState(false);

  const workspace = useWorkspaceStore((state) => state.workspace);
  const documents = useWorkspaceStore((state) => state.documents);
  const selectedId = useWorkspaceStore((state) => state.selectedId);
  const content = useWorkspaceStore((state) => state.content);
  const updateContent = useWorkspaceStore((state) => state.updateContent);
  const query = useWorkspaceStore((state) => state.query);
  const setQuery = useWorkspaceStore((state) => state.setQuery);
  const saveState = useWorkspaceStore((state) => state.saveState);
  const revisions = useWorkspaceStore((state) => state.revisions);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const selectDocumentSafely = useWorkspaceStore((state) => state.selectDocumentSafely);
  const restoreRevision = useWorkspaceStore((state) => state.restoreRevision);

  const publishTasks = useContentStore((state) => state.publishTasks);
  const createPublishTaskFromDraft = useContentStore((state) => state.createPublishTaskFromDraft);

  const docListOpen = useUiStore((state) => state.docListOpen);
  const toggleDocList = useUiStore((state) => state.toggleDocList);
  const viewMode = useUiStore((state) => state.viewMode);
  const setViewMode = useUiStore((state) => state.setViewMode);
  const rightPanel = useUiStore((state) => state.rightPanel);
  const setRightPanel = useUiStore((state) => state.setRightPanel);
  const toggleRightPanel = useUiStore((state) => state.toggleRightPanel);
  const setIsCreating = useUiStore((state) => state.setIsCreating);
  const showToast = useUiStore((state) => state.showToast);

  const selected = useMemo(
    () => documents.find((doc) => doc.id === selectedId) ?? null,
    [documents, selectedId],
  );
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

  const saveMeta =
    saveState === "saving"
      ? { label: "保存中...", dotClass: "bg-amber-400" }
      : saveState === "unsaved"
        ? { label: "未保存", dotClass: "bg-amber-400" }
        : saveState === "error"
          ? { label: "保存失败", dotClass: "bg-red-500" }
          : { label: "已保存", dotClass: "bg-emerald-500" };

  return (
    <div className="flex h-full w-full overflow-hidden bg-white">
      {/* Left Collapsible Document List */}
      {docListOpen && (
        <aside className="flex h-full w-[260px] shrink-0 flex-col border-r border-black/[0.06] bg-[#FAFAFB]">
          <div className="flex h-12 items-center justify-between border-b border-black/[0.06] px-4">
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold text-[#1D1D1F]">本地主稿</h2>
              <span className="rounded-full bg-black/[0.06] px-1.5 py-0.5 text-[10px] font-medium text-[#6E6E73]">
                {documents.length}
              </span>
            </div>
            <button
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-[#6E6E73] transition-colors hover:bg-black/[0.05] hover:text-[#1D1D1F]"
              aria-label="新建文章"
              title="新建文章"
              onClick={() => setIsCreating(true)}
            >
              <FilePlus2 size={14} />
            </button>
          </div>

          <div className="p-3 pb-2">
            <div className="flex h-8 items-center gap-2 rounded-lg border border-black/[0.08] bg-white px-2.5 text-[#8E8E93] focus-within:border-[#2563EB]/60">
              <Search size={12} className="shrink-0" />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索主稿..."
                aria-label="搜索文章"
                className="min-w-0 flex-1 bg-transparent text-xs text-[#1D1D1F] placeholder:text-[#AEAEB2] focus:outline-none"
              />
              {query && (
                <button
                  onClick={() => setQuery("")}
                  aria-label="清空搜索"
                  className="cursor-pointer text-[#8E8E93] hover:text-[#1D1D1F]"
                >
                  <X size={12} />
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 space-y-1 overflow-y-auto px-2.5 py-1.5" role="list">
            {documents.map((doc) => {
              const isSelected = doc.id === selectedId;
              return (
                <button
                  key={doc.id}
                  role="listitem"
                  onClick={() => void selectDocumentSafely(doc)}
                  className={`relative block w-full cursor-pointer rounded-lg border px-3 py-2.5 text-left transition-all ${
                    isSelected
                      ? "border-black/[0.08] bg-white shadow-[0_1px_3px_rgba(0,0,0,0.05)]"
                      : "border-transparent bg-transparent hover:bg-black/[0.035]"
                  }`}
                >
                  {isSelected && (
                    <span className="absolute top-2.5 bottom-2.5 left-0 w-[2.5px] rounded-r-full bg-[#2563EB]" />
                  )}
                  <div className="flex items-center justify-between gap-2 text-[11px] text-[#8E8E93]">
                    <span>{formatRelativeTime(doc.updatedAt)}</span>
                    <span>{doc.charCount || doc.wordCount} 字</span>
                  </div>
                  <strong
                    className={`mt-1 line-clamp-2 block text-[13px] leading-snug ${
                      isSelected ? "font-semibold text-[#1D1D1F]" : "font-medium text-[#3A3A3C]"
                    }`}
                  >
                    {doc.title}
                  </strong>
                  <p className="mt-1 line-clamp-1 text-[11.5px] text-[#8E8E93]">{doc.preview}</p>
                  {doc.tags.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1">
                      {doc.tags.slice(0, 2).map((tag) => (
                        <span
                          key={tag}
                          className={`rounded px-1.5 py-0.5 text-[10px] ${
                            isSelected
                              ? "bg-[#EFF6FF] text-[#2563EB]"
                              : "bg-black/[0.045] text-[#6E6E73]"
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
              <div className="py-10 text-center text-xs text-[#8E8E93]">没有找到匹配的主稿</div>
            )}
          </div>

          {/* Bottom Local Workspace Info */}
          <div className="border-t border-black/[0.06] p-3">
            <div className="flex items-center gap-2 rounded-lg border border-black/[0.07] bg-white px-2.5 py-2">
              <div className="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-[#F5F5F7] text-[#58585D]">
                <FolderOpen size={12} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
                  <span className="text-[11px] font-medium text-[#1D1D1F]">本地工作区</span>
                </div>
                <p className="truncate text-[10px] text-[#8E8E93]">
                  {workspace?.path ?? "尚未设置路径"}
                </p>
              </div>
              <button
                className="grid h-6 w-6 shrink-0 cursor-pointer place-items-center rounded text-[#8E8E93] hover:bg-[#F5F5F7] hover:text-[#1D1D1F]"
                onClick={() => void chooseWorkspace()}
                aria-label="更换工作区"
                title="更换工作区"
              >
                <ArrowUpRight size={12} />
              </button>
            </div>
          </div>
        </aside>
      )}

      {/* Center Distraction-Free Writing Area */}
      <main className="flex min-w-0 flex-1 flex-col bg-white">
        {/* Single Unified Header Toolbar */}
        <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-black/[0.06] bg-white px-4">
          <div className="flex min-w-0 items-center gap-2.5">
            <button
              onClick={toggleDocList}
              aria-label={docListOpen ? "隐藏文章列表" : "显示文章列表"}
              title={docListOpen ? "隐藏文章列表" : "显示文章列表"}
              className={`grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md transition-colors ${
                docListOpen
                  ? "bg-[#F5F5F7] text-[#1D1D1F]"
                  : "text-[#8E8E93] hover:bg-[#F5F5F7] hover:text-[#1D1D1F]"
              }`}
            >
              <PanelLeft size={14} />
            </button>

            <div className="flex min-w-0 items-center gap-2">
              <h1 className="truncate text-[13px] font-medium text-[#1D1D1F]">
                {selected?.title ?? "未选择文章"}
              </h1>
              {selected && (
                <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full bg-[#F5F5F7] px-2 py-0.5 text-[11px] text-[#6E6E73]">
                  <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dotClass}`} />
                  {saveMeta.label}
                </span>
              )}
            </div>
          </div>

          {/* Segmented Write / Preview Switch */}
          <div className="flex shrink-0 items-center rounded-lg bg-[#F5F5F7] p-0.5">
            <button
              onClick={() => setViewMode("write")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-all ${
                viewMode === "write"
                  ? "bg-white text-[#1D1D1F] shadow-2xs"
                  : "text-[#6E6E73] hover:text-[#1D1D1F]"
              }`}
            >
              <AlignLeft size={12} />
              编辑
            </button>
            <button
              onClick={() => setViewMode("preview")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-all ${
                viewMode === "preview"
                  ? "bg-white text-[#1D1D1F] shadow-2xs"
                  : "text-[#6E6E73] hover:text-[#1D1D1F]"
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
              className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-md px-2 text-xs transition-colors ${
                rightPanel === "info"
                  ? "bg-[#EFF6FF] font-medium text-[#2563EB]"
                  : "text-[#6E6E73] hover:bg-[#F5F5F7] hover:text-[#1D1D1F]"
              }`}
            >
              <Info size={13} />
              <span className="hidden xl:inline">信息</span>
            </button>
            <button
              onClick={() => toggleRightPanel("history")}
              aria-label="版本历史"
              title="版本历史"
              className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-md px-2 text-xs transition-colors ${
                rightPanel === "history"
                  ? "bg-[#EFF6FF] font-medium text-[#2563EB]"
                  : "text-[#6E6E73] hover:bg-[#F5F5F7] hover:text-[#1D1D1F]"
              }`}
            >
              <History size={13} />
              <span className="hidden xl:inline">历史</span>
            </button>
            <button
              onClick={() => showToast("已准备 Markdown 导出文件")}
              aria-label="导出"
              title="导出 Markdown"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-[#6E6E73] transition-colors hover:bg-[#F5F5F7] hover:text-[#1D1D1F]"
            >
              <ArrowDownToLine size={13} />
            </button>
            <div className="mx-1 h-4 w-px bg-black/[0.08]" />
            <button
              onClick={() => navigate("/publishing")}
              className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-lg bg-[#2563EB] px-3 text-xs font-medium text-white transition-colors hover:bg-[#1D4ED8]"
            >
              发布准备
              <ArrowRight size={12} />
            </button>
          </div>
        </header>

        {/* Subtle Multi-Platform Relay Strip */}
        {selected && (
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-black/[0.05] bg-[#FAFAFB] px-5 py-2">
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="mr-1 inline-flex items-center gap-1.5 text-[11px] font-medium text-[#6E6E73]">
                <Send size={11} className="text-[#8E8E93]" />
                分发到平台：
              </span>
              {RELAY_TARGET_PLATFORMS.map((target) => {
                const visual = getPlatformVisual(target.platform);
                const existingTask = publishTasks.find(
                  (task) =>
                    task.articleTitle === selected.title && task.platform === target.platform,
                );
                return (
                  <button
                    key={target.platform}
                    type="button"
                    onClick={() => {
                      if (existingTask) {
                        navigate("/publishing");
                      } else {
                        createPublishTaskFromDraft({
                          articleTitle: selected.title,
                          platform: target.platform,
                          platformMark: target.mark,
                          accountAlias: target.accountAlias,
                        });
                      }
                    }}
                    className={`inline-flex cursor-pointer items-center gap-1.5 rounded-md border px-2 py-0.5 text-[11px] font-medium transition-all ${
                      existingTask
                        ? "border-[#2563EB]/25 bg-[#EFF6FF] text-[#1D4ED8] hover:bg-[#DBEAFE]/70"
                        : "border-black/[0.08] bg-white text-[#58585D] hover:border-black/[0.16] hover:text-[#1D1D1F]"
                    }`}
                    title={
                      existingTask
                        ? `已在 ${target.platform} 发布队列中，点击前往查看`
                        : `为 ${target.platform} 生成独立适配稿`
                    }
                  >
                    <span
                      className="h-1.5 w-1.5 rounded-full"
                      style={{ backgroundColor: visual.color }}
                    />
                    <span>{target.platform}</span>
                    {existingTask ? (
                      <span className="text-[10px] opacity-80">
                        {existingTask.status === "SUCCESS" ? "已发" : "队列中"}
                      </span>
                    ) : (
                      <Plus size={10} className="text-[#AEAEB2]" />
                    )}
                  </button>
                );
              })}
            </div>
            <button
              type="button"
              onClick={() => navigate("/assistant")}
              className="inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-[#6E6E73] hover:text-[#2563EB]"
            >
              <Sparkles size={11} />
              AI 润色标题
            </button>
          </div>
        )}

        {/* Writing Canvas */}
        <div className="min-h-0 flex-1 overflow-y-auto px-8 py-9 sm:px-14">
          {isTauri() && !workspace ? (
            <div className="mx-auto mt-20 max-w-md">
              <div className="mb-5 grid h-11 w-11 place-items-center rounded-xl border border-black/[0.08] bg-[#F5F5F7] text-[#1D1D1F]">
                <FolderOpen size={20} />
              </div>
              <span className="text-[11px] font-semibold tracking-widest text-[#8E8E93] uppercase">
                LOCAL WORKSPACE
              </span>
              <h2 className="mt-2 text-2xl font-semibold tracking-tight text-[#1D1D1F]">
                先选择你的本地写作文件夹
              </h2>
              <p className="mt-2 text-sm leading-relaxed text-[#6E6E73]">
                主稿以纯 Markdown 文件保存在你的硬盘中，支持自由接力分发到各大创作平台，正文永远属于你。
              </p>
              <button
                onClick={() => void chooseWorkspace()}
                className="mt-6 inline-flex h-9 cursor-pointer items-center gap-2 rounded-lg bg-[#2563EB] px-4 text-xs font-medium text-white transition-colors hover:bg-[#1D4ED8]"
              >
                <FolderOpen size={14} />
                选择工作区
              </button>
            </div>
          ) : selected ? (
            <div className="mx-auto flex min-h-full w-full max-w-[740px] flex-col">
              {/* Clean Document Meta Strip */}
              <div className="mb-6 flex flex-wrap items-center gap-2 border-b border-black/[0.06] pb-3 text-[11px] text-[#8E8E93]">
                <span className="font-mono text-[10.5px] text-[#8E8E93]">{selected.path}</span>
                {selected.tags.length > 0 && (
                  <>
                    <span className="text-[#D1D1D6]">·</span>
                    <div className="flex items-center gap-1.5">
                      {selected.tags.map((tag) => (
                        <span
                          key={tag}
                          className="rounded bg-[#F5F5F7] px-2 py-0.5 text-[10.5px] text-[#58585D]"
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
                    className="ml-auto cursor-pointer rounded border border-black/[0.08] bg-[#FAFAFB] px-2 py-0.5 text-[10.5px] text-[#6E6E73] hover:text-[#1D1D1F]"
                  >
                    {showFrontmatter ? "隐藏 Frontmatter" : "查看 Frontmatter"}
                  </button>
                )}
              </div>

              {showFrontmatter && frontmatter && (
                <pre className="mb-5 overflow-x-auto rounded-lg border border-black/[0.07] bg-[#F5F5F7] px-4 py-3 font-mono text-[11.5px] leading-relaxed text-[#58585D]">
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
                  className="min-h-[calc(100vh-280px)] w-full flex-1 resize-none border-0 bg-transparent text-[15px] leading-[1.85] text-[#1D1D1F] placeholder:text-[#C7C7CC] focus:outline-none"
                />
              ) : (
                <article className="studio-markdown-preview pb-12">
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
                </article>
              )}
            </div>
          ) : (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-[#8E8E93]">
              <FileText size={32} strokeWidth={1.5} />
              <strong className="text-sm font-medium text-[#1D1D1F]">选择一篇主稿开始写作</strong>
              <span className="text-xs">或者创建一篇新的 Markdown 主稿</span>
              <button
                onClick={() => setIsCreating(true)}
                className="mt-2 inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-[#2563EB] px-3.5 text-xs font-medium text-white hover:bg-[#1D4ED8]"
              >
                <Plus size={13} />
                新建文章
              </button>
            </div>
          )}
        </div>

        {/* Minimal Footer Status Bar */}
        <footer className="flex h-8 shrink-0 items-center justify-between border-t border-black/[0.06] bg-white px-5 text-[11px] text-[#8E8E93]">
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
        </footer>
      </main>

      {/* Right On-Demand Slide-Over Drawer (Info / History) */}
      {rightPanel !== "none" && (
        <aside className="flex h-full w-[296px] shrink-0 flex-col border-l border-black/[0.06] bg-[#FAFAFB]">
          <div className="flex h-12 items-center justify-between border-b border-black/[0.06] px-4">
            <div className="flex items-center gap-1 rounded-lg bg-black/[0.05] p-0.5">
              <button
                onClick={() => setRightPanel("info")}
                className={`cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "info"
                    ? "bg-white text-[#1D1D1F] shadow-2xs"
                    : "text-[#6E6E73] hover:text-[#1D1D1F]"
                }`}
              >
                文章信息
              </button>
              <button
                onClick={() => setRightPanel("history")}
                className={`cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "history"
                    ? "bg-white text-[#1D1D1F] shadow-2xs"
                    : "text-[#6E6E73] hover:text-[#1D1D1F]"
                }`}
              >
                版本历史
              </button>
            </div>
            <button
              onClick={() => setRightPanel("none")}
              aria-label="关闭侧边抽屉"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-[#8E8E93] hover:bg-black/[0.05] hover:text-[#1D1D1F]"
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
                      <span className="text-xs font-semibold text-[#1D1D1F]">发布平台适配</span>
                      <button
                        onClick={() => navigate("/platforms")}
                        className="cursor-pointer text-[11px] font-medium text-[#2563EB] hover:text-[#1D4ED8]"
                      >
                        管理
                      </button>
                    </div>
                    <div className="space-y-2">
                      {RELAY_TARGET_PLATFORMS.slice(0, 3).map((target) => {
                        const visual = getPlatformVisual(target.platform);
                        const existing = publishTasks.find(
                          (task) =>
                            task.articleTitle === selected.title &&
                            task.platform === target.platform,
                        );
                        return (
                          <div
                            key={target.platform}
                            className="flex items-center gap-2.5 rounded-lg border border-black/[0.07] bg-white px-3 py-2"
                          >
                            <span
                              className="h-2.5 w-2.5 shrink-0 rounded-full"
                              style={{ backgroundColor: visual.color }}
                            />
                            <div className="min-w-0 flex-1">
                              <strong className="block text-xs font-medium text-[#1D1D1F]">
                                {target.platform}
                              </strong>
                              <small className="text-[11px] text-[#8E8E93]">
                                {existing ? "已加入发布队列" : "点击生成独立适配稿"}
                              </small>
                            </div>
                            <button
                              onClick={() => {
                                if (existing) {
                                  navigate("/publishing");
                                } else {
                                  createPublishTaskFromDraft({
                                    articleTitle: selected.title,
                                    platform: target.platform,
                                    platformMark: target.mark,
                                    accountAlias: target.accountAlias,
                                  });
                                }
                              }}
                              aria-label={`为${target.platform}创建平台稿`}
                              className="grid h-6 w-6 cursor-pointer place-items-center rounded text-[#6E6E73] hover:bg-[#F5F5F7] hover:text-[#2563EB]"
                            >
                              {existing ? <Check size={13} className="text-emerald-600" /> : <Plus size={13} />}
                            </button>
                          </div>
                        );
                      })}
                    </div>
                    <p className="mt-2.5 rounded-lg bg-black/[0.03] px-3 py-2 text-[11px] leading-relaxed text-[#6E6E73]">
                      平台稿独立保存，主稿更新不会覆盖你在各平台的定制修改。
                    </p>
                  </section>

                  {/* Tags */}
                  <section className="border-t border-black/[0.06] pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-[#1D1D1F]">标签</span>
                      <button
                        onClick={() => showToast("标签编辑正在准备中")}
                        aria-label="编辑标签"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-[#8E8E93] hover:bg-black/[0.05] hover:text-[#1D1D1F]"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {selected.tags.length > 0 ? (
                        selected.tags.map((tag) => (
                          <span
                            key={tag}
                            className="rounded-md border border-black/[0.08] bg-white px-2 py-1 text-[11px] text-[#58585D]"
                          >
                            {tag}
                          </span>
                        ))
                      ) : (
                        <button
                          onClick={() => showToast("标签编辑正在准备中")}
                          className="inline-flex cursor-pointer items-center gap-1.5 rounded-md border border-dashed border-black/[0.15] px-2.5 py-1 text-[11px] text-[#6E6E73] hover:border-[#2563EB] hover:text-[#2563EB]"
                        >
                          <Tags size={12} />
                          添加标签
                        </button>
                      )}
                    </div>
                  </section>

                  {/* Assets */}
                  <section className="border-t border-black/[0.06] pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-[#1D1D1F]">素材</span>
                      <button
                        onClick={() => showToast("素材导入将在下一步接入工作区文件系统")}
                        aria-label="导入素材"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-[#8E8E93] hover:bg-black/[0.05] hover:text-[#1D1D1F]"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <button
                      onClick={() => showToast("支持拖入图片素材，素材目录接口正在开发中")}
                      className="flex w-full cursor-pointer items-center gap-2 rounded-lg border border-dashed border-black/[0.15] bg-white px-3 py-2.5 text-left text-[11.5px] text-[#6E6E73] transition-colors hover:border-[#2563EB] hover:text-[#1D1D1F]"
                    >
                      <Image size={14} className="shrink-0" />
                      <span className="flex-1">拖入图片或从本地添加</span>
                      <ChevronRight size={12} className="shrink-0 text-[#AEAEB2]" />
                    </button>
                  </section>

                  {/* Source Version */}
                  <section className="border-t border-black/[0.06] pt-4">
                    <span className="mb-2.5 block text-xs font-semibold text-[#1D1D1F]">
                      当前版本
                    </span>
                    <div className="flex items-center gap-2.5 rounded-lg border border-black/[0.07] bg-white px-3 py-2.5">
                      <div className="grid h-7 w-7 shrink-0 place-items-center rounded-md bg-[#F5F5F7] text-[#58585D]">
                        <Clock3 size={13} />
                      </div>
                      <div className="min-w-0 flex-1">
                        <strong className="block text-xs font-medium text-[#1D1D1F]">
                          主稿版本 03
                        </strong>
                        <small className="text-[11px] text-[#8E8E93]">
                          {formatRelativeTime(selected.updatedAt)} · 最新
                        </small>
                      </div>
                      <Check size={13} className="shrink-0 text-emerald-600" />
                    </div>
                  </section>
                </>
              ) : (
                <div className="py-12 text-center text-xs text-[#8E8E93]">选择文章后查看详情</div>
              )}
            </div>
          ) : (
            <div className="flex flex-1 flex-col overflow-y-auto p-4">
              <p className="mb-4 text-[11.5px] leading-relaxed text-[#6E6E73]">
                编辑过程会自动保留本地快照。恢复历史版本将创建一个新的当前版本。
              </p>
              <div className="space-y-4">
                {revisions.map((revision, index) => (
                  <div key={revision.id} className="relative flex gap-3">
                    <div className="flex flex-col items-center">
                      <span
                        className={`mt-1 h-2.5 w-2.5 shrink-0 rounded-full border-2 ${
                          index === 0
                            ? "border-[#2563EB] bg-[#2563EB]"
                            : "border-[#D1D1D6] bg-white"
                        }`}
                      />
                      {index < revisions.length - 1 && (
                        <span className="mt-1 w-px flex-1 bg-black/[0.08]" />
                      )}
                    </div>
                    <div className="min-w-0 flex-1 pb-2">
                      <div className="flex items-center gap-2">
                        <strong className="text-xs font-medium text-[#1D1D1F]">
                          {revision.label}
                        </strong>
                        {index === 0 && (
                          <span className="rounded bg-[#EFF6FF] px-1.5 py-0.5 text-[10px] font-medium text-[#2563EB]">
                            当前
                          </span>
                        )}
                      </div>
                      <small className="mt-0.5 block text-[11px] text-[#8E8E93]">
                        {formatRelativeTime(revision.createdAt)}
                      </small>
                      <p className="mt-1.5 line-clamp-2 text-xs leading-relaxed text-[#6E6E73]">
                        {revision.preview}
                      </p>
                      {index > 0 && (
                        <button
                          onClick={() => void restoreRevision(revision)}
                          className="mt-2 inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-[#2563EB] hover:text-[#1D4ED8]"
                        >
                          恢复此版本
                          <ArrowRight size={11} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-auto flex items-center gap-2 border-t border-black/[0.06] pt-3 text-[11px] text-[#8E8E93]">
                <History size={12} />
                <span>最多保留最近 30 个版本</span>
              </div>
            </div>
          )}
        </aside>
      )}
    </div>
  );
}
