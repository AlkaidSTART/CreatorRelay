import {
  ChevronsUpDown,
  FilePlus2,
  FileText,
  ListFilter,
  MoreHorizontal,
  PanelLeftClose,
  Plus,
  X,
} from "lucide-react";
import { NavLink, Outlet, useLocation, useNavigate } from "react-router";
import {
  formatRelativeTime,
  useWorkspaceDocuments,
  type WorkspaceController,
} from "../hooks/useWorkspaceDocuments";
import { PRIMARY_ROUTES, SECONDARY_ROUTES } from "../routes/navigation";

export type AppOutletContext = WorkspaceController;

export function AppLayout() {
  const controller = useWorkspaceDocuments();
  const navigate = useNavigate();
  const location = useLocation();
  const {
    workspace,
    documents,
    selectedId,
    isCreating,
    setIsCreating,
    newTitle,
    setNewTitle,
    loading,
    error,
    setError,
    notice,
    sidebarOpen,
    setSidebarOpen,
    showDemoBanner,
    setShowDemoBanner,
    chooseWorkspace,
    selectDocumentSafely,
    createDocument,
    showToast,
  } = controller;

  const isStudioRoute = location.pathname.startsWith("/studio");

  async function handleSelectDocument(documentId: string) {
    const target = documents.find((item) => item.id === documentId);
    if (!target) return;
    const switched = await selectDocumentSafely(target);
    if (switched && !isStudioRoute) {
      navigate("/studio");
    }
  }

  async function handleCreateSubmit() {
    const created = await createDocument();
    if (created && !isStudioRoute) {
      navigate("/studio");
    }
  }

  if (loading) {
    return (
      <div className="app-loading">
        <span className="loading-mark">cr</span>
        <span>正在整理你的工作区</span>
      </div>
    );
  }

  return (
    <div className={`app-shell ${sidebarOpen ? "" : "app-shell-collapsed"}`}>
      <aside className={`sidebar ${sidebarOpen ? "" : "sidebar-collapsed"}`}>
        <div className="brand-row">
          <div className="brand-mark">cr</div>
          {sidebarOpen && (
            <>
              <span className="brand-name">creatorrelay</span>
              <button
                className="icon-button sidebar-toggle"
                aria-label="收起侧栏"
                onClick={() => setSidebarOpen(false)}
              >
                <PanelLeftClose size={16} />
              </button>
            </>
          )}
        </div>

        {!sidebarOpen && (
          <button
            className="icon-button sidebar-toggle-collapsed"
            aria-label="展开侧栏"
            onClick={() => setSidebarOpen(true)}
          >
            <PanelLeftClose size={16} />
          </button>
        )}

        <button className="workspace-select" onClick={() => void chooseWorkspace()}>
          <span className="workspace-avatar">
            {workspace?.name.slice(0, 1).toUpperCase() ?? "C"}
          </span>
          {sidebarOpen && (
            <>
              <span className="workspace-copy">
                <strong>{workspace?.name ?? "选择工作区"}</strong>
                <small>个人空间</small>
              </span>
              <ChevronsUpDown size={14} className="muted-icon" />
            </>
          )}
        </button>

        {sidebarOpen ? (
          <>
            <div className="side-section-label">工作区</div>
            <nav className="primary-nav" aria-label="主导航">
              {PRIMARY_ROUTES.map((item) => {
                const Icon = item.icon;
                return (
                  <NavLink
                    key={item.id}
                    to={item.path}
                    className={({ isActive }) => `nav-item ${isActive ? "active" : ""}`}
                  >
                    <Icon size={17} strokeWidth={1.7} />
                    <span>{item.label}</span>
                    {item.badge && <span className="nav-count">{item.badge}</span>}
                  </NavLink>
                );
              })}
            </nav>

            <div className="sidebar-divider" />
            <div className="list-heading">
              <span className="side-section-label">最近编辑</span>
              <button
                className="tiny-icon-button"
                aria-label="筛选"
                onClick={() => showToast("已按最近编辑时间排序")}
              >
                <ListFilter size={14} />
              </button>
            </div>
            <div className="document-list" role="list">
              {documents.slice(0, 4).map((document) => (
                <button
                  key={document.id}
                  className={`document-item ${isStudioRoute && document.id === selectedId ? "selected" : ""}`}
                  onClick={() => void handleSelectDocument(document.id)}
                  role="listitem"
                >
                  <FileText size={16} strokeWidth={1.7} className="document-icon" />
                  <span className="document-item-copy">
                    <strong>{document.title}</strong>
                    <small>{formatRelativeTime(document.updatedAt)}</small>
                  </span>
                  {isStudioRoute && document.id === selectedId && (
                    <span className="selected-indicator" />
                  )}
                </button>
              ))}
              {!documents.length && <div className="empty-docs">没有找到匹配的稿件</div>}
            </div>
            <button className="new-document-side" onClick={() => setIsCreating(true)}>
              <Plus size={15} />
              <span>新建文章</span>
              <kbd>⌘ N</kbd>
            </button>

            <div className="sidebar-bottom">
              {SECONDARY_ROUTES.map((item) => {
                const Icon = item.icon;
                return (
                  <NavLink
                    key={item.id}
                    to={item.path}
                    className={({ isActive }) => `nav-item muted-nav ${isActive ? "active" : ""}`}
                  >
                    <Icon size={17} strokeWidth={1.7} />
                    <span>{item.label}</span>
                    {item.id === "assistant" && <span className="setting-dot" />}
                  </NavLink>
                );
              })}
              <div className="profile-row">
                <div className="profile-avatar">林</div>
                <span className="profile-copy">
                  <strong>创作者</strong>
                  <small>本地工作区</small>
                </span>
                <MoreHorizontal size={17} className="muted-icon" />
              </div>
            </div>
          </>
        ) : (
          <nav className="collapsed-nav" aria-label="快捷导航">
            {[...PRIMARY_ROUTES, ...SECONDARY_ROUTES].map((item) => {
              const Icon = item.icon;
              return (
                <NavLink
                  key={item.id}
                  to={item.path}
                  title={`${item.label} (${item.enLabel})`}
                  className={({ isActive }) =>
                    `nav-item collapsed-nav-item ${isActive ? "active" : ""}`
                  }
                >
                  <Icon size={17} strokeWidth={1.7} />
                </NavLink>
              );
            })}
          </nav>
        )}
      </aside>

      <div className="route-stage">
        {showDemoBanner && (
          <div className="demo-notice">
            <span>
              <strong>预览模式</strong>　以下为本地演示内容，桌面版将读写你选择的 SQLite 工作区。
            </span>
            <button onClick={() => setShowDemoBanner(false)} aria-label="关闭提示">
              <X size={14} />
            </button>
          </div>
        )}
        {error && (
          <div className="error-banner">
            <span>{error}</span>
            <button onClick={() => setError("")} aria-label="关闭错误">
              <X size={14} />
            </button>
          </div>
        )}
        {notice && (
          <div className="toast-notice" role="status">
            {notice}
          </div>
        )}

        <div className="route-viewport">
          <Outlet context={controller} />
        </div>
      </div>

      {isCreating && (
        <div
          className="modal-backdrop"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setIsCreating(false);
          }}
        >
          <form
            className="create-dialog"
            onSubmit={(event) => {
              event.preventDefault();
              void handleCreateSubmit();
            }}
          >
            <div className="dialog-icon">
              <FilePlus2 size={19} />
            </div>
            <button
              type="button"
              className="icon-button dialog-close"
              onClick={() => setIsCreating(false)}
              aria-label="关闭"
            >
              <X size={16} />
            </button>
            <span className="context-eyebrow">NEW DOCUMENT</span>
            <h2>开始一篇新文章</h2>
            <p>创建一个独立的 Markdown 主稿，内容保存在当前工作区。</p>
            <label htmlFor="document-title">文章标题</label>
            <input
              id="document-title"
              autoFocus
              value={newTitle}
              onChange={(event) => setNewTitle(event.target.value)}
              placeholder="例如：一次关于边缘计算的实践"
            />
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setIsCreating(false)}
              >
                取消
              </button>
              <button type="submit" className="primary-button" disabled={!newTitle.trim()}>
                <Plus size={15} />
                创建文章
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
