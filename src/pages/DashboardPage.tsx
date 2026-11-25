import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  Clock3,
  FileText,
  Globe2,
  Plus,
  Send,
} from "lucide-react";
import { useNavigate, useOutletContext } from "react-router";
import { PageHeader } from "../components/PageHeader";
import { demoPlatformAccounts, demoPublishedContents, demoPublishTasks } from "../data/demoData";
import { formatRelativeTime } from "../hooks/useWorkspaceDocuments";
import type { AppOutletContext } from "../layouts/AppLayout";

export function DashboardPage() {
  const { documents, setIsCreating } = useOutletContext<AppOutletContext>();
  const navigate = useNavigate();

  const activeAccounts = demoPlatformAccounts.filter((item) => item.status === "ACTIVE").length;
  const issueAccounts = demoPlatformAccounts.filter((item) => item.status !== "ACTIVE");
  const pendingTasks = demoPublishTasks.filter(
    (task) => task.status === "NEEDS_MANUAL_ACTION" || task.status === "SCHEDULED",
  );

  return (
    <main className="page-shell">
      <PageHeader
        eyebrow="DASHBOARD"
        title="运营总览"
        description="统一查看已接入平台、待处理发布任务、账号会话健康度与近期作品表现。"
        priority="P0"
        actions={
          <button className="publish-button" onClick={() => setIsCreating(true)}>
            <Plus size={14} />
            新建文章
          </button>
        }
      />

      <div className="page-body">
        <section className="metric-grid">
          <article className="surface-card stat-card">
            <div className="stat-head">
              <span>已接入平台与账号</span>
              <Globe2 size={15} />
            </div>
            <strong>
              {demoPlatformAccounts.length} <small>个账号</small>
            </strong>
            <p>
              {activeAccounts} 个会话有效 · {issueAccounts.length} 个需重新验证
            </p>
          </article>

          <article className="surface-card stat-card">
            <div className="stat-head">
              <span>本地主稿库</span>
              <FileText size={15} />
            </div>
            <strong>
              {documents.length} <small>篇主稿</small>
            </strong>
            <p>Markdown 文件优先 · SQLite 索引同步</p>
          </article>

          <article className="surface-card stat-card">
            <div className="stat-head">
              <span>待处理发布任务</span>
              <Send size={15} />
            </div>
            <strong>
              {pendingTasks.length} <small>项待办</small>
            </strong>
            <p>1 项需人工确认 · 1 项定时发布排队中</p>
          </article>

          <article className="surface-card stat-card">
            <div className="stat-head">
              <span>已同步外部作品</span>
              <CheckCircle2 size={15} />
            </div>
            <strong>
              {demoPublishedContents.length} <small>篇作品</small>
            </strong>
            <p>已保留各平台独立指标来源与口径</p>
          </article>
        </section>

        <div className="callout-bar">
          <span className="note-line" />
          <span>
            数据口径说明：不同平台的阅读量、播放量与粉丝统计口径存在差异，总览仅按平台维度独立呈现，不做未定义口径的直接相加。
          </span>
        </div>

        <section className="two-col-grid">
          <article className="surface-card">
            <div className="card-heading">
              <div>
                <span className="context-eyebrow">ALERTS &amp; SESSIONS</span>
                <h2>账号健康与异常提醒</h2>
              </div>
              <button className="text-action" onClick={() => navigate("/platforms")}>
                管理平台
              </button>
            </div>
            <div className="stack-list">
              {issueAccounts.map((account) => (
                <div className="list-row" key={account.id}>
                  <div className="status-icon warn">
                    <AlertTriangle size={14} />
                  </div>
                  <div className="list-row-copy">
                    <strong>
                      {account.platformName} · {account.alias}
                    </strong>
                    <small>
                      状态：{account.status} · 上次同步 {account.lastSyncedAt}
                    </small>
                  </div>
                  <button className="secondary-button" onClick={() => navigate("/platforms")}>
                    去处理
                  </button>
                </div>
              ))}
              {pendingTasks.map((task) => (
                <div className="list-row" key={task.id}>
                  <div className="status-icon info">
                    <Clock3 size={14} />
                  </div>
                  <div className="list-row-copy">
                    <strong>
                      [{task.platform}] {task.articleTitle}
                    </strong>
                    <small>{task.note}</small>
                  </div>
                  <button className="secondary-button" onClick={() => navigate("/publishing")}>
                    查看任务
                  </button>
                </div>
              ))}
            </div>
          </article>

          <article className="surface-card">
            <div className="card-heading">
              <div>
                <span className="context-eyebrow">RECENT DRAFTS</span>
                <h2>最近编辑的主稿</h2>
              </div>
              <button className="text-action" onClick={() => navigate("/studio")}>
                进入写作空间
              </button>
            </div>
            <div className="stack-list">
              {documents.slice(0, 4).map((doc) => (
                <button
                  className="list-row list-row-clickable"
                  key={doc.id}
                  onClick={() => navigate("/studio")}
                >
                  <div className="status-icon neutral">
                    <FileText size={14} />
                  </div>
                  <div className="list-row-copy">
                    <strong>{doc.title}</strong>
                    <small>
                      {formatRelativeTime(doc.updatedAt)} · {doc.wordCount} 词
                    </small>
                  </div>
                  <ArrowRight size={14} className="muted-icon" />
                </button>
              ))}
            </div>
          </article>
        </section>

        <article className="surface-card">
          <div className="card-heading">
            <div>
              <span className="context-eyebrow">RECENT PERFORMANCE</span>
              <h2>近期外部作品表现</h2>
            </div>
            <button className="text-action" onClick={() => navigate("/content")}>
              查看全部作品
            </button>
          </div>
          <div className="data-table">
            <div className="table-head">
              <span>作品标题</span>
              <span>平台 / 账号</span>
              <span>主指标（按平台口径）</span>
              <span>互动</span>
            </div>
            {demoPublishedContents.map((item) => (
              <div className="table-row" key={item.id}>
                <div className="table-primary">
                  <strong>{item.title}</strong>
                  <small>{item.publishedAt}</small>
                </div>
                <div>
                  <span className="tag-chip">{item.platform}</span>{" "}
                  <span className="muted-inline">{item.accountAlias}</span>
                </div>
                <div className="table-primary">
                  <strong>{item.readsLabel}</strong>
                  <small>{item.readsMetric}</small>
                </div>
                <div className="muted-inline">
                  赞 {item.likes} · 评 {item.comments} · 藏 {item.favorites}
                </div>
              </div>
            ))}
          </div>
        </article>
      </div>
    </main>
  );
}
