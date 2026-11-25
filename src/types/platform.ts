export type AccountStatus =
  "UNAUTHORIZED" | "AUTHENTICATING" | "ACTIVE" | "EXPIRED" | "NEEDS_REAUTH" | "ERROR" | "DISABLED";

export type PlatformCategory = "tech" | "article" | "video" | "social";
export type AuthMode = "browser" | "oauth" | "manual" | "embeddedWebView";
export type CapabilityLevel = "supported" | "restricted" | "experimental" | "unsupported";

export interface PlatformCapabilities {
  contentRead: CapabilityLevel;
  contentPublish: CapabilityLevel;
  analytics: CapabilityLevel;
}

export interface PlatformAuthMethod {
  method: AuthMode;
  available: boolean;
  label: string;
  loginUrl?: string | null;
  manualSteps: string[];
  reason?: string | null;
}

export interface PlatformCatalogEntry {
  id: string;
  name: string;
  category: PlatformCategory;
  categoryLabel: string;
  contentFormats: string;
  capabilities: PlatformCapabilities;
  authMethods: PlatformAuthMethod[];
}

export interface PlatformAccountItem {
  id: string;
  platformId: string;
  platformName: string;
  category: PlatformCategory;
  contentFormats: string;
  authMode: Exclude<AuthMode, "embeddedWebView">;
  alias: string;
  nickname?: string;
  uid: string;
  status: AccountStatus;
  enabled?: boolean;
  isDefault?: boolean;
  profileId?: string;
  credentialRef?: string;
  lastCheckedAt?: string | null;
  lastSyncedAt?: string;
  lastError?: string | null;
  capabilities: PlatformCapabilities;
}

export interface AuthChallenge {
  accountId: string;
  challengeId: string;
  method: "browser";
  loginUrl: string;
  manualSteps: string[];
  credentialLabel: string;
  credentialHelp: string;
}

export interface AccountCheckResult {
  account: PlatformAccountItem;
  message: string;
}

export interface DeletionPreview {
  accountId: string;
  platformName: string;
  alias: string;
  willDeleteCredentials: boolean;
  willDeleteSessionProfile: string;
  retainedHistoryCount: number;
  retainedData: string[];
}

export interface DeletionConfirmation {
  credentials: boolean;
  sessions: boolean;
  historyRetainedUnderstood: boolean;
}

export interface DeletionResult {
  accountId: string;
  deletedCredentials: boolean;
  deletedSessionProfile: boolean;
  retainedHistoryCount: number;
  message: string;
}

export interface AccountAuditLog {
  id: number;
  platformId: string;
  accountAlias: string;
  action: string;
  detail: string;
  createdAt: string;
}
