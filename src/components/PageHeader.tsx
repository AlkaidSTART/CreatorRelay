import type { ReactNode } from "react";
import { ChevronRight } from "../components/Icons";

interface PageHeaderProps {
  eyebrow: string;
  title: string;
  description: string;
  priority?: "P0" | "P0/P1" | "P1";
  breadcrumbParent?: string;
  actions?: ReactNode;
}

export function PageHeader({
  eyebrow,
  title,
  description,
  breadcrumbParent = "CreatorRelay",
  actions,
}: PageHeaderProps) {
  return (
    <div className="shrink-0 bg-surface-soft">
      <header className="flex h-12 items-center justify-between border-b border-line px-7">
        <div className="flex min-w-0 items-center gap-1.5 text-xs text-ink-muted">
          <span>{breadcrumbParent}</span>
          <ChevronRight size={12} className="text-ink-faint" />
          <strong className="truncate font-medium text-ink">{title}</strong>
        </div>
        {actions && <div className="flex items-center gap-2.5">{actions}</div>}
      </header>
      <section className="border-b border-line px-7 pt-5 pb-4">
        <span className="text-[10px] font-semibold tracking-[0.12em] text-ink-muted uppercase">
          {eyebrow}
        </span>
        <h1 className="mt-0.5 text-[21px] font-semibold tracking-tight text-ink">{title}</h1>
        <p className="mt-1 text-[13px] leading-relaxed text-ink-secondary">{description}</p>
      </section>
    </div>
  );
}
