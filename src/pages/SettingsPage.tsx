import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Check, FolderOpen, Palette, Shield } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useUiStore, useWorkspaceStore } from "../stores";
import { THEMES } from "../utils/themes";

type RuntimeDiagnosticsSummary = {
  databasePath: string;
};

export function SettingsPage() {
  const workspace = useWorkspaceStore((state) => state.workspace);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const theme = useUiStore((state) => state.theme);
  const setTheme = useUiStore((state) => state.setTheme);
  const showToast = useUiStore((state) => state.showToast);
  const setError = useUiStore((state) => state.setError);
  const [rebuilding, setRebuilding] = useState(false);
  const [databasePath, setDatabasePath] = useState<string | null>(null);

  useEffect(() => {
    if (!isTauri()) return;
    void invoke<RuntimeDiagnosticsSummary>("get_runtime_diagnostics")
      .then((info) => setDatabasePath(info.databasePath))
      .catch(() => {});
  }, []);

  async function rebuildIndex() {
    if (!isTauri()) {
      showToast("请在桌面端重建工作区索引");
      return;
    }
    setRebuilding(true);
    try {
      const count = await invoke<number>("rebuild_workspace_index");
      await useWorkspaceStore.getState().loadDocuments(useWorkspaceStore.getState().query);
      showToast(`索引重建完成，发现 ${count} 篇文章`);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setRebuilding(false);
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="SETTINGS"
        title="常规与外观"
        description="管理界面主题、本地工作区文件夹以及索引安全备份策略。"
      />

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
        <div className="max-w-4xl space-y-5">
          {/* Appearance & Themes */}
          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center justify-between gap-4 border-b border-line px-5 py-3.5">
              <div className="flex items-center gap-2.5">
                <Palette size={14} className="shrink-0 text-ink-muted" />
                <div>
                  <h2 className="text-[13px] font-semibold text-ink">外观与工作台主题</h2>
                  <small className="text-[11.5px] text-ink-muted">
                    内置 5 套低饱和度高对比度主题（含深空暗色模式），切换后实时生效并自动保存
                  </small>
                </div>
              </div>
            </div>
            <div className="p-5">
              <div
                className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-5"
                role="radiogroup"
                aria-label="选择应用主题"
              >
                {THEMES.map((item) => {
                  const isActive = item.id === theme;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      role="radio"
                      aria-checked={isActive}
                      onClick={() => {
                        setTheme(item.id);
                        showToast(`已切换为「${item.name}」主题`);
                      }}
                      className={`group flex cursor-pointer flex-col rounded-md border p-3 text-left transition-colors ${
                        isActive
                          ? "border-accent bg-accent-soft/50 ring-1 ring-accent/30"
                          : "border-line bg-surface hover:border-line-strong hover:bg-subtle/60"
                      }`}
                    >
                      <div
                        className="mb-2.5 flex h-11 w-full items-end justify-between rounded-sm border border-black/10 p-2"
                        style={{ backgroundColor: item.swatches[0] }}
                      >
                        <div className="flex items-center gap-1">
                          {item.swatches.map((color) => (
                            <span
                              key={color}
                              className="h-3 w-3 rounded-xs border border-black/15"
                              style={{ backgroundColor: color }}
                            />
                          ))}
                        </div>
                        {isActive && (
                          <span
                            className="grid h-4 w-4 place-items-center rounded-xs text-white"
                            style={{ backgroundColor: item.swatches[2] }}
                          >
                            <Check size={9} />
                          </span>
                        )}
                      </div>
                      <div className="flex items-center justify-between gap-1">
                        <strong className="text-xs font-semibold text-ink">{item.name}</strong>
                        {item.isDark && (
                          <span className="rounded-xs border border-accent-border bg-accent-soft px-1 py-px text-[9.5px] font-medium text-accent-deep">
                            深色
                          </span>
                        )}
                      </div>
                      <span className="font-mono text-[10px] text-ink-muted">{item.enName}</span>
                      <small className="mt-1 line-clamp-2 text-[10.5px] leading-snug text-ink-secondary">
                        {item.description}
                      </small>
                    </button>
                  );
                })}
              </div>
            </div>
          </article>

          {/* Workspace Storage */}
          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center justify-between gap-4 border-b border-line px-5 py-3.5">
              <div className="flex items-center gap-2.5">
                <FolderOpen size={14} className="shrink-0 text-ink-muted" />
                <div>
                  <h2 className="text-[13px] font-semibold text-ink">工作区与本地存储</h2>
                  <small className="text-[11.5px] text-ink-muted">
                    Markdown 文件为正文权威数据源，本地索引随时可无损重建
                  </small>
                </div>
              </div>
              <button
                className="h-7.5 shrink-0 cursor-pointer rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink transition-colors hover:bg-subtle"
                onClick={() => void chooseWorkspace()}
              >
                切换工作区
              </button>
            </div>
            <div className="grid grid-cols-1 divide-y divide-line sm:grid-cols-3 sm:divide-x sm:divide-y-0">
              <div className="px-5 py-3.5">
                <span className="block text-[11px] font-medium text-ink-muted">当前工作区路径</span>
                <code className="mt-1 block truncate font-mono text-[11.5px] text-ink">
                  {workspace?.path ?? "尚未选择工作区"}
                </code>
              </div>
              <div className="px-5 py-3.5">
                <span className="block text-[11px] font-medium text-ink-muted">SQLite 索引库</span>
                <code className="mt-1 block break-all font-mono text-[11px] text-ink">
                  {databasePath ?? "由 Rust Core 在应用数据目录管理"}
                </code>
              </div>
              <div className="px-5 py-3.5">
                <span className="block text-[11px] font-medium text-ink-muted">素材与导出目录</span>
                <code className="mt-1 block break-all font-mono text-[11px] text-ink">
                  {workspace?.path ? `${workspace.path}/assets · exports` : "选择工作区后创建"}
                </code>
              </div>
            </div>
          </article>

          {/* Local Backup & Index */}
          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center justify-between gap-4 border-b border-line px-5 py-3.5">
              <div className="flex items-center gap-2.5">
                <Shield size={14} className="shrink-0 text-ink-muted" />
                <div>
                  <h2 className="text-[13px] font-semibold text-ink">安全与本地备份</h2>
                  <small className="text-[11.5px] text-ink-muted">
                    Markdown 为正文权威；索引可安全重建
                  </small>
                </div>
              </div>
              <button
                className="inline-flex h-7.5 shrink-0 cursor-pointer items-center gap-1.5 rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink transition-colors hover:bg-subtle"
                onClick={() => void rebuildIndex()}
              >
                {rebuilding ? "正在重建..." : "重建本地索引"}
              </button>
            </div>
            <div className="px-5 py-3.5">
              <p className="text-xs leading-relaxed text-ink-secondary">
                索引重建只扫描当前工作区内的 Markdown 文件，不会修改文章正文或 Git 版本历史。
              </p>
            </div>
          </article>
        </div>
      </div>
    </main>
  );
}
