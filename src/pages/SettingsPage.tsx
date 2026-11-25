import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  Check,
  FolderOpen,
  Palette,
  Plug,
  RefreshCw,
  Server,
  Shield,
  Terminal,
} from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { AiProviderSettings } from "../components/AiProviderSettings";
import { useUiStore, useWorkspaceStore } from "../stores";
import { THEMES } from "../utils/themes";

type RuntimeDiagnostics = {
  appVersion: string;
  dataDir: string;
  databasePath: string;
  logPath: string;
  schemaVersion: number;
  targetSchemaVersion: number;
  workspacePath?: string | null;
  logAvailable: boolean;
  databaseIntegrity: string;
  uptimeSeconds: number;
};

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function SettingsPage() {
  const workspace = useWorkspaceStore((state) => state.workspace);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const theme = useUiStore((state) => state.theme);
  const setTheme = useUiStore((state) => state.setTheme);
  const showToast = useUiStore((state) => state.showToast);
  const setError = useUiStore((state) => state.setError);
  const [rebuilding, setRebuilding] = useState(false);
  const [diagnostics, setDiagnostics] = useState<RuntimeDiagnostics | null>(null);
  const [diagnosticsError, setDiagnosticsError] = useState("");
  const [diagnosticsBusy, setDiagnosticsBusy] = useState("");

  async function refreshDiagnostics() {
    if (!isTauri()) return;
    try {
      setDiagnostics(await invoke<RuntimeDiagnostics>("get_runtime_diagnostics"));
      setDiagnosticsError("");
    } catch (cause) {
      setDiagnosticsError(errorMessage(cause));
    }
  }

  useEffect(() => {
    void refreshDiagnostics();
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

  async function runProbe(probe: "ipc_payload" | "database_error" | "invalid_path") {
    if (!isTauri()) {
      showToast("诊断探针仅在桌面应用中运行");
      return;
    }
    setDiagnosticsBusy(probe);
    setError("");
    try {
      if (probe === "invalid_path") {
        await invoke("set_workspace_path", {
          path: "/__creatorrelay_missing_m1_ipc_probe__",
        });
        setError("诊断失败：非法路径探针未返回预期错误");
      } else {
        await invoke("run_diagnostic_probe", { probe });
        setError("诊断失败：错误探针未返回预期错误");
      }
    } catch (cause) {
      setError(errorMessage(cause));
      await refreshDiagnostics();
    } finally {
      setDiagnosticsBusy("");
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="SETTINGS"
        title="系统设置"
        description="管理外观主题、本地工作区文件夹、自带 AI 模型密钥、MCP 扩展与安全备份策略。"
      />

      <div className="min-h-0 flex-1 overflow-y-auto px-7 py-6">
        <div className="max-w-4xl space-y-5">
          {/* Appearance & Themes */}
          <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(0,0,0,0.03)]">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent-deep">
                  <Palette size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-ink">外观与马卡龙主题</h2>
                  <small className="text-xs text-ink-muted">
                    内置 5 套低饱和度高对比主题（含护眼深色模式），切换后实时生效并自动保存
                  </small>
                </div>
              </div>
            </div>
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
                    className={`group flex cursor-pointer flex-col rounded-xl border p-3 text-left transition-all ${
                      isActive
                        ? "border-accent bg-accent-soft/70 ring-1 ring-accent/40"
                        : "border-line bg-subtle hover:border-line-strong hover:bg-hover/60"
                    }`}
                  >
                    <div
                      className="mb-2.5 flex h-12 w-full items-end justify-between rounded-lg border border-black/10 p-2"
                      style={{ backgroundColor: item.swatches[0] }}
                    >
                      <div className="flex items-center gap-1">
                        {item.swatches.map((color) => (
                          <span
                            key={color}
                            className="h-3.5 w-3.5 rounded-full border border-black/10"
                            style={{ backgroundColor: color }}
                          />
                        ))}
                      </div>
                      {isActive && (
                        <span
                          className="grid h-4 w-4 place-items-center rounded-full text-white"
                          style={{ backgroundColor: item.swatches[2] }}
                        >
                          <Check size={9} />
                        </span>
                      )}
                    </div>
                    <div className="flex items-center justify-between gap-1">
                      <strong className="text-xs font-semibold text-ink">{item.name}</strong>
                      {item.isDark && (
                        <span className="rounded bg-accent-soft px-1.5 py-0.5 text-[9.5px] font-medium text-accent-deep">
                          深色
                        </span>
                      )}
                    </div>
                    <small className="mt-0.5 line-clamp-2 text-[10.5px] leading-snug text-ink-muted">
                      {item.description}
                    </small>
                  </button>
                );
              })}
            </div>
          </article>

          {/* Workspace Storage */}
          <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-sky-bg text-sky-text">
                  <FolderOpen size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-ink">工作区与本地存储</h2>
                  <small className="text-xs text-ink-muted">
                    Markdown 文件为正文权威数据源，本地索引随时可无损重建
                  </small>
                </div>
              </div>
              <button
                className="h-8 shrink-0 cursor-pointer rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink hover:bg-subtle"
                onClick={() => void chooseWorkspace()}
              >
                切换工作区
              </button>
            </div>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="block text-[11px] text-ink-muted">当前工作区路径</span>
                <code className="mt-1 block truncate font-mono text-xs text-ink">
                  {workspace?.path ?? "尚未选择工作区"}
                </code>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="block text-[11px] text-ink-muted">SQLite 索引库</span>
                <code className="mt-1 block break-all font-mono text-[10.5px] text-ink">
                  {diagnostics?.databasePath ?? "由 Rust Core 在应用数据目录管理"}
                </code>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="block text-[11px] text-ink-muted">素材与导出目录</span>
                <code className="mt-1 block break-all font-mono text-[10.5px] text-ink">
                  {workspace?.path ? `${workspace.path}/assets · exports` : "选择工作区后创建"}
                </code>
              </div>
            </div>
          </article>

          <AiProviderSettings />

          {/* MCP & Security */}
          <section className="grid grid-cols-1 gap-4 md:grid-cols-2">
            <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
              <div className="mb-3 flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-rose-bg text-rose-text">
                  <Server size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-ink">MCP Server 与扩展</h2>
                  <small className="text-[11px] text-ink-muted">
                    M7 规划能力；独立进程隔离、健康检查与超时保护尚未交付
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-ink-secondary">
                M1 只展示能力边界。插件运行时、权限审查和 MCP Server
                将在后续阶段实现并通过验收后开放。
              </p>
              <button
                disabled
                className="inline-flex h-8 cursor-not-allowed items-center gap-1.5 rounded-xl border border-line bg-sidebar px-3 text-xs font-medium text-ink-muted"
              >
                <Plug size={12} />
                规划中
              </button>
            </article>

            <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
              <div className="mb-3 flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-mint-bg text-mint-text">
                  <Shield size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-ink">安全与本地备份</h2>
                  <small className="text-[11px] text-ink-muted">
                    Markdown 为正文权威；索引可安全重建
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-ink-secondary">
                索引重建只扫描当前工作区内的 Markdown 文件，不会修改文章正文或 Git 版本历史。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink hover:bg-subtle"
                onClick={() => void rebuildIndex()}
              >
                {rebuilding ? "正在重建..." : "重建本地索引"}
              </button>
            </article>
          </section>

          <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
            <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent-deep">
                  <Terminal size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-ink">M1 运行时诊断</h2>
                  <small className="text-xs text-ink-muted">
                    验证 schema、数据库完整性、日志路径以及 IPC/数据库错误反馈
                  </small>
                </div>
              </div>
              <button
                type="button"
                onClick={() => void refreshDiagnostics()}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
              >
                <RefreshCw size={11} />
                刷新
              </button>
            </div>

            {diagnostics ? (
              <div className="grid gap-2 text-[11px] sm:grid-cols-2">
                <div className="rounded-xl border border-line bg-canvas px-3 py-2.5">
                  <span className="block text-ink-muted">应用版本 / 运行时长</span>
                  <strong className="mt-0.5 block font-mono text-ink">
                    {diagnostics.appVersion} · {diagnostics.uptimeSeconds}s
                  </strong>
                </div>
                <div className="rounded-xl border border-line bg-canvas px-3 py-2.5">
                  <span className="block text-ink-muted">Schema / 完整性</span>
                  <strong className="mt-0.5 block font-mono text-mint-text">
                    v{diagnostics.schemaVersion} / v{diagnostics.targetSchemaVersion} ·{" "}
                    {diagnostics.databaseIntegrity}
                  </strong>
                </div>
                <div className="rounded-xl border border-line bg-canvas px-3 py-2.5">
                  <span className="block text-ink-muted">数据库</span>
                  <strong className="mt-0.5 block break-all font-mono text-ink">
                    {diagnostics.databasePath}
                  </strong>
                </div>
                <div className="rounded-xl border border-line bg-canvas px-3 py-2.5">
                  <span className="block text-ink-muted">脱敏日志</span>
                  <strong className="mt-0.5 block break-all font-mono text-ink">
                    {diagnostics.logPath}
                    {diagnostics.logAvailable ? " · available" : " · missing"}
                  </strong>
                </div>
                <div className="rounded-xl border border-line bg-canvas px-3 py-2.5 sm:col-span-2">
                  <span className="block text-ink-muted">应用数据目录 / 当前工作区</span>
                  <strong className="mt-0.5 block break-all font-mono text-ink">
                    {diagnostics.dataDir}
                    {" · "}
                    {diagnostics.workspacePath ?? "未设置工作区"}
                  </strong>
                </div>
              </div>
            ) : diagnosticsError ? (
              <p className="rounded-xl bg-coral-bg px-3 py-2 text-[11px] text-coral-text">
                {diagnosticsError}
              </p>
            ) : (
              <p className="rounded-xl bg-sidebar px-3 py-2 text-[11px] text-ink-muted">
                {isTauri() ? "正在读取运行时诊断..." : "浏览器预览不连接本地数据库和日志。"}
              </p>
            )}

            <div className="mt-4 flex flex-wrap gap-2">
              <button
                type="button"
                disabled={Boolean(diagnosticsBusy)}
                onClick={() => void runProbe("invalid_path")}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle disabled:cursor-not-allowed disabled:opacity-50"
              >
                <AlertTriangle size={11} />
                测试非法参数 IPC
              </button>
              <button
                type="button"
                disabled={Boolean(diagnosticsBusy)}
                onClick={() => void runProbe("ipc_payload")}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle disabled:cursor-not-allowed disabled:opacity-50"
              >
                <Shield size={11} />
                测试日志脱敏
              </button>
              <button
                type="button"
                disabled={Boolean(diagnosticsBusy)}
                onClick={() => void runProbe("database_error")}
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle disabled:cursor-not-allowed disabled:opacity-50"
              >
                <Terminal size={11} />
                测试数据库错误
              </button>
            </div>
            <p className="mt-3 rounded-xl bg-sidebar px-3 py-2 text-[10.5px] leading-relaxed text-ink-muted">
              三个探针会返回可识别的错误并在顶部显示；应用保持运行。日志脱敏探针使用固定的模拟
              password/cookie/token，不读取任何真实凭据。
            </p>
          </article>
        </div>
      </div>
    </main>
  );
}
