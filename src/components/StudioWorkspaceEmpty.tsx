import { motion, useReducedMotion } from "motion/react";
import { ArrowRight, FileText, FolderOpen, History, Plus, Send } from "./Icons";

type WorkspaceOnboardingProps = {
  onChooseWorkspace: () => void;
};

const PILLARS = [
  {
    icon: FileText,
    title: "纯 Markdown 存储",
    description: "本地 .md 文件，正文完全归你",
    badgeClass: "bg-accent-soft text-accent-deep",
  },
  {
    icon: History,
    title: "自动版本快照",
    description: "编辑自动留存，随时回溯历史",
    badgeClass: "bg-sky-bg text-sky-text",
  },
  {
    icon: Send,
    title: "多平台接力分发",
    description: "一稿多发，各平台独立适配",
    badgeClass: "bg-mint-bg text-mint-text",
  },
] as const;

export function StudioWorkspaceOnboarding({ onChooseWorkspace }: WorkspaceOnboardingProps) {
  const reducedMotion = useReducedMotion();

  return (
    <div className="studio-onboarding-stage flex min-h-full w-full items-center justify-center px-6 py-10">
      <motion.section
        initial={reducedMotion ? false : { opacity: 0, y: 12, scale: 0.99 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        transition={{ duration: 0.32, ease: [0.22, 1, 0.36, 1] }}
        className="w-full max-w-[540px] rounded-[26px] border border-line bg-surface p-7 shadow-[0_20px_50px_-28px_rgba(51,43,41,0.14),0_2px_8px_rgba(51,43,41,0.03)] sm:p-9"
      >
        {/* Minimalist Editorial Workspace Schematic */}
        <div
          className="studio-onboarding-preview relative mb-7 overflow-hidden rounded-2xl border border-line/90 p-5"
          aria-hidden="true"
        >
          <div className="relative z-10 grid grid-cols-1 gap-3.5 sm:grid-cols-[1fr_1.15fr]">
            {/* Mini Local Directory Tree */}
            <div className="flex flex-col justify-between rounded-xl border border-line bg-surface/95 p-3.5 shadow-[0_4px_14px_-8px_rgba(51,43,41,0.08)]">
              <div className="flex items-center gap-2 border-b border-line/70 pb-2.5">
                <span className="grid h-6 w-6 shrink-0 place-items-center rounded-lg bg-accent-soft text-accent-deep">
                  <FolderOpen size={11} />
                </span>
                <div className="min-w-0">
                  <div className="truncate text-[11px] font-semibold text-ink">WritingVault</div>
                  <div className="truncate font-mono text-[9.5px] text-ink-muted">~/Documents</div>
                </div>
              </div>

              <div className="mt-2.5 space-y-1.5 font-mono text-[10px]">
                <div className="flex items-center gap-1.5 rounded-md bg-accent-soft/85 px-2 py-1 text-accent-deep">
                  <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-accent" />
                  <span className="truncate font-medium">edge-runtime.md</span>
                </div>
                <div className="flex items-center gap-1.5 px-2 py-0.5 text-ink-muted">
                  <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-line-strong" />
                  <span className="truncate">weekly-notes.md</span>
                </div>
                <div className="flex items-center gap-1.5 px-2 py-0.5 text-ink-muted/80">
                  <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-line-strong" />
                  <span className="truncate">assets/cover.png</span>
                </div>
              </div>
            </div>

            {/* Mini Markdown Document Sheet */}
            <div className="flex flex-col justify-between rounded-xl border border-line bg-surface p-3.5 shadow-[0_8px_22px_-12px_rgba(51,43,41,0.12)]">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1">
                  <span className="h-1.5 w-1.5 rounded-full bg-line-strong" />
                  <span className="h-1.5 w-1.5 rounded-full bg-line-strong" />
                  <span className="h-1.5 w-1.5 rounded-full bg-line-strong" />
                </div>
                <span className="rounded-md bg-subtle px-1.5 py-0.5 font-mono text-[9px] font-medium text-ink-muted">
                  Markdown
                </span>
              </div>

              <div className="mt-3 space-y-2">
                <div className="inline-flex items-center gap-1 rounded bg-accent-soft/75 px-1.5 py-0.5 font-mono text-[9px] text-accent-deep">
                  <span># 本地优先写作工作流</span>
                </div>
                <div className="space-y-1.5 pt-0.5">
                  <div className="h-1.5 w-full rounded-full bg-subtle" />
                  <div className="h-1.5 w-[86%] rounded-full bg-subtle" />
                  <div className="h-1.5 w-[64%] rounded-full bg-subtle" />
                </div>
              </div>

              <div className="mt-3.5 flex items-center justify-between border-t border-line/70 pt-2 text-[9.5px] text-ink-muted">
                <span className="inline-flex items-center gap-1 text-mint-text">
                  <span className="h-1.5 w-1.5 rounded-full bg-mint-text/70" />
                  已保存至本地
                </span>
                <span className="font-mono">UTF-8</span>
              </div>
            </div>
          </div>
        </div>

        {/* Copy & Primary Action */}
        <div className="text-center">
          <span className="inline-flex items-center gap-1.5 rounded-full border border-accent-border bg-accent-soft/70 px-2.5 py-0.5 text-[10.5px] font-medium tracking-wide text-accent-deep">
            <span className="h-1.5 w-1.5 rounded-full bg-accent" />
            本地工作区 · Local-First
          </span>

          <h2 className="mt-3.5 text-[22px] font-semibold tracking-tight text-ink">
            选择你的本地写作文件夹
          </h2>

          <p className="mx-auto mt-2 max-w-[400px] text-[13px] leading-[1.75] text-ink-secondary">
            主稿以纯 Markdown 文件保存在你的硬盘中，支持离线写作与自由迁移，并可一键接力分发至各大创作平台。
          </p>

          <div className="mt-6 flex flex-col items-center gap-2.5">
            <button
              type="button"
              onClick={onChooseWorkspace}
              className="group inline-flex h-10 cursor-pointer items-center gap-2 rounded-xl bg-accent px-5 text-[13px] font-medium text-accent-on shadow-[0_8px_20px_-8px_rgba(91,72,136,0.5)] transition-all hover:-translate-y-px hover:bg-accent-hover active:translate-y-0"
            >
              <FolderOpen size={14} />
              <span>选择本地工作区</span>
              <ArrowRight
                size={12}
                className="opacity-80 transition-transform group-hover:translate-x-0.5"
              />
            </button>

            <span className="text-[11px] text-ink-muted">
              支持已有 Markdown 目录或新建空文件夹 · 不会修改原有文件
            </span>
          </div>
        </div>

        {/* Quiet Value Pillars */}
        <div className="mt-7 grid grid-cols-1 gap-3 border-t border-line/80 pt-5 sm:grid-cols-3">
          {PILLARS.map((item) => {
            const Icon = item.icon;
            return (
              <div
                key={item.title}
                className="flex items-start gap-2.5 rounded-xl bg-subtle/55 px-3 py-2.5 sm:flex-col sm:gap-2"
              >
                <span
                  className={`grid h-6 w-6 shrink-0 place-items-center rounded-lg ${item.badgeClass}`}
                >
                  <Icon size={11} />
                </span>
                <div className="min-w-0">
                  <div className="text-[11.5px] font-semibold text-ink">{item.title}</div>
                  <div className="mt-0.5 text-[10.5px] leading-snug text-ink-muted">
                    {item.description}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </motion.section>
    </div>
  );
}

type NoDocumentSelectedProps = {
  onCreateDocument: () => void;
};

export function StudioNoDocumentState({ onCreateDocument }: NoDocumentSelectedProps) {
  return (
    <div className="flex min-h-full w-full items-center justify-center px-6 py-10">
      <div className="flex max-w-[340px] flex-col items-center rounded-2xl border border-line bg-surface px-8 py-9 text-center shadow-[0_12px_32px_-20px_rgba(51,43,41,0.1)]">
        <div className="grid h-11 w-11 place-items-center rounded-2xl border border-accent-border bg-accent-soft text-accent-deep">
          <FileText size={18} />
        </div>
        <strong className="mt-4 text-[15px] font-semibold tracking-tight text-ink">
          选择一篇主稿开始写作
        </strong>
        <p className="mt-1.5 text-xs leading-relaxed text-ink-secondary">
          从左侧列表打开已有文章，或直接创建一篇新的 Markdown 主稿。
        </p>
        <button
          type="button"
          onClick={onCreateDocument}
          className="mt-5 inline-flex h-9 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-4 text-xs font-medium text-accent-on shadow-[0_6px_16px_-8px_rgba(91,72,136,0.45)] transition-all hover:-translate-y-px hover:bg-accent-hover"
        >
          <Plus size={12} />
          新建文章
          <span className="ml-1 rounded bg-white/15 px-1.5 py-0.5 font-mono text-[10px]">⌘N</span>
        </button>
      </div>
    </div>
  );
}
