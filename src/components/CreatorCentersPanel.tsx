import { invoke, isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { ExternalLink, Link2, LoaderCircle, Save, ShieldAlert } from "./Icons";
import { PlatformLogo } from "./PlatformLogo";
import { useCreatorCenterStore } from "../stores/creatorCenterStore";
import { useUiStore, useMcpStore } from "../stores";
import type { McpAuthorizationPreview, McpCallResult } from "../types/mcp";

interface BrowserBridgeStatus {
  state: "running" | "stopped";
  browserVersion?: string | null;
  currentPlatformId?: string | null;
  currentUrl?: string | null;
  currentPageStatus?: string | null;
  stagedContentId?: string | null;
  lastError?: string | null;
  isolatedProfile: boolean;
  devtoolsReady: boolean;
}

interface BrowserPageSnapshot {
  platformId: string;
  platformName: string;
  url: string;
  title: string;
  text: string;
  pageStatus: string;
  note: string;
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function CreatorCentersPanel() {
  const links = useCreatorCenterStore((state) => state.links);
  const loading = useCreatorCenterStore((state) => state.loading);
  const error = useCreatorCenterStore((state) => state.error);
  const initialize = useCreatorCenterStore((state) => state.initialize);
  const saveUrl = useCreatorCenterStore((state) => state.saveUrl);
  const showToast = useUiStore((state) => state.showToast);
  const initializeMcp = useMcpStore((state) => state.initialize);
  const mcpHostStatus = useMcpStore((state) => state.status);
  const mcpReadEnabled = useMcpStore((state) => state.settings?.readEnabled ?? false);
  const lastBrowsedPlatformId = useCreatorCenterStore((state) => state.lastBrowsedPlatformId);
  const setLastBrowsedPlatformId = useCreatorCenterStore((state) => state.setLastBrowsedPlatformId);
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [savingId, setSavingId] = useState("");
  const [browserStatus, setBrowserStatus] = useState<BrowserBridgeStatus | null>(null);
  const [pageSnapshot, setPageSnapshot] = useState<BrowserPageSnapshot | null>(null);
  const [browserBusyId, setBrowserBusyId] = useState("");
  const [pendingOpen, setPendingOpen] = useState<{
    preview: McpAuthorizationPreview;
    platformId: string;
  } | null>(null);

  useEffect(() => {
    void initialize();
    void initializeMcp();
    if (isTauri()) {
      void invoke<BrowserBridgeStatus>("creator_browser_status")
        .then(setBrowserStatus)
        .catch(() => setBrowserStatus(null));
    }
  }, [initialize, initializeMcp]);

  async function openCenter(url: string) {
    try {
      const parsed = new URL(url);
      if (parsed.protocol !== "https:" || parsed.username || parsed.password) {
        throw new Error("仅允许打开不含凭据的 HTTPS 链接");
      }
      if (isTauri()) await openUrl(parsed.toString());
      else window.open(parsed.toString(), "_blank", "noopener,noreferrer");
    } catch (cause) {
      showToast(`无法打开创作者中心：${errorMessage(cause)}`);
    }
  }

  async function openInIsolatedBrowser(platformId: string) {
    if (!isTauri()) {
      showToast("请在 CreatorRelay 桌面版中使用 MCP 专用浏览器");
      return;
    }
    setBrowserBusyId(platformId);
    try {
      const preview = await invoke<McpAuthorizationPreview>("mcp_preview_tool_call", {
        request: { toolName: "creator_browser_open", arguments: { platformId }, threadId: null },
      });
      if (preview.fieldErrors.length) {
        showToast(preview.fieldErrors.map((item) => item.message).join("；"));
        return;
      }
      setPendingOpen({ preview, platformId });
    } catch (cause) {
      showToast(`无法预览浏览器操作：${errorMessage(cause)}`);
    } finally {
      setBrowserBusyId("");
    }
  }

  async function confirmOpenInIsolatedBrowser() {
    if (!pendingOpen) return;
    const { preview, platformId } = pendingOpen;
    setBrowserBusyId(platformId);
    try {
      const result = await invoke<McpCallResult>("mcp_call_tool", {
        request: {
          callId: preview.callId,
          toolName: preview.toolName,
          arguments: { platformId },
          threadId: null,
          authorizationToken: preview.authorizationToken ?? null,
          timeoutMs: 30_000,
        },
      });
      if (result.status !== "success") {
        throw new Error(result.error?.message ?? `工具状态：${result.status}`);
      }
      const page = result.result as BrowserPageSnapshot;
      setPageSnapshot(page);
      setLastBrowsedPlatformId(platformId);
      const status = await invoke<BrowserBridgeStatus>("creator_browser_status");
      setBrowserStatus(status);
      setPendingOpen(null);
      showToast(`${page.platformName}已由 MCP 在隔离浏览器中打开`);
    } catch (cause) {
      showToast(`无法打开隔离浏览器：${errorMessage(cause)}`);
    } finally {
      setBrowserBusyId("");
    }
  }

  async function stopBrowser() {
    try {
      const status = await invoke<BrowserBridgeStatus>("creator_browser_stop");
      setBrowserStatus(status);
      setPageSnapshot(null);
      setLastBrowsedPlatformId(null);
      showToast("CreatorRelay 专用浏览器已关闭");
    } catch (cause) {
      showToast(`无法关闭专用浏览器：${errorMessage(cause)}`);
    }
  }

  async function refreshPage(platformId: string) {
    if (!isTauri()) return;
    setBrowserBusyId(platformId);
    try {
      const preview = await invoke<McpAuthorizationPreview>("mcp_preview_tool_call", {
        request: {
          toolName: "creator_browser_read_page",
          arguments: { platformId },
          threadId: null,
        },
      });
      if (preview.fieldErrors.length) {
        showToast(preview.fieldErrors.map((item) => item.message).join("；"));
        return;
      }
      setPendingOpen({ preview, platformId });
    } catch (cause) {
      showToast(`无法预览页面读取：${errorMessage(cause)}`);
    } finally {
      setBrowserBusyId("");
    }
  }

  async function confirmReadPage() {
    if (!pendingOpen || pendingOpen.preview.toolName !== "creator_browser_read_page") return;
    const { preview, platformId } = pendingOpen;
    setBrowserBusyId(platformId);
    try {
      const result = await invoke<McpCallResult>("mcp_call_tool", {
        request: {
          callId: preview.callId,
          toolName: preview.toolName,
          arguments: { platformId },
          threadId: null,
          authorizationToken: preview.authorizationToken ?? null,
          timeoutMs: 30_000,
        },
      });
      if (result.status !== "success")
        throw new Error(result.error?.message ?? `工具状态：${result.status}`);
      const page = result.result as BrowserPageSnapshot;
      setPageSnapshot(page);
      setLastBrowsedPlatformId(platformId);
      const status = await invoke<BrowserBridgeStatus>("creator_browser_status");
      setBrowserStatus(status);
      setPendingOpen(null);
      showToast(`已读取页面可见文本：${page.title || page.platformName}`);
    } catch (cause) {
      showToast(`无法读取页面：${errorMessage(cause)}`);
    } finally {
      setBrowserBusyId("");
    }
  }

  async function persist(platformId: string) {
    const url = drafts[platformId];
    if (!url) return;
    setSavingId(platformId);
    try {
      const saved = await saveUrl(platformId, url);
      setDrafts((current) => ({ ...current, [platformId]: saved.url }));
      showToast(`${saved.platformName}创作者中心链接已保存`);
    } catch (cause) {
      showToast(errorMessage(cause));
    } finally {
      setSavingId("");
    }
  }

  return (
    <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 py-5">
      <section className="rounded-2xl border border-lemon-border bg-lemon-bg/50 p-4">
        <div className="flex items-start gap-3">
          <ShieldAlert className="mt-0.5 shrink-0 text-lemon-text" size={17} />
          <div className="space-y-1.5">
            <h2 className="text-sm font-semibold text-ink">MCP 平台能力边界</h2>
            <p className="text-xs leading-5 text-ink-secondary">
              CreatorRelay 可通过 MCP 调用专用 Chrome
              配置，打开已配置的平台链接并读取当前页面可见文本。 这不是平台远程 MCP/API 接入；不读取
              Cookie
              或隐藏接口、不自动发布。账号作品/指标是否可见取决于用户在隔离浏览器的登录状态和平台页面权限。
            </p>
            <p className="text-[11px] leading-4 text-ink-muted">
              网页可达或能打开创作者中心不等同于已登录、已读取实时数据或已验证发布。授权后仍需逐平台完成官方接口与账号级验证。
            </p>
          </div>
        </div>
      </section>

      <section className="rounded-2xl border border-line bg-surface-soft p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-semibold text-ink">MCP 专用浏览器</h2>
            <p className="mt-1 text-xs text-ink-muted">
              独立 Chrome
              配置与日常浏览器隔离；打开平台会向其发起网络请求并暴露常规连接元数据。首次使用请在新窗口中手动登录。
            </p>
          </div>
          <div className="flex items-center gap-2">
            <span
              className={`rounded-full px-2.5 py-1 text-[10px] font-medium ${browserStatus?.state === "running" ? "bg-mint-bg text-mint-text" : "bg-sidebar text-ink-muted"}`}
            >
              {browserStatus?.state === "running" ? "运行中" : "按需启动"}
              {browserStatus?.browserVersion ? ` · ${browserStatus.browserVersion}` : ""}
            </span>
            {browserStatus?.state === "running" && (
              <button
                type="button"
                onClick={() => void stopBrowser()}
                className="h-7 cursor-pointer rounded-lg border border-line px-2.5 text-[10px] text-ink-secondary hover:bg-sidebar"
              >
                关闭浏览器
              </button>
            )}
          </div>
        </div>
        {isTauri() && (!mcpHostStatus || mcpHostStatus.state !== "healthy") && (
          <p className="mt-2 rounded-lg border border-coral-border bg-coral-bg px-2.5 py-1.5 text-[10px] text-coral-text">
            MCP 服务未就绪，浏览器桥接工具当前不可用。
          </p>
        )}
        {isTauri() && !mcpReadEnabled && (
          <p className="mt-2 rounded-lg border border-lemon-border bg-lemon-bg px-2.5 py-1.5 text-[10px] text-lemon-text">
            MCP 只读权限已关闭，无法读取页面文本。
          </p>
        )}
        {browserStatus?.lastError && (
          <p className="mt-2 rounded-lg border border-coral-border bg-coral-bg px-2.5 py-1.5 text-[10px] text-coral-text">
            {browserStatus.lastError}
          </p>
        )}
        {browserStatus?.currentPageStatus && (
          <p className="mt-2 text-[10px] text-ink-muted">
            页面状态：
            {browserStatus.currentPageStatus === "login_required"
              ? "需要在专用浏览器登录"
              : browserStatus.currentPageStatus === "page_shell_or_loading"
                ? "页面加载未完成"
                : "页面已加载"}
          </p>
        )}
        {browserStatus?.currentUrl && (
          <p className="mt-2 truncate text-[10px] text-ink-muted">
            当前页面：{browserStatus.currentUrl}
          </p>
        )}
        {pageSnapshot && (
          <div className="mt-3 rounded-xl border border-line bg-surface p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="min-w-0">
                <div className="truncate text-xs font-semibold text-ink">
                  {pageSnapshot.title || pageSnapshot.platformName}
                </div>
                <div className="truncate text-[10px] text-ink-muted">{pageSnapshot.url}</div>
              </div>
              <span className="rounded-full bg-lemon-bg px-2 py-1 text-[10px] text-lemon-text">
                {pageSnapshot.pageStatus === "login_required"
                  ? "需在专用浏览器登录"
                  : pageSnapshot.pageStatus === "page_shell_or_loading"
                    ? "页面未加载完成"
                    : "当前页面可见文本"}
              </span>
            </div>
            <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap break-words text-[11px] leading-5 text-ink-secondary">
              {pageSnapshot.text || "页面正文暂不可读，请等待登录或页面加载后重试。"}
            </pre>
          </div>
        )}
      </section>

      <div className="flex items-end justify-between gap-4">
        <div>
          <h2 className="text-sm font-semibold text-ink">创作者中心链接</h2>
          <p className="mt-1 text-xs text-ink-muted">
            桌面版将链接保存在本机 SQLite。平台调整后台地址后，可在此编辑 HTTPS
            地址并保存；浏览器预览不写入 SQLite。
          </p>
        </div>
        <span className="shrink-0 rounded-full bg-sidebar px-2.5 py-1 text-[10px] font-medium text-ink-secondary">
          {links.length} 个平台
        </span>
      </div>

      {error && (
        <div className="rounded-xl border border-coral-border bg-coral-bg px-3 py-2 text-xs text-coral-text">
          读取 SQLite 中的创作者中心链接失败：{error}
        </div>
      )}

      {loading ? (
        <div className="flex items-center gap-2 py-8 text-sm text-ink-muted">
          <LoaderCircle className="animate-spin" size={16} /> 正在读取链接配置…
        </div>
      ) : (
        <div className="space-y-2.5">
          {links.map((link) => {
            const value = drafts[link.platformId] ?? link.url;
            const changed = value.trim() !== link.url;
            return (
              <article
                key={link.platformId}
                className="rounded-2xl border border-line bg-surface-soft px-4 py-3.5"
              >
                <div className="flex flex-wrap items-center gap-3">
                  <div className="flex min-w-[145px] flex-1 items-center gap-2.5">
                    <PlatformLogo
                      platformId={link.platformId}
                      platformName={link.platformName}
                      size={28}
                    />
                    <div>
                      <div className="text-xs font-semibold text-ink">{link.platformName}</div>
                      <div className="mt-0.5 text-[10px] text-ink-muted">
                        浏览器页面可见文本读取
                        <span className="mx-1.5">·</span>
                        官方 API/自动发布未验证
                      </div>
                    </div>
                  </div>
                  <div className="flex min-w-[260px] flex-[2] items-center gap-2">
                    <label className="sr-only" htmlFor={`creator-center-${link.platformId}`}>
                      {link.platformName}创作者中心网址
                    </label>
                    <input
                      id={`creator-center-${link.platformId}`}
                      type="url"
                      inputMode="url"
                      value={value}
                      onChange={(event) =>
                        setDrafts((current) => ({
                          ...current,
                          [link.platformId]: event.target.value,
                        }))
                      }
                      className="h-9 min-w-0 flex-1 rounded-xl border border-line bg-surface px-3 text-xs text-ink outline-none transition-colors placeholder:text-ink-muted focus:border-accent"
                    />
                    <button
                      type="button"
                      title="保存链接到 SQLite"
                      disabled={!changed || savingId === link.platformId}
                      onClick={() => void persist(link.platformId)}
                      className="inline-flex h-9 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl border border-line px-2.5 text-xs font-medium text-ink-secondary transition-colors hover:bg-sidebar disabled:cursor-not-allowed disabled:opacity-40"
                    >
                      {savingId === link.platformId ? (
                        <LoaderCircle className="animate-spin" size={13} />
                      ) : (
                        <Save size={13} />
                      )}
                      保存
                    </button>
                    <button
                      type="button"
                      title="在 CreatorRelay 隔离浏览器打开并读取页面"
                      disabled={browserBusyId === link.platformId}
                      onClick={() => void openInIsolatedBrowser(link.platformId)}
                      className="inline-flex h-9 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl border border-accent px-2.5 text-xs font-medium text-accent-deep transition-colors hover:bg-accent/10 disabled:cursor-wait disabled:opacity-50"
                    >
                      {browserBusyId === link.platformId ? (
                        <LoaderCircle className="animate-spin" size={13} />
                      ) : (
                        <ExternalLink size={13} />
                      )}
                      隔离浏览器
                    </button>
                    {(browserStatus?.currentPlatformId === link.platformId ||
                      lastBrowsedPlatformId === link.platformId) && (
                      <button
                        type="button"
                        disabled={browserBusyId === link.platformId}
                        onClick={() => void refreshPage(link.platformId)}
                        className="inline-flex h-9 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl border border-line px-2.5 text-xs font-medium text-ink-secondary transition-colors hover:bg-sidebar disabled:opacity-50"
                      >
                        <LoaderCircle
                          className={browserBusyId === link.platformId ? "animate-spin" : ""}
                          size={13}
                        />
                        读取当前页
                      </button>
                    )}
                    <button
                      type="button"
                      title="在系统浏览器打开"
                      onClick={() => void openCenter(link.url)}
                      className="inline-flex h-9 shrink-0 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-2.5 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover"
                    >
                      <ExternalLink size={13} /> 打开
                    </button>
                  </div>
                </div>
                <div className="mt-2 flex items-center gap-1.5 pl-10 text-[10px] text-ink-muted">
                  <Link2 size={11} />
                  最近修改：
                  {link.updatedAt ? new Date(link.updatedAt).toLocaleString("zh-CN") : "默认链接"}
                </div>
              </article>
            );
          })}
        </div>
      )}
      {pendingOpen && (
        <div className="fixed inset-0 z-[70] grid place-items-center bg-black/35 p-4 backdrop-blur-[2px]">
          <section
            role="dialog"
            aria-modal="true"
            aria-label={
              pendingOpen.preview.toolName === "creator_browser_read_page"
                ? "确认读取当前页面"
                : "确认打开创作者中心"
            }
            className="w-full max-w-lg rounded-2xl border border-line bg-surface p-5 shadow-2xl"
          >
            <h3 className="text-sm font-semibold text-ink">
              {pendingOpen.preview.toolName === "creator_browser_read_page"
                ? "确认读取当前页面？"
                : "确认让 MCP 打开创作者中心？"}
            </h3>
            <p className="mt-2 text-xs leading-5 text-ink-secondary">
              {pendingOpen.preview.toolName === "creator_browser_read_page"
                ? "这会把当前页面可见文本（可能包含账号个人信息、作品及指标）返回给 MCP 调用方。仅读取可见正文，不读取 Cookie、密码或隐藏 API，不提交表单或发布。"
                : "这会在 CreatorRelay 隔离 Chrome 配置中导航到目标平台。平台会收到网络连接请求和常规连接元数据；不会提交表单或发布作品。日常 Chrome 的 Cookie/密码不会被读取。"}
            </p>
            <div className="mt-3 rounded-xl border border-line bg-canvas p-3 text-[11px] text-ink-secondary">
              <div>
                <strong>平台：</strong>
                {String(pendingOpen.preview.details.platformName ?? pendingOpen.platformId)}
              </div>
              <div className="mt-1 break-all">
                <strong>目标网址：</strong>
                {String(pendingOpen.preview.details.url ?? "")}
              </div>
            </div>
            {pendingOpen.preview.effects.map((effect) => (
              <p key={effect} className="mt-2 text-[10px] leading-4 text-ink-muted">
                • {effect}
              </p>
            ))}
            <div className="mt-4 flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setPendingOpen(null)}
                className="h-8 cursor-pointer rounded-lg border border-line px-3 text-xs text-ink-secondary"
              >
                取消
              </button>
              <button
                type="button"
                disabled={Boolean(browserBusyId)}
                onClick={() =>
                  void (pendingOpen.preview.toolName === "creator_browser_read_page"
                    ? confirmReadPage()
                    : confirmOpenInIsolatedBrowser())
                }
                className="h-8 cursor-pointer rounded-lg bg-accent px-3 text-xs font-medium text-accent-on disabled:opacity-50"
              >
                {browserBusyId
                  ? "处理中…"
                  : pendingOpen.preview.toolName === "creator_browser_read_page"
                    ? "确认并读取"
                    : "确认并打开"}
              </button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
