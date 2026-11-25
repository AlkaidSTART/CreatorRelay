import { useEffect, useState } from "react";
import { Check, KeyRound, LoaderCircle, Plus, RefreshCw, Trash2 } from "./Icons";
import { useAiStore } from "../stores/aiStore";
import { useUiStore } from "../stores/uiStore";
import type { AiProvider, AiTestResult } from "../types/ai";

const PRESETS = [
  { label: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "gpt-4.1-mini" },
  { label: "DeepSeek", baseUrl: "https://api.deepseek.com/v1", model: "deepseek-chat" },
  {
    label: "OpenAI Compatible",
    baseUrl: "http://127.0.0.1:11434/v1",
    model: "local-model",
  },
];

interface FormState {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  apiKey: string;
  isDefault: boolean;
}

const emptyForm: FormState = {
  id: "",
  name: "OpenAI Compatible",
  baseUrl: "https://api.openai.com/v1",
  model: "gpt-4.1-mini",
  apiKey: "",
  isDefault: true,
};

export function AiProviderSettings() {
  const providers = useAiStore((state) => state.providers);
  const initialize = useAiStore((state) => state.initialize);
  const saveProvider = useAiStore((state) => state.saveProvider);
  const deleteProvider = useAiStore((state) => state.deleteProvider);
  const setDefaultProvider = useAiStore((state) => state.setDefaultProvider);
  const testProvider = useAiStore((state) => state.testProvider);
  const busyAction = useAiStore((state) => state.busyAction);
  const showToast = useUiStore((state) => state.showToast);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [editing, setEditing] = useState(false);
  const [testResult, setTestResult] = useState<AiTestResult | null>(null);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  function editProvider(provider: AiProvider) {
    setEditing(true);
    setTestResult(null);
    setForm({
      id: provider.id,
      name: provider.name,
      baseUrl: provider.baseUrl,
      model: provider.model,
      apiKey: "",
      isDefault: provider.isDefault,
    });
  }

  async function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    const saved = await saveProvider({
      id: form.id || null,
      name: form.name,
      baseUrl: form.baseUrl,
      model: form.model,
      apiKey: form.apiKey || null,
      apiStyle: "openai-compatible",
      isDefault: form.isDefault,
    });
    if (saved) {
      setEditing(false);
      setForm(emptyForm);
      setTestResult(null);
    }
  }

  async function handleTest(providerId: string) {
    const result = await testProvider(providerId);
    setTestResult(result);
    if (result?.ok) showToast("模型连接测试成功");
  }

  return (
    <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
      <div className="mb-4 flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent-deep">
            <KeyRound size={15} />
          </div>
          <div>
            <h2 className="text-sm font-semibold text-ink">自带 AI 模型（BYOK）</h2>
            <small className="text-xs text-ink-muted">
              API Key 写入系统钥匙串；SQLite 只保存端点、模型与测试状态
            </small>
          </div>
        </div>
        <button
          type="button"
          onClick={() => {
            setEditing(true);
            setForm(emptyForm);
            setTestResult(null);
          }}
          className="inline-flex h-8 shrink-0 items-center gap-1 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink hover:bg-subtle"
        >
          <Plus size={11} /> 添加供应商
        </button>
      </div>

      <div className="space-y-2">
        {providers.length === 0 ? (
          <p className="rounded-xl bg-canvas px-3 py-3 text-[11px] text-ink-muted">
            尚未配置模型。添加 OpenAI 兼容端点并测试连接后，内容工作室的 AI 辅助才可启用。
          </p>
        ) : (
          providers.map((provider) => (
            <div
              key={provider.id}
              className="rounded-xl border border-line bg-canvas px-3.5 py-3"
            >
              <div className="flex items-center gap-2">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1.5">
                    <strong className="truncate text-xs font-medium text-ink">
                      {provider.name}
                    </strong>
                    {provider.isDefault && (
                      <span className="rounded bg-accent-soft px-1.5 py-0.5 text-[9px] font-medium text-accent-deep">
                        默认
                      </span>
                    )}
                    <span
                      className={`rounded px-1.5 py-0.5 text-[9px] ${
                        provider.configured
                          ? "bg-mint-bg text-mint-text"
                          : "bg-amber-100 text-amber-800"
                      }`}
                    >
                      {provider.configured ? "钥匙串已配置" : "缺少 API Key"}
                    </span>
                  </div>
                  <small className="mt-0.5 block truncate text-[10.5px] text-ink-muted">
                    {provider.baseUrl} · {provider.model}
                  </small>
                  {provider.lastTestStatus && (
                    <small
                      className={`mt-0.5 block ${
                        provider.lastTestStatus === "ok" ? "text-mint-text" : "text-coral-text"
                      }`}
                    >
                      最近测试：{provider.lastTestStatus === "ok" ? "成功" : provider.lastTestError}
                    </small>
                  )}
                </div>
                <button
                  type="button"
                  disabled={busyAction === `test:${provider.id}`}
                  onClick={() => void handleTest(provider.id)}
                  className="inline-flex h-7 items-center gap-1 rounded-lg border border-line bg-surface px-2 text-[10px] font-medium text-ink-secondary hover:border-accent disabled:opacity-50"
                >
                  {busyAction === `test:${provider.id}` ? (
                    <LoaderCircle size={9} />
                  ) : (
                    <RefreshCw size={9} />
                  )}
                  测试
                </button>
                <button
                  type="button"
                  disabled={provider.isDefault || !provider.configured}
                  onClick={() => void setDefaultProvider(provider.id)}
                  className="inline-flex h-7 items-center gap-1 rounded-lg border border-line bg-surface px-2 text-[10px] font-medium text-ink-secondary hover:border-accent disabled:opacity-40"
                >
                  <Check size={9} /> 设为默认
                </button>
                <button
                  type="button"
                  onClick={() => editProvider(provider)}
                  className="h-7 rounded-lg border border-line bg-surface px-2 text-[10px] text-ink-secondary hover:border-accent"
                >
                  编辑
                </button>
                <button
                  type="button"
                  onClick={() => void deleteProvider(provider.id)}
                  className="grid h-7 w-7 place-items-center rounded-lg border border-line bg-surface text-ink-muted hover:text-coral-text"
                  aria-label="删除供应商"
                >
                  <Trash2 size={9} />
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      {testResult && (
        <p
          className={`mt-3 rounded-xl px-3 py-2 text-[11px] ${
            testResult.ok ? "bg-mint-bg text-mint-text" : "bg-coral-bg text-coral-text"
          }`}
        >
          {testResult.message}
          {testResult.ok ? ` · ${testResult.latencyMs}ms` : ""}
        </p>
      )}

      {editing && (
        <form onSubmit={(event) => void handleSubmit(event)} className="mt-4 rounded-xl border border-line bg-canvas p-3">
          <div className="mb-2 flex flex-wrap gap-1">
            {PRESETS.map((preset) => (
              <button
                key={preset.label}
                type="button"
                onClick={() =>
                  setForm((current) => ({
                    ...current,
                    name: preset.label,
                    baseUrl: preset.baseUrl,
                    model: preset.model,
                  }))
                }
                className="rounded-md border border-line bg-surface px-2 py-1 text-[9.5px] text-ink-secondary hover:border-accent"
              >
                {preset.label}
              </button>
            ))}
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            <label className="text-[10.5px] text-ink-muted">
              名称
              <input
                value={form.name}
                onChange={(event) => setForm((current) => ({ ...current, name: event.target.value }))}
                required
                className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 text-xs text-ink outline-none focus:border-accent"
              />
            </label>
            <label className="text-[10.5px] text-ink-muted">
              模型
              <input
                value={form.model}
                onChange={(event) => setForm((current) => ({ ...current, model: event.target.value }))}
                required
                className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 text-xs text-ink outline-none focus:border-accent"
              />
            </label>
            <label className="text-[10.5px] text-ink-muted sm:col-span-2">
              Base URL（OpenAI Compatible）
              <input
                value={form.baseUrl}
                onChange={(event) => setForm((current) => ({ ...current, baseUrl: event.target.value }))}
                required
                className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 font-mono text-[11px] text-ink outline-none focus:border-accent"
              />
              <span className="mt-0.5 block text-[9px] text-ink-faint">
                非本地端点必须使用 HTTPS；会自动调用 /chat/completions。
              </span>
            </label>
            <label className="text-[10.5px] text-ink-muted sm:col-span-2">
              API Key {form.id ? "（留空则保留原 Key）" : ""}
              <input
                type="password"
                autoComplete="off"
                value={form.apiKey}
                onChange={(event) => setForm((current) => ({ ...current, apiKey: event.target.value }))}
                placeholder={form.id ? "••••••••（保留现有）" : "sk-..."}
                className="mt-1 w-full rounded-lg border border-line bg-surface px-2 py-1.5 font-mono text-[11px] text-ink outline-none focus:border-accent"
              />
            </label>
          </div>
          <label className="mt-2 flex items-center gap-1.5 text-[10.5px] text-ink-secondary">
            <input
              type="checkbox"
              checked={form.isDefault}
              onChange={(event) =>
                setForm((current) => ({ ...current, isDefault: event.target.checked }))
              }
              className="accent-[var(--color-accent)]"
            />
            保存后设为默认模型
          </label>
          <div className="mt-3 flex justify-end gap-2">
            <button
              type="button"
              onClick={() => {
                setEditing(false);
                setForm(emptyForm);
              }}
              className="rounded-lg border border-line px-3 py-1.5 text-xs text-ink-secondary"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={busyAction === "save"}
              className="rounded-lg bg-accent px-3 py-1.5 text-xs font-medium text-accent-on disabled:opacity-50"
            >
              {busyAction === "save" ? "保存中..." : form.id ? "更新供应商" : "添加供应商"}
            </button>
          </div>
        </form>
      )}
    </article>
  );
}
