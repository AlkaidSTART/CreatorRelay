import { useEffect, useMemo, useState } from "react";
import { useNavigate } from "react-router";
import { AlertTriangle, ArrowRight, Check, ChevronRight, Save, Sparkles } from "../Icons";
import { usePlatformStore } from "../../stores/platformStore";
import { usePluginStore } from "../../stores/pluginStore";
import { useStudioStore } from "../../stores/studioStore";
import { useUiStore } from "../../stores/uiStore";
import { useOperationsStore } from "../../stores/operationsStore";
import { buildDraftTargets } from "../../utils/contentTargets";
import { evaluateContentChecks } from "../../utils/contentChecks";
import { formatRelativeTime } from "../../utils/format";
import { MediaThumb } from "./MediaAsset";

export function PlatformDraftPanel({ onShowChecks }: { onShowChecks: () => void }) {
  const navigate = useNavigate();
  const editor = useStudioStore((state) => state.editor);
  const drafts = useStudioStore((state) => state.drafts);
  const createDrafts = useStudioStore((state) => state.createDrafts);
  const selectDraft = useStudioStore((state) => state.selectDraft);
  const selectedDraftId = useStudioStore((state) => state.selectedDraftId);
  const draftEditor = useStudioStore((state) => state.draftEditor);
  const updateDraftEditor = useStudioStore((state) => state.updateDraftEditor);
  const saveDraft = useStudioStore((state) => state.saveDraft);
  const draftSaveState = useStudioStore((state) => state.draftSaveState);
  const draftError = useStudioStore((state) => state.draftError);
  const acknowledgeDraftSource = useStudioStore((state) => state.acknowledgeDraftSource);
  const regenerateDraft = useStudioStore((state) => state.regenerateDraft);
  const createPublishTasks = useOperationsStore((state) => state.createTasks);
  const accounts = usePlatformStore((state) => state.accounts);
  const initializePlatforms = usePlatformStore((state) => state.initialize);
  const plugins = usePluginStore((state) => state.plugins);
  const initializePlugins = usePluginStore((state) => state.initialize);
  const showToast = useUiStore((state) => state.showToast);
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [creating, setCreating] = useState(false);
  const [detailOpen, setDetailOpen] = useState(false);

  useEffect(() => {
    void initializePlatforms();
    void initializePlugins();
  }, [initializePlatforms, initializePlugins]);

  const targets = useMemo(
    () => (editor ? buildDraftTargets(editor.kind, accounts, plugins) : []),
    [editor, accounts, plugins],
  );
  const selectableTargets = targets.filter((target) => target.selectable);
  const checkedTargets = selectableTargets.filter(
    (target) => selected[`${target.platformId}:${target.accountId}`],
  );

  if (!editor) return null;

  async function handleGenerate() {
    if (checkedTargets.length === 0) return;
    setCreating(true);
    try {
      const created = await createDrafts(
        checkedTargets.map((target) => ({
          platformId: target.platformId,
          platformName: target.platformName,
          accountId: target.accountId,
          accountAlias: target.accountAlias,
          adapterId: target.adapterId,
        })),
      );
      if (created[0]) {
        await selectDraft(created[0].id);
        setDetailOpen(true);
      }
      setSelected({});
    } finally {
      setCreating(false);
    }
  }

  async function openDraft(draftId: string) {
    await selectDraft(draftId);
    setDetailOpen(true);
  }

  async function handleCreatePublishTask() {
    if (!draftEditor) return;
    const saved = await saveDraft();
    if (!saved) return;
    const tasks = await createPublishTasks({
      sourceType: "platformDraft",
      sourceId: draftEditor.id,
      targets: [
        {
          platformId: draftEditor.platformId,
          platformName: draftEditor.platformName,
          accountId: draftEditor.accountId,
          accountAlias: draftEditor.accountAlias,
          adapterId: draftEditor.adapterId ?? null,
        },
      ],
      scheduledAt: null,
    });
    if (tasks.length > 0) {
      showToast(`已为 ${draftEditor.platformName} 创建发布任务，请预览最终内容包后确认`);
      navigate("/publishing");
    }
  }

  if (detailOpen && draftEditor) {
    const check = evaluateContentChecks(
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
    );
    return (
      <div className="flex h-full flex-col">
        <div className="flex items-center justify-between border-b border-line px-3 py-2.5">
          <button
            type="button"
            onClick={() => setDetailOpen(false)}
            className="text-[11px] font-medium text-ink-muted hover:text-ink"
          >
            ← 返回目标列表
          </button>
          <span className="rounded bg-accent-soft px-1.5 py-0.5 text-[10px] text-accent-deep">
            独立平台稿
          </span>
        </div>
        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-3">
          <div>
            <div className="flex items-center justify-between gap-2">
              <strong className="text-xs text-ink">{draftEditor.platformName}</strong>
              <span className="text-[10px] text-ink-muted">{draftEditor.accountAlias}</span>
            </div>
            <p className="mt-0.5 text-[10px] text-ink-muted">
              生成于 {formatRelativeTime(draftEditor.generatedAt)}
            </p>
          </div>

          {draftEditor.sourceChanged && (
            <div className="rounded-xl border border-amber-300 bg-amber-50 p-2.5 text-[11px] text-amber-950">
              <div className="flex items-center gap-1.5 font-medium">
                <AlertTriangle size={12} /> 源内容已变化
              </div>
              <p className="mt-1 leading-relaxed">
                当前平台稿基于旧版本生成。手动修改已保留，不会自动覆盖。
              </p>
              <div className="mt-2 flex gap-1.5">
                <button
                  type="button"
                  onClick={() => void acknowledgeDraftSource(draftEditor.id)}
                  className="rounded border border-amber-400 bg-white px-2 py-1 text-[10px] font-medium"
                >
                  保留当前平台稿
                </button>
                <button
                  type="button"
                  onClick={() => {
                    if (
                      draftEditor.manuallyEdited &&
                      !window.confirm("重新生成会覆盖当前平台稿的手动修改，是否继续？")
                    ) {
                      return;
                    }
                    void regenerateDraft(draftEditor.id);
                  }}
                  className="rounded bg-amber-200 px-2 py-1 text-[10px] font-medium"
                >
                  重新按主稿生成
                </button>
              </div>
            </div>
          )}

          {draftError && (
            <p className="rounded-lg bg-coral-bg px-2.5 py-2 text-[10.5px] text-coral-text">
              {draftError}
            </p>
          )}

          <label className="block text-[11px] text-ink-muted">
            平台稿标题
            <input
              value={draftEditor.title}
              onChange={(event) => updateDraftEditor({ title: event.target.value })}
              className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 text-xs text-ink outline-none focus:border-accent"
            />
          </label>
          <label className="block text-[11px] text-ink-muted">
            摘要 / 短文案
            <textarea
              value={draftEditor.summary}
              onChange={(event) => updateDraftEditor({ summary: event.target.value })}
              className="mt-1 min-h-20 w-full resize-y rounded-lg border border-line bg-surface px-2 py-1.5 text-xs text-ink outline-none focus:border-accent"
            />
          </label>
          <label className="block text-[11px] text-ink-muted">
            正文
            <textarea
              value={draftEditor.body}
              onChange={(event) => updateDraftEditor({ body: event.target.value })}
              className="mt-1 min-h-44 w-full resize-y rounded-lg border border-line bg-surface p-2 font-mono text-[11px] leading-relaxed text-ink outline-none focus:border-accent"
            />
          </label>

          {draftEditor.media.length > 0 && (
            <div>
              <span className="text-[11px] text-ink-muted">
                素材快照（{draftEditor.media.length}）
              </span>
              <div className="mt-1.5 grid grid-cols-3 gap-1.5">
                {draftEditor.media.slice(0, 6).map((asset) => (
                  <div key={asset.id} className="rounded-lg border border-line p-1">
                    <MediaThumb
                      source={{ kind: "draft", draftId: draftEditor.id }}
                      asset={asset}
                      className="h-14 w-full"
                    />
                    <span className="mt-0.5 block truncate text-[9px] text-ink-muted">
                      {asset.order}. {asset.originalName}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="rounded-xl border border-line bg-canvas p-2.5">
            <div className="flex items-center justify-between">
              <span className="text-[11px] font-medium text-ink">平台检查</span>
              <button
                type="button"
                onClick={onShowChecks}
                className="text-[10px] font-medium text-accent hover:text-accent-deep"
              >
                查看完整规则
              </button>
            </div>
            <div className="mt-1.5 flex gap-2 text-[10px]">
              <span className="rounded bg-coral-bg px-1.5 py-0.5 text-coral-text">
                {check.errorCount} 错误
              </span>
              <span className="rounded bg-amber-100 px-1.5 py-0.5 text-amber-800">
                {check.warningCount} 警告
              </span>
              <span className="rounded bg-sidebar px-1.5 py-0.5 text-ink-muted">
                {check.uncheckedCount} 未检查
              </span>
            </div>
          </div>
        </div>
        <div className="border-t border-line p-3">
          <button
            type="button"
            onClick={() => void saveDraft()}
            disabled={draftSaveState === "saving"}
            className="inline-flex w-full items-center justify-center gap-1.5 rounded-xl bg-accent py-2 text-xs font-medium text-accent-on disabled:opacity-50"
          >
            {draftSaveState === "saved" ? <Check size={12} /> : <Save size={12} />}
            {draftSaveState === "saving"
              ? "保存中..."
              : draftSaveState === "saved"
                ? "平台稿已独立保存"
                : "保存平台稿"}
          </button>
          {draftSaveState === "saved" && (
            <p className="mt-1.5 text-center text-[10px] text-ink-muted">
              修改不会影响主稿或其他平台稿
            </p>
          )}
          <button
            type="button"
            onClick={() => void handleCreatePublishTask()}
            disabled={draftSaveState === "saving"}
            className="mt-2 inline-flex w-full items-center justify-center gap-1.5 rounded-xl border border-accent-border bg-accent-soft py-2 text-xs font-medium text-accent-deep disabled:opacity-50"
          >
            <ArrowRight size={12} />
            创建发布任务并预览确认
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-line px-4 py-3.5">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <h3 className="text-[13px] font-semibold text-ink">平台适配目标</h3>
            <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
              仅列出账号声明与运行时探测结果；能力未知不会显示为支持。
            </p>
          </div>
          <button
            type="button"
            onClick={handleGenerate}
            disabled={checkedTargets.length === 0 || creating}
            className="mt-0.5 shrink-0 cursor-pointer whitespace-nowrap rounded-lg bg-accent px-3 py-1.5 text-[11px] font-medium text-accent-on shadow-xs transition-colors hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-45"
          >
            {creating
              ? "生成中..."
              : checkedTargets.length > 0
                ? `生成 ${checkedTargets.length} 份`
                : "生成平台稿"}
          </button>
        </div>
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto p-4">
        {targets.length === 0 ? (
          <div className="rounded-2xl border border-dashed border-line-strong bg-surface/75 p-5 text-center">
            <p className="text-xs font-medium text-ink-secondary">暂无可用适配目标</p>
            <p className="mt-1.5 text-[11px] leading-relaxed text-ink-muted">
              请先在「平台账号」连接账号，或在「适配器与插件」中启用测试适配器。
            </p>
            <button
              type="button"
              onClick={() => navigate("/platforms")}
              className="mt-3.5 inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-surface px-3 py-1.5 text-[11px] font-medium text-ink-secondary shadow-xs transition-colors hover:border-accent hover:text-accent-deep"
            >
              前往平台账号
              <ArrowRight size={10} />
            </button>
          </div>
        ) : (
          <div className="space-y-2">
            {targets.map((target) => {
              const key = `${target.platformId}:${target.accountId}`;
              return (
                <label
                  key={key}
                  className={`flex cursor-pointer gap-2.5 rounded-xl border p-3 transition-all ${
                    target.selectable
                      ? "border-line bg-surface hover:border-accent-border"
                      : "cursor-not-allowed border-line bg-canvas/70 opacity-65"
                  }`}
                >
                  <input
                    type="checkbox"
                    checked={Boolean(selected[key])}
                    disabled={!target.selectable}
                    onChange={(event) =>
                      setSelected((current) => ({ ...current, [key]: event.target.checked }))
                    }
                    className="mt-0.5 accent-[var(--color-accent)]"
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center justify-between gap-2">
                      <strong className="truncate text-xs font-medium text-ink">
                        {target.platformName}
                      </strong>
                      <span
                        className={`shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-medium ${
                          target.capability === "supported"
                            ? "bg-mint-bg text-mint-text"
                            : target.capability === "experimental" ||
                                target.capability === "restricted"
                              ? "bg-lemon-bg text-lemon-text"
                              : "bg-subtle text-ink-muted"
                        }`}
                      >
                        {target.capability}
                      </span>
                    </div>
                    <span className="mt-0.5 block truncate text-[10.5px] text-ink-muted">
                      {target.source === "adapter" ? "Adapter 探测" : target.accountAlias}
                    </span>
                    <p className="mt-1 text-[10.5px] leading-snug text-ink-secondary">
                      {target.reason}
                    </p>
                  </div>
                </label>
              );
            })}
          </div>
        )}

        <div className="mt-6">
          <div className="flex items-center justify-between">
            <h3 className="text-[13px] font-semibold text-ink">已生成平台稿</h3>
            <span className="rounded-full bg-subtle px-2 py-0.5 text-[10.5px] font-medium text-ink-muted">
              {drafts.length} 份
            </span>
          </div>
          <div className="mt-2.5 space-y-2">
            {drafts.length === 0 ? (
              <div className="rounded-xl border border-line/70 bg-canvas/70 px-3.5 py-4 text-center text-[11px] text-ink-muted">
                暂无平台稿。选择兼容目标后即可生成独立副本。
              </div>
            ) : (
              drafts.map((draft) => (
                <button
                  key={draft.id}
                  type="button"
                  onClick={() => void openDraft(draft.id)}
                  className={`flex w-full cursor-pointer items-center gap-2.5 rounded-xl border px-3 py-2.5 text-left transition-all ${
                    selectedDraftId === draft.id
                      ? "border-accent bg-accent-soft/50 shadow-xs"
                      : "border-line bg-surface hover:border-accent-border"
                  }`}
                >
                  <Sparkles size={12} className="shrink-0 text-accent" />
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-ink">
                      {draft.platformName}
                    </strong>
                    <span className="mt-0.5 block truncate text-[10.5px] text-ink-muted">
                      {draft.title || "未命名"} · {draft.manuallyEdited ? "已手动修改" : "自动生成"}
                    </span>
                  </div>
                  {draft.sourceChanged && <AlertTriangle size={12} className="text-amber-600" />}
                  <ChevronRight size={11} className="text-ink-muted" />
                </button>
              ))
            )}
          </div>
        </div>
      </div>

      <div className="border-t border-line px-4 py-3">
        <button
          type="button"
          onClick={() => navigate("/publishing")}
          className="inline-flex h-9 w-full cursor-pointer items-center justify-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary transition-all hover:border-accent hover:text-accent-deep"
        >
          前往发布中心
          <ArrowRight size={11} />
        </button>
      </div>
    </div>
  );
}
