import type { ContentAsset, ContentKind } from "./content";
import type { CapabilityLevel, PublishIssue, PublishValidationResult } from "./plugin";

export type PublishTaskStatus =
  | "CREATED"
  | "VALIDATING"
  | "PENDING"
  | "SCHEDULED"
  | "PUBLISHING"
  | "SUCCESS"
  | "FAILED"
  | "CANCELLED"
  | "NEEDS_MANUAL_ACTION";

export type PublishExecutionMode = "auto" | "manual";
export type PublishSourceType = "platformDraft" | "content";

export interface PublishTask {
  id: string;
  sourceType: PublishSourceType;
  sourceId: string;
  contentId: string;
  contentVersion: string;
  contentKind: ContentKind;
  platformId: string;
  platformName: string;
  accountId: string;
  accountAlias: string;
  adapterId?: string | null;
  title: string;
  body: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  executionMode: PublishExecutionMode;
  status: PublishTaskStatus;
  paused: boolean;
  pauseReason?: string | null;
  scheduledAt?: string | null;
  externalId?: string | null;
  externalUrl?: string | null;
  errorCode?: string | null;
  errorMessage?: string | null;
  nextAction?: string | null;
  attemptCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface PublishTargetInput {
  platformId: string;
  platformName: string;
  accountId: string;
  accountAlias: string;
  adapterId?: string | null;
}

export interface CreatePublishTasksInput {
  sourceType: PublishSourceType;
  sourceId: string;
  targets: PublishTargetInput[];
  scheduledAt?: string | null;
}

export interface CreatePublishTasksResult {
  tasks: PublishTask[];
  deduplicatedCount: number;
}

export interface PreviewAction {
  id: "auto_publish" | "manual_package" | "cancel";
  label: string;
  available: boolean;
  reason?: string | null;
}

export interface PublishPreview {
  task: PublishTask;
  taskId: string;
  confirmationToken?: string | null;
  confirmationExpiresAt?: string | null;
  executionMode: PublishExecutionMode;
  capability: CapabilityLevel;
  capabilityReason?: string | null;
  validation: PublishValidationResult;
  actions: PreviewAction[];
}

export interface MetricSnapshot {
  id: string;
  workId: string;
  metricName: string;
  metricDefinition: string;
  unit: string;
  value: number;
  capturedAt: string;
  snapshotDay: string;
  source: string;
  canonicalKey?: string | null;
  comparable: boolean;
}

export interface ExternalWork {
  id: string;
  sourceKey: string;
  accountId?: string | null;
  platformId: string;
  platformName: string;
  accountAlias: string;
  externalId: string;
  title: string;
  contentKind: ContentKind | "unknown";
  originalContentType: string;
  rawFields: Record<string, unknown>;
  url?: string | null;
  publishedAt: string;
  source: string;
  syncedAt: string;
  createdAt: string;
  updatedAt: string;
  metrics: MetricSnapshot[];
}

export interface ExternalWorkFilter {
  keyword?: string | null;
  platformId?: string | null;
  accountId?: string | null;
  contentKind?: ContentKind | "unknown" | null;
  publishedFrom?: string | null;
  publishedTo?: string | null;
}

export interface SyncPlatformWorksInput {
  accountId: string;
  platformId: string;
  platformName: string;
  accountAlias: string;
  adapterId: string;
  asOf?: string | null;
}

export interface SyncRunResult {
  id: string;
  accountId: string;
  platformId: string;
  adapterId: string;
  status: "SUCCESS" | "PARTIAL" | "FAILED" | "ACCOUNT_INVALID";
  message: string;
  importedCount: number;
  updatedCount: number;
  skippedCount: number;
  errorCount: number;
  accountInvalid: boolean;
  errors: string[];
  startedAt: string;
  completedAt: string;
}

export type AnalyticsRange = "7d" | "30d" | "90d";
export type AnalyticsGranularity = "day" | "week" | "month";

export interface AnalyticsQuery {
  range: AnalyticsRange;
  granularity: AnalyticsGranularity;
  platformId?: string | null;
  accountId?: string | null;
  contentKind?: ContentKind | null;
  workId?: string | null;
}

export interface AnalyticsPoint {
  bucket: string;
  capturedAt: string;
  value: number;
}

export interface AnalyticsSeries {
  key: string;
  workId: string;
  title: string;
  platformId: string;
  platformName: string;
  accountAlias: string;
  contentKind: ContentKind | "unknown";
  metricName: string;
  metricDefinition: string;
  unit: string;
  points: AnalyticsPoint[];
  comparable: boolean;
  comparabilityNote: string;
}

export interface AnalyticsResult {
  range: AnalyticsRange;
  from: string;
  to: string;
  granularity: AnalyticsGranularity;
  series: AnalyticsSeries[];
  warnings: string[];
}

export interface DashboardFilter {
  range?: AnalyticsRange;
  platformId?: string | null;
  accountId?: string | null;
  contentKind?: ContentKind | null;
}

export interface DashboardFilterApplied {
  range: AnalyticsRange;
  platformId?: string | null;
  accountId?: string | null;
  contentKind?: ContentKind | null;
  from: string;
  to: string;
}

export interface OperationsDashboard {
  platformCount: number;
  accountCount: number;
  recentPublishCount: number;
  pendingTaskCount: number;
  syncErrorCount: number;
  recentWorks: ExternalWork[];
  pendingTasks: PublishTask[];
  recentPublishes: PublishTask[];
  syncRuns: SyncRunResult[];
  filter: DashboardFilterApplied;
}

export interface ManualPackageAsset {
  assetId: string;
  kind: "video" | "image";
  role: string;
  order: number;
  file: string;
  originalName: string;
  mimeType: string;
  byteSize: number;
  caption: string;
}

export interface ManualPublishPackage {
  task: PublishTask;
  directory: string;
  manifestPath: string;
  bodyPath: string;
  assets: ManualPackageAsset[];
  generatedAt: string;
}

export interface ResolveUncertainInput {
  outcome: "success" | "failed" | "cancelled";
  externalId?: string | null;
  externalUrl?: string | null;
  note?: string | null;
}

export interface ManualCompletionInput {
  externalId?: string | null;
  externalUrl?: string | null;
}

export interface PublishTaskEvent {
  id: number;
  taskId: string;
  action: string;
  detail: string;
  createdAt: string;
}

export type PublishIssueView = PublishIssue;
