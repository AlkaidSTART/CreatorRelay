import type { PlatformAccountItem } from "../types/platform";
import type { CapabilityLevel, ContentKind, PluginSummary } from "../types/plugin";

export interface DraftTargetCandidate {
  platformId: string;
  platformName: string;
  accountId: string;
  accountAlias: string;
  adapterId?: string | null;
  source: "platform" | "adapter";
  capability: CapabilityLevel | "unavailable";
  selectable: boolean;
  reason: string;
}

const PLATFORM_KINDS: Record<string, ContentKind[]> = {
  juejin: ["article", "imageArticle"],
  csdn: ["article", "imageArticle"],
  zhihu: ["article", "imageArticle"],
  wechat: ["article", "imageArticle", "imageText", "video"],
  bilibili: ["video", "article", "imageArticle"],
  youtube: ["video"],
  douyin: ["video", "imageText"],
  kuaishou: ["video"],
  xiaohongshu: ["imageText", "video"],
  weibo: ["imageText", "article"],
};

export function platformSupportsKind(platformId: string, kind: ContentKind): boolean {
  return PLATFORM_KINDS[platformId]?.includes(kind) ?? false;
}

function capabilityReason(
  capability: CapabilityLevel,
  platformName: string,
  kind: ContentKind,
  source: "platform" | "adapter",
): string {
  if (capability === "supported") {
    return source === "adapter"
      ? `运行时探测：${platformName} 支持 ${kind} 发布`
      : `${platformName} 已声明支持该内容形态；发布前仍需通过平台检查`;
  }
  if (capability === "restricted") {
    return `${platformName} 将该形态标记为受限能力，生成平台稿后需要人工确认`;
  }
  if (capability === "experimental") {
    return `${platformName} 将该形态标记为实验能力，请人工复核`;
  }
  if (capability === "unavailable") {
    return `${platformName} 的能力探测失败，当前不可作为已支持目标`;
  }
  return `${platformName} 未声明支持该内容形态`;
}

export function buildDraftTargets(
  kind: ContentKind,
  accounts: PlatformAccountItem[],
  plugins: PluginSummary[],
): DraftTargetCandidate[] {
  const targets: DraftTargetCandidate[] = [];
  for (const account of accounts) {
    if (account.status !== "ACTIVE" || account.enabled === false) continue;
    if (!platformSupportsKind(account.platformId, kind)) {
      targets.push({
        platformId: account.platformId,
        platformName: account.platformName,
        accountId: account.id,
        accountAlias: account.alias,
        source: "platform",
        capability: "unsupported",
        selectable: false,
        reason: capabilityReason("unsupported", account.platformName, kind, "platform"),
      });
      continue;
    }
    const capability = account.capabilities.contentPublish;
    targets.push({
      platformId: account.platformId,
      platformName: account.platformName,
      accountId: account.id,
      accountAlias: account.alias,
      source: "platform",
      capability,
      selectable: capability !== "unsupported",
      reason: capabilityReason(capability, account.platformName, kind, "platform"),
    });
  }

  for (const plugin of plugins) {
    if (plugin.status !== "ENABLED") continue;
    const capability = plugin.capabilities.find((item) => item.kind === kind);
    if (!capability) continue;
    const effective = capability.effectivePublish;
    targets.push({
      platformId: plugin.id,
      platformName: plugin.name,
      accountId: `adapter:${plugin.id}:fixture`,
      accountAlias: "Adapter 测试账号作用域",
      adapterId: plugin.id,
      source: "adapter",
      capability: effective,
      selectable: effective === "supported" || effective === "restricted" || effective === "experimental",
      reason: capabilityReason(effective, plugin.name, kind, "adapter"),
    });
  }
  return targets;
}
