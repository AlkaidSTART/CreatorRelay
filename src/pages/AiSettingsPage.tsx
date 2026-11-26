import { AiProviderSettings } from "../components/AiProviderSettings";
import { PageHeader } from "../components/PageHeader";

export function AiSettingsPage() {
  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="AI MODELS"
        title="AI 模型配置"
        description="管理自带（BYOK）模型供应商、兼容端点、本地模型与系统钥匙串中的 API Key。"
      />
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
        <div className="max-w-4xl">
          <AiProviderSettings />
        </div>
      </div>
    </main>
  );
}
