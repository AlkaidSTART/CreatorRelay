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
    <>
      <header className="topbar">
        <div className="breadcrumb">
          <span>{breadcrumbParent}</span>
          <ChevronRight size={14} />
          <strong>{title}</strong>
        </div>
        <div className="topbar-actions">
          {priority && <span className="priority-pill">{priority}</span>}
          {actions}
        </div>
      </header>
      <section className="page-hero">
        <div>
          <span className="context-eyebrow">{eyebrow}</span>
          <h1 className="page-title">{title}</h1>
          <p className="page-description">{description}</p>
        </div>
      </section>
    </>
  );
}
