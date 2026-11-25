import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import { createStore } from "zustand/vanilla";
import type { StateCreator } from "zustand";
import type { CapabilityLevel } from "../types/plugin";
import type {
  AccountCapabilitySnapshot,
  AdmissionRegistry,
  CompatibilityReport,
  EcosystemOverview,
  MarketplacePolicy,
  PlatformAdmission,
} from "../types/ecosystem";

const capabilityKinds = ["video", "imageText", "imageArticle", "article"] as const;

const demoPlatforms: Array<{
  id: string;
  name: string;
  source: string;
  statuses: [CapabilityLevel, CapabilityLevel, CapabilityLevel, CapabilityLevel];
}> = [
  {
    id: "bilibili",
    name: "Bilibili",
    source: "https://open.bilibili.com/",
    statuses: ["experimental", "experimental", "experimental", "restricted"],
  },
  {
    id: "youtube",
    name: "YouTube",
    source: "https://developers.google.com/youtube/v3/docs",
    statuses: ["experimental", "unsupported", "unsupported", "unsupported"],
  },
  {
    id: "juejin",
    name: "掘金",
    source: "https://open.juejin.cn/",
    statuses: ["unsupported", "experimental", "experimental", "experimental"],
  },
];

const demoAdmissions: PlatformAdmission[] = demoPlatforms.map((platform) => ({
  platformId: platform.id,
  platformName: platform.name,
  adapterContractVersion: 1,
  admissionStatus: "evaluation_only",
  autoPublishAllowed: false,
  publishDispatch: "manual_only",
  accountTypes: ["个人创作者"],
  reviewedAt: "2026-11-25",
  officialSources: [
    {
      title: "官方开放平台/接口文档",
      url: platform.source,
      httpStatus: null,
      checkedAt: "2026-11-25",
      result: "浏览器预览数据；桌面端读取静态准入登记",
    },
  ],
  contentKinds: capabilityKinds.map((kind, index) => ({
    kind,
    label: ["视频", "图文", "图文文章", "纯文章"][index],
    read: {
      level: platform.statuses[index],
      evidence:
        platform.statuses[index] === "unsupported" ? "official_scope" : "official_docs_only",
      reason: "账号级运行验证尚未完成",
    },
    publish: {
      level: platform.statuses[index],
      evidence: platform.statuses[index] === "unsupported" ? "official_scope" : "not_verified",
      reason: "自动发布尚未通过平台准入，仅保留人工发布",
    },
  })),
  unsupported: ["绕过平台审核", "未授权自动登录"],
  notes: "浏览器预览只展示保守能力，不代表真实平台支持。",
}));

const demoAdmissionRegistry: AdmissionRegistry = {
  schemaVersion: 1,
  policyVersion: "preview",
  reviewedAt: "2026-11-25",
  statusSemantics: {
    supported: "有当前账号级运行证据并通过发布准入",
    restricted: "受账号类型或人工审核限制",
    experimental: "官方入口存在但当前版本未完成运行验证",
    unsupported: "平台不提供该原生形态",
    unavailable: "账号或接口当前不可用",
  },
  platforms: demoAdmissions,
};

const demoMarketplace: MarketplacePolicy = {
  schemaVersion: 1,
  decision: "closed",
  reviewedAt: "2026-11-25",
  rationale: "强隔离、签名分发与应急下架尚未全部完成，第三方市场保持关闭。",
  gates: [
    {
      id: "threat_assessment",
      label: "威胁评估",
      required: true,
      passed: true,
      evidence: "ADR-002 / ADR-003",
    },
    {
      id: "source_signature",
      label: "来源与签名",
      required: true,
      passed: false,
      evidence: "尚未建立第三方签名链",
    },
    {
      id: "strong_isolation",
      label: "强隔离",
      required: true,
      passed: false,
      evidence: "Windows/Linux 仍只有进程隔离",
    },
    {
      id: "update_rollback",
      label: "更新与回滚",
      required: true,
      passed: true,
      evidence: "版本回退与权限交集已测试",
    },
    {
      id: "permission_revocation",
      label: "权限撤销",
      required: true,
      passed: true,
      evidence: "Core hostCall 强制拒绝",
    },
    {
      id: "emergency_takedown",
      label: "应急下架",
      required: true,
      passed: false,
      evidence: "尚无远程吊销链",
    },
  ],
  allowedDistribution: "bundled_official_only",
  thirdPartyInstallAllowed: false,
  controlledPilotAllowed: false,
};

const demoCompatibility: CompatibilityReport = {
  contractVersion: 1,
  checkedAt: new Date().toISOString(),
  adapterCount: 6,
  passed: true,
  checks: [
    { id: "manifest_contract", label: "Manifest 合约", passed: true, detail: "浏览器预览" },
    { id: "unique_adapter_ids", label: "身份唯一", passed: true, detail: "浏览器预览" },
    { id: "content_kind_matrix", label: "四形态覆盖", passed: true, detail: "浏览器预览" },
  ],
};

const demoAccounts: AccountCapabilitySnapshot[] = [
  {
    accountId: "demo-bilibili",
    accountAlias: "视频主号",
    platformId: "bilibili",
    platformName: "Bilibili",
    accountStatus: "NEEDS_REAUTH",
    admissionStatus: "evaluation_only",
    autoPublishAllowed: false,
    publishDispatch: "manual_only",
    degraded: true,
    degradationReason: "账号会话已过期；需要重新授权，自动发布保持关闭",
    capabilities: demoAdmissions[0].contentKinds.map((item) => ({
      kind: item.kind,
      label: item.label,
      read: "unavailable",
      publish: "unavailable",
      readEvidence: item.read.evidence,
      publishEvidence: item.publish.evidence,
      autoPublishAllowed: false,
      reason: "账号非 ACTIVE，能力已降级",
    })),
    officialSources: demoAdmissions[0].officialSources,
    computedAt: new Date().toISOString(),
  },
];

const demoOverview: EcosystemOverview = {
  admissions: demoAdmissionRegistry,
  accounts: demoAccounts,
  marketplace: demoMarketplace,
  compatibility: demoCompatibility,
};

export interface EcosystemStore {
  overview: EcosystemOverview | null;
  loading: boolean;
  initialized: boolean;
  error: string;
  initialize: (force?: boolean) => Promise<void>;
  refresh: () => Promise<void>;
}

const createEcosystemStoreState: StateCreator<EcosystemStore> = (set, get) => ({
  overview: null,
  loading: false,
  initialized: false,
  error: "",

  initialize: async (force = false) => {
    if (get().initialized && !force) return;
    set({ loading: true, error: "" });
    if (!isTauri()) {
      set({ overview: demoOverview, loading: false, initialized: true });
      return;
    }
    try {
      const overview = await invoke<EcosystemOverview>("get_ecosystem_overview");
      set({ overview, loading: false, initialized: true });
    } catch (cause) {
      set({
        loading: false,
        initialized: true,
        error: cause instanceof Error ? cause.message : String(cause),
      });
    }
  },

  refresh: async () => get().initialize(true),
});

export const useEcosystemStore = create<EcosystemStore>(createEcosystemStoreState);
export const createEcosystemStore = () => createStore<EcosystemStore>(createEcosystemStoreState);
