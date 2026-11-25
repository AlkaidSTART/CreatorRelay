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
import { NavLink, Outlet, useLocation, useNavigate } from "react-router";
import { PRIMARY_ROUTES, SECONDARY_ROUTES } from "../routes/navigation";
import { useContentStore, useUiStore, useWorkspaceStore } from "../stores";

export function AppLayout() {
  const navigate = useNavigate();
  const location = useLocation();
  const reducedMotion = useReducedMotion();

  const workspace = useWorkspaceStore((state) => state.workspace);
  const loading = useWorkspaceStore((state) => state.loading);
  const initialize = useWorkspaceStore((state) => state.initialize);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const createDocument = useWorkspaceStore((state) => state.createDocument);
  const saveCurrentDocument = useWorkspaceStore((state) => state.saveCurrentDocument);

  const sidebarOpen = useUiStore((state) => state.sidebarOpen);
  const setSidebarOpen = useUiStore((state) => state.setSidebarOpen);
  const showDemoBanner = useUiStore((state) => state.showDemoBanner);
  const setShowDemoBanner = useUiStore((state) => state.setShowDemoBanner);
  const isCreating = useUiStore((state) => state.isCreating);
  const setIsCreating = useUiStore((state) => state.setIsCreating);
  const newTitle = useUiStore((state) => state.newTitle);
  const setNewTitle = useUiStore((state) => state.setNewTitle);
  const error = useUiStore((state) => state.error);
  const clearError = useUiStore((state) => state.clearError);
  const notice = useUiStore((state) => state.notice);

  const pendingTaskCount = useContentStore(
    (state) =>
      state.publishTasks.filter(
        (task) => task.status === "NEEDS_MANUAL_ACTION" || task.status === "SCHEDULED",
      ).length,
  );

  const isStudioRoute = location.pathname.startsWith("/studio");

  useEffect(() => {
    void initialize();
  }, [initialize]);

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
    const created = await createDocument();
    if (created && !isStudioRoute) {
      navigate("/studio");
    }
  }

  if (loading) {
    return (
      <div className="flex h-full w-full flex-col items-center justify-center gap-3 bg-white text-zinc-500">
        <div className="grid h-9 w-9 animate-pulse place-items-center rounded-xl bg-zinc-900 text-xs font-semibold tracking-tight text-white">
          cr
        </div>
        <span className="text-xs text-zinc-400">正在整理你的工作区...</span>
      </div>
    );
  }

  return (
    <div className="flex h-full w-full overflow-hidden bg-white text-zinc-900">
      <motion.aside
        className="relative z-10 flex h-full shrink-0 flex-col overflow-hidden border-r border-zinc-200/80 bg-zinc-50/90 px-3 py-4"
        initial={false}
        animate={{ width: sidebarOpen ? 240 : 72 }}
        transition={reducedMotion ? { duration: 0 } : { type: "spring", stiffness: 340, damping: 34, mass: 0.8 }}
        aria-label="应用侧边栏"
      >
        <div className="flex h-8 shrink-0 items-center justify-between gap-2 px-0.5">
          <div className="flex min-w-0 items-center gap-2.5">
            <div className="grid h-7 w-7 shrink-0 place-items-center rounded-lg bg-zinc-900 text-[11px] font-semibold tracking-tight text-white">
              cr
            </div>
            <AnimatePresence initial={false}>
              {sidebarOpen && (
                <motion.span
                  key="brand-label"
                  initial={reducedMotion ? false : { opacity: 0, x: -7 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={reducedMotion ? undefined : { opacity: 0, x: -5 }}
                  transition={{ duration: reducedMotion ? 0 : 0.16, delay: sidebarOpen && !reducedMotion ? 0.06 : 0 }}
                  className="whitespace-nowrap text-[14px] font-semibold tracking-tight text-zinc-900"
                >
                  CreatorRelay
                </motion.span>
              )}
            </AnimatePresence>
          </div>
          <motion.button
            type="button"
            whileHover={reducedMotion ? undefined : { scale: 1.06, backgroundColor: "#e4e4e7" }}
            whileTap={reducedMotion ? undefined : { scale: 0.94 }}
            transition={{ duration: reducedMotion ? 0 : 0.14 }}
            className="grid h-7 w-7 shrink-0 cursor-pointer place-items-center rounded-md text-zinc-500 outline-none hover:text-zinc-900 focus-visible:ring-2 focus-visible:ring-zinc-400"
            aria-label={sidebarOpen ? "收起侧边栏" : "展开侧边栏"}
            aria-expanded={sidebarOpen}
            title={sidebarOpen ? "收起侧边栏" : "展开侧边栏"}
            onClick={() => setSidebarOpen(!sidebarOpen)}
          >
            {sidebarOpen ? <PanelLeftClose size={15} /> : <PanelLeftOpen size={15} />}
          </motion.button>
        </div>

        <button
          className={`mt-4 flex h-10 shrink-0 cursor-pointer items-center overflow-hidden rounded-lg border border-zinc-200/80 bg-white text-left transition-colors hover:border-zinc-300 hover:bg-zinc-50 ${sidebarOpen ? "w-full gap-2.5 px-2" : "w-10 justify-center self-center p-0"}`}
          onClick={() => void chooseWorkspace()}
          title={workspace?.name ?? "选择工作区"}
          aria-label={sidebarOpen ? undefined : `工作区：${workspace?.name ?? "选择工作区"}`}
        >
          <span className="grid h-6 w-6 shrink-0 place-items-center rounded-md bg-zinc-100 text-xs font-semibold text-zinc-700">
            {workspace?.name.slice(0, 1).toUpperCase() ?? "C"}
          </span>
          <AnimatePresence initial={false}>
            {sidebarOpen && (
              <motion.span
                key="workspace-label"
                initial={reducedMotion ? false : { opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.15 }}
                className="flex min-w-0 flex-1 items-center gap-2.5"
              >
                <span className="flex min-w-0 flex-1 flex-col">
                  <strong className="truncate text-xs font-medium text-zinc-800">
                    {workspace?.name ?? "选择工作区"}
                  </strong>
                  <small className="text-[10px] text-zinc-400">本地空间</small>
                </span>
                <ChevronsUpDown size={13} className="shrink-0 text-zinc-400" />
              </motion.span>
            )}
          </AnimatePresence>
        </button>

        <motion.button
          whileHover={reducedMotion ? undefined : { y: -1, backgroundColor: "#27272a" }}
          whileTap={reducedMotion ? undefined : { scale: 0.97 }}
          transition={{ duration: reducedMotion ? 0 : 0.14 }}
          className={`mt-3 flex h-9 shrink-0 cursor-pointer items-center justify-center gap-2 overflow-hidden whitespace-nowrap rounded-lg bg-zinc-900 text-xs font-medium text-white ${sidebarOpen ? "w-full px-3" : "w-10 self-center p-0"}`}
          onClick={() => setIsCreating(true)}
          title="新建文章 (⌘N)"
          aria-label={sidebarOpen ? undefined : "新建文章"}
        >
          <Plus size={14} />
          <AnimatePresence initial={false}>
            {sidebarOpen && (
              <motion.span
                key="create-label"
                initial={reducedMotion ? false : { opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.15 }}
                className="flex min-w-0 flex-1 items-center whitespace-nowrap"
              >
                <span>新建文章</span>
                <kbd className="ml-auto rounded bg-white/15 px-1.5 py-0.5 text-[10px] font-normal text-zinc-300">⌘N</kbd>
              </motion.span>
            )}
          </AnimatePresence>
        </motion.button>

        <div className={`mt-6 min-h-0 flex-1 ${sidebarOpen ? "flex flex-col" : "flex flex-col items-center"}`}>
          <AnimatePresence mode="wait" initial={false}>
            {sidebarOpen ? (
              <motion.div
                key="expanded-navigation"
                initial={reducedMotion ? false : { opacity: 0, x: -5 }}
                animate={{ opacity: 1, x: 0 }}
                exit={reducedMotion ? undefined : { opacity: 0, x: -4 }}
                transition={{ duration: reducedMotion ? 0 : 0.15 }}
                className="flex h-full min-h-0 flex-col"
              >
                <div className="mb-1.5 px-2 text-[10px] font-medium tracking-wider text-zinc-400 uppercase">工作区</div>
                <nav className="flex flex-col gap-0.5" aria-label="主导航">
                  {PRIMARY_ROUTES.map((item) => {
                    const Icon = item.icon;
                    const badge = item.id === "publishing" && pendingTaskCount > 0 ? String(pendingTaskCount) : undefined;
                    return (
                      <motion.div key={item.id} whileHover={reducedMotion ? undefined : { x: 2 }} whileTap={reducedMotion ? undefined : { scale: 0.98 }} transition={{ duration: reducedMotion ? 0 : 0.13 }}>
                        <NavLink
                          to={item.path}
                          className={({ isActive }) => `group flex h-9 items-center gap-2.5 rounded-lg px-2.5 text-[13px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 ${isActive ? "bg-zinc-200/80 font-medium text-zinc-950" : "text-zinc-600 hover:bg-zinc-200/60 hover:text-zinc-950"}`}
                        >
                          <Icon size={16} className="shrink-0 transition-transform duration-150 group-hover:scale-105" />
                          <span className="truncate">{item.label}</span>
                          {badge && <span className="ml-auto rounded-md bg-zinc-300/70 px-1.5 py-0.5 text-[10px] font-medium text-zinc-700">{badge}</span>}
                        </NavLink>
                      </motion.div>
                    );
                  })}
                </nav>

                <div className="mt-auto flex flex-col gap-0.5 border-t border-zinc-200/80 pt-3">
                  <div className="mb-1.5 px-2 text-[10px] font-medium tracking-wider text-zinc-400 uppercase">工具与系统</div>
                  {SECONDARY_ROUTES.map((item) => {
                    const Icon = item.icon;
                    return (
                      <motion.div key={item.id} whileHover={reducedMotion ? undefined : { x: 2 }} whileTap={reducedMotion ? undefined : { scale: 0.98 }} transition={{ duration: reducedMotion ? 0 : 0.13 }}>
                        <NavLink
                          to={item.path}
                          className={({ isActive }) => `group flex h-8 items-center gap-2.5 rounded-lg px-2.5 text-[12.5px] transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 ${isActive ? "bg-zinc-200/80 font-medium text-zinc-950" : "text-zinc-500 hover:bg-zinc-200/60 hover:text-zinc-950"}`}
                        >
                          <Icon size={15} className="shrink-0 transition-transform duration-150 group-hover:scale-105" />
                          <span className="truncate">{item.label}</span>
                          {item.id === "assistant" && <span className="ml-auto h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500" />}
                        </NavLink>
                      </motion.div>
                    );
                  })}
                  <button
                    onClick={() => navigate("/settings")}
                    className="mt-2 flex cursor-pointer items-center gap-2.5 border-t border-zinc-200/80 px-1.5 pt-3 text-left transition-opacity hover:opacity-80"
                  >
                    <div className="grid h-7 w-7 shrink-0 place-items-center rounded-full border border-zinc-200 bg-white text-xs font-medium text-zinc-700">林</div>
                    <div className="flex min-w-0 flex-1 flex-col">
                      <strong className="truncate text-xs font-medium text-zinc-800">创作者</strong>
                      <small className="truncate text-[10px] text-zinc-400">本地优先模式</small>
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
                transition={{ duration: reducedMotion ? 0 : 0.13 }}
                className="flex h-full min-h-0 flex-col items-center gap-1 overflow-y-auto"
                aria-label="快捷导航"
              >
                {[...PRIMARY_ROUTES, ...SECONDARY_ROUTES].map((item) => {
                  const Icon = item.icon;
                  return (
                    <motion.div key={item.id} whileHover={reducedMotion ? undefined : { scale: 1.07, y: -1 }} whileTap={reducedMotion ? undefined : { scale: 0.94 }} transition={{ duration: reducedMotion ? 0 : 0.12 }}>
                      <NavLink
                        to={item.path}
                        title={`${item.label} (${item.enLabel})`}
                        aria-label={item.label}
                        className={({ isActive }) => `grid h-9 w-10 place-items-center rounded-lg transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-zinc-400 ${isActive ? "bg-zinc-200/80 text-zinc-950" : "text-zinc-500 hover:bg-zinc-200/60 hover:text-zinc-950"}`}
                      >
                        <Icon size={16} />
                      </NavLink>
                    </motion.div>
                  );
                })}
              </motion.nav>
            )}
          </AnimatePresence>
        </div>
      </motion.aside>

      {/* Main Content Stage */}
      <div className="relative flex min-w-0 flex-1 flex-col overflow-hidden bg-white">
        {showDemoBanner && (
          <div className="flex items-center justify-between border-b border-zinc-200/80 bg-zinc-50 px-5 py-1.5 text-[11px] text-zinc-500">
            <span>
              <strong className="mr-2 font-medium text-zinc-800">预览模式</strong>
              当前为本地演示内容，桌面版将直接读写你的 SQLite 工作区。
            </span>
            <button
              onClick={() => setShowDemoBanner(false)}
              aria-label="关闭提示"
              className="grid h-5 w-5 cursor-pointer place-items-center rounded text-zinc-400 hover:bg-zinc-200/60 hover:text-zinc-700"
            >
              <X size={13} />
            </button>
          </div>
        )}

        {error && (
          <div className="flex items-center justify-between border-b border-red-200 bg-red-50 px-5 py-2 text-xs text-red-700">
            <span>{error}</span>
            <button
              onClick={clearError}
              aria-label="关闭错误"
              className="grid h-5 w-5 cursor-pointer place-items-center rounded text-red-500 hover:bg-red-100"
            >
              <X size={13} />
            </button>
          </div>
        )}

        {notice && (
          <div
            role="status"
            className="pointer-events-none fixed top-4 left-1/2 z-50 -translate-x-1/2 rounded-lg bg-zinc-900 px-3.5 py-2 text-xs font-medium text-white shadow-lg"
          >
            {notice}
          </div>
        )}

        <div className="min-h-0 min-w-0 flex-1 overflow-hidden">
          <Outlet />
        </div>
      </div>

      {/* Create Document Modal */}
      {isCreating && (
        <div
          className="fixed inset-0 z-40 grid place-items-center bg-zinc-950/25 backdrop-blur-[2px]"
          onMouseDown={(event) => {
            if (event.target === event.currentTarget) setIsCreating(false);
          }}
        >
          <form
            className="relative w-[min(420px,calc(100vw-32px))] rounded-2xl border border-zinc-200 bg-white p-6 shadow-2xl"
            onSubmit={(event) => {
              event.preventDefault();
              void handleCreateSubmit();
            }}
          >
            <button
              type="button"
              className="absolute top-4 right-4 grid h-7 w-7 cursor-pointer place-items-center rounded-md text-zinc-400 transition-colors hover:bg-zinc-100 hover:text-zinc-700"
              onClick={() => setIsCreating(false)}
              aria-label="关闭"
            >
              <X size={16} />
            </button>

            <div className="mb-4 grid h-9 w-9 place-items-center rounded-xl bg-zinc-100 text-zinc-800">
              <FilePlus2 size={18} />
            </div>

            <h2 className="text-lg font-semibold tracking-tight text-zinc-900">开始一篇新文章</h2>
            <p className="mt-1 text-xs leading-relaxed text-zinc-500">
              创建一个独立的 Markdown 主稿，内容将保存在当前工作区目录中。
            </p>

            <label
              htmlFor="document-title"
              className="mt-5 mb-1.5 block text-xs font-medium text-zinc-700"
            >
              文章标题
            </label>
            <input
              id="document-title"
              autoFocus
              value={newTitle}
              onChange={(event) => setNewTitle(event.target.value)}
              placeholder="例如：一次关于边缘计算的实践"
              className="h-9 w-full rounded-lg border border-zinc-200 bg-white px-3 text-sm text-zinc-900 placeholder:text-zinc-400 focus:border-zinc-900 focus:outline-none"
            />

            <div className="mt-6 flex justify-end gap-2">
              <button
                type="button"
                className="h-8 cursor-pointer rounded-lg border border-zinc-200 bg-white px-3.5 text-xs font-medium text-zinc-600 transition-colors hover:bg-zinc-50"
                onClick={() => setIsCreating(false)}
              >
                取消
              </button>
              <button
                type="submit"
                disabled={!newTitle.trim()}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3.5 text-xs font-medium text-white transition-colors hover:bg-zinc-800 disabled:cursor-not-allowed disabled:bg-zinc-300"
              >
                <Plus size={14} />
                创建文章
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}
