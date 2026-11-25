import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, CheckCircle2, Info, Shield } from "../Icons";
import { usePlatformStore } from "../../stores/platformStore";
import { usePluginStore } from "../../stores/pluginStore";
import { useStudioStore } from "../../stores/studioStore";
import { buildDraftTargets } from "../../utils/contentTargets";
import { evaluateContentChecks, type CheckReport } from "../../utils/contentChecks";

function ReportView({ report }: { report: CheckReport }) {
  const errors = report.items.filter((item) => item.level === "error");
  const warnings = report.items.filter((item) => item.level === "warning");
  const unchecked = report.items.filter((item) => item.level === "unchecked");
  return (
    <div className="space-y-3">
      <div className="grid grid-cols-3 gap-1.5">
        <div className="rounded-lg bg-coral-bg p-2 text-center">
          <strong className="block text-sm text-coral-text">{report.errorCount}</strong>
          <span className="text-[9.5px] text-coral-text">硬性错误</span>
        </div>
        <div className="rounded-lg bg-amber-100 p-2 text-center">
          <strong className="block text-sm text-amber-800">{report.warningCount}</strong>
          <span className="text-[9.5px] text-amber-800">警告</span>
        </div>
        <div className="rounded-lg bg-sidebar p-2 text-center">
          <strong className="block text-sm text-ink-secondary">{report.uncheckedCount}</strong>
          <span className="text-[9.5px] text-ink-muted">未检查</span>
        </div>
      </div>
      <div
        className={`rounded-lg border px-2.5 py-2 ${
          report.verified && report.errorCount === 0
            ? "border-mint-text/30 bg-mint-bg"
            : report.verified
              ? "border-coral-border bg-coral-bg"
              : "border-line bg-surface"
        }`}
      >
        <div className="flex items-center gap-1.5">
          {report.verified && report.errorCount === 0 ? (
            <CheckCircle2 size={11} className="text-mint-text" />
          ) : report.verified ? (
            <AlertTriangle size={11} className="text-coral-text" />
          ) : (
            <Info size={11} className="text-ink-muted" />
          )}
          <strong className="text-[10.5px] text-ink">{report.targetLabel}</strong>
          {report.verified && (
            <span
              className={`ml-auto rounded px-1.5 py-0.5 text-[9px] font-medium ${
                report.errorCount === 0
                  ? "bg-white/60 text-mint-text"
                  : "bg-white/60 text-coral-text"
              }`}
            >
              {report.errorCount === 0 ? "检查通过" : "检查失败"}
            </span>
          )}
        </div>
        <p className="mt-0.5 text-[9.5px] leading-snug text-ink-muted">
          {report.verified
            ? "已应用本地可验证规则；未提供的规则保持为“未检查”。"
            : "该目标没有可验证形态规则，不能把结果视为通过。"}
        </p>
      </div>

      {errors.length > 0 && (
        <section>
          <h4 className="mb-1 text-[10.5px] font-medium text-coral-text">硬性错误</h4>
          <div className="space-y-1">
            {errors.map((item) => (
              <div
                key={`${item.id}-error`}
                className="flex gap-1.5 rounded-lg bg-coral-bg p-2 text-[10px] text-coral-text"
              >
                <AlertTriangle size={10} className="mt-0.5 shrink-0" />
                <span>
                  <strong>{item.label}：</strong>
                  {item.message}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
      {warnings.length > 0 && (
        <section>
          <h4 className="mb-1 text-[10.5px] font-medium text-amber-800">警告</h4>
          <div className="space-y-1">
            {warnings.map((item) => (
              <div
                key={`${item.id}-warning`}
                className="flex gap-1.5 rounded-lg bg-amber-50 p-2 text-[10px] text-amber-900"
              >
                <AlertTriangle size={10} className="mt-0.5 shrink-0" />
                <span>
                  <strong>{item.label}：</strong>
                  {item.message}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
      {unchecked.length > 0 && (
        <section>
          <h4 className="mb-1 text-[10.5px] font-medium text-ink-muted">未检查</h4>
          <div className="space-y-1">
            {unchecked.map((item) => (
              <div
                key={`${item.id}-unchecked`}
                className="flex gap-1.5 rounded-lg bg-canvas p-2 text-[10px] text-ink-muted"
              >
                <Info size={10} className="mt-0.5 shrink-0" />
                <span>
                  <strong>{item.label}：</strong>
                  {item.message}
                </span>
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

export function ContentChecksPanel() {
  const editor = useStudioStore((state) => state.editor);
  const draftEditor = useStudioStore((state) => state.draftEditor);
  const accounts = usePlatformStore((state) => state.accounts);
  const initializePlatforms = usePlatformStore((state) => state.initialize);
  const plugins = usePluginStore((state) => state.plugins);
  const initializePlugins = usePluginStore((state) => state.initialize);
  const [targetKey, setTargetKey] = useState("");
  const [mode, setMode] = useState<"source" | "draft">("source");

  useEffect(() => {
    void initializePlatforms();
    void initializePlugins();
  }, [initializePlatforms, initializePlugins]);

  const targets = useMemo(
    () => (editor ? buildDraftTargets(editor.kind, accounts, plugins) : []),
    [editor, accounts, plugins],
  );

  useEffect(() => {
    if (!targetKey && targets[0]) {
      setTargetKey(`${targets[0].platformId}:${targets[0].accountId}`);
    }
  }, [targets, targetKey]);

  const target = targets.find((item) => `${item.platformId}:${item.accountId}` === targetKey);

  if (!editor) return null;

  const draftReport =
    draftEditor && editor
      ? evaluateContentChecks(
          {
            kind: editor.kind,
            title: draftEditor.title,
            summary: draftEditor.summary,
            body: draftEditor.body,
            cover: draftEditor.cover,
            media: draftEditor.media,
          },
          draftEditor.platformId,
          draftEditor.platformName,
        )
      : null;

  const sourceReport =
    target && editor
      ? evaluateContentChecks(
          {
            kind: editor.kind,
            title: editor.title,
            summary: editor.summary,
            body: editor.body,
            cover: editor.cover,
            media: editor.media,
          },
          target.platformId,
          target.platformName,
        )
      : null;

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-line px-4 py-3.5">
        <div className="flex items-center gap-2">
          <Shield size={13} className="text-accent" />
          <h3 className="text-[13px] font-semibold text-ink">平台内容检查</h3>
        </div>
        <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
          硬性错误、警告和未验证规则分开显示，不把未知检查写成通过。
        </p>
        <div className="mt-3 flex gap-1 rounded-xl bg-subtle p-1 ring-1 ring-line/60">
          <button
            type="button"
            onClick={() => setMode("source")}
            className={`flex-1 cursor-pointer rounded-lg px-2.5 py-1.5 text-[11px] font-medium transition-all ${
              mode === "source"
                ? "bg-surface text-ink shadow-[0_1px_2px_rgba(51,43,41,0.06)]"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            主稿
          </button>
          <button
            type="button"
            disabled={!draftEditor}
            onClick={() => setMode("draft")}
            className={`flex-1 cursor-pointer rounded-lg px-2.5 py-1.5 text-[11px] font-medium transition-all disabled:cursor-not-allowed disabled:opacity-40 ${
              mode === "draft"
                ? "bg-surface text-ink shadow-[0_1px_2px_rgba(51,43,41,0.06)]"
                : "text-ink-muted hover:text-ink"
            }`}
          >
            当前平台稿
          </button>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {mode === "source" ? (
          <>
            <label className="block text-[11px] font-medium text-ink-muted">
              检查目标
              <select
                value={targetKey}
                onChange={(event) => setTargetKey(event.target.value)}
                className="mt-1.5 w-full rounded-xl border border-line bg-surface px-2.5 py-2 text-xs text-ink outline-none transition-colors focus:border-accent"
              >
                {targets.map((item) => (
                  <option
                    key={`${item.platformId}:${item.accountId}`}
                    value={`${item.platformId}:${item.accountId}`}
                  >
                    {item.platformName} · {item.capability}
                  </option>
                ))}
              </select>
            </label>
            <div className="mt-3.5">
              {sourceReport ? (
                <ReportView report={sourceReport} />
              ) : (
                <div className="rounded-xl border border-dashed border-line-strong bg-surface/75 p-4 text-center text-[11px] leading-relaxed text-ink-muted">
                  没有可用检查目标。连接账号或启用测试适配器后可检查。
                </div>
              )}
            </div>
          </>
        ) : draftReport && draftEditor ? (
          <ReportView report={draftReport} />
        ) : (
          <div className="rounded-xl border border-dashed border-line-strong bg-surface/75 p-4 text-center text-[11px] text-ink-muted">
            请先在平台稿面板打开一份平台稿。
          </div>
        )}
      </div>
    </div>
  );
}
