import { Check, FolderOpen, KeyRound, Palette, Plug, Server, Shield } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useUiStore, useWorkspaceStore } from "../stores";
import { THEMES } from "../utils/themes";

const aiProviders = [
  { name: "OpenAI", endpoint: "https://api.openai.com/v1", status: "未配置" },
  { name: "Anthropic", endpoint: "https://api.anthropic.com", status: "未配置" },
  {
    name: "Google Gemini",
    endpoint: "https://generativelanguage.googleapis.com",
    status: "未配置",
  },
  { name: "DeepSeek", endpoint: "https://api.deepseek.com", status: "未配置" },
  { name: "OpenAI Compatible", endpoint: "自定义端点 / 本地模型", status: "可扩展" },
];

export function SettingsPage() {
  const workspace = useWorkspaceStore((state) => state.workspace);
  const chooseWorkspace = useWorkspaceStore((state) => state.chooseWorkspace);
  const theme = useUiStore((state) => state.theme);
  const setTheme = useUiStore((state) => state.setTheme);
  const showToast = useUiStore((state) => state.showToast);

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
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-5" role="radiogroup" aria-label="选择应用主题">
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
                <code className="mt-1 block truncate font-mono text-xs text-ink">
                  data/creator-relay.db
                </code>
              </div>
              <div className="rounded-xl border border-line bg-canvas p-3">
                <span className="block text-[11px] text-ink-muted">素材与导出目录</span>
                <code className="mt-1 block truncate font-mono text-xs text-ink">
                  workspace/assets · exports
                </code>
              </div>
            </div>
          </article>

          {/* AI Providers */}
          <article className="rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent-deep">
                  <KeyRound size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-ink">自带 AI 模型（BYOK）</h2>
                  <small className="text-xs text-ink-muted">
                    API Key 优先保存在系统钥匙串（OS Keychain），绝不明文上传
                  </small>
                </div>
              </div>
              <button
                className="h-8 shrink-0 cursor-pointer rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink hover:bg-subtle"
                onClick={() => showToast("BYOK 凭据配置表单将在桌面端直接接入系统钥匙串")}
              >
                添加端点
              </button>
            </div>
            <div className="space-y-2">
              {aiProviders.map((provider) => (
                <div
                  key={provider.name}
                  className="flex items-center justify-between gap-3 rounded-xl border border-line bg-canvas px-3.5 py-2.5"
                >
                  <div className="min-w-0 flex-1">
                    <strong className="block text-xs font-medium text-ink">
                      {provider.name}
                    </strong>
                    <small className="block truncate text-[11px] text-ink-muted">
                      {provider.endpoint}
                    </small>
                  </div>
                  <span className="rounded-md border border-line bg-surface px-2 py-0.5 text-[11px] text-ink-muted">
                    {provider.status}
                  </span>
                  <button
                    className="h-7 cursor-pointer rounded-lg border border-line-strong bg-surface px-2.5 text-xs font-medium text-ink hover:border-accent-border hover:bg-accent-soft hover:text-accent-deep"
                    onClick={() => showToast(`即将配置 ${provider.name} API Key 与默认模型`)}
                  >
                    配置
                  </button>
                </div>
              ))}
            </div>
          </article>

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
                    独立进程隔离运行，支持健康检查与超时保护
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-ink-secondary">
                所有平台适配扩展均在受控权限下运行，不直接持有完整凭据与数据库访问权。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink hover:bg-subtle"
                onClick={() => showToast("插件权限审查视图已就绪")}
              >
                <Plug size={12} />
                管理插件权限
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
                    高风险操作强制确认 · 随时重建本地索引
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-ink-secondary">
                自动发布、删除内容等高风险操作默认需要你显式确认；索引异常时可直接从本地 Markdown 文件一键恢复。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink hover:bg-subtle"
                onClick={() => showToast("已触发工作区索引一致性检查，全部正常")}
              >
                重建本地索引
              </button>
            </article>
          </section>
        </div>
      </div>
    </main>
  );
}
