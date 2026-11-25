import { useEffect } from "react";
import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import {
  ChevronsUpDown,
  FilePlus2,
  PanelLeftClose,
  PanelLeftOpen,
  Plus,
  X,
} from "../components/Icons";
import { NavLink, useLocation, useNavigate, useOutlet } from "react-router";
import {
  findRouteByPathname,
  PRIMARY_ROUTES,
  routeAvailabilityLabel,
  SECONDARY_ROUTES,
} from "../routes/navigation";
import { usePlatformStore, useStudioStore, useUiStore, useWorkspaceStore } from "../stores";
import { CONTENT_KIND_META } from "../types/content";
import { applyThemeToDocument } from "../utils/themes";

const ROUTE_ENTER_EASE = [0.22, 1, 0.36, 1] as const;
const ROUTE_EXIT_EASE = [0.4, 0, 1, 1] as const;

const ROUTE_VIEW_VARIANTS = {
  initial: {
    opacity: 0,
    y: 10,
    scale: 0.994,
  },
  animate: {
    opacity: 1,
    y: 0,
    scale: 1,
    transition: {
      duration: 0.28,
      ease: ROUTE_ENTER_EASE,
    },
  },
  exit: {
    opacity: 0,
    y: -6,
    scale: 0.996,
    transition: {
      duration: 0.16,
      ease: ROUTE_EXIT_EASE,
    },
  },
};

const SIDEBAR_PILL_TRANSITION = {
  type: "spring" as const,
  stiffness: 380,
  damping: 34,
  mass: 0.85,
};

