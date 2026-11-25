import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useState } from "react";
import { ExternalLink, Link2, LoaderCircle, Save, ShieldAlert } from "./Icons";
import { PlatformLogo } from "./PlatformLogo";
import { useCreatorCenterStore } from "../stores/creatorCenterStore";
import { useUiStore } from "../stores";

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

  useEffect(() => {
    void initialize();
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
              CreatorRelay 的 MCP
              可读取本地同步的作品/指标记录和平台能力配置；目前没有连接这些平台的远程 MCP
              服务或实时创作者 API。
              平台专属发布工具执行本地内容校验与人工交接，不会直接提交作品，自动发布保持关闭。
            </p>
            <p className="text-[11px] leading-4 text-ink-muted">
              网页可达或能打开创作者中心不等同于已登录、已读取实时数据或已验证发布。授权后仍需逐平台完成官方接口与账号级验证。
            </p>
          </div>
        </div>
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
