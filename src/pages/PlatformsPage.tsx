import { CheckCircle2, Globe2, Plus, RefreshCw, ShieldAlert } from "lucide-react";
import { useOutletContext } from "react-router";
import { PageHeader } from "../components/PageHeader";
import { demoPlatformAccounts, plannedPlatforms, type AccountStatus } from "../data/demoData";
import type { AppOutletContext } from "../layouts/AppLayout";

const accountStatusLabel: Record<AccountStatus, { label: string; tone: string }> = {
  ACTIVE: { label: "会话有效", tone: "ok" },
  EXPIRED: { label: "会话已过期", tone: "danger" },
  NEEDS_REAUTH: { label: "需重新授权", tone: "warn" },
  UNAUTHORIZED: { label: "未登录", tone: "neutral" },
};

const capabilityLabel: Record<string, string> = {
  supported: "支持",
  restricted: "受限",
  experimental: "实验性",
  unsupported: "人工辅助",
};

export function PlatformsPage() {
  const { showToast } = useOutletContext<AppOutletContext>();

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="PLATFORMS"
        title="平台管理"
        description="采用 Platform → Accounts → Platform Sessions 三级模型管理多账号会话隔离与适配器能力声明。"
        priority="P0"
        actions={
          <button
            className="publish-button"
            onClick={() => showToast("接入新平台流程（选择平台 → 登录验证 → 能力检测）准备中")}
          >
            <Plus size={14} />
            接入平台账号
          </button>
        }
      />

      <div className="page-body">
        <div className="callout-bar">
          <span className="note-line" />
          <span>
            安全与合规原则：登录状态以实际运行检测为准，不绕过验证码或平台风控；不支持自动发布的平台提供人工发布辅助流程。
          </span>
        </div>

        <section className="two-col-grid">
          {demoPlatformAccounts.map((account) => {
            const status = accountStatusLabel[account.status];
            return (
              <article className="surface-card platform-card" key={account.id}>
                <div className="card-heading">
                  <div className="platform-card-title">
                    <div className="status-icon neutral">
                      <Globe2 size={14} />
                    </div>
                    <div>
                      <h2>
                        {account.platformName} · {account.alias}
                      </h2>
                      <small>
                        UID: {account.uid} · {account.contentFormats} · 认证：{account.authMode}
                      </small>
                    </div>
                  </div>
                  <span className={`status-badge ${status.tone}`}>
                    {account.status === "ACTIVE" ? (
                      <CheckCircle2 size={11} />
                    ) : (
                      <ShieldAlert size={11} />
                    )}
                    {status.label}
                  </span>
                </div>

                <div className="capability-row">
                  <span className="tag-chip">
                    作品读取：{capabilityLabel[account.capabilities.contentRead]}
                  </span>
                  <span className="tag-chip">
                    发布能力：{capabilityLabel[account.capabilities.contentPublish]}
                  </span>
                  <span className="tag-chip">
                    数据分析：{capabilityLabel[account.capabilities.analytics]}
                  </span>
                </div>

                <div className="platform-card-footer">
                  <small>最近检测/同步：{account.lastSyncedAt}</small>
                  <div className="platform-card-actions">
                    <button
                      className="secondary-button"
                      onClick={() => showToast(`正在检测 ${account.platformName} (${account.alias}) 会话状态`)}
                    >
                      <RefreshCw size={12} />
                      检测状态
                    </button>
                    {account.status !== "ACTIVE" && (
                      <button
                        className="primary-button"
                        onClick={() => showToast(`将打开受控浏览器重新登录 ${account.platformName}`)}
                      >
                        重新登录
                      </button>
                    )}
                  </div>
                </div>
              </article>
            );
          })}
        </section>

        <article className="surface-card">
          <div className="card-heading">
            <div>
              <span className="context-eyebrow">ROADMAP ADAPTERS</span>
              <h2>首期规划接入的其他平台</h2>
            </div>
          </div>
          <div className="planned-grid">
            {plannedPlatforms.map((platform) => (
              <div className="planned-item" key={platform.name}>
                <strong>{platform.name}</strong>
                <span className="tag-chip">{platform.formats}</span>
                <small>{platform.note}</small>
              </div>
            ))}
          </div>
        </article>
      </div>
    </main>
  );
}
