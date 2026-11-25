import type { ContentKind } from "./content";

export type ToolPermission = "read" | "write" | "externalAction";

export interface McpToolDefinition {
  name: string;
  title: string;
  description: string;
  schemaVersion: string;
  permission: ToolPermission;
  inputSchema: Record<string, unknown>;
}

export interface McpHostConfig {
  autoRestart: boolean;
  maxRestarts: number;
}

export interface McpHostStatus {
  state: "healthy" | "stopped" | "unhealthy" | "restarting" | string;
  serverId: string;
  serverName: string;
  protocolVersion: string;
  pid?: number | null;
  startedAt?: string | null;
  lastHealthAt?: string | null;
  lastError?: string | null;
  restartCount: number;
  config: McpHostConfig;
}

export interface McpSettings {
  readEnabled: boolean;
  writePolicy: "confirm" | "allow";
  externalPolicy: "confirm_each_time";
  autoRestart: boolean;
  maxRestarts: number;
  defaultContextScope: "title_only" | "title_summary" | "title_summary_media" | "selected";
}

export interface McpFieldError {
  path: string;
  message: string;
}

export interface McpCallError {
  code: string;
  message: string;
  fieldErrors: McpFieldError[];
}

export interface McpCallResult {
  callId: string;
  auditId: string;
  toolName: string;
  permission: ToolPermission;
  status: "success" | "partial" | "failed" | "cancelled" | "timedOut" | string;
  contentKind?: string | null;
  targetSummary: string;
  parameterSummary: string;
  result?: unknown;
  error?: McpCallError | null;
  durationMs: number;
}

export interface McpToolCallView {
  callId: string;
  toolName: string;
  permission: ToolPermission;
  targetSummary: string;
  parameterSummary: string;
  arguments?: Record<string, unknown> | null;
  status: string;
  errorCode?: string | null;
  errorMessage?: string | null;
}

export interface McpAuthorizationPreview {
  callId: string;
  toolName: string;
  title: string;
  permission: ToolPermission;
  contentKind?: string | null;
  targetSummary: string;
  parameterSummary: string;
  effects: string[];
  details: Record<string, unknown>;
  requiresConfirmation: boolean;
  authorizationToken?: string | null;
  confirmationExpiresAt?: string | null;
  fieldErrors: McpFieldError[];
}

export interface McpAuditRecord {
  id: string;
  threadId?: string | null;
  callId: string;
  serverId: string;
  toolName: string;
  permission: ToolPermission;
  contentKind?: string | null;
  targetSummary: string;
  parameterSummary: string;
  status: string;
  errorCode?: string | null;
  errorMessage?: string | null;
  startedAt: string;
  completedAt?: string | null;
}

export interface McpSecurityEvidence {
  processIsolated: boolean;
  transport: string;
  databasePathPassed: boolean;
  credentialReferencePassed: boolean;
  directDatabaseApiAvailable: boolean;
  directCredentialApiAvailable: boolean;
  businessToolsForwardToCore: boolean;
  forbiddenArgumentFields: string[];
  allowedMediaReferenceFields: string[];
  observations: string[];
}

export interface ModelContextScope {
  includeTitle: boolean;
  includeSummary: boolean;
  includeBody: boolean;
  includeMediaMetadata: boolean;
}

export interface AssistantThread {
  id: string;
  title: string;
  providerId?: string | null;
  providerName: string;
  model: string;
  createdAt: string;
  updatedAt: string;
}

export interface AssistantMessage {
  id: string;
  threadId: string;
  role: "user" | "assistant";
  content: string;
  providerName?: string | null;
  model?: string | null;
  toolCall?: McpToolCallView | null;
  resultStatus?: string | null;
  errorMessage?: string | null;
  createdAt: string;
}

export interface AssistantThreadDetail {
  thread: AssistantThread;
  messages: AssistantMessage[];
}

export interface AssistantPlan {
  thread: AssistantThread;
  userMessage: AssistantMessage;
  assistantMessage: AssistantMessage;
}

export interface AssistantContextPreview {
  providerName: string;
  model: string;
  contentKind?: ContentKind | string | null;
  context: string;
  characterCount: number;
  includedFields: string[];
  excludedFields: string[];
  redactionCount: number;
}

export interface AssistantGenerateRequest {
  threadId: string;
  prompt?: string | null;
  providerId?: string | null;
  contentId?: string | null;
  scope: ModelContextScope;
  instruction?: string | null;
}

export interface AssistantGenerateResult {
  userMessage?: AssistantMessage | null;
  assistantMessage: AssistantMessage;
  contextPreview: AssistantContextPreview;
}
