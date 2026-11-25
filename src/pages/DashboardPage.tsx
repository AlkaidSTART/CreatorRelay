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
import { useNavigate } from "react-router";
import { PageHeader } from "../components/PageHeader";
import { useContentStore, useUiStore, useWorkspaceStore } from "../stores";
import { formatRelativeTime } from "../utils/format";

export function DashboardPage() {
  const navigate = useNavigate();

  const documents = useWorkspaceStore((state) => state.documents);
  const selectDocumentSafely = useWorkspaceStore((state) => state.selectDocumentSafely);
  const setIsCreating = useUiStore((state) => state.setIsCreating);

  const platformAccounts = useContentStore((state) => state.platformAccounts);
  const publishedContents = useContentStore((state) => state.publishedContents);
  const publishTasks = useContentStore((state) => state.publishTasks);

  const activeAccounts = platformAccounts.filter((item) => item.status === "ACTIVE").length;
  const issueAccounts = platformAccounts.filter((item) => item.status !== "ACTIVE");
  const pendingTasks = publishTasks.filter(
    (task) => task.status === "NEEDS_MANUAL_ACTION" || task.status === "SCHEDULED",
  );

  return (
    <main className="flex h-full min-w-0 flex-col bg-white">
      <PageHeader
        eyebrow="DASHBOARD"
        title="运营总览"
        description="统一查看已接入平台、待处理发布任务、账号会话健康度与近期作品表现。"
        priority="P0"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-lg bg-zinc-900 px-3 text-xs font-medium text-white transition-colors hover:bg-zinc-800"
            onClick={() => setIsCreating(true)}
          >
            <Plus size={14} />
            新建文章
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-7 py-6">
        {/* Metrics Grid */}
        <section className="grid grid-cols-2 gap-4 xl:grid-cols-4">
          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="flex items-center justify-between text-xs text-zinc-500">
              <span>已接入平台与账号</span>
              <Globe2 size={15} className="text-zinc-400" />
            </div>
            <div className="mt-3 flex items-baseline gap-1.5">
              <strong className="text-2xl font-semibold tracking-tight text-zinc-900">
                {platformAccounts.length}
              </strong>
              <span className="text-xs text-zinc-400">个账号</span>
            </div>
            <p className="mt-1.5 text-xs text-zinc-400">
              {activeAccounts} 个会话有效 · {issueAccounts.length} 个需重新验证
            </p>
          </article>

          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="flex items-center justify-between text-xs text-zinc-500">
              <span>本地主稿库</span>
              <FileText size={15} className="text-zinc-400" />
            </div>
            <div className="mt-3 flex items-baseline gap-1.5">
              <strong className="text-2xl font-semibold tracking-tight text-zinc-900">
                {documents.length}
              </strong>
              <span className="text-xs text-zinc-400">篇主稿</span>
            </div>
            <p className="mt-1.5 text-xs text-zinc-400">Markdown 文件优先 · SQLite 索引同步</p>
          </article>

          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="flex items-center justify-between text-xs text-zinc-500">
              <span>待处理发布任务</span>
              <Send size={15} className="text-zinc-400" />
            </div>
            <div className="mt-3 flex items-baseline gap-1.5">
              <strong className="text-2xl font-semibold tracking-tight text-zinc-900">
                {pendingTasks.length}
              </strong>
              <span className="text-xs text-zinc-400">项待办</span>
            </div>
            <p className="mt-1.5 text-xs text-zinc-400">1 项需人工确认 · 1 项定时发布排队中</p>
          </article>

          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="flex items-center justify-between text-xs text-zinc-500">
              <span>已同步外部作品</span>
              <CheckCircle2 size={15} className="text-zinc-400" />
            </div>
            <div className="mt-3 flex items-baseline gap-1.5">
              <strong className="text-2xl font-semibold tracking-tight text-zinc-900">
                {publishedContents.length}
              </strong>
              <span className="text-xs text-zinc-400">篇作品</span>
            </div>
            <p className="mt-1.5 text-xs text-zinc-400">保留各平台独立指标来源与口径</p>
          </article>
        </section>

        {/* Notice Bar */}
        <div className="flex items-center gap-3 rounded-xl border border-zinc-200/80 bg-zinc-50/80 px-4 py-3 text-xs text-zinc-500">
          <span className="h-4 w-0.5 shrink-0 rounded-full bg-zinc-400" />
          <span>
            数据口径说明：不同平台的阅读量、播放量与粉丝统计口径存在差异，总览仅按平台维度独立呈现，不做未定义口径的直接相加。
          </span>
        </div>

        {/* Two Column Cards */}
        <section className="grid grid-cols-1 gap-5 lg:grid-cols-2">
          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="mb-4 flex items-start justify-between">
              <div>
                <span className="text-[10px] font-semibold tracking-widest text-zinc-400 uppercase">
                  ALERTS &amp; SESSIONS
                </span>
                <h2 className="mt-0.5 text-base font-semibold text-zinc-900">账号健康与异常提醒</h2>
              </div>
              <button
                className="cursor-pointer text-xs font-medium text-zinc-500 hover:text-zinc-900"
                onClick={() => navigate("/platforms")}
              >
                管理平台
              </button>
            </div>
            <div className="space-y-2.5">
              {issueAccounts.map((account) => (
                <div
                  key={account.id}
                  className="flex items-center gap-3 rounded-lg border border-zinc-100 bg-zinc-50/60 px-3.5 py-2.5"
                >
                  <div className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-amber-50 text-amber-600">
                    <AlertTriangle size={15} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-zinc-800">
                      {account.platformName} · {account.alias}
                    </strong>
                    <small className="text-[11px] text-zinc-400">
                      状态：{account.status} · 上次同步 {account.lastSyncedAt}
                    </small>
                  </div>
                  <button
                    className="h-7 shrink-0 cursor-pointer rounded-md border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-50"
                    onClick={() => navigate("/platforms")}
                  >
                    去处理
                  </button>
                </div>
              ))}
              {pendingTasks.map((task) => (
                <div
                  key={task.id}
                  className="flex items-center gap-3 rounded-lg border border-zinc-100 bg-zinc-50/60 px-3.5 py-2.5"
                >
                  <div className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-blue-50 text-blue-600">
                    <Clock3 size={15} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-zinc-800">
                      [{task.platform}] {task.articleTitle}
                    </strong>
                    <small className="block truncate text-[11px] text-zinc-400">{task.note}</small>
                  </div>
                  <button
                    className="h-7 shrink-0 cursor-pointer rounded-md border border-zinc-200 bg-white px-2.5 text-xs font-medium text-zinc-600 hover:bg-zinc-50"
                    onClick={() => navigate("/publishing")}
                  >
                    查看任务
                  </button>
                </div>
              ))}
            </div>
          </article>

          <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
            <div className="mb-4 flex items-start justify-between">
              <div>
                <span className="text-[10px] font-semibold tracking-widest text-zinc-400 uppercase">
                  RECENT DRAFTS
                </span>
                <h2 className="mt-0.5 text-base font-semibold text-zinc-900">最近编辑的主稿</h2>
              </div>
              <button
                className="cursor-pointer text-xs font-medium text-zinc-500 hover:text-zinc-900"
                onClick={() => navigate("/studio")}
              >
                进入写作空间
              </button>
            </div>
            <div className="space-y-2.5">
              {documents.slice(0, 4).map((doc) => (
                <button
                  key={doc.id}
                  onClick={async () => {
                    const ok = await selectDocumentSafely(doc);
                    if (ok) navigate("/studio");
                  }}
                  className="flex w-full cursor-pointer items-center gap-3 rounded-lg border border-zinc-100 bg-zinc-50/50 px-3.5 py-2.5 text-left transition-colors hover:border-zinc-200 hover:bg-zinc-50"
                >
                  <div className="grid h-8 w-8 shrink-0 place-items-center rounded-lg bg-zinc-100 text-zinc-600">
                    <FileText size={14} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <strong className="block truncate text-xs font-medium text-zinc-800">
                      {doc.title}
                    </strong>
                    <small className="text-[11px] text-zinc-400">
                      {formatRelativeTime(doc.updatedAt)} · {doc.wordCount} 词
                    </small>
                  </div>
                  <ArrowRight size={14} className="shrink-0 text-zinc-400" />
                </button>
              ))}
            </div>
          </article>
        </section>

        {/* Recent Performance Table */}
        <article className="rounded-xl border border-zinc-200/80 bg-white p-5">
          <div className="mb-4 flex items-start justify-between">
            <div>
              <span className="text-[10px] font-semibold tracking-widest text-zinc-400 uppercase">
                RECENT PERFORMANCE
              </span>
              <h2 className="mt-0.5 text-base font-semibold text-zinc-900">近期外部作品表现</h2>
            </div>
            <button
              className="cursor-pointer text-xs font-medium text-zinc-500 hover:text-zinc-900"
              onClick={() => navigate("/content")}
            >
              查看全部作品
            </button>
          </div>
          <div className="divide-y divide-zinc-100">
            <div className="grid grid-cols-[minmax(200px,2fr)_minmax(140px,1.2fr)_minmax(160px,1.3fr)_minmax(120px,1fr)] gap-4 pb-2.5 text-xs font-medium text-zinc-400">
              <span>作品标题</span>
              <span>平台 / 账号</span>
              <span>主指标（按平台口径）</span>
              <span>互动</span>
            </div>
            {publishedContents.map((item) => (
              <div
                key={item.id}
                className="grid grid-cols-[minmax(200px,2fr)_minmax(140px,1.2fr)_minmax(160px,1.3fr)_minmax(120px,1fr)] items-center gap-4 py-3"
              >
                <div className="min-w-0">
                  <strong className="block truncate text-xs font-medium text-zinc-800">
                    {item.title}
                  </strong>
                  <small className="text-[11px] text-zinc-400">{item.publishedAt}</small>
                </div>
                <div className="flex items-center gap-2">
                  <span className="rounded-md border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[11px] text-zinc-700">
                    {item.platform}
                  </span>
                  <span className="truncate text-xs text-zinc-500">{item.accountAlias}</span>
                </div>
                <div>
                  <strong className="block text-xs font-semibold text-zinc-900">
                    {item.readsLabel}
                  </strong>
                  <small className="text-[11px] text-zinc-400">{item.readsMetric}</small>
                </div>
                <div className="text-xs text-zinc-500">
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
