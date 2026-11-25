import type { ContentKind } from "./content";
import type { CapabilityLevel } from "./plugin";

export interface OfficialSource {
  title: string;
  url: string;
  httpStatus?: number | null;
  checkedAt: string;
  result: string;
}

export interface CapabilityAssessment {
  level: CapabilityLevel;
  evidence: "runtime_verified" | "official_docs_only" | "official_scope" | "not_verified";
  reason: string;
}

export interface ContentKindCapability {
  kind: ContentKind;
  label: string;
  read: CapabilityAssessment;
  publish: CapabilityAssessment;
}

export interface PlatformAdmission {
  platformId: string;
  platformName: string;
  adapterContractVersion: number;
  admissionStatus: "evaluation_only" | "restricted" | "experimental" | "verified";
  autoPublishAllowed: boolean;
  publishDispatch: "manual_only" | "verified_auto";
  accountTypes: string[];
  reviewedAt: string;
  officialSources: OfficialSource[];
  contentKinds: ContentKindCapability[];
  unsupported: string[];
  notes: string;
}

export interface StatusSemantics {
  supported: string;
  restricted: string;
  experimental: string;
  unsupported: string;
  unavailable: string;
}

export interface AdmissionRegistry {
  schemaVersion: number;
  policyVersion: string;
  reviewedAt: string;
  statusSemantics: StatusSemantics;
  platforms: PlatformAdmission[];
}

export interface EffectiveCapability {
  kind: ContentKind;
  label: string;
  read: CapabilityLevel;
  publish: CapabilityLevel;
  readEvidence: CapabilityAssessment["evidence"];
  publishEvidence: CapabilityAssessment["evidence"];
  autoPublishAllowed: boolean;
  reason?: string | null;
}

export interface AccountCapabilitySnapshot {
  accountId: string;
  accountAlias: string;
  platformId: string;
  platformName: string;
  accountStatus: string;
  admissionStatus: string;
  autoPublishAllowed: boolean;
  publishDispatch: string;
  degraded: boolean;
  degradationReason?: string | null;
  capabilities: EffectiveCapability[];
  officialSources: OfficialSource[];
  computedAt: string;
}

export interface MarketplaceGate {
  id: string;
  label: string;
  required: boolean;
  passed: boolean;
  evidence: string;
}

export interface MarketplacePolicy {
  schemaVersion: number;
  decision: "closed" | "controlled_pilot" | "open";
  reviewedAt: string;
  rationale: string;
  gates: MarketplaceGate[];
  allowedDistribution: "bundled_official_only" | "signed_third_party" | "open_marketplace";
  thirdPartyInstallAllowed: boolean;
  controlledPilotAllowed: boolean;
}

export interface CompatibilityCheck {
  id: string;
  label: string;
  passed: boolean;
  detail: string;
}

export interface CompatibilityReport {
  contractVersion: number;
  checkedAt: string;
  adapterCount: number;
  passed: boolean;
  checks: CompatibilityCheck[];
}

export interface EcosystemOverview {
  admissions: AdmissionRegistry;
  accounts: AccountCapabilitySnapshot[];
  marketplace: MarketplacePolicy;
  compatibility: CompatibilityReport;
}

export interface PublishPolicy {
  platformId: string;
  accountId: string;
  contentKind: ContentKind;
  admissionStatus: string;
  capability: CapabilityLevel;
  autoPublishAllowed: boolean;
  manualPublishAllowed: boolean;
  reason: string;
}
