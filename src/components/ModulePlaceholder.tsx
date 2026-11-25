import { ArrowRight, Clock3, ShieldCheck } from "./Icons";
import { PageHeader } from "./PageHeader";
import { useNavigate } from "react-router";
import { findRouteByPathname } from "../routes/navigation";

export function ModulePlaceholder({ routeId }: { routeId: string }) {
  const navigate = useNavigate();
  const route = findRouteByPathname(`/${routeId}`);

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader eyebrow={route.eyebrow} title={route.label} description={route.description} />
      <div className="min-h-0 flex-1 overflow-y-auto px-7 py-6">
        <section className="mx-auto max-w-3xl rounded-2xl border border-line bg-surface p-8 shadow-[0_2px_12px_rgba(74,58,48,0.03)]">
          <div className="flex flex-wrap items-start justify-between gap-4">
            <div className="flex items-start gap-4">
              <div className="grid h-11 w-11 shrink-0 place-items-center rounded-2xl border border-lemon-border bg-lemon-bg text-lemon-text">
                <Clock3 size={19} />
              </div>
              <div>
                <span className="inline-flex rounded-lg bg-lemon-bg px-2 py-0.5 text-[10.5px] font-semibold text-lemon-text">
                  {route.milestone} · 规划中
                </span>
                <h2 className="mt-2 text-lg font-semibold tracking-tight text-ink">
                  {route.label}尚未开放
                </h2>
                <p className="mt-1 max-w-xl text-xs leading-relaxed text-ink-secondary">
                  M1
                  只交付桌面基础设施和模块入口。该模块需要后续阶段完成并验收后开放，当前页面不会展示演示业务数据。
                </p>
              </div>
            </div>
          </div>

          <div className="mt-6 grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-line bg-canvas p-4">
              <div className="flex items-center gap-2 text-xs font-medium text-ink">
                <ShieldCheck size={13} className="text-mint-text" />
                当前可验证能力
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-ink-secondary">
                Tauri 桌面壳、Rust IPC、SQLite 版本化迁移、错误反馈与脱敏日志。
              </p>
            </div>
            <div className="rounded-xl border border-line bg-canvas p-4">
              <div className="flex items-center gap-2 text-xs font-medium text-ink">
                <Clock3 size={13} className="text-lemon-text" />
                开放条件
              </div>
              <p className="mt-2 text-[11px] leading-relaxed text-ink-secondary">
                对应阶段功能完成、自动化与平台手工检查通过，并在阶段验收记录中标记为 accepted。
              </p>
            </div>
          </div>

          <div className="mt-6 flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => navigate("/settings")}
              className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on hover:bg-accent-hover"
            >
              查看系统与诊断
              <ArrowRight size={12} />
            </button>
            <button
              type="button"
              onClick={() => navigate("/studio")}
              className="inline-flex h-8.5 cursor-pointer items-center rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary hover:bg-subtle"
            >
              返回内容工作室
            </button>
          </div>
        </section>
      </div>
    </main>
  );
}
