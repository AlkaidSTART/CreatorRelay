import { useMemo, useRef } from "react";
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
  Command,
  FilePlus2,
  FileText,
  FolderOpen,
  History,
  Image,
  Info,
  PanelLeft,
  Plus,
  Search,
  Tags,
  X,
} from "../components/Icons";
import { useNavigate } from "react-router";
import { useUiStore, useWorkspaceStore } from "../stores";
import { formatRelativeTime } from "../utils/format";
import { bodyFromMarkdown, countWords } from "../utils/markdown";
import "./StudioPage.css";

export function StudioPage() {
  const navigate = useNavigate();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

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
  const body = useMemo(() => bodyFromMarkdown(content), [content]);
  const words = useMemo(() => countWords(content), [content]);
  const chars = useMemo(() => body.replace(/\s/gu, "").length, [body]);
  const readingMinutes = Math.max(1, Math.ceil(words / 300));

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
        <aside className="flex h-full w-[264px] shrink-0 flex-col border-r border-zinc-200/80 bg-zinc-50/40">
          <div className="flex h-13 items-center justify-between border-b border-zinc-200/80 px-4">
            <div className="flex items-center gap-2">
              <h2 className="text-xs font-semibold text-zinc-800">文章列表</h2>
              <span className="rounded-full bg-zinc-200/80 px-1.5 py-0.5 text-[10px] font-medium text-zinc-600">
                {documents.length}
              </span>
            </div>
            <button
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-zinc-500 transition-colors hover:bg-zinc-200/70 hover:text-zinc-900"
              aria-label="新建文章"
              title="新建文章"
              onClick={() => setIsCreating(true)}
            >
              <FilePlus2 size={15} />
            </button>
          </div>

          <div className="p-3 pb-2">
            <div className="flex h-8 items-center gap-2 rounded-lg border border-zinc-200/80 bg-white px-2.5 text-zinc-400 focus-within:border-zinc-400">
              <Search size={13} className="shrink-0" />
              <input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="搜索文章..."
                aria-label="搜索文章"
                className="min-w-0 flex-1 bg-transparent text-xs text-zinc-800 placeholder:text-zinc-400 focus:outline-none"
              />
              {query && (
                <button
                  onClick={() => setQuery("")}
                  aria-label="清空搜索"
                  className="cursor-pointer text-zinc-400 hover:text-zinc-600"
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
                  className={`block w-full cursor-pointer rounded-xl border px-3 py-2.5 text-left transition-all ${
                    isSelected
                      ? "border-zinc-200/90 bg-white shadow-2xs"
                      : "border-transparent bg-transparent hover:bg-zinc-100/80"
                  }`}
                >
                  <div className="flex items-center justify-between gap-2 text-[11px] text-zinc-400">
                    <span>{formatRelativeTime(doc.updatedAt)}</span>
                    <span>{doc.wordCount} 词</span>
                  </div>
                  <strong
                    className={`mt-1 line-clamp-2 block text-[13px] leading-snug ${
                      isSelected ? "font-semibold text-zinc-950" : "font-medium text-zinc-700"
                    }`}
                  >
                    {doc.title}
                  </strong>
                  <p className="mt-1 line-clamp-1 text-[11.5px] text-zinc-400">{doc.preview}</p>
                  {doc.tags.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1">
                      {doc.tags.slice(0, 2).map((tag) => (
                        <span
                          key={tag}
                          className={`rounded-md px-1.5 py-0.5 text-[10px] ${
                            isSelected
                              ? "bg-zinc-100 text-zinc-600"
                              : "bg-zinc-200/50 text-zinc-500"
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
              <div className="py-10 text-center text-xs text-zinc-400">没有找到匹配的文章</div>
            )}
          </div>

          {/* Bottom Local Workspace Info */}
          <div className="border-t border-zinc-200/80 p-3">
            <div className="flex items-center gap-2 rounded-lg border border-zinc-200/80 bg-white px-2.5 py-2">
              <div className="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-zinc-100 text-zinc-600">
                <FolderOpen size={13} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5">
                  <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
                  <span className="text-[11px] font-medium text-zinc-700">本地工作区</span>
                </div>
                <p className="truncate text-[10px] text-zinc-400">
                  {workspace?.path ?? "尚未设置路径"}
                </p>
              </div>
              <button
                className="grid h-6 w-6 shrink-0 cursor-pointer place-items-center rounded text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700"
                onClick={() => void chooseWorkspace()}
                aria-label="更换工作区"
                title="更换工作区"
              >
                <ArrowUpRight size={13} />
              </button>
            </div>
          </div>
        </aside>
      )}

      {/* Center Distraction-Free Writing Area */}
      <main className="flex min-w-0 flex-1 flex-col bg-white">
        {/* Single Unified Header Toolbar */}
        <header className="flex h-13 shrink-0 items-center justify-between gap-3 border-b border-zinc-200/80 px-4">
          <div className="flex min-w-0 items-center gap-2.5">
            <button
              onClick={toggleDocList}
              aria-label={docListOpen ? "隐藏文章列表" : "显示文章列表"}
              title={docListOpen ? "隐藏文章列表" : "显示文章列表"}
              className={`grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md transition-colors ${
                docListOpen
                  ? "bg-zinc-100 text-zinc-800"
                  : "text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700"
              }`}
            >
              <PanelLeft size={15} />
            </button>

            <div className="flex min-w-0 items-center gap-2">
              <h1 className="truncate text-[13px] font-medium text-zinc-800">
                {selected?.title ?? "未选择文章"}
              </h1>
              {selected && (
                <span className="inline-flex shrink-0 items-center gap-1.5 rounded-full bg-zinc-100 px-2 py-0.5 text-[11px] text-zinc-500">
                  <span className={`h-1.5 w-1.5 rounded-full ${saveMeta.dotClass}`} />
                  {saveMeta.label}
                </span>
              )}
            </div>
          </div>

          {/* Segmented Write / Preview Switch */}
          <div className="flex shrink-0 items-center rounded-lg bg-zinc-100 p-0.5">
            <button
              onClick={() => setViewMode("write")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-all ${
                viewMode === "write"
                  ? "bg-white text-zinc-900 shadow-2xs"
                  : "text-zinc-500 hover:text-zinc-800"
              }`}
            >
              <AlignLeft size={13} />
              编辑
            </button>
            <button
              onClick={() => setViewMode("preview")}
              className={`inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md px-2.5 text-xs font-medium transition-all ${
                viewMode === "preview"
                  ? "bg-white text-zinc-900 shadow-2xs"
                  : "text-zinc-500 hover:text-zinc-800"
              }`}
            >
              <BookOpen size={13} />
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
                  ? "bg-zinc-900 text-white"
                  : "text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900"
              }`}
            >
              <Info size={14} />
              <span className="hidden xl:inline">信息</span>
            </button>
            <button
              onClick={() => toggleRightPanel("history")}
              aria-label="版本历史"
              title="版本历史"
              className={`inline-flex h-7 cursor-pointer items-center gap-1 rounded-md px-2 text-xs transition-colors ${
                rightPanel === "history"
                  ? "bg-zinc-900 text-white"
                  : "text-zinc-500 hover:bg-zinc-100 hover:text-zinc-900"
              }`}
            >
              <History size={14} />
              <span className="hidden xl:inline">历史</span>
            </button>
            <button
              onClick={() => showToast("Markdown 导出功能将在下一迭代开放")}
              aria-label="导出"
              title="导出 Markdown"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-zinc-500 transition-colors hover:bg-zinc-100 hover:text-zinc-900"
            >
              <ArrowDownToLine size={14} />
            </button>
            <div className="mx-1 h-4 w-px bg-zinc-200" />
            <button
              onClick={() => navigate("/publishing")}
              className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3 text-xs font-medium text-white transition-colors hover:bg-zinc-800"
            >
              发布准备
              <ArrowRight size={13} />
            </button>
          </div>
        </header>

        {/* Writing Canvas */}
        <div className="min-h-0 flex-1 overflow-y-auto px-8 py-10 sm:px-14">
          {isTauri() && !workspace ? (
            <div className="mx-auto mt-20 max-w-md">
              <div className="mb-5 grid h-11 w-11 place-items-center rounded-xl border border-zinc-200 bg-zinc-50 text-zinc-700">
                <FolderOpen size={20} />
              </div>
              <span className="text-[11px] font-semibold tracking-widest text-zinc-400 uppercase">
                A QUIET PLACE TO WRITE
              </span>
              <h2 className="mt-2 text-2xl font-semibold tracking-tight text-zinc-900">
                先选择你的工作区
              </h2>
              <p className="mt-2 text-sm leading-relaxed text-zinc-500">
                文章以 Markdown 文件保存在你选择的文件夹中。CreatorRelay 使用本地 SQLite
                索引管理稿件，不会把正文上传到云端。
              </p>
              <button
                onClick={() => void chooseWorkspace()}
                className="mt-6 inline-flex h-9 cursor-pointer items-center gap-2 rounded-lg bg-zinc-900 px-4 text-xs font-medium text-white transition-colors hover:bg-zinc-800"
              >
                <FolderOpen size={15} />
                选择工作区
              </button>
            </div>
          ) : selected ? (
            <div className="mx-auto flex min-h-full w-full max-w-[740px] flex-col">
              {/* Subtle Document Meta Strip */}
              <div className="mb-6 flex flex-wrap items-center gap-2 border-b border-zinc-100 pb-3 text-[11px] text-zinc-400">
                <span className="font-mono text-[10.5px] text-zinc-400">{selected.path}</span>
                {selected.tags.length > 0 && (
                  <>
                    <span className="text-zinc-300">·</span>
                    <div className="flex items-center gap-1.5">
                      {selected.tags.map((tag) => (
                        <span
                          key={tag}
                          className="rounded-md bg-zinc-100 px-2 py-0.5 text-[10.5px] text-zinc-600"
                        >
                          {tag}
                        </span>
                      ))}
                    </div>
                  </>
                )}
              </div>

              {viewMode === "write" ? (
                <textarea
                  ref={textareaRef}
                  aria-label="Markdown 正文"
                  spellCheck={false}
                  value={content}
                  onChange={(event) => updateContent(event.target.value)}
                  placeholder="开始写下你的想法..."
                  className="min-h-[calc(100vh-240px)] w-full flex-1 resize-none border-0 bg-transparent text-[15px] leading-[1.85] text-zinc-800 placeholder:text-zinc-300 focus:outline-none"
                />
              ) : (
                <article className="studio-markdown-preview pb-12">
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
                </article>
              )}
            </div>
          ) : (
            <div className="flex h-full flex-col items-center justify-center gap-3 text-zinc-400">
              <FileText size={32} strokeWidth={1.5} />
              <strong className="text-sm font-medium text-zinc-700">选择一篇文章开始写作</strong>
              <span className="text-xs">或者创建一篇新的 Markdown 草稿</span>
              <button
                onClick={() => setIsCreating(true)}
                className="mt-2 inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3.5 text-xs font-medium text-white hover:bg-zinc-800"
              >
                <Plus size={14} />
                新建文章
              </button>
            </div>
          )}
        </div>

        {/* Minimal Footer Status Bar */}
        <footer className="flex h-8 shrink-0 items-center justify-between border-t border-zinc-100 bg-white px-5 text-[11px] text-zinc-400">
          <div className="flex items-center gap-3">
            <span className="inline-flex items-center gap-1">
              <AlignLeft size={12} />
              Markdown
            </span>
            <span>{words.toLocaleString()} 词</span>
            <span>{chars.toLocaleString()} 字符</span>
            <span>约 {readingMinutes} 分钟阅读</span>
          </div>
          <div className="flex items-center gap-3">
            <span className="inline-flex items-center gap-1">
              <Command size={11} />S 自动保存
            </span>
            <span>UTF-8</span>
          </div>
        </footer>
      </main>

      {/* Right On-Demand Slide-Over Drawer (Info / History) */}
      {rightPanel !== "none" && (
        <aside className="flex h-full w-[300px] shrink-0 flex-col border-l border-zinc-200/80 bg-zinc-50/40">
          <div className="flex h-13 items-center justify-between border-b border-zinc-200/80 px-4">
            <div className="flex items-center gap-1 rounded-lg bg-zinc-200/60 p-0.5">
              <button
                onClick={() => setRightPanel("info")}
                className={`cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "info"
                    ? "bg-white text-zinc-900 shadow-2xs"
                    : "text-zinc-500 hover:text-zinc-800"
                }`}
              >
                文章信息
              </button>
              <button
                onClick={() => setRightPanel("history")}
                className={`cursor-pointer rounded-md px-2.5 py-1 text-xs font-medium transition-colors ${
                  rightPanel === "history"
                    ? "bg-white text-zinc-900 shadow-2xs"
                    : "text-zinc-500 hover:text-zinc-800"
                }`}
              >
                版本历史
              </button>
            </div>
            <button
              onClick={() => setRightPanel("none")}
              aria-label="关闭侧边抽屉"
              className="grid h-7 w-7 cursor-pointer place-items-center rounded-md text-zinc-400 hover:bg-zinc-200/60 hover:text-zinc-700"
            >
              <X size={15} />
            </button>
          </div>

          {rightPanel === "info" ? (
            <div className="flex flex-1 flex-col gap-5 overflow-y-auto p-4">
              {selected ? (
                <>
                  {/* Publish Platforms */}
                  <section>
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-zinc-700">发布平台</span>
                      <button
                        onClick={() => navigate("/platforms")}
                        className="cursor-pointer text-[11px] font-medium text-zinc-500 hover:text-zinc-900"
                      >
                        管理
                      </button>
                    </div>
                    <div className="space-y-2">
                      {[
                        { mark: "稀", name: "掘金", tint: "bg-blue-50 text-blue-600" },
                        { mark: "C", name: "CSDN", tint: "bg-orange-50 text-orange-600" },
                      ].map((platform) => (
                        <div
                          key={platform.name}
                          className="flex items-center gap-2.5 rounded-xl border border-zinc-200/80 bg-white px-3 py-2"
                        >
                          <div
                            className={`grid h-7 w-7 shrink-0 place-items-center rounded-lg text-xs font-semibold ${platform.tint}`}
                          >
                            {platform.mark}
                          </div>
                          <div className="min-w-0 flex-1">
                            <strong className="block text-xs font-medium text-zinc-800">
                              {platform.name}
                            </strong>
                            <small className="text-[11px] text-zinc-400">平台稿未创建</small>
                          </div>
                          <button
                            onClick={() => navigate("/publishing")}
                            aria-label={`为${platform.name}创建平台稿`}
                            className="grid h-6 w-6 cursor-pointer place-items-center rounded-md text-zinc-400 hover:bg-zinc-100 hover:text-zinc-800"
                          >
                            <Plus size={14} />
                          </button>
                        </div>
                      ))}
                    </div>
                    <p className="mt-2.5 rounded-lg bg-zinc-100/80 px-3 py-2 text-[11px] leading-relaxed text-zinc-500">
                      平台稿独立保存，主稿更新不会覆盖你在各平台的定制修改。
                    </p>
                  </section>

                  {/* Tags */}
                  <section className="border-t border-zinc-200/70 pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-zinc-700">标签</span>
                      <button
                        onClick={() => showToast("标签编辑正在准备中")}
                        aria-label="编辑标签"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-zinc-400 hover:bg-zinc-200/60 hover:text-zinc-800"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <div className="flex flex-wrap gap-1.5">
                      {selected.tags.length > 0 ? (
                        selected.tags.map((tag) => (
                          <span
                            key={tag}
                            className="rounded-md border border-zinc-200 bg-white px-2 py-1 text-[11px] text-zinc-600"
                          >
                            {tag}
                          </span>
                        ))
                      ) : (
                        <button
                          onClick={() => showToast("标签编辑正在准备中")}
                          className="inline-flex cursor-pointer items-center gap-1.5 rounded-md border border-dashed border-zinc-300 px-2.5 py-1 text-[11px] text-zinc-500 hover:border-zinc-400 hover:text-zinc-800"
                        >
                          <Tags size={12} />
                          添加标签
                        </button>
                      )}
                    </div>
                  </section>

                  {/* Assets */}
                  <section className="border-t border-zinc-200/70 pt-4">
                    <div className="mb-2.5 flex items-center justify-between">
                      <span className="text-xs font-semibold text-zinc-700">素材</span>
                      <button
                        onClick={() => showToast("素材导入将在下一步接入工作区文件系统")}
                        aria-label="导入素材"
                        className="grid h-6 w-6 cursor-pointer place-items-center rounded text-zinc-400 hover:bg-zinc-200/60 hover:text-zinc-800"
                      >
                        <Plus size={13} />
                      </button>
                    </div>
                    <button
                      onClick={() => showToast("支持拖入图片素材，素材目录接口正在开发中")}
                      className="flex w-full cursor-pointer items-center gap-2 rounded-xl border border-dashed border-zinc-300 bg-white px-3 py-2.5 text-left text-[11.5px] text-zinc-500 transition-colors hover:border-zinc-400 hover:text-zinc-800"
                    >
                      <Image size={15} className="shrink-0" />
                      <span className="flex-1">拖入图片或从本地添加</span>
                      <ChevronRight size={13} className="shrink-0 text-zinc-400" />
                    </button>
                  </section>

                  {/* Source Version */}
                  <section className="border-t border-zinc-200/70 pt-4">
                    <span className="mb-2.5 block text-xs font-semibold text-zinc-700">
                      当前版本
                    </span>
                    <div className="flex items-center gap-2.5 rounded-xl border border-zinc-200/80 bg-white px-3 py-2.5">
                      <div className="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-600">
                        <Clock3 size={14} />
                      </div>
                      <div className="min-w-0 flex-1">
                        <strong className="block text-xs font-medium text-zinc-800">
                          主稿版本 03
                        </strong>
                        <small className="text-[11px] text-zinc-400">
                          {formatRelativeTime(selected.updatedAt)} · 最新
                        </small>
                      </div>
                      <Check size={14} className="shrink-0 text-emerald-600" />
                    </div>
                  </section>
                </>
              ) : (
                <div className="py-12 text-center text-xs text-zinc-400">选择文章后查看详情</div>
              )}
            </div>
          ) : (
            <div className="flex flex-1 flex-col overflow-y-auto p-4">
              <p className="mb-4 text-[11.5px] leading-relaxed text-zinc-500">
                编辑过程会自动保留快照。恢复历史版本将创建一个新的当前版本。
              </p>
              <div className="space-y-4">
                {revisions.map((revision, index) => (
                  <div key={revision.id} className="relative flex gap-3">
                    <div className="flex flex-col items-center">
                      <span
                        className={`mt-1 h-2.5 w-2.5 shrink-0 rounded-full border-2 ${
                          index === 0
                            ? "border-zinc-900 bg-zinc-900"
                            : "border-zinc-300 bg-white"
                        }`}
                      />
                      {index < revisions.length - 1 && (
                        <span className="mt-1 w-px flex-1 bg-zinc-200" />
                      )}
                    </div>
                    <div className="min-w-0 flex-1 pb-2">
                      <div className="flex items-center gap-2">
                        <strong className="text-xs font-medium text-zinc-800">
                          {revision.label}
                        </strong>
                        {index === 0 && (
                          <span className="rounded bg-zinc-900 px-1.5 py-0.5 text-[10px] font-medium text-white">
                            当前
                          </span>
                        )}
                      </div>
                      <small className="mt-0.5 block text-[11px] text-zinc-400">
                        {formatRelativeTime(revision.createdAt)}
                      </small>
                      <p className="mt-1.5 line-clamp-2 text-xs leading-relaxed text-zinc-500">
                        {revision.preview}
                      </p>
                      {index > 0 && (
                        <button
                          onClick={() => void restoreRevision(revision)}
                          className="mt-2 inline-flex cursor-pointer items-center gap-1 text-[11px] font-medium text-zinc-700 hover:text-zinc-950"
                        >
                          恢复此版本
                          <ArrowRight size={12} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="mt-auto flex items-center gap-2 border-t border-zinc-200/80 pt-3 text-[11px] text-zinc-400">
                <History size={13} />
                <span>最多保留最近 30 个版本</span>
              </div>
            </div>
          )}
        </aside>
      )}
    </div>
  );
}
