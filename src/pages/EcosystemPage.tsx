import { EcosystemPanel } from "../components/EcosystemPanel";
import { PageHeader } from "../components/PageHeader";

export function EcosystemPage() {
  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="ECOSYSTEM"
        title="生态治理"
        description="核对平台准入来源、账号级逐形态能力、Adapter 兼容性与第三方插件市场门禁。"
      />
      <div className="min-h-0 flex-1 overflow-y-auto px-7 py-6">
        <EcosystemPanel />
      </div>
    </main>
  );
}
