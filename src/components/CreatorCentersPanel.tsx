import { invoke, isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { ExternalLink, Link2, LoaderCircle, Save, ShieldAlert } from "./Icons";
import { PlatformLogo } from "./PlatformLogo";
import { useCreatorCenterStore } from "../stores/creatorCenterStore";
import { useUiStore } from "../stores";

interface BrowserBridgeStatus {
  state: "running" | "stopped";
  browserVersion?: string | null;
  currentPlatformId?: string | null;
  currentUrl?: string | null;
  isolatedProfile: boolean;
}

interface BrowserPageSnapshot {
  platformId: string;
  platformName: string;
  url: string;
  title: string;
  text: string;
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
  const [drafts, setDrafts] = useState<Record<string, string>>({});
  const [savingId, setSavingId] = useState("");
  const [browserStatus, setBrowserStatus] = useState<BrowserBridgeStatus | null>(null);
  const [pageSnapshot, setPageSnapshot] = useState<BrowserPageSnapshot | null>(null);
  const [browserBusyId, setBrowserBusyId] = useState("");

  useEffect(() => {
    void initialize();
    if (isTauri()) {
      void invoke<BrowserBridgeStatus>("creator_browser_status")
        .then(setBrowserStatus)
        .catch(() => setBrowserStatus(null));
    }
  }, [initialize]);

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
    setBrowserBusyId(platformId);
    setPageSnapshot(null);
    try {
      const page = await invoke<BrowserPageSnapshot>("creator_browser_open", { platformId });
      setPageSnapshot(page);
      const status = await invoke<BrowserBridgeStatus>("creator_browser_status");
      setBrowserStatus(status);
      showToast(`${page.platformName}已在 CreatorRelay 隔离浏览器打开`);
    } catch (cause) {
      showToast(`无法打开隔离浏览器：${errorMessage(cause)}`);
    } finally {
      setBrowserBusyId("");
    }
  }

  async function refreshPage(platformId: string) {
    setBrowserBusyId(platformId);
    try {
      const page = await invoke<BrowserPageSnapshot>("creator_browser_read_page", { platformId });
      setPageSnapshot(page);
      showToast(`已读取当前页面：${page.title || page.platformName}`);
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
    <div className="min-h-0 flex-1 space-y-5 overflow-y-auto px-7 py-6">
      <section className="rounded-2xl border border-lemon-border bg-lemon-bg/50 p-4">
        <div className="flex items-start gap-3">
          <ShieldAlert className="mt-0.5 shrink-0 text-lemon-text" size={17} />
          <div className="space-y-1.5">
            <h2 className="text-sm font-semibold text-ink">MCP 平台能力边界</h2>
            <p className="text-xs leading-5 text-ink-secondary">
              CreatorRelay 可通过 MCP 启动一个隔离的 Chrome
              配置，打开已配置的平台链接，并读取当前页面可见文本。 这不是平台远程 MCP/API
              接入；不读取隐藏接口或
              Cookie，不自动发布。实际能否读到作品数据取决于用户在隔离浏览器中的登录状态和页面权限。
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
              独立 Chrome 配置与日常浏览器隔离；首次使用请在打开的窗口中手动登录平台。
            </p>
          </div>
          <span
            className={`rounded-full px-2.5 py-1 text-[10px] font-medium ${browserStatus?.state === "running" ? "bg-mint-bg text-mint-text" : "bg-sidebar text-ink-muted"}`}
          >
            {browserStatus?.state === "running" ? "运行中" : "按需启动"}
            {browserStatus?.browserVersion ? ` · ${browserStatus.browserVersion}` : ""}
          </span>
        </div>
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
                仅页面可见文本
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
                        MCP 实时读取未验证
                        <span className="mx-1.5">·</span>
                        自动发布关闭
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
                    {browserStatus?.currentPlatformId === link.platformId && (
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
    </div>
  );
}
