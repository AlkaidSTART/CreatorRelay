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
    <main className="flex h-full min-w-0 flex-col bg-[#FAF8F3]">
      <PageHeader
        eyebrow="SETTINGS"
        title="系统设置"
        description="管理本地工作区文件夹、自带 AI 模型密钥、MCP 扩展与安全备份策略。"
      />

      <div className="min-h-0 flex-1 overflow-y-auto px-7 py-6">
        <div className="max-w-4xl space-y-5">
          {/* Workspace Storage */}
          <article className="rounded-2xl border border-[#E6E2D8] bg-white p-5">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-[#EAF0EA] text-[#2F4A36]">
                  <FolderOpen size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-[#1E241E]">工作区与本地存储</h2>
                  <small className="text-xs text-[#7D847D]">
                    Markdown 文件为正文权威数据源，本地索引随时可无损重建
                  </small>
                </div>
              </div>
              <button
                className="h-8 shrink-0 cursor-pointer rounded-xl border border-[#DCD7CB] bg-white px-3 text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
                onClick={() => void chooseWorkspace()}
              >
                切换工作区
              </button>
            </div>
            <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
              <div className="rounded-xl border border-[#EFECE4] bg-[#FCFBF8] p-3">
                <span className="block text-[11px] text-[#858B83]">当前工作区路径</span>
                <code className="mt-1 block truncate font-mono text-xs text-[#242A24]">
                  {workspace?.path ?? "尚未选择工作区"}
                </code>
              </div>
              <div className="rounded-xl border border-[#EFECE4] bg-[#FCFBF8] p-3">
                <span className="block text-[11px] text-[#858B83]">SQLite 索引库</span>
                <code className="mt-1 block truncate font-mono text-xs text-[#242A24]">
                  data/creator-relay.db
                </code>
              </div>
              <div className="rounded-xl border border-[#EFECE4] bg-[#FCFBF8] p-3">
                <span className="block text-[11px] text-[#858B83]">素材与导出目录</span>
                <code className="mt-1 block truncate font-mono text-xs text-[#242A24]">
                  workspace/assets · exports
                </code>
              </div>
            </div>
          </article>

          {/* AI Providers */}
          <article className="rounded-2xl border border-[#E6E2D8] bg-white p-5">
            <div className="mb-4 flex items-center justify-between gap-4">
              <div className="flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-[#FDF5E6] text-[#B88128]">
                  <KeyRound size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-[#1E241E]">自带 AI 模型（BYOK）</h2>
                  <small className="text-xs text-[#7D847D]">
                    API Key 优先保存在系统钥匙串（OS Keychain），绝不明文上传
                  </small>
                </div>
              </div>
              <button
                className="h-8 shrink-0 cursor-pointer rounded-xl border border-[#DCD7CB] bg-white px-3 text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
                onClick={() => showToast("BYOK 凭据配置表单将在桌面端直接接入系统钥匙串")}
              >
                添加端点
              </button>
            </div>
            <div className="space-y-2">
              {aiProviders.map((provider) => (
                <div
                  key={provider.name}
                  className="flex items-center justify-between gap-3 rounded-xl border border-[#EFECE4] bg-[#FCFBF8] px-3.5 py-2.5"
                >
                  <div className="min-w-0 flex-1">
                    <strong className="block text-xs font-medium text-[#242A24]">
                      {provider.name}
                    </strong>
                    <small className="block truncate text-[11px] text-[#858B83]">
                      {provider.endpoint}
                    </small>
                  </div>
                  <span className="rounded-md border border-[#E6E2D8] bg-white px-2 py-0.5 text-[11px] text-[#7D847D]">
                    {provider.status}
                  </span>
                  <button
                    className="h-7 cursor-pointer rounded-lg border border-[#DCD7CB] bg-white px-2.5 text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
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
            <article className="rounded-2xl border border-[#E6E2D8] bg-white p-5">
              <div className="mb-3 flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-[#EAF0EA] text-[#2F4A36]">
                  <Server size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-[#1E241E]">MCP Server 与扩展</h2>
                  <small className="text-[11px] text-[#7D847D]">
                    独立进程隔离运行，支持健康检查与超时保护
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-[#646B64]">
                所有平台适配扩展均在受控权限下运行，不直接持有完整凭据与数据库访问权。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-[#DCD7CB] bg-white px-3 text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
                onClick={() => showToast("插件权限审查视图已就绪")}
              >
                <Plug size={12} />
                管理插件权限
              </button>
            </article>

            <article className="rounded-2xl border border-[#E6E2D8] bg-white p-5">
              <div className="mb-3 flex items-center gap-3">
                <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-[#FDF5E6] text-[#B88128]">
                  <Shield size={15} />
                </div>
                <div>
                  <h2 className="text-sm font-semibold text-[#1E241E]">安全与本地备份</h2>
                  <small className="text-[11px] text-[#7D847D]">
                    高风险操作强制确认 · 随时重建本地索引
                  </small>
                </div>
              </div>
              <p className="mb-4 text-xs leading-relaxed text-[#646B64]">
                自动发布、删除内容等高风险操作默认需要你显式确认；索引异常时可直接从本地 Markdown 文件一键恢复。
              </p>
              <button
                className="inline-flex h-8 cursor-pointer items-center rounded-xl border border-[#DCD7CB] bg-white px-3 text-xs font-medium text-[#2F4A36] hover:bg-[#EAF0EA]"
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
