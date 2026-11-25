import { FolderOpen, KeyRound, Plug, Server, Shield } from "lucide-react";
import { useOutletContext } from "react-router";
import { PageHeader } from "../components/PageHeader";
import type { AppOutletContext } from "../layouts/AppLayout";

const aiProviders = [
  { name: "OpenAI", endpoint: "https://api.openai.com/v1", status: "未配置" },
  { name: "Anthropic", endpoint: "https://api.anthropic.com", status: "未配置" },
  { name: "Google Gemini", endpoint: "https://generativelanguage.googleapis.com", status: "未配置" },
  { name: "DeepSeek", endpoint: "https://api.deepseek.com", status: "未配置" },
  { name: "OpenAI Compatible", endpoint: "自定义端点 / 本地模型", status: "可扩展" },
];

export function SettingsPage() {
  const { workspace, chooseWorkspace, showToast } = useOutletContext<AppOutletContext>();

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="SETTINGS"
        title="系统设置"
        description="管理本地工作区与存储路径、BYOK AI 模型供应商、MCP Server、插件权限及安全备份策略。"
        priority="P0"
      />

      <div className="page-body settings-stack">
        <article className="surface-card">
          <div className="card-heading">
            <div className="platform-card-title">
              <div className="status-icon neutral">
                <FolderOpen size={14} />
              </div>
              <div>
                <h2>工作区与本地存储</h2>
                <small>Markdown 文件为正文权威数据源，SQLite 保存索引与元数据</small>
              </div>
            </div>
            <button className="secondary-button" onClick={() => void chooseWorkspace()}>
              切换工作区
            </button>
          </div>
          <div className="settings-kv-grid">
            <div>
              <span>当前工作区路径</span>
              <code>{workspace?.path ?? "尚未选择工作区"}</code>
            </div>
            <div>
              <span>SQLite 索引库</span>
              <code>data/creator-relay.db</code>
            </div>
            <div>
              <span>素材与导出目录</span>
              <code>workspace/assets · workspace/exports</code>
            </div>
          </div>
        </article>

        <article className="surface-card">
          <div className="card-heading">
            <div className="platform-card-title">
              <div className="status-icon neutral">
                <KeyRound size={14} />
              </div>
              <div>
                <h2>BYOK AI 模型供应商</h2>
                <small>API Key 优先保存在操作系统安全凭据存储（OS Keychain），不明文落库</small>
              </div>
            </div>
            <button
              className="secondary-button"
              onClick={() => showToast("BYOK 凭据配置表单将在下一迭代接入 Keychain")}
            >
              添加端点
            </button>
          </div>
          <div className="stack-list">
            {aiProviders.map((provider) => (
              <div className="list-row" key={provider.name}>
                <div className="list-row-copy">
                  <strong>{provider.name}</strong>
                  <small>{provider.endpoint}</small>
                </div>
                <span className="tag-chip">{provider.status}</span>
                <button
                  className="secondary-button"
                  onClick={() => showToast(`即将配置 ${provider.name} API Key 与默认模型`)}
                >
                  配置
                </button>
              </div>
            ))}
          </div>
        </article>

        <section className="two-col-grid">
          <article className="surface-card">
            <div className="card-heading">
              <div className="platform-card-title">
                <div className="status-icon neutral">
                  <Server size={14} />
                </div>
                <div>
                  <h2>MCP Server 与插件运行时</h2>
                  <small>独立进程 + stdio 通信，支持健康检查与超时控制</small>
                </div>
              </div>
            </div>
            <p className="card-body-copy">
              平台适配器插件不直接持有数据库连接和完整凭据访问权限，所有工具调用均经过受控 Core
              API 与权限校验。
            </p>
            <button
              className="secondary-button"
              onClick={() => showToast("插件管理与权限审查视图开发中")}
            >
              <Plug size={12} />
              管理插件权限
            </button>
          </article>

          <article className="surface-card">
            <div className="card-heading">
              <div className="platform-card-title">
                <div className="status-icon neutral">
                  <Shield size={14} />
                </div>
                <div>
                  <h2>安全、备份与语言</h2>
                  <small>本地备份恢复、审计日志与中英文界面预留</small>
                </div>
              </div>
            </div>
            <p className="card-body-copy">
              高风险操作（自动发布、删除内容、公开资料修改）默认需要用户显式确认；索引异常时支持从
              Markdown 文件重建索引。
            </p>
            <button
              className="secondary-button"
              onClick={() => showToast("已触发工作区索引一致性检查（演示模式）")}
            >
              重建本地索引
            </button>
          </article>
        </section>
      </div>
    </main>
  );
}
