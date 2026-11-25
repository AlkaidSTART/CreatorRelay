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
      ? { label: "保存中...", dotClass: "bg-[#C6923A]" }
      : saveState === "unsaved"
        ? { label: "未保存", dotClass: "bg-[#C6923A]" }
        : saveState === "error"
          ? { label: "保存失败", dotClass: "bg-red-500" }
          : { label: "已保存", dotClass: "bg-[#2F7D5B]" };

  return (
    <div className="flex h-full w-full overflow-hidden bg-[#FDFCF9]">
      {/* Left Collapsible Document List */}
      {docListOpen && (
        <aside className="flex h-full w-[264px] shrink-0 flex-col border-r border-[#E6E2D8] bg-[#F8F6F0]">
          <div className="flex h-12 items-center justify-between border-b border-[#E6E2D8] px-4">
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold text-[#1E241E]">本地主稿</h2>
              <span className="rounded-full bg-[#E8E4DA] px-2 py-0.5 text-[10px] font-semibold text-[#4C554C]">
                {documents.length}
              </span>
            </div>
            <button
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-[#5C645C] transition-colors hover:bg-[#EAE6DC] hover:text-[#233A29]"
              aria-label="新建文章"
              title="新建文章"
              onClick={() => setIsCreating(true)}
            >
              <FilePlus2 size={14} />
            </button>
          </div>

          <div className="p-3 pb-2">
            <div className="flex h-8.5 items-center gap-2 rounded-xl border border-[#E3DED2] bg-white px-2.5 text-[#858B83] focus-within:border-[#2F4A36]/50">
              <Search size={13} className="shrink-0" />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索主稿标题或标签..."
                aria-label="搜索文章"
                className="min-w-0 flex-1 bg-transparent text-xs text-[#1E241E] placeholder:text-[#9CA19A] focus:outline-none"
              />
              {query && (
                <button
                  onClick={() => setQuery("")}
                  aria-label="清空搜索"
                  className="cursor-pointer text-[#858B83] hover:text-[#1E241E]"
                >
                  <X size={12} />
                </button>
              )}
            </div>
          </div>

          <div className="flex-1 space-y-1.5 overflow-y-auto px-2.5 py-1.5" role="list">
            {documents.map((doc) => {
              const isSelected = doc.id === selectedId;
              return (
                <button
                  key={doc.id}
                  role="listitem"
                  onClick={() => void selectDocumentSafely(doc)}
                  className={`relative block w-full cursor-pointer rounded-xl border px-3 py-2.5 text-left transition-all ${
                    isSelected
                      ? "border-[#D6D0C2] bg-white shadow-[0_2px_8px_rgba(31,36,31,0.05)]"
                      : "border-transparent bg-transparent hover:bg-[#EFECE4]/70"
                  }`}
                >
                  {isSelected && (
                    <span className="absolute top-2.5 bottom-2.5 left-0 w-[3px] rounded-r-full bg-[#2F4A36]" />
                  )}
                  <div className="flex items-center justify-between gap-2 text-[11px] text-[#858B83]">
                    <span>{formatRelativeTime(doc.updatedAt)}</span>
                    <span>{doc.charCount || doc.wordCount} 字</span>
                  </div>
                  <strong
                    className={`mt-1 line-clamp-2 block text-[13px] leading-snug ${
                      isSelected ? "font-semibold text-[#1E241E]" : "font-medium text-[#3E453E]"
                    }`}
                  >
                    {doc.title}
                  </strong>
                  <p className="mt-1 line-clamp-1 text-[11.5px] text-[#858B83]">{doc.preview}</p>
                  {doc.tags.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1">
                      {doc.tags.slice(0, 2).map((tag) => (
                        <span
                          key={tag}
                          className={`rounded-md px-1.5 py-0.5 text-[10px] font-medium ${
                            isSelected
                              ? "bg-[#EAF0EA] text-[#2F4A36]"
                              : "bg-[#EAE6DC]/80 text-[#646B64]"
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
              <div className="py-10 text-center text-xs text-[#858B83]">没有找到匹配的主稿</div>
            )}
          </div>

          {/* Bottom Local Workspace Info */}
          <div className="border-t border-[#E6E2D8] p-3">
            <div className="flex items-center gap-2 rounded-xl border border-[#E5E0D5] bg-white px-2.5 py-2">
              <div className="grid h-6 w-6 shrink-0 place-items-center rounded-lg bg-[#EAF0EA] text-[#2F4A36]">
                <FolderOpen size={12} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-[#2F7D5B]" />
                  <span className="text-[11px] font-medium text-[#242A24]">本地工作区</span>
                </div>
                <p className="truncate text-[10px] text-[#858B83]">
                  {workspace?.path ?? "尚未设置路径"}
                </p>
              </div>
              <button
                className="grid h-6 w-6 shrink-0 cursor-pointer place-items-center rounded-md text-[#858B83] hover:bg-[#F4F1EA] hover:text-[#233A29]"
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
      <main className="flex min-w-0 flex-1 flex-col bg-[#FDFCF9]">
        {/* Single Unified Header Toolbar */}
        <header className="flex h-12 shrink-0 items-center justify-between gap-3 border-b border-[#E6E2D8] bg-[#FDFCF9] px-4">
          <div className="flex min-w-0 items-center gap-2.5">
            <button
              onClick={toggleDocList}
              aria-label={docListOpen ? "隐藏文章列表" : "显示文章列表"}
              title={docListOpen ? "隐藏文章列表" : "显示文章列表"}
              className={`grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-lg transition-colors ${
                docListOpen
                  ? "bg-[#EAF0EA] text-[#2F4A36]"
                  : "text-[#7D847D] hover:bg-[#EFECE4] hover:text-[#1E241E]"
              }`}
            >
              <PanelLeft size={14} />
            </button>

            <div className="flex min-w-0 items-center gap-2">
              <h1 className="truncate text-[13px] font-semibold text-[#1E241E]">
                {selected?.title ?? "未选择文章"}
              </h1>
              {selected && (
                <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full bg-[#F4F1EA] px-2.5 py-0.5 text-[11px] text-[#5C645C]">
                  <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dotClass}`} />
                  {saveMeta.label}
                </span>
              )}
            </div>
          </div>

          {/* Segmented Write / Preview Switch */}
          <div className="flex shrink-0 items-center rounded-xl border border-[#E6E2D8] bg-[#F4F1EA] p-0.5">
            <button
              onClick={() => setViewMode("write")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-xs font-medium transition-all ${
                viewMode === "write"
                  ? "bg-white text-[#233A29] shadow-2xs"
                  : "text-[#6B726B] hover:text-[#1E241E]"
              }`}
            >
              <AlignLeft size={12} />
              编辑
            </button>
            <button
              onClick={() => setViewMode("preview")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg px-2.5 text-xs font-medium transition-all ${
                viewMode === "preview"
                  ? "bg-white text-[#233A29] shadow-2xs"
                  : "text-[#6B726B] hover:text-[#1E241E]"
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
                  ? "bg-[#2F4A36] text-white"
                  : "text-[#646B64] hover:bg-[#EFECE4] hover:text-[#1E241E]"
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
                  ? "bg-[#2F4A36] text-white"
                  : "text-[#646B64] hover:bg-[#EFECE4] hover:text-[#1E241E]"
              }`}
            >
              <History size={13} />
              <span className="hidden xl:inline">历史</span>
            </button>
            <button
              onClick={() => showToast("已准备 Markdown 导出文件")}
              aria-label="导出"
              title="导出 Markdown"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-[#646B64] transition-colors hover:bg-[#EFECE4] hover:text-[#1E241E]"
            >
              <ArrowDownToLine size={13} />
            </button>
            <div className="mx-1 h-4 w-px bg-[#E3DED2]" />
            <button
              onClick={() => navigate("/publishing")}
              className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-xl bg-[#2F4A36] px-3 text-xs font-medium text-white shadow-xs transition-colors hover:bg-[#233A29]"
            >
              发布接力
              <ArrowRight size={12} />
            </button>
          </div>
        </header>

        {/* Inline Multi-Platform Relay Bar (Instant 3-Second Product Magic!) */}
        {selected && (
          <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#EFECE4] bg-[#F8F6F0] px-5 py-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="inline-flex items-center gap-1.5 text-[11px] font-semibold text-[#4C6450]">
                <Send size={11} />
                一键接力分发：
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
                    className={`inline-flex cursor-pointer items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-medium transition-all ${
                      existingTask
                        ? "border-[#2F4A36]/25 bg-[#EAF0EA] text-[#233A29] hover:bg-[#DCE8DC]"
                        : "border-[#E3DED2] bg-white text-[#545C54] hover:border-[#2F4A36]/40 hover:text-[#1E241E]"
                    }`}
                    title={
                      existingTask
                        ? `已在 ${target.platform} 发布队列中，点击前往查看`
                        : `为 ${target.platform} 生成独立适配稿`
                    }
                  >
                    <span
                      className="h-2 w-2 rounded-full"
                      style={{ backgroundColor: visual.color }}
                    />
                    <span>{target.platform}</span>
                    {existingTask ? (
                      <span className="text-[10px] text-[#2F7D5B]">
                        {existingTask.status === "SUCCESS" ? "已发布" : "已入队"}
                      </span>
                    ) : (
                      <Plus size={10} className="text-[#9BA19A]" />
                    )}
                  </button>
                );
              })}
            </div>
            <button
              type="button"
              onClick={() => navigate("/assistant")}
              className="inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-[#8A6120] hover:text-[#5C4012]"
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
              <div className="mb-5 grid h-11 w-11 place-items-center rounded-xl border border-[#E6E2D8] bg-[#F4F1EA] text-[#2F4A36]">
                <FolderOpen size={20} />
              </div>
              <span className="text-[11px] font-semibold tracking-widest text-[#4C6450] uppercase">
                A QUIET PLACE TO WRITE
              </span>
              <h2 className="mt-2 text-2xl font-semibold tracking-tight text-[#1E241E]">
                先选择你的本地写作文件夹
              </h2>
              <p className="mt-2 text-sm leading-relaxed text-[#646B64]">
                主稿以纯 Markdown 文件保存在你的硬盘中，支持自由接力分发到各大创作平台，正文永远属于你。
              </p>
              <button
                onClick={() => void chooseWorkspace()}
                className="mt-6 inline-flex h-9 cursor-pointer items-center gap-2 rounded-xl bg-[#2F4A36] px-4 text-xs font-medium text-white transition-colors hover:bg-[#233A29]"
              >
                <FolderOpen size={14} />
                选择工作区
              </button>
            </div>
          ) : selected ? (
            <div className="mx-auto flex min-h-full w-full max-w-[740px] flex-col">
              {/* Clean Document Meta Strip */}
              <div className="mb-6 flex flex-wrap items-center gap-2 border-b border-[#EFECE4] pb-3 text-[11px] text-[#858B83]">
                <span className="font-mono text-[10.5px] text-[#858B83]">{selected.path}</span>
                {selected.tags.length > 0 && (
                  <>
                    <span className="text-[#D4CEC2]">·</span>
                    <div className="flex items-center gap-1.5">
                      {selected.tags.map((tag) => (
                        <span
                          key={tag}
                          className="rounded-md bg-[#EAF0EA] px-2 py-0.5 text-[10.5px] font-medium text-[#2F4A36]"
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
                    className="ml-auto cursor-pointer rounded-md border border-[#E6E2D8] bg-[#F8F6F0] px-2 py-0.5 text-[10.5px] text-[#646B64] hover:border-[#CFC8B9] hover:text-[#1E241E]"
                  >
                    {showFrontmatter ? "隐藏 Frontmatter" : "查看 Frontmatter"}
                  </button>
                )}
              </div>

              {showFrontmatter && frontmatter && (
                <pre className="mb-5 overflow-x-auto rounded-xl border border-[#E6E2D8] bg-[#F6F4EE] px-4 py-3 font-mono text-[11.5px] leading-relaxed text-[#5C645C]">
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
                  placeholder="开始写下你的想法，稍后可一键接力分发至多平台..."
                  className="min-h-[calc(100vh-280px)] w-full flex-1 resize-none border-0 bg-transparent text-[15px] leading-[1.85] text-[#242A24] placeholder:text-[#B5B9B2] focus:outline-none"
                />
              ) : (
                <article className="studio-markdown-preview pb-12">
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
                </article>
              )}
            </div>
          ) : (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-[#858B83]">
              <FileText size={32} strokeWidth={1.5} />
              <strong className="text-sm font-medium text-[#242A24]">选择一篇主稿开始写作</strong>
              <span className="text-xs">或者创建一篇新的 Markdown 主稿</span>
              <button
                onClick={() => setIsCreating(true)}
                className="mt-2 inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-[#2F4A36] px-3.5 text-xs font-medium text-white hover:bg-[#233A29]"
              >
                <Plus size={13} />
                新建文章
              </button>
            </div>
          )}
        </div>

        {/* Minimal Footer Status Bar */}
        <footer className="flex h-8 shrink-0 items-center justify-between border-t border-[#EFECE4] bg-[#FAF8F3] px-5 text-[11px] text-[#7D847D]">
          <div className="flex items-center gap-3">
            <span className="inline-flex items-center gap-1 font-medium text-[#4C6450]">
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
        <aside className="flex h-full w-[300px] shrink-0 flex-col border-l border-[#E6E2D8] bg-[#F8F6F0]">
          <div className="flex h-12 items-center justify-between border-b border-[#E6E2D8] px-4">
            <div className="flex items-center gap-1 rounded-xl bg-[#EAE6DC] p-0.5">
              <button
                onClick={() => setRightPanel("info")}
                className={`cursor-pointer rounded-lg px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "info"
                    ? "bg-white text-[#233A29] shadow-2xs"
                    : "text-[#646B64] hover:text-[#1E241E]"
                }`}
              >
                文章信息
              </button>
              <button
                onClick={() => setRightPanel("history")}
                className={`cursor-pointer rounded-lg px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "history"
                    ? "bg-white text-[#233A29] shadow-2xs"
                    : "text-[#646B64] hover:text-[#1E241E]"
                }`}
              >
                版本历史
              </button>
            </div>
            <button
              onClick={() => setRightPanel("none")}
              aria-label="关闭侧边抽屉"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-[#858B83] hover:bg-[#EAE6DC] hover:text-[#1E241E]"
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
                      <span className="text-xs font-semibold text-[#242A24]">发布平台适配</span>
                      <button
                        onClick={() => navigate("/platforms")}
                        className="cursor-pointer text-[11px] font-medium text-[#4C6450] hover:text-[#233A29]"
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
                            className="flex items-center gap-2.5 rounded-xl border border-[#E6E2D8] bg-white px-3 py-2"
                          >
                            <div
                              className={`grid h-7 w-7 shrink-0 place-items-center rounded-lg text-xs font-semibold ${visual.avatarClass}`}
                            >
                              {target.mark}
                            </div>
                            <div className="min-w-0 flex-1">
                              <strong className="block text-xs font-medium text-[#242A24]">
                                {target.platform}
                              </strong>
                              <small className="text-[11px] text-[#858B83]">
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
                              className="grid h-6 w-6 cursor-pointer place-items-center rounded-md text-[#646B64] hover:bg-[#EAF0EA] hover:text-[#2F4A36]"
                            >
                              {existing ? <Check size={13} className="text-[#2F7D5B]" /> : <Plus size={13} />}
                            </button>
                          </div>
                        );
                      })}
                    </div>
                    <p className="mt-2.5 rounded-xl bg-[#EFECE4]/70 px-3 py-2 text-[11px] leading-relaxed text-[#646B64]">
                      平台稿独立保存，主稿更新不会覆盖你在各平台的定制修改。
                    </p>
                  </section>

                  {/* Tags */}
                  <section className="border-t border-[#E6E2D8] pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-[#242A24]">标签</span>
                      <button
                        onClick={() => showToast("标签编辑正在准备中")}
                        aria-label="编辑标签"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-[#858B83] hover:bg-[#EAE6DC] hover:text-[#233A29]"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {selected.tags.length > 0 ? (
                        selected.tags.map((tag) => (
                          <span
                            key={tag}
                            className="rounded-md border border-[#E6E2D8] bg-white px-2 py-1 text-[11px] text-[#4C554C]"
                          >
                            {tag}
                          </span>
                        ))
                      ) : (
                        <button
                          onClick={() => showToast("标签编辑正在准备中")}
                          className="inline-flex cursor-pointer items-center gap-1.5 rounded-md border border-dashed border-[#CFC8B9] px-2.5 py-1 text-[11px] text-[#646B64] hover:border-[#2F4A36] hover:text-[#233A29]"
                        >
                          <Tags size={12} />
                          添加标签
                        </button>
                      )}
                    </div>
                  </section>

                  {/* Assets */}
                  <section className="border-t border-[#E6E2D8] pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-[#242A24]">素材</span>
                      <button
                        onClick={() => showToast("素材导入将在下一步接入工作区文件系统")}
                        aria-label="导入素材"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-[#858B83] hover:bg-[#EAE6DC] hover:text-[#233A29]"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <button
                      onClick={() => showToast("支持拖入图片素材，素材目录接口正在开发中")}
                      className="flex w-full cursor-pointer items-center gap-2 rounded-xl border border-dashed border-[#CFC8B9] bg-white px-3 py-2.5 text-left text-[11.5px] text-[#646B64] transition-colors hover:border-[#2F4A36] hover:text-[#233A29]"
                    >
                      <Image size={14} className="shrink-0" />
                      <span className="flex-1">拖入图片或从本地添加</span>
                      <ChevronRight size={12} className="shrink-0 text-[#9CA19A]" />
                    </button>
                  </section>

                  {/* Source Version */}
                  <section className="border-t border-[#E6E2D8] pt-4">
                    <span className="mb-2.5 block text-xs font-semibold text-[#242A24]">
                      当前版本
                    </span>
                    <div className="flex items-center gap-2.5 rounded-xl border border-[#E6E2D8] bg-white px-3 py-2.5">
                      <div className="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-[#EAF0EA] text-[#2F4A36]">
                        <Clock3 size={13} />
                      </div>
                      <div className="min-w-0 flex-1">
                        <strong className="block text-xs font-medium text-[#242A24]">
                          主稿版本 03
                        </strong>
                        <small className="text-[11px] text-[#858B83]">
                          {formatRelativeTime(selected.updatedAt)} · 最新
                        </small>
                      </div>
                      <Check size={13} className="shrink-0 text-[#2F7D5B]" />
                    </div>
                  </section>
                </>
              ) : (
                <div className="py-12 text-center text-xs text-[#858B83]">选择文章后查看详情</div>
              )}
            </div>
          ) : (
            <div className="flex flex-1 flex-col overflow-y-auto p-4">
              <p className="mb-4 text-[11.5px] leading-relaxed text-[#646B64]">
                编辑过程会自动保留本地快照。恢复历史版本将创建一个新的当前版本。
              </p>
              <div className="space-y-4">
                {revisions.map((revision, index) => (
                  <div key={revision.id} className="relative flex gap-3">
                    <div className="flex flex-col items-center">
                      <span
                        className={`mt-1 h-2.5 w-2.5 shrink-0 rounded-full border-2 ${
                          index === 0
                            ? "border-[#2F4A36] bg-[#2F4A36]"
                            : "border-[#CFC8B9] bg-white"
                        }`}
                      />
                      {index < revisions.length - 1 && (
                        <span className="mt-1 w-px flex-1 bg-[#E3DED2]" />
                      )}
                    </div>
                    <div className="min-w-0 flex-1 pb-2">
                      <div className="flex items-center gap-2">
                        <strong className="text-xs font-medium text-[#242A24]">
                          {revision.label}
                        </strong>
                        {index === 0 && (
                          <span className="rounded bg-[#2F4A36] px-1.5 py-0.5 text-[10px] font-medium text-white">
                            当前
                          </span>
                        )}
                      </div>
                      <small className="mt-0.5 block text-[11px] text-[#858B83]">
                        {formatRelativeTime(revision.createdAt)}
                      </small>
                      <p className="mt-1.5 line-clamp-2 text-xs leading-relaxed text-[#646B64]">
                        {revision.preview}
                      </p>
                      {index > 0 && (
                        <button
                          onClick={() => void restoreRevision(revision)}
                          className="mt-2 inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-[#2F4A36] hover:text-[#1E241E]"
                        >
                          恢复此版本
                          <ArrowRight size={11} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-auto flex items-center gap-2 border-t border-[#E6E2D8] pt-3 text-[11px] text-[#858B83]">
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
