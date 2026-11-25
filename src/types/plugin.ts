export type PluginStatus = "REVIEW_REQUIRED" | "READY" | "ENABLED" | "DISABLED" | "ERROR";
export type PluginTrustLevel = "OFFICIAL" | "LOCAL_UNTRUSTED";
export type PluginSource = "BUNDLED" | "LOCAL";
export type ContentKind = "video" | "imageText" | "imageArticle" | "article";
export type CapabilityLevel =
  "supported" | "unsupported" | "restricted" | "experimental" | "unavailable";
export type PermissionRisk = "medium" | "high";

export interface PluginPermissionView {
  id: string;
  label: string;
  risk: PermissionRisk;
  description: string;
  reason: string;
  required: boolean;
  granted?: boolean | null;
}

export interface CapabilityStateView {
  kind: ContentKind;
  label: string;
  declaredRead: CapabilityLevel;
  declaredPublish: CapabilityLevel;
  effectiveRead: CapabilityLevel;
  effectivePublish: CapabilityLevel;
  source: "declared" | "probe";
  reason?: string | null;
  checkedAt?: string | null;
}

export interface IsolationSummary {
  mode: string;
  strong: boolean;
  network: string;
  description: string;
}

export interface AdapterCatalogEntry {
  id: string;
  name: string;
  description: string;
  version: string;
  adapterContractVersion: number;
  pluginType: string;
  source: PluginSource;
  trustLevel: PluginTrustLevel;
  permissions: PluginPermissionView[];
  capabilities: CapabilityStateView[];
  installed: boolean;
  installable: boolean;
  blockedReason?: string | null;
  isolation: IsolationSummary;
}

export interface PluginSummary {
  id: string;
  name: string;
  description: string;
  version: string;
  adapterContractVersion: number;
  pluginType: string;
  runtimeKind: string;
  runtimeProfile?: string | null;
  entry: string;
  source: PluginSource;
  trustLevel: PluginTrustLevel;
  status: PluginStatus;
  enabled: boolean;
  runtimeState: "running" | "stopped";
  sandboxed: boolean;
  isolation: IsolationSummary;
  manifestHash: string;
  installPath: string;
  installedAt: string;
  updatedAt: string;
  lastError?: string | null;
  previousVersion?: string | null;
  lastProbeAt?: string | null;
  permissions: PluginPermissionView[];
  capabilities: CapabilityStateView[];
}

export interface HostAccessObservation {
  method: string;
  allowed: boolean;
  code?: string | null;
  detail?: string | null;
}

export interface PluginProbeResult {
  pluginId: string;
  accountRef: string;
  ok: boolean;
  error?: string | null;
  checkedAt: string;
  sandboxed: boolean;
  hostAccess: HostAccessObservation[];
  capabilities: CapabilityStateView[];
}

export interface PluginFaultResult {
  pluginId: string;
  fault: "crash" | "timeout";
  error: string;
  status: PluginStatus;
  runtimeState: string;
}

export interface PluginUninstallResult {
  pluginId: string;
  deletedPluginFiles: boolean;
  deletedPluginData: boolean;
  retainedEvents: number;
  message: string;
}

export interface PluginPermissionDecision {
  id: string;
  granted: boolean;
}

export interface PluginEvent {
  id: number;
  pluginId: string;
  pluginVersion: string;
  action: string;
  detail: string;
  createdAt: string;
}

export interface SandboxProbeObservation {
  target: string;
  allowed: boolean;
  error?: string | null;
}

export interface SecuritySelfTestResult {
  pluginId: string;
  sandboxed: boolean;
  databaseDenied: boolean;
  workspaceDenied: boolean;
  credentialDenied: boolean;
  allDenied: boolean;
  observations: SandboxProbeObservation[];
}

export interface ManifestIssue {
  field: string;
  code: string;
  message: string;
}

export interface ManifestValidation {
  valid: boolean;
  issues: ManifestIssue[];
  manifest?: unknown;
  manifestHash?: string | null;
  supportedContractVersion: number;
}

export type MediaKind = "video" | "image";

export interface PublishMediaRef {
  assetId: string;
  kind: MediaKind;
  mimeType: string;
  byteSize: number;
  order: number;
  role?: string | null;
}

export interface PublishInput {
  contentKind: ContentKind;
  title: string;
  body?: string | null;
  media: PublishMediaRef[];
  cover?: PublishMediaRef | null;
}

export interface PublishIssue {
  field: string;
  code: string;
  message: string;
}

export interface PublishValidationResult {
  ok: boolean;
  contentKind: ContentKind;
  issues: PublishIssue[];
  mediaOrder: number[];
  coverAssetId?: string | null;
}

export interface PluginPublishValidationResult {
  ok: boolean;
  payload: PublishValidationResult;
  capability?: CapabilityLevel | null;
  adapterChecked: boolean;
  adapterMessage?: string | null;
}

export interface PublishPayloadCheckResult {
  id: string;
  contentKind: ContentKind;
  expectedOk: boolean;
  actualOk: boolean;
  passed: boolean;
  message: string;
}
