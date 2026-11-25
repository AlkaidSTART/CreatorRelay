import { CreatorCentersPanel } from "../components/CreatorCentersPanel";
import { PageHeader } from "../components/PageHeader";

export function CreatorCentersPage() {
  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="CREATOR CENTERS"
        title="创作者中心"
        description="管理创作者中心网址，并查看 MCP 对平台实时数据与发布能力的实际边界。"
      />
      <CreatorCentersPanel />
    </main>
  );
}
