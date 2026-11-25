import type { ReactNode } from "react";
import { ChevronRight } from "lucide-react";

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
  priority,
  breadcrumbParent = "CreatorRelay",
  actions,
}: PageHeaderProps) {
  return (
    <div className="shrink-0 bg-white">
      <header className="flex h-13 items-center justify-between border-b border-zinc-200/80 px-7">
        <div className="flex min-w-0 items-center gap-1.5 text-xs text-zinc-400">
          <span>{breadcrumbParent}</span>
          <ChevronRight size={13} className="text-zinc-300" />
          <strong className="truncate font-medium text-zinc-800">{title}</strong>
        </div>
        <div className="flex items-center gap-2.5">
          {priority && (
            <span className="rounded-md border border-zinc-200 bg-zinc-50 px-2 py-0.5 text-[11px] font-medium text-zinc-500">
              {priority}
            </span>
          )}
          {actions}
        </div>
      </header>
      <section className="border-b border-zinc-100 px-7 pt-6 pb-5">
        <span className="text-[10px] font-semibold tracking-[0.12em] text-zinc-400 uppercase">
          {eyebrow}
        </span>
        <h1 className="mt-1 text-2xl font-semibold tracking-tight text-zinc-900">{title}</h1>
        <p className="mt-1 text-[13px] leading-relaxed text-zinc-500">{description}</p>
      </section>
    </div>
  );
}
