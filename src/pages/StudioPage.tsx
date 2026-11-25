import { useRef } from "react";
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
  ChevronDown,
  ChevronRight,
  CircleHelp,
  Clock3,
  Command,
  FilePlus2,
  FileText,
  FolderOpen,
  History,
  Image,
  MoreHorizontal,
  Plus,
  Search,
  Tags,
  X,
} from "lucide-react";
import { useNavigate, useOutletContext } from "react-router";
import { formatRelativeTime } from "../hooks/useWorkspaceDocuments";
import type { AppOutletContext } from "../layouts/AppLayout";

export function StudioPage() {
  const {
    workspace,
    documents,
    selected,
    selectedId,
    content,
    updateContent,
    query,
    setQuery,
    saveState,
    viewMode,
    setViewMode,
    showHistory,
    setShowHistory,
    revisions,
    setIsCreating,
    sidebarOpen,
    body,
    words,
    chars,
    showToast,
    loadDocuments,
    chooseWorkspace,
    selectDocumentSafely,
    restoreRevision,
  } = useOutletContext<AppOutletContext>();

  const navigate = useNavigate();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const saveLabel =
    saveState === "saving"
      ? "正在保存"
      : saveState === "unsaved"
        ? "未保存"
        : saveState === "error"
          ? "保存失败"
          : "所有更改已保存";

  return (
    <div className="studio-layout">
      <main className="main-column">
        <header className="topbar">
          <div className="breadcrumb">
            <span>写作空间</span>
            <ChevronRight size={14} />
            <strong>{selected?.title ?? "未选择文章"}</strong>
          </div>
          <div className="topbar-actions">
            <button
              className="icon-button help-button"
              aria-label="帮助"
              onClick={() => showToast("CreatorRelay 本地创作工作台")}
            >
              <CircleHelp size={17} />
            </button>
            <button
              className="avatar-small"
              aria-label="系统设置"
              onClick={() => navigate("/settings")}
            >
              林
            </button>
          </div>
        </header>

        <section className="editor-toolbar">
          <div className="editor-status">
            <span className={`save-indicator ${saveState}`} />
            {saveLabel}
            <span className="status-divider" /> <FolderOpen size={13} />
            {workspace?.path ?? "未选择工作区"}
          </div>
          <div className="toolbar-actions">
            <button
              className={`toolbar-button ${viewMode === "write" ? "selected" : ""}`}
              onClick={() => setViewMode("write")}
            >
              <AlignLeft size={15} />
              编辑
            </button>
            <button
              className={`toolbar-button ${viewMode === "preview" ? "selected" : ""}`}
              onClick={() => setViewMode("preview")}
            >
              <BookOpen size={15} />
              预览
            </button>
            <span className="toolbar-divider" />
            <button
              className="icon-button"
              onClick={() => setShowHistory((value) => !value)}
              aria-label="版本历史"
            >
              <History size={16} />
            </button>
            <button
              className="icon-button"
              onClick={() => showToast("Markdown 导出功能将在下一迭代开放")}
              aria-label="导出"
            >
              <ArrowDownToLine size={16} />
            </button>
            <button className="publish-button" onClick={() => navigate("/publishing")}>
              发布准备
              <ArrowRight size={14} />
            </button>
          </div>
        </section>

        <div className={`editor-workspace ${showHistory ? "with-history" : ""}`}>
          <section className="writing-pane">
            <div className="editor-paper">
              {isTauri() && !workspace ? (
                <div className="workspace-onboarding flex flex-col">
                  <div className="onboarding-mark">
                    <FolderOpen size={21} />
                  </div>
                  <span className="context-eyebrow">A QUIET PLACE TO WRITE</span>
                  <h1>先选择你的工作区</h1>
                  <p>
                    文章以 Markdown 文件保存在你选择的文件夹中。CreatorRelay 使用本地 SQLite
                    索引管理稿件，不会把正文上传到云端。
                  </p>
                  <button className="primary-button" onClick={() => void chooseWorkspace()}>
                    <FolderOpen size={15} />
                    选择工作区
                  </button>
                  <small>你可以选择已有文章目录，也可以创建一个新的空文件夹。</small>
                </div>
              ) : selected ? (
                <>
                  <div className="paper-overline">
                    <span className="overline-rule" />
                    ARTICLE DRAFT <span className="overline-dot">/</span>{" "}
                    {selected.path.split("/")[1]?.toUpperCase() ?? "LOCAL"}
                  </div>
                  {viewMode === "write" ? (
                    <textarea
                      ref={textareaRef}
                      className="markdown-editor"
                      aria-label="Markdown 正文"
                      spellCheck={false}
                      value={content}
                      onChange={(event) => updateContent(event.target.value)}
                    />
                  ) : (
                    <article className="markdown-preview">
                      <ReactMarkdown remarkPlugins={[remarkGfm]}>{body}</ReactMarkdown>
                    </article>
                  )}
                  {viewMode === "write" && (
                    <div className="editor-bottom-hint">
                      <Command size={12} />
                      <span>Markdown 已启用</span>
                      <span className="hint-separator">·</span>
                      <span>编辑时自动保存</span>
                    </div>
                  )}
                </>
              ) : (
                <div className="no-document">
                  <FileText size={28} />
                  <strong>选择一篇文章开始写作</strong>
                  <span>或者创建一篇新的 Markdown 草稿</span>
                  <button className="primary-button" onClick={() => setIsCreating(true)}>
                    <Plus size={15} />
                    新建文章
                  </button>
                </div>
              )}
            </div>
            <footer className="editor-footer">
              <div className="footer-left">
                <span>
                  <AlignLeft size={13} />
                  Markdown
                </span>
                <span className="footer-dot" /> <span>{words.toLocaleString()} 个词</span>
                <span className="footer-dot" />
                <span>{chars.toLocaleString()} 字符</span>
              </div>
              <div className="footer-right">
                <span className="keyboard-hint">
                  <Command size={11} /> S
                </span>
                <span>保存</span>
                <span className="footer-dot" />
                <span>UTF-8</span>
              </div>
            </footer>
          </section>

          <aside className="context-pane">
            <div className="context-header">
              <div>
                <span className="context-eyebrow">DOCUMENT</span>
                <h2>文章信息</h2>
              </div>
              <button
                className="icon-button"
                aria-label="前往 AI 助手"
                onClick={() => navigate("/assistant")}
              >
                <MoreHorizontal size={17} />
              </button>
            </div>
            {selected ? (
              <>
                <div className="context-section">
                  <div className="section-caption">
                    发布平台{" "}
                    <button className="text-action" onClick={() => navigate("/platforms")}>
                      管理
                    </button>
                  </div>
                  <div className="platform-row">
                    <div className="platform-mark juejin-mark">稀</div>
                    <div className="platform-meta">
                      <strong>掘金</strong>
                      <small>平台稿未创建</small>
                    </div>
                    <span className="platform-status idle" />
                    <button
                      className="tiny-icon-button"
                      aria-label="为掘金创建平台稿"
                      onClick={() => navigate("/publishing")}
                    >
                      <Plus size={15} />
                    </button>
                  </div>
                  <div className="platform-row">
                    <div className="platform-mark csdn-mark">C</div>
                    <div className="platform-meta">
                      <strong>CSDN</strong>
                      <small>平台稿未创建</small>
                    </div>
                    <span className="platform-status idle" />
                    <button
                      className="tiny-icon-button"
                      aria-label="为 CSDN 创建平台稿"
                      onClick={() => navigate("/publishing")}
                    >
                      <Plus size={15} />
                    </button>
                  </div>
                  <div className="platform-note">
                    <span className="note-line" />
                    平台稿独立保存，主稿更新不会覆盖你的修改。
                  </div>
                </div>
                <div className="context-section">
                  <div className="section-caption">
                    标签{" "}
                    <button
                      className="tiny-icon-button"
                      aria-label="编辑标签"
                      onClick={() => showToast("标签编辑正在准备中")}
                    >
                      <Plus size={14} />
                    </button>
                  </div>
                  <div className="tag-list">
                    {selected.tags.length ? (
                      selected.tags.map((tag) => (
                        <span className="tag-chip" key={tag}>
                          {tag}
                        </span>
                      ))
                    ) : (
                      <button className="add-tag" onClick={() => showToast("标签编辑正在准备中")}>
                        <Tags size={13} />
                        添加标签
                      </button>
                    )}
                  </div>
                </div>
                <div className="context-section">
                  <div className="section-caption">
                    素材{" "}
                    <button
                      className="tiny-icon-button"
                      aria-label="导入素材"
                      onClick={() => showToast("素材导入将在下一步接入工作区文件系统")}
                    >
                      <Plus size={14} />
                    </button>
                  </div>
                  <button
                    className="asset-empty"
                    onClick={() => showToast("支持拖入图片素材，素材目录接口正在开发中")}
                  >
                    <Image size={16} />
                    <span>将图片拖到编辑区，或从本地添加</span>
                    <ChevronRight size={14} />
                  </button>
                </div>
                <div className="context-section source-version">
                  <div className="section-caption">来源版本</div>
                  <div className="source-card">
                    <div className="source-icon">
                      <Clock3 size={14} />
                    </div>
                    <div>
                      <strong>主稿版本 03</strong>
                      <small>{formatRelativeTime(selected.updatedAt)} · 当前版本</small>
                    </div>
                    <Check size={15} className="source-check" />
                  </div>
                </div>
                <div className="context-spacer" />
                <div className="context-bottom">
                  <div className="reading-time">
                    <span>预计阅读</span>
                    <strong>{Math.max(1, Math.ceil(words / 300))} 分钟</strong>
                  </div>
                  <button className="context-more" onClick={() => setShowHistory(true)}>
                    <History size={14} />
                    查看版本历史
                    <ChevronRight size={14} />
                  </button>
                </div>
              </>
            ) : (
              <div className="context-empty">选择文章后查看详情</div>
            )}
          </aside>

          {showHistory && (
            <aside className="history-drawer">
              <div className="history-heading">
                <div>
                  <span className="context-eyebrow">HISTORY</span>
                  <h2>版本历史</h2>
                </div>
                <button
                  className="icon-button"
                  onClick={() => setShowHistory(false)}
                  aria-label="关闭版本历史"
                >
                  <X size={16} />
                </button>
              </div>
              <p className="history-description">
                编辑过程会自动保留快照。恢复版本将创建新的当前版本。
              </p>
              <div className="history-list">
                {revisions.map((revision, index) => (
                  <div
                    className={`revision-item ${index === 0 ? "current" : ""}`}
                    key={revision.id}
                  >
                    <div className="revision-marker">
                      <span />
                    </div>
                    <div className="revision-content">
                      <div className="revision-title">
                        <strong>{revision.label}</strong>
                        {index === 0 && <span className="current-pill">当前</span>}
                      </div>
                      <small>{formatRelativeTime(revision.createdAt)}</small>
                      <p>{revision.preview}</p>
                      {index > 0 && (
                        <button
                          className="restore-button"
                          onClick={() => void restoreRevision(revision)}
                        >
                          恢复此版本
                          <ArrowRight size={13} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
              <div className="history-footnote">
                <History size={14} />
                <span>最多保留最近 30 个版本</span>
              </div>
            </aside>
          )}
        </div>
      </main>

      <aside className={`document-rail ${sidebarOpen ? "" : "rail-wide"}`}>
        <div className="rail-search">
          <Search size={15} />
          <input
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              void loadDocuments(event.target.value);
            }}
            placeholder="搜索文章"
            aria-label="搜索文章"
          />
          <kbd>⌘ K</kbd>
        </div>
        <div className="rail-title-row">
          <div>
            <span className="context-eyebrow">YOUR LIBRARY</span>
            <h1>
              文章 <span>{documents.length.toString().padStart(2, "0")}</span>
            </h1>
          </div>
          <button
            className="create-icon-button"
            aria-label="新建文章"
            onClick={() => setIsCreating(true)}
          >
            <FilePlus2 size={17} />
          </button>
        </div>
        <div className="rail-filter-row">
          <span>最近编辑</span>
          <button className="filter-button" onClick={() => showToast("已按最近编辑排序")}>
            <ChevronDown size={13} />
          </button>
        </div>
        <div className="rail-document-list">
          {documents.map((document, index) => (
            <button
              key={document.id}
              className={`rail-document ${document.id === selectedId ? "active" : ""}`}
              onClick={() => void selectDocumentSafely(document)}
            >
              <div className="rail-doc-top">
                <span className="rail-doc-date">{formatRelativeTime(document.updatedAt)}</span>
                <span className={`rail-doc-index ${document.id === selectedId ? "current" : ""}`}>
                  {String(index + 1).padStart(2, "0")}
                </span>
              </div>
              <strong>{document.title}</strong>
              <p>{document.preview}</p>
              <div className="rail-doc-tags">
                {document.tags.slice(0, 2).map((tag) => (
                  <span key={tag}>{tag}</span>
                ))}
              </div>
            </button>
          ))}
          {!documents.length && <div className="rail-empty">暂无文章</div>}
        </div>
        <div className="rail-bottom">
          <div className="storage-card">
            <div className="storage-card-icon">
              <FolderOpen size={15} />
            </div>
            <div className="storage-card-copy">
              <strong>本地工作区</strong>
              <span>{workspace?.path ?? "尚未设置路径"}</span>
            </div>
            <button
              className="tiny-icon-button"
              onClick={() => void chooseWorkspace()}
              aria-label="更换工作区"
            >
              <ArrowUpRight size={14} />
            </button>
          </div>
          <div className="privacy-caption">
            <span className="privacy-dot" />
            你的文章保存在本地设备
          </div>
        </div>
      </aside>
    </div>
  );
}
