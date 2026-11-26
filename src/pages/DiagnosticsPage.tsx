import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { AlertTriangle, Plug, RefreshCw, Server, Shield, Terminal } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useUiStore } from "../stores";

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

export function DiagnosticsPage() {
  const showToast = useUiStore((state) => state.showToast);
  const setError = useUiStore((state) => state.setError);
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
        eyebrow="DIAGNOSTICS"
        title="运行时诊断"
        description="查看桌面端数据库 schema、脱敏日志路径、扩展运行时边界，并执行 IPC 与数据库诊断探针。"
        actions={
          <button
            type="button"
            onClick={() => void refreshDiagnostics()}
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle hover:text-ink"
          >
            <RefreshCw size={12} />
            刷新状态
          </button>
        }
      />

      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
        <div className="max-w-4xl space-y-5">
          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center gap-2.5 border-b border-line px-5 py-3.5">
              <Terminal size={14} className="shrink-0 text-ink-muted" />
              <div>
                <h2 className="text-[13px] font-semibold text-ink">M1 运行时诊断</h2>
                <small className="text-[11.5px] text-ink-muted">
                  验证 schema、数据库完整性、日志路径以及 IPC/数据库错误反馈
                </small>
              </div>
            </div>

            {diagnostics ? (
              <div className="grid divide-y divide-line text-[11.5px] sm:grid-cols-2 sm:divide-x">
                <div className="px-5 py-3">
                  <span className="block text-[11px] text-ink-muted">应用版本 / 运行时长</span>
                  <strong className="mt-0.5 block font-mono text-ink">
                    {diagnostics.appVersion} · {diagnostics.uptimeSeconds}s
                  </strong>
                </div>
                <div className="px-5 py-3">
                  <span className="block text-[11px] text-ink-muted">Schema / 完整性</span>
                  <strong className="mt-0.5 block font-mono text-mint-text">
                    v{diagnostics.schemaVersion} / v{diagnostics.targetSchemaVersion} ·{" "}
                    {diagnostics.databaseIntegrity}
                  </strong>
                </div>
                <div className="px-5 py-3">
                  <span className="block text-[11px] text-ink-muted">数据库</span>
                  <strong className="mt-0.5 block break-all font-mono text-ink">
                    {diagnostics.databasePath}
                  </strong>
                </div>
                <div className="px-5 py-3">
                  <span className="block text-[11px] text-ink-muted">脱敏日志</span>
                  <strong className="mt-0.5 block break-all font-mono text-ink">
                    {diagnostics.logPath}
                    {diagnostics.logAvailable ? " · available" : " · missing"}
                  </strong>
                </div>
                <div className="px-5 py-3 sm:col-span-2 sm:border-t sm:border-line">
                  <span className="block text-[11px] text-ink-muted">应用数据目录 / 当前工作区</span>
                  <strong className="mt-0.5 block break-all font-mono text-ink">
                    {diagnostics.dataDir}
                    {" · "}
                    {diagnostics.workspacePath ?? "未设置工作区"}
                  </strong>
                </div>
              </div>
            ) : diagnosticsError ? (
              <div className="p-5">
                <p className="rounded-md border border-coral-border bg-coral-bg px-3 py-2 text-[11.5px] text-coral-text">
                  {diagnosticsError}
                </p>
              </div>
            ) : (
              <div className="p-5">
                <p className="rounded-md border border-line bg-subtle px-3 py-2 text-[11.5px] text-ink-muted">
                  {isTauri() ? "正在读取运行时诊断..." : "浏览器预览不连接本地数据库和日志。"}
                </p>
              </div>
            )}

            <div className="border-t border-line bg-surface-soft px-5 py-3.5">
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={Boolean(diagnosticsBusy)}
                  onClick={() => void runProbe("invalid_path")}
                  className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle hover:text-ink disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <AlertTriangle size={11} />
                  测试非法参数 IPC
                </button>
                <button
                  type="button"
                  disabled={Boolean(diagnosticsBusy)}
                  onClick={() => void runProbe("ipc_payload")}
                  className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle hover:text-ink disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <Shield size={11} />
                  测试日志脱敏
                </button>
                <button
                  type="button"
                  disabled={Boolean(diagnosticsBusy)}
                  onClick={() => void runProbe("database_error")}
                  className="inline-flex h-7.5 cursor-pointer items-center gap-1.5 rounded-md border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary transition-colors hover:bg-subtle hover:text-ink disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <Terminal size={11} />
                  测试数据库错误
                </button>
              </div>
              <p className="mt-2.5 text-[11px] leading-relaxed text-ink-muted">
                三个探针会返回可识别的错误并在顶部显示；应用保持运行。日志脱敏探针使用固定的模拟
                password/cookie/token，不读取任何真实凭据。
              </p>
            </div>
          </article>

          <article className="overflow-hidden rounded-lg border border-line bg-surface">
            <div className="flex items-center justify-between gap-4 border-b border-line px-5 py-3.5">
              <div className="flex items-center gap-2.5">
                <Server size={14} className="shrink-0 text-ink-muted" />
                <div>
                  <h2 className="text-[13px] font-semibold text-ink">MCP Server 与扩展边界</h2>
                  <small className="text-[11.5px] text-ink-muted">
                    外部 MCP 扩展接入、独立进程隔离与权限边界说明
                  </small>
                </div>
              </div>
              <button
                disabled
                className="inline-flex h-7.5 cursor-not-allowed items-center gap-1.5 rounded-md border border-line bg-subtle px-3 text-xs font-medium text-ink-muted"
              >
                <Plug size={12} />
                外部扩展规划中
              </button>
            </div>
            <div className="px-5 py-3.5">
              <p className="text-xs leading-relaxed text-ink-secondary">
                内置 MCP 工具与安全审计已在「AI 助手」页面提供；外部自定义 MCP Server
                的独立进程托管将在后续阶段开放。
              </p>
            </div>
          </article>
        </div>
      </div>
    </main>
  );
}
