import { PageHeader } from "../components/PageHeader";
import { PluginManager } from "../components/PluginManager";

export function AdaptersPage() {
  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="ADAPTERS"
        title="适配器与插件"
        description="发现、安装并审查平台适配器；按内容形态探测真实能力，限制插件对账号、凭据和本地数据的访问。"
      />
      <div className="min-h-0 flex-1 overflow-y-auto px-7 py-6">
        <PluginManager />
      </div>
    </main>
  );
}
