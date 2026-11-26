import type { ReactNode } from "react";

interface PageHeaderProps {
  eyebrow: string;
  title: string;
  description: string;
  priority?: "P0" | "P0/P1" | "P1";
  breadcrumbParent?: string;
  actions?: ReactNode;
}

export function PageHeader({ eyebrow, title, description, actions }: PageHeaderProps) {
  return (
    <header className="shrink-0 border-b border-line bg-surface-soft px-6 py-4">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <h1 className="text-[17px] font-semibold tracking-[-0.02em] text-ink">{title}</h1>
            <span className="rounded-xs border border-line bg-subtle px-1.5 py-px font-mono text-[9.5px] font-medium tracking-[0.06em] text-ink-muted uppercase">
              {eyebrow}
            </span>
          </div>
          <p className="mt-1 text-[12px] leading-relaxed text-ink-secondary">{description}</p>
        </div>
        {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
      </div>
    </header>
  );
}
