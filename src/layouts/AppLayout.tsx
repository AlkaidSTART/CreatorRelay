import { useEffect, useState } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  ChevronsUpDown,
  FilePlus2,
  FolderOpen,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  Settings2,
  X,
} from "../components/Icons";
import { NavLink, useLocation, useNavigate, useOutlet } from "react-router";
import {
  findRouteByPathname,
  NAV_GROUPS,
  routeAvailabilityLabel,
} from "../routes/navigation";
import { usePlatformStore, useStudioStore, useUiStore, useWorkspaceStore } from "../stores";
import { CONTENT_KIND_META } from "../types/content";
import { applyThemeToDocument } from "../utils/themes";

const ROUTE_VIEW_VARIANTS = {
  initial: { opacity: 0 },
  animate: {
    opacity: 1,
    transition: { duration: 0.14, ease: "easeOut" as const },
  },
  exit: {
    opacity: 0,
    transition: { duration: 0.08, ease: "easeIn" as const },
  },
};

const QUICK_STARTER_TITLES = [
  "一次生产环境的性能调优复盘",
  "本地优先工具链的 3 个设计取舍",
  "从零搭建个人多平台内容工作流",
];

interface CollapsedRouteTooltip {
  id: string;
  label: string;
  enLabel: string;
  top: number;
  left: number;
}