const QUICK_STARTER_TITLES = [
  "一次生产环境的性能调优复盘",
  "本地优先工具链的 3 个设计取舍",
  "从零搭建个人多平台内容工作流",
];

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
      <div className="flex h-full w-full flex-col items-center justify-center gap-3 bg-canvas text-ink-secondary">
        <img className="h-9 w-9 animate-pulse object-contain" src="/icon.svg" alt="" />
        <span className="text-xs text-ink-muted">正在整理工作区...</span>
      </div>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden bg-canvas text-ink">
      <motion.aside
        className="relative z-10 flex h-full shrink-0 flex-col overflow-hidden border-r border-line bg-sidebar py-4"
        initial={false}
        animate={{ width: sidebarOpen ? 236 : 70 }}
        transition={
          reducedMotion
            ? { duration: 0 }
            : { type: "spring", stiffness: 360, damping: 36, mass: 0.8 }
        }
        aria-label="应用侧边栏"
      >
        {/* Brand Header */}
        <div
          className={`flex h-9 shrink-0 items-center justify-between gap-2 ${sidebarOpen ? "mx-3.5" : "mx-auto w-10"}`}
        >
          <button
            type="button"
            onClick={() => navigate("/dashboard")}
            className="flex min-w-0 cursor-pointer items-center gap-2.5 text-left"
          >
            <img className="h-7 w-7 shrink-0 object-contain" src="/icon.svg" alt="CreatorRelay" />
            <AnimatePresence initial={false}>
              {sidebarOpen && (
                <motion.div
                  key="brand-label"
                  initial={reducedMotion ? false : { opacity: 0, x: -6 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                  transition={{
                    duration: reducedMotion ? 0 : 0.15,
                    delay: sidebarOpen && !reducedMotion ? 0.04 : 0,
                  }}
                  className="flex min-w-0 flex-col"
                >
                  <span className="whitespace-nowrap text-[14px] font-semibold tracking-tight text-ink">
                    CreatorRelay
                  </span>
                  <span className="whitespace-nowrap text-[10px] text-ink-muted">
                    一稿多发工作台
                  </span>
                </motion.div>
              )}
            </AnimatePresence>
          </button>
          <motion.button
            type="button"

            whileTap={reducedMotion ? undefined : { scale: 0.94 }}
            transition={{ duration: reducedMotion ? 0 : 0.12 }}
            className="grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md text-ink-muted outline-none hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/40"
            aria-label={sidebarOpen ? "收起侧边栏" : "展开侧边栏"}
            aria-expanded={sidebarOpen}
            title={sidebarOpen ? "收起侧边栏" : "展开侧边栏"}
            onClick={() => setSidebarOpen(!sidebarOpen)}
          >
            {sidebarOpen ? <PanelLeftClose size={14} /> : <PanelLeftOpen size={14} />}
          </motion.button>
        </div>

        {/* Workspace Switcher */}
        <button
          className={`mx-auto mt-3.5 flex h-9.5 shrink-0 cursor-pointer items-center overflow-hidden rounded-xl border border-line bg-surface/90 text-left shadow-[0_1px_2px_rgba(74,58,48,0.03)] transition-colors hover:border-line-strong hover:bg-surface ${sidebarOpen ? "w-[calc(100%-24px)] gap-2.5 px-2.5" : "w-10 justify-center self-center p-0"}`}
          onClick={() => void chooseWorkspace()}
          title={workspace?.name ?? "选择工作区"}
          aria-label={sidebarOpen ? undefined : `工作区：${workspace?.name ?? "选择工作区"}`}
        >
          <span className="grid h-5.5 w-5.5 shrink-0 place-items-center rounded-md bg-accent-soft text-[11px] font-semibold text-accent-deep">
            {workspace?.name.slice(0, 1).toUpperCase() ?? "C"}
          </span>
          <AnimatePresence initial={false}>
            {sidebarOpen && (
              <motion.span
                key="workspace-label"
                initial={reducedMotion ? false : { opacity: 0, x: -5 }}
                animate={{ opacity: 1, x: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.14 }}
                className="flex min-w-0 flex-1 items-center gap-2"
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <strong className="truncate text-xs font-medium text-ink">
                    {workspace?.name ?? "选择工作区"}
                  </strong>
                  <small className="text-[10px] text-ink-muted">本地空间</small>
                </span>
                <ChevronsUpDown size={12} className="shrink-0 text-ink-muted" />
              </motion.span>
            )}
          </AnimatePresence>
        </button>

        {/* Primary CTA — Adaptive Theme Accent */}
        <motion.button
          whileTap={reducedMotion ? undefined : { scale: 0.98 }}
          transition={{ duration: reducedMotion ? 0 : 0.12 }}
          className={`mx-auto mt-2.5 flex h-8.5 shrink-0 cursor-pointer items-center justify-center gap-2 overflow-hidden whitespace-nowrap rounded-xl bg-accent text-xs font-medium text-accent-on shadow-sm transition-colors hover:bg-accent-hover ${sidebarOpen ? "w-[calc(100%-24px)] px-3" : "w-10 p-0"}`}
          onClick={() => setIsCreating(true)}
          title="新建文章 (⌘N)"
          aria-label={sidebarOpen ? undefined : "新建文章"}
        >
          <Plus size={13} />
          <AnimatePresence initial={false}>
            {sidebarOpen && (
              <motion.span
                key="create-label"
                initial={reducedMotion ? false : { opacity: 0, x: -5 }}
                animate={{ opacity: 1, x: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.14 }}
                className="flex min-w-0 flex-1 items-center whitespace-nowrap"
              >
                <span>新建文章</span>
                <kbd className="ml-auto rounded bg-surface/18 px-1.5 py-0.5 text-[10px] font-normal text-white/90">
                  ⌘N
                </kbd>
              </motion.span>
            )}
          </AnimatePresence>
        </motion.button>

        {/* Navigation */}
        <div
          className={`mt-5 min-h-0 flex-1 overflow-hidden ${sidebarOpen ? "px-3" : "flex flex-col items-center"}`}
        >
          <AnimatePresence mode="wait" initial={false}>
            {sidebarOpen ? (
              <motion.div
                key="expanded-navigation"
                initial={reducedMotion ? false : { opacity: 0, x: -4 }}
                animate={{ opacity: 1, x: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.14 }}
                className="flex h-full min-h-0 flex-col"
              >
                <div className="mb-1.5 px-2.5 text-[10px] font-medium tracking-wider text-ink-muted uppercase">
                  工作区
                </div>
                <nav className="flex flex-col gap-0.5" aria-label="主导航">
                  {PRIMARY_ROUTES.map((item) => {
                    const Icon = item.icon;
                    return (
                      <NavLink
                        key={item.id}
                        to={item.path}
                        className={({ isActive }) =>
                          `group relative flex h-8.5 items-center gap-2.5 rounded-xl px-2.5 text-[13px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                            isActive
                              ? "font-medium text-ink"
                              : "text-ink-secondary hover:bg-hover/80 hover:text-ink"
                          }`
                        }
                      >
                        {({ isActive }) => (
                          <>
                            {isActive && (
                              <motion.span
                                layoutId="sidebar-active-pill"
                                transition={
                                  reducedMotion ? { duration: 0 } : SIDEBAR_PILL_TRANSITION
                                }
                                className="pointer-events-none absolute inset-0 rounded-xl bg-surface shadow-[0_2px_6px_rgba(91,72,136,0.08)] ring-1 ring-line"
                              />
                            )}
                            <Icon
                              size={14}
                              className={`relative z-10 shrink-0 transition-colors ${
                                isActive
                                  ? "text-accent"
                                  : "text-ink-muted group-hover:text-ink-secondary"
                              }`}
                            />
                            <span className="relative z-10 truncate">{item.label}</span>
                            {item.availability !== "available" && (
                              <span
                                className={`relative z-10 ml-auto rounded-md px-1.5 py-0.5 text-[9.5px] font-medium ${
                                  item.availability === "development"
                                    ? "bg-lemon-bg text-lemon-text"
                                    : "bg-sidebar text-ink-muted"
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

                <div className="mt-auto flex flex-col gap-0.5 border-t border-line pt-3">
                  <div className="mb-1.5 px-2.5 text-[10px] font-medium tracking-wider text-ink-muted uppercase">
                    工具与连接
                  </div>
                  {SECONDARY_ROUTES.map((item) => {
                    const Icon = item.icon;
                    return (
                      <NavLink
                        key={item.id}
                        to={item.path}
                        className={({ isActive }) =>
                          `group relative flex h-8 items-center gap-2.5 rounded-xl px-2.5 text-[12.5px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                            isActive
                              ? "font-medium text-ink"
                              : "text-ink-secondary hover:bg-hover/80 hover:text-ink"
                          }`
                        }
                      >
                        {({ isActive }) => (
                          <>
                            {isActive && (
                              <motion.span
                                layoutId="sidebar-active-pill"
                                transition={
                                  reducedMotion ? { duration: 0 } : SIDEBAR_PILL_TRANSITION
                                }
                                className="pointer-events-none absolute inset-0 rounded-xl bg-surface shadow-[0_2px_6px_rgba(91,72,136,0.08)] ring-1 ring-line"
                              />
                            )}
                            <Icon
                              size={14}
                              className={`relative z-10 shrink-0 transition-colors ${
                                isActive
                                  ? "text-accent"
                                  : "text-ink-muted group-hover:text-ink-secondary"
                              }`}
                            />
                            <span className="relative z-10 truncate">{item.label}</span>
                            {item.availability !== "available" && (
                              <span
                                className={`relative z-10 ml-auto rounded-md px-1.5 py-0.5 text-[9.5px] font-medium ${
                                  item.availability === "development"
                                    ? "bg-lemon-bg text-lemon-text"
                                    : "bg-sidebar text-ink-muted"
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
                  <button
                    onClick={() => navigate("/settings")}
                    className="mt-2 flex cursor-pointer items-center gap-2.5 border-t border-line px-1.5 pt-3 text-left transition-opacity hover:opacity-80"
                  >
                    <div className="grid h-7 w-7 shrink-0 place-items-center rounded-full border border-rose-border bg-rose-bg text-xs font-medium text-rose-text">
                      林
                    </div>
                    <div className="flex min-w-0 flex-1 flex-col">
                      <strong className="truncate text-xs font-medium text-ink">创作者</strong>
                      <small className="truncate text-[10px] text-ink-muted">本地优先模式</small>
                    </div>
                  </button>
                </div>
              </motion.div>
            ) : (
              <motion.nav
                key="collapsed-navigation"
                initial={reducedMotion ? false : { opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, y: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.12 }}
                className="mx-auto flex h-full min-h-0 w-10 flex-col items-center gap-1 overflow-y-auto"
                aria-label="快捷导航"
              >
                {[...PRIMARY_ROUTES, ...SECONDARY_ROUTES].map((item) => {
                  const Icon = item.icon;
                  return (
                    <NavLink
                      key={item.id}
                      to={item.path}
                      title={`${item.label} (${item.enLabel})`}
                      aria-label={item.label}
                      className={({ isActive }) =>
                        `relative grid h-9 w-10 place-items-center rounded-xl transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent/40 ${
                          isActive
                            ? "text-accent"
                            : "text-ink-secondary hover:bg-hover/80 hover:text-ink"
                        }`
                      }
                    >
                      {({ isActive }) => (
                        <>
                          {isActive && (
                            <motion.span
                              layoutId="sidebar-collapsed-active-pill"
                              transition={reducedMotion ? { duration: 0 } : SIDEBAR_PILL_TRANSITION}
                              className="pointer-events-none absolute inset-0 rounded-xl bg-surface shadow-[0_2px_6px_rgba(91,72,136,0.08)] ring-1 ring-line"
                            />
                          )}
                          <Icon size={15} className="relative z-10" />
                        </>
                      )}
                    </NavLink>
                  );
                })}
              </motion.nav>
            )}
          </AnimatePresence>
        </div>
      </motion.aside>

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
              className="grid h-5 w-5 cursor-pointer place-items-center rounded text-ink-muted hover:bg-line/60 hover:text-ink"
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
              className="grid h-5 w-5 cursor-pointer place-items-center rounded text-coral-text hover:bg-coral-border/40"
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
              initial={reducedMotion ? false : { opacity: 0, y: -8, scale: 0.97 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={reducedMotion ? undefined : { opacity: 0, y: -6, scale: 0.97 }}
              transition={{ duration: 0.16 }}
              className="pointer-events-none fixed top-4 left-1/2 z-50 -translate-x-1/2 rounded-xl bg-toast-bg px-3.5 py-2 text-xs font-medium text-toast-ink shadow-[0_8px_24px_rgba(51,43,41,0.18)]"
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

      {/* Create Document Modal */}
      {isCreating && (
        <div
          className="fixed inset-0 z-40 grid place-items-center bg-black/30 backdrop-blur-[2px]"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setIsCreating(false);
          }}
        >
          <form
            className="relative w-[min(440px,calc(100vw-32px))] rounded-2xl border border-line bg-surface-soft p-6 shadow-[0_20px_50px_rgba(51,43,41,0.14)]"
            onSubmit={(event) => {
              event.preventDefault();
              void handleCreateSubmit();
            }}
          >
            <button
              type="button"
              className="absolute top-4 right-4 grid h-7 w-7 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-sidebar hover:text-ink"
              onClick={() => setIsCreating(false)}
              aria-label="关闭"
            >
              <X size={15} />
            </button>

            <div className="mb-4 grid h-9 w-9 place-items-center rounded-xl border border-accent-border bg-accent-soft text-accent-deep">
              <FilePlus2 size={17} />
            </div>

            <h2 className="text-lg font-semibold tracking-tight text-ink">新建内容主稿</h2>
            <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
              先选择内容形态，再创建本地内容包；生成平台稿时不会改写主稿。
            </p>

            <div className="mt-4">
              <span className="mb-1.5 block text-xs font-medium text-ink">内容形态</span>
              <div className="grid grid-cols-2 gap-1.5">
                {CONTENT_KIND_META.map((item) => (
                  <button
                    key={item.kind}
                    type="button"
                    onClick={() => setNewKind(item.kind)}
                    className={`rounded-xl border p-2.5 text-left transition-colors ${
                      newKind === item.kind
                        ? "border-accent bg-accent-soft/70 ring-1 ring-accent/30"
                        : "border-line bg-canvas hover:border-line-strong"
                    }`}
                  >
                    <strong className="block text-[11.5px] font-medium text-ink">
                      {item.label}
                    </strong>
                    <small className="mt-0.5 block text-[10px] leading-snug text-ink-muted">
                      {item.description}
                    </small>
                  </button>
                ))}
              </div>
            </div>

            <label
              htmlFor="document-title"
              className="mt-4 mb-1.5 block text-xs font-medium text-ink"
            >
              内容标题
            </label>
            <input
              id="document-title"
              autoFocus
              value={newTitle}
              onChange={(event) => setNewTitle(event.target.value)}
              placeholder="例如：一次关于边缘计算的轻量化实践"
              className="h-9.5 w-full rounded-xl border border-line-strong bg-surface px-3 text-sm text-ink placeholder:text-ink-muted focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
            />

            <div className="mt-3">
              <span className="text-[11px] text-ink-muted">快速填入示例标题：</span>
              <div className="mt-1.5 flex flex-wrap gap-1.5">
                {QUICK_STARTER_TITLES.map((suggestion) => (
                  <button
                    key={suggestion}
                    type="button"
                    onClick={() => setNewTitle(suggestion)}
                    className="cursor-pointer rounded-lg border border-line bg-canvas px-2.5 py-1 text-[11px] text-ink-secondary transition-colors hover:border-accent-border hover:bg-accent-soft hover:text-accent-deep"
                  >
                    {suggestion}
                  </button>
                ))}
              </div>
            </div>

            <div className="mt-6 flex justify-end gap-2">
              <button
                type="button"
                className="h-8.5 cursor-pointer rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary transition-colors hover:bg-sidebar"
                onClick={() => setIsCreating(false)}
              >
                取消
              </button>
              <button
                type="submit"
                disabled={!newTitle.trim()}
                className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.24)] transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
              >
                <Plus size={13} />
                创建并开始编辑
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
