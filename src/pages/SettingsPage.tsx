import { FolderOpen, KeyRound, Plug, Server, Shield } from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { useUiStore, useWorkspaceStore } from "../stores";

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
  const showToast = useUiStore((state) => state.showToast);

  return (
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="SETTINGS"
        title="系统设置"
        description="管理本地工作区与存储路径、BYOK AI 模型供应商、MCP Server、插件权限及安全备份策略。"
        priority="P0"
      />

      <div className="min-h-0 flex-1 overflow-y-auto px-7 py-6">
        <div className="max-w-4xl space-y-5">
          {/* Workspace Storage */}
          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-700">
                  <FolderOpen size={16} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-900">工作区与本地存储</h2>
                  <small className="text-xs text-zinc-400">
                    Markdown 文件为正文权威数据源，SQLite 保存索引与元数据
                  </small>
                </div>
              </div>
              <button
                className="h-8 shrink-0 cursor-pointer rounded-lg border border-zinc-200 bg-white px-3 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                onClick={() => void chooseWorkspace()}
              >
                切换工作区
              </button>
            </div>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
              <div className="rounded-lg border border-zinc-100 bg-zinc-50/70 p-3">
                <span className="block text-[11px] text-zinc-400">当前工作区路径</span>
                <code className="mt-1 block truncate font-mono text-xs text-zinc-800">
                  {workspace?.path ?? "尚未选择工作区"}
                </code>
              </div>
              <div className="rounded-lg border border-zinc-100 bg-zinc-50/70 p-3">
                <span className="block text-[11px] text-zinc-400">SQLite 索引库</span>
                <code className="mt-1 block truncate font-mono text-xs text-zinc-800">
                  data/creator-relay.db
                </code>
              </div>
              <div className="rounded-lg border border-zinc-100 bg-zinc-50/70 p-3">
                <span className="block text-[11px] text-zinc-400">素材与导出目录</span>
                <code className="mt-1 block truncate font-mono text-xs text-zinc-800">
                  workspace/assets · exports
                </code>
              </div>
            </div>
          </article>

          {/* AI Providers */}
          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-700">
                  <KeyRound size={16} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-900">BYOK AI 模型供应商</h2>
                  <small className="text-xs text-zinc-400">
                    API Key 优先保存在操作系统安全凭据存储（OS Keychain），不明文落库
                  </small>
                </div>
              </div>
              <button
                className="h-8 shrink-0 cursor-pointer rounded-lg border border-zinc-200 bg-white px-3 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                onClick={() => showToast("BYOK 凭据配置表单将在下一迭代接入 Keychain")}
              >
                添加端点
              </button>
            </div>
            <div className="space-y-2">
              {aiProviders.map((provider) => (
                <div
                  key={provider.name}
                  className="flex items-center justify-between gap-3 rounded-lg border border-zinc-100 bg-zinc-50/60 px-3.5 py-2.5"
                >
                  <div className="min-w-0 flex-1">
                    <strong className="block text-xs font-medium text-zinc-800">
                      {provider.name}
                    </strong>
                    <small className="block truncate text-[11px] text-zinc-400">
                      {provider.endpoint}
                    </small>
                  </div>
                  <span className="rounded-md border border-zinc-200 bg-white px-2 py-0.5 text-[11px] text-zinc-500">
                    {provider.status}
                  </span>
                  <button
                    className="h-7 cursor-pointer rounded-md border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-50"
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
            <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
              <div className="mb-3 flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-700">
                  <Server size={16} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-900">MCP Server 与插件运行时</h2>
                  <small className="text-[11px] text-zinc-400">
                    独立进程 + stdio 通信，支持健康检查与超时控制
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-zinc-500">
                平台适配器插件不直接持有数据库连接和完整凭据访问权限，所有工具调用均经过受控 Core
                API 与权限校验。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg border border-zinc-200 bg-white px-3 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                onClick={() => showToast("插件管理与权限审查视图开发中")}
              >
                <Plug size={13} />
                管理插件权限
              </button>
            </article>

            <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
              <div className="mb-3 flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-700">
                  <Shield size={16} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-zinc-900">安全、备份与语言</h2>
                  <small className="text-[11px] text-zinc-400">
                    本地备份恢复、审计日志与中英文界面预留
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-zinc-500">
                高风险操作（自动发布、删除内容、公开资料修改）默认需要用户显式确认；索引异常时支持从
                Markdown 文件重建索引。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center rounded-lg border border-zinc-200 bg-white px-3 text-xs font-medium text-zinc-700 hover:bg-zinc-50"
                onClick={() => showToast("已触发工作区索引一致性检查（演示模式）")}
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