export function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const currentOutlet = useOutlet();
  const reducedMotion = useReducedMotion();

  const workspace = useWorkspaceStore((state) => state.workspace);
  const loading = useWorkspaceStore((state) => state.loading);
  const initialize = useWorkspaceStore((state) => state.initialize);
  const initializePlatforms = usePlatformStore((state) => state.initialize);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const createPackage = useStudioStore((state) => state.createPackage);
  const saveCurrentDocument = useWorkspaceStore((state) => state.saveCurrentDocument);

  const theme = useUiStore((state) => state.theme);
  const sidebarOpen = useUiStore((state) => state.sidebarOpen);
  const setSidebarOpen = useUiStore((state) => state.setSidebarOpen);
  const showDemoBanner = useUiStore((state) => state.showDemoBanner);
  const setShowDemoBanner = useUiStore((state) => state.setShowDemoBanner);
  const isCreating = useUiStore((state) => state.isCreating);
  const setIsCreating = useUiStore((state) => state.setIsCreating);
  const newTitle = useUiStore((state) => state.newTitle);
  const setNewTitle = useUiStore((state) => state.setNewTitle);
  const newKind = useUiStore((state) => state.newKind);
  const setNewKind = useUiStore((state) => state.setNewKind);
  const error = useUiStore((state) => state.error);
  const clearError = useUiStore((state) => state.clearError);
  const notice = useUiStore((state) => state.notice);

  const activeRoute = findRouteByPathname(location.pathname);
  const isStudioRoute = location.pathname.startsWith("/studio");
  const [hoveredRoute, setHoveredRoute] = useState<CollapsedRouteTooltip | null>(null);

  useEffect(() => {
    if (sidebarOpen) {
      setHoveredRoute(null);
    }
  }, [sidebarOpen]);

  function showRouteTooltip(
    item: { id: string; label: string; enLabel: string },
    target: HTMLElement,
  ) {
    const rect = target.getBoundingClientRect();
    setHoveredRoute({
      id: item.id,
      label: item.label,
      enLabel: item.enLabel,
      top: rect.top + rect.height / 2,
      left: rect.right + 8,
    });
  }

  useEffect(() => {
    applyThemeToDocument(theme);
  }, [theme]);

  useEffect(() => {
    void initialize();
    void initializePlatforms();
  }, [initialize, initializePlatforms]);

  useEffect(() => {
    document.title = `CreatorRelay — ${activeRoute.label}`;
  }, [activeRoute.label]);

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key.toLowerCase() === "n") {
        event.preventDefault();
        setIsCreating(true);
      } else if (event.key.toLowerCase() === "s") {
        event.preventDefault();
        void saveCurrentDocument();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [saveCurrentDocument, setIsCreating]);

  async function handleCreateSubmit() {
    const created = await createPackage(newKind, newTitle);
    if (created) {
      setNewTitle("");
      setIsCreating(false);
      if (!isStudioRoute) navigate("/studio");
    }
  }

  if (loading) {
    return (
      <div className="flex h-full w-full flex-col items-center justify-center gap-2.5 bg-canvas text-ink-secondary">
        <img className="h-7 w-7 opacity-80 object-contain" src="/icon.svg" alt="" />
        <span className="font-mono text-[11px] text-ink-muted">正在加载工作区...</span>
      </div>
    );
  }

  return (
    <div className="flex h-full w-full flex-col overflow-hidden bg-canvas text-ink">
      {/* Slim Precision Top Workbench Bar */}
      <header className="z-20 flex h-10 shrink-0 items-center justify-between gap-3 border-b border-line bg-sidebar px-3">
        <div className="flex min-w-0 items-center gap-2">
          <button
            type="button"
            className="grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent"
            aria-label={sidebarOpen ? "收起侧边栏" : "展开侧边栏"}
            aria-expanded={sidebarOpen}
            title={sidebarOpen ? "收起侧边栏" : "展开侧边栏"}
            onClick={() => setSidebarOpen(!sidebarOpen)}
          >
            {sidebarOpen ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
          </button>

          <button
            type="button"
            onClick={() => navigate("/dashboard")}
            className="flex shrink-0 cursor-pointer items-center gap-1.5 text-left"
          >
            <img className="h-5 w-5 shrink-0 object-contain" src="/icon.svg" alt="CreatorRelay" />
            <span className="whitespace-nowrap text-[13px] font-semibold tracking-tight text-ink">
              CreatorRelay
            </span>
          </button>

          <span className="text-ink-faint select-none" aria-hidden="true">
            /
          </span>

          <span className="hidden truncate text-[12px] font-medium text-ink-secondary sm:inline">
            {activeRoute.label}
          </span>

          <span className="mx-0.5 hidden h-3.5 w-px shrink-0 bg-line-strong sm:inline-block" aria-hidden="true" />

          {/* Workspace Path Switcher */}
          <button
            type="button"
            onClick={() => void chooseWorkspace()}
            title={
              workspace?.path
                ? `当前工作空间：${workspace.path}（点击切换文件夹）`
                : "选择本地工作空间文件夹"
            }
            aria-label={`切换工作空间文件夹：${workspace?.name ?? "未选择"}`}
            className="group flex h-7 min-w-0 max-w-[380px] cursor-pointer items-center gap-1.5 rounded-md border border-line bg-surface px-2 text-left transition-colors hover:border-line-strong hover:bg-surface-soft focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent"
          >
            <FolderOpen size={12} className="shrink-0 text-ink-muted group-hover:text-accent" />
            <strong className="truncate text-[11.5px] font-medium text-ink">
              {workspace?.name ?? "选择工作区"}
            </strong>
            {workspace?.path && (
              <span className="hidden max-w-[180px] truncate font-mono text-[10.5px] text-ink-muted lg:inline">
                {workspace.path}
              </span>
            )}
            <ChevronsUpDown
              size={11}
              className="ml-0.5 shrink-0 text-ink-muted transition-colors group-hover:text-ink"
            />
          </button>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <button
            type="button"
            onClick={() => setIsCreating(true)}
            title="新建文章 (⌘N)"
            className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-md bg-accent px-2.5 text-[11.5px] font-medium text-accent-on transition-colors hover:bg-accent-hover"
          >
            <Plus size={12} />
            <span>新建内容</span>
            <kbd className="ml-0.5 hidden rounded bg-white/15 px-1 py-px font-mono text-[9.5px] font-normal text-white/90 sm:inline">
              ⌘N
            </kbd>
          </button>
        </div>
      </header>

      <div className="flex min-h-0 flex-1 overflow-hidden">
        {/* Crisp Architectural Sidebar */}
        <aside
          className="relative z-10 flex h-full shrink-0 flex-col overflow-hidden border-r border-line bg-sidebar transition-[width] duration-150 ease-out"
          style={{ width: sidebarOpen ? 216 : 52 }}
          aria-label="应用侧边栏"
        >
          <div className="min-h-0 flex-1 overflow-y-auto px-2 py-2.5">
            {sidebarOpen ? (
              <div className="space-y-4">
                {NAV_GROUPS.map((group, groupIndex) => (
                  <section
                    key={group.id}
                    className={groupIndex > 0 ? "border-t border-line pt-3" : undefined}
                  >
                    <div className="mb-1 px-2 text-[10px] font-semibold tracking-[0.08em] text-ink-muted uppercase">
                      {group.label}
                    </div>
                    <nav className="flex flex-col gap-0.5" aria-label={group.label}>
                      {group.routes.map((item) => {
                        const Icon = item.icon;
                        return (
                          <NavLink
                            key={item.id}
                            to={item.path}
                            end={item.exact}
                            className={({ isActive }) =>
                              `group relative flex h-7.5 items-center gap-2 rounded-md px-2 text-[12.5px] transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent ${
                                isActive
                                  ? "bg-hover font-medium text-ink"
                                  : "text-ink-secondary hover:bg-hover/60 hover:text-ink"
                              }`
                            }
                          >
                            {({ isActive }) => (
                              <>
                                {isActive && (
                                  <span
                                    aria-hidden="true"
                                    className="absolute top-1.5 bottom-1.5 left-0 w-[2.5px] rounded-r-xs bg-accent"
                                  />
                                )}
                                <Icon
                                  size={13.5}
                                  className={`shrink-0 transition-colors ${
                                    isActive
                                      ? "text-accent"
                                      : "text-ink-muted group-hover:text-ink-secondary"
                                  }`}
                                />
                                <span className="truncate">{item.label}</span>
                                {item.availability !== "available" && (
                                  <span
                                    className={`ml-auto rounded-xs px-1.5 py-px text-[9.5px] font-medium ${
                                      item.availability === "development"
                                        ? "bg-lemon-bg text-lemon-text"
                                        : "bg-subtle text-ink-muted"
                                    }`}
                                  >
                                    {routeAvailabilityLabel(item)}
                                  </span>
                                )}
                              </>
                            )}
                          </NavLink>
                        );
                      })}
                    </nav>
                  </section>
                ))}
              </div>
            ) : (
              <nav
                className="flex flex-col items-center gap-1"
                aria-label="快捷导航"
                onMouseLeave={() => setHoveredRoute(null)}
                onScroll={() => setHoveredRoute(null)}
              >
                {NAV_GROUPS.map((group, groupIndex) => (
                  <div
                    key={group.id}
                    className={`flex w-full flex-col items-center gap-0.5 ${
                      groupIndex > 0 ? "mt-1.5 border-t border-line pt-2" : ""
                    }`}
                  >
                    {group.routes.map((item) => {
                      const Icon = item.icon;
                      return (
                        <NavLink
                          key={item.id}
                          to={item.path}
                          end={item.exact}
                          aria-label={item.label}
                          aria-describedby={
                            hoveredRoute?.id === item.id ? "collapsed-nav-tooltip" : undefined
                          }
                          onMouseEnter={(event) => showRouteTooltip(item, event.currentTarget)}
                          onFocus={(event) => showRouteTooltip(item, event.currentTarget)}
                          onBlur={() => setHoveredRoute(null)}
                          onClick={() => setHoveredRoute(null)}
                          className={({ isActive }) =>
                            `relative grid h-8 w-8 place-items-center rounded-md transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-accent ${
                              isActive
                                ? "bg-hover text-accent"
                                : "text-ink-secondary hover:bg-hover/60 hover:text-ink"
                            }`
                          }
                        >
                          {({ isActive }) => (
                            <>
                              {isActive && (
                                <span
                                  aria-hidden="true"
                                  className="absolute top-1.5 bottom-1.5 left-0 w-[2.5px] rounded-r-xs bg-accent"
                                />
                              )}
                              <Icon size={14} />
                            </>
                          )}
                        </NavLink>
                      );
                    })}
                  </div>
                ))}
              </nav>
            )}
          </div>

          {/* Compact Bottom Status / Settings Footer */}
          <div className="shrink-0 border-t border-line p-2">
            {sidebarOpen ? (
              <button
                type="button"
                onClick={() => navigate("/settings")}
                className="flex w-full cursor-pointer items-center gap-2 rounded-md px-2 py-1.5 text-left transition-colors hover:bg-hover"
              >
                <span className="grid h-6 w-6 shrink-0 place-items-center rounded-sm border border-line bg-surface text-[10px] font-semibold text-ink-secondary">
                  CR
                </span>
                <div className="flex min-w-0 flex-1 flex-col">
                  <strong className="truncate text-[11.5px] font-medium text-ink">
                    本地优先工作台
                  </strong>
                  <small className="truncate font-mono text-[10px] text-ink-muted">
                    Local-First · v0.1
                  </small>
                </div>
                <Settings2 size={12} className="shrink-0 text-ink-muted" />
              </button>
            ) : (
              <button
                type="button"
                onClick={() => navigate("/settings")}
                title="常规与外观设置"
                aria-label="常规与外观设置"
                className="mx-auto grid h-8 w-8 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink"
              >
                <Settings2 size={14} />
              </button>
            )}
          </div>
        </aside>

        {/* Main Content Stage */}
        <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-canvas">
          {activeRoute.availability === "development" && (
            <div className="flex items-center justify-between border-b border-lemon-border bg-lemon-bg px-5 py-1.5 text-[11px] text-lemon-text">
              <span>
                <strong className="mr-1.5 font-medium">开发中 · {activeRoute.milestone}</strong>
                该模块尚未完成阶段验收；桌面端仅展示本地真实记录，浏览器预览中的示例数据不代表已交付能力。
              </span>
            </div>
          )}
          {showDemoBanner && (
            <div className="flex items-center justify-between border-b border-line bg-subtle px-5 py-1.5 text-[11px] text-ink-secondary">
              <span>
                <strong className="mr-1.5 font-medium text-accent-deep">演示环境</strong>
                所有写作、分发与状态切换均可直接交互体验；桌面端直接读写本地文件夹。
              </span>
              <button
                onClick={() => setShowDemoBanner(false)}
                aria-label="关闭提示"
                className="grid h-5 w-5 cursor-pointer place-items-center rounded-sm text-ink-muted hover:bg-line/60 hover:text-ink"
              >
                <X size={12} />
              </button>
            </div>
          )}

          {error && (
            <div className="flex items-center justify-between border-b border-coral-border bg-coral-bg px-5 py-2 text-xs text-coral-text">
              <span>{error}</span>
              <button
                onClick={clearError}
                aria-label="关闭错误"
                className="grid h-5 w-5 cursor-pointer place-items-center rounded-sm text-coral-text hover:bg-coral-border/40"
              >
                <X size={13} />
              </button>
            </div>
          )}

          <AnimatePresence>
            {notice && (
              <motion.div
                key="toast"
                role="status"
                initial={reducedMotion ? false : { opacity: 0, y: -6 }}
                animate={{ opacity: 1, y: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, y: -4 }}
                transition={{ duration: 0.14 }}
                className="pointer-events-none fixed top-3 left-1/2 z-50 -translate-x-1/2 rounded-md border border-line-strong/30 bg-toast-bg px-3 py-1.5 text-xs font-medium text-toast-ink shadow-lg"
              >
                {notice}
              </motion.div>
            )}
          </AnimatePresence>

          <div className="relative min-h-0 min-w-0 flex-1 overflow-hidden">
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={location.pathname}
                variants={reducedMotion ? undefined : ROUTE_VIEW_VARIANTS}
                initial={reducedMotion ? false : "initial"}
                animate="animate"
                exit={reducedMotion ? undefined : "exit"}
                className="h-full w-full min-w-0"
              >
                {currentOutlet}
              </motion.div>
            </AnimatePresence>
          </div>
        </div>
      </div>

      {/* Collapsed Sidebar Route Tooltip */}
      {!sidebarOpen && hoveredRoute && (
        <div
          id="collapsed-nav-tooltip"
          role="tooltip"
          style={{ top: hoveredRoute.top, left: hoveredRoute.left }}
          className="pointer-events-none fixed z-50 -translate-y-1/2"
        >
          <div className="flex items-center gap-1.5 rounded-md bg-toast-bg px-2.5 py-1 text-toast-ink shadow-md">
            <span className="whitespace-nowrap text-[11.5px] font-medium">
              {hoveredRoute.label}
            </span>
            <span className="whitespace-nowrap font-mono text-[10px] opacity-65">
              {hoveredRoute.enLabel}
            </span>
          </div>
        </div>
      )}

      {/* Create Document Modal */}
      {isCreating && (
        <div
          className="fixed inset-0 z-40 grid place-items-center bg-black/40 p-4 backdrop-blur-[1px]"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setIsCreating(false);
          }}
        >
          <form
            className="relative w-[min(460px,calc(100vw-32px))] rounded-lg border border-line-strong bg-surface shadow-xl"
            onSubmit={(event) => {
              event.preventDefault();
              void handleCreateSubmit();
            }}
          >
            <div className="flex items-center justify-between border-b border-line px-5 py-3.5">
              <div className="flex items-center gap-2">
                <FilePlus2 size={15} className="text-accent" />
                <h2 className="text-[14px] font-semibold tracking-tight text-ink">新建内容主稿</h2>
              </div>
              <button
                type="button"
                className="grid h-6 w-6 cursor-pointer place-items-center rounded-sm text-ink-muted transition-colors hover:bg-hover hover:text-ink"
                onClick={() => setIsCreating(false)}
                aria-label="关闭"
              >
                <X size={14} />
              </button>
            </div>

            <div className="p-5">
              <p className="text-xs leading-relaxed text-ink-secondary">
                先选择内容形态，再创建本地内容包；生成平台稿时不会改写主稿。
              </p>

              <div className="mt-4">
                <span className="mb-1.5 block text-[11.5px] font-medium text-ink">内容形态</span>
                <div className="grid grid-cols-2 gap-2">
                  {CONTENT_KIND_META.map((item) => (
                    <button
                      key={item.kind}
                      type="button"
                      onClick={() => setNewKind(item.kind)}
                      className={`cursor-pointer rounded-md border p-2.5 text-left transition-colors ${
                        newKind === item.kind
                          ? "border-accent bg-accent-soft/60"
                          : "border-line bg-canvas hover:border-line-strong"
                      }`}
                    >
                      <strong className="block text-[12px] font-medium text-ink">
                        {item.label}
                      </strong>
                      <small className="mt-0.5 block text-[10.5px] leading-snug text-ink-muted">
                        {item.description}
                      </small>
                    </button>
                  ))}
                </div>
              </div>

              <label
                htmlFor="document-title"
                className="mt-4 mb-1.5 block text-[11.5px] font-medium text-ink"
              >
                内容标题
              </label>
              <input
                id="document-title"
                autoFocus
                value={newTitle}
                onChange={(event) => setNewTitle(event.target.value)}
                placeholder="例如：一次关于边缘计算的轻量化实践"
                className="h-9 w-full rounded-md border border-line-strong bg-surface px-3 text-[13px] text-ink placeholder:text-ink-muted focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent"
              />

              <div className="mt-3">
                <span className="text-[11px] text-ink-muted">快速填入示例标题：</span>
                <div className="mt-1.5 flex flex-wrap gap-1.5">
                  {QUICK_STARTER_TITLES.map((suggestion) => (
                    <button
                      key={suggestion}
                      type="button"
                      onClick={() => setNewTitle(suggestion)}
                      className="cursor-pointer rounded-sm border border-line bg-subtle px-2 py-1 text-[11px] text-ink-secondary transition-colors hover:border-line-strong hover:text-ink"
                    >
                      {suggestion}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="flex justify-end gap-2 border-t border-line bg-surface-soft px-5 py-3">
              <button
                type="button"
                className="h-8 cursor-pointer rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                onClick={() => setIsCreating(false)}
              >
                取消
              </button>
              <button
                type="submit"
                disabled={!newTitle.trim()}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md bg-accent px-3.5 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-45"
              >
                <Plus size={12} />
                创建并开始编辑
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
