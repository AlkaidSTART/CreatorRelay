import { ArrowRight, FileText, FolderOpen, History, Plus, Send } from "./Icons";

type WorkspaceOnboardingProps = {
  onChooseWorkspace: () => void;
};

const PILLARS = [
  {
    icon: FileText,
    title: "纯 Markdown 存储",
    description: "本地 .md 文件，正文完全归你",
  },
  {
    icon: History,
    title: "自动版本快照",
    description: "编辑自动留存，随时回溯历史",
  },
  {
    icon: Send,
    title: "多平台接力分发",
    description: "一稿多发，各平台独立适配",
  },
] as const;

export function StudioWorkspaceOnboarding({ onChooseWorkspace }: WorkspaceOnboardingProps) {
  return (
    <div className="studio-onboarding-stage flex min-h-full w-full items-center justify-center px-6 py-10">
      <section className="w-full max-w-[540px] rounded-lg border border-line bg-surface p-7 shadow-xs">
        {/* Minimalist Editorial Workspace Schematic */}
        <div
          className="studio-onboarding-preview relative mb-6 overflow-hidden rounded-md border border-line p-4"
          aria-hidden="true"
        >
          <div className="relative z-10 grid grid-cols-1 gap-3 sm:grid-cols-[1fr_1.15fr]">
            {/* Mini Local Directory Tree */}
            <div className="flex flex-col justify-between rounded-md border border-line bg-surface p-3">
              <div className="flex items-center gap-2 border-b border-line pb-2">
                <FolderOpen size={13} className="shrink-0 text-accent" />
                <div className="min-w-0">
                  <div className="truncate text-[11px] font-semibold text-ink">WritingVault</div>
                  <div className="truncate font-mono text-[9.5px] text-ink-muted">~/Documents</div>
                </div>
              </div>

              <div className="mt-2 space-y-1 font-mono text-[10px]">
                <div className="flex items-center gap-1.5 rounded-sm bg-accent-soft px-2 py-1 text-accent-deep">
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
            <div className="flex flex-col justify-between rounded-md border border-line bg-surface p-3">
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono text-[9.5px] text-ink-muted">master.md</span>
                <span className="rounded-xs border border-line bg-subtle px-1.5 py-px font-mono text-[9px] font-medium text-ink-muted">
                  Markdown
                </span>
              </div>

              <div className="mt-2.5 space-y-2">
                <div className="font-mono text-[10px] font-medium text-ink">
                  # 本地优先写作工作流
                </div>
                <div className="space-y-1.5 pt-0.5">
                  <div className="h-1.5 w-full rounded-xs bg-subtle" />
                  <div className="h-1.5 w-[86%] rounded-xs bg-subtle" />
                  <div className="h-1.5 w-[64%] rounded-xs bg-subtle" />
                </div>
              </div>

              <div className="mt-3 flex items-center justify-between border-t border-line pt-2 text-[9.5px] text-ink-muted">
                <span className="inline-flex items-center gap-1 text-mint-text">
                  <span className="h-1.5 w-1.5 rounded-full bg-mint-text" />
                  已保存至本地
                </span>
                <span className="font-mono">UTF-8</span>
              </div>
            </div>
          </div>
        </div>

        {/* Copy & Primary Action */}
        <div>
          <span className="font-mono text-[10px] font-semibold tracking-[0.08em] text-accent uppercase">
            LOCAL-FIRST WORKSPACE
          </span>

          <h2 className="mt-1.5 text-[19px] font-semibold tracking-tight text-ink">
            选择你的本地写作文件夹
          </h2>

          <p className="mt-1.5 text-[12.5px] leading-relaxed text-ink-secondary">
            主稿以纯 Markdown 文件保存在你的硬盘中，支持离线写作与自由迁移，并可一键接力分发至各大创作平台。
          </p>

          <div className="mt-5 flex flex-wrap items-center gap-3">
            <button
              type="button"
              onClick={onChooseWorkspace}
              className="group inline-flex h-9 cursor-pointer items-center gap-2 rounded-md bg-accent px-4 text-[12.5px] font-medium text-accent-on transition-colors hover:bg-accent-hover"
            >
              <FolderOpen size={13} />
              <span>选择本地工作区</span>
              <ArrowRight
                size={12}
                className="opacity-80 transition-transform group-hover:translate-x-0.5"
              />
            </button>

            <span className="text-[11px] text-ink-muted">
              支持已有目录或新建空文件夹 · 不会修改原有文件
            </span>
          </div>
        </div>

        {/* Structured Value Pillars */}
        <div className="mt-6 grid grid-cols-1 divide-y divide-line border-t border-line pt-4 sm:grid-cols-3 sm:divide-x sm:divide-y-0">
          {PILLARS.map((item, idx) => {
            const Icon = item.icon;
            return (
              <div
                key={item.title}
                className={`py-2 sm:py-0 ${idx > 0 ? "sm:pl-4" : ""} ${idx < 2 ? "sm:pr-4" : ""}`}
              >
                <div className="flex items-center gap-1.5 text-[11.5px] font-semibold text-ink">
                  <Icon size={12} className="text-ink-muted" />
                  <span>{item.title}</span>
                </div>
                <div className="mt-1 text-[11px] leading-snug text-ink-muted">
                  {item.description}
                </div>
              </div>
            );
          })}
        </div>
      </section>
    </div>
  );
}

type NoDocumentSelectedProps = {
  onCreateDocument: () => void;
};

export function StudioNoDocumentState({ onCreateDocument }: NoDocumentSelectedProps) {
  return (
    <div className="flex min-h-full w-full items-center justify-center px-6 py-10">
      <div className="flex max-w-[340px] flex-col items-center rounded-lg border border-line bg-surface px-7 py-8 text-center">
        <div className="grid h-10 w-10 place-items-center rounded-md border border-line bg-subtle text-ink-secondary">
          <FileText size={16} />
        </div>
        <strong className="mt-3.5 text-[14px] font-semibold tracking-tight text-ink">
          选择一篇主稿开始写作
        </strong>
        <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
          从左侧列表打开已有文章，或直接创建一篇新的 Markdown 主稿。
        </p>
        <button
          type="button"
          onClick={onCreateDocument}
          className="mt-4 inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-md bg-accent px-3.5 text-xs font-medium text-accent-on transition-colors hover:bg-accent-hover"
        >
          <Plus size={12} />
          新建文章
          <span className="ml-1 rounded-xs bg-white/15 px-1 py-px font-mono text-[9.5px]">⌘N</span>
        </button>
      </div>
    </div>
  );
}
