import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import type {
  AssistantContextPreview,
  AssistantGenerateRequest,
  AssistantGenerateResult,
  AssistantMessage,
  AssistantThread,
  AssistantThreadDetail,
  McpAuditRecord,
  McpAuthorizationPreview,
  McpCallResult,
  McpHostStatus,
  McpSecurityEvidence,
  McpSettings,
  McpToolDefinition,
  ModelContextScope,
} from "../types/mcp";
import { useUiStore } from "./uiStore";

const demoThread: AssistantThread = {
  id: "demo-thread",
  title: "MCP 工作流演示",
  providerId: null,
  providerName: "本地编排器",
  model: "deterministic-v1",
  createdAt: "2026-11-25T00:00:00Z",
  updatedAt: "2026-11-25T00:00:00Z",
};

const demoTools: McpToolDefinition[] = [
  {
    name: "account_get_info",
    title: "读取账号信息",
    description: "通过 Core 读取脱敏账号状态",
    schemaVersion: "1.0",
    permission: "read",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "content_list",
    title: "查询本地内容",
    description: "检索四类内容草稿",
    schemaVersion: "1.0",
    permission: "read",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "studio_create_content",
    title: "创建内容草稿",
    description: "创建四类本地内容草稿",
    schemaVersion: "1.0",
    permission: "write",
    inputSchema: { type: "object", properties: {} },
  },
  {
    name: "publishing_publish_task",
    title: "发布已确认内容",
    description: "外部动作，每次执行前确认",
    schemaVersion: "1.0",
    permission: "externalAction",
    inputSchema: { type: "object", properties: {} },
  },
];

const demoStatus: McpHostStatus = {
  state: "healthy",
  serverId: "creatorrelay.demo-mcp",
  serverName: "演示 MCP Host",
  protocolVersion: "2024-11-05",
  pid: null,
  startedAt: "2026-11-25T00:00:00Z",
  lastHealthAt: "2026-11-25T00:00:00Z",
  restartCount: 0,
  config: { autoRestart: true, maxRestarts: 3 },
};

const demoSettings: McpSettings = {
  readEnabled: true,
  writePolicy: "confirm",
  externalPolicy: "confirm_each_time",
  autoRestart: true,
  maxRestarts: 3,
  defaultContextScope: "title_summary_media",
};

const demoSecurity: McpSecurityEvidence = {
  processIsolated: true,
  transport: "stdio JSON-RPC · Core host forwarding",
  databasePathPassed: false,
  credentialReferencePassed: false,
  directDatabaseApiAvailable: false,
  directCredentialApiAvailable: false,
  businessToolsForwardToCore: true,
  forbiddenArgumentFields: ["path", "filePath", "credentialRef", "cookie", "token", "apiKey"],
  allowedMediaReferenceFields: ["sourceContentId", "assetId", "order", "caption"],
  observations: ["Web 演示模式只展示结构；实际生命周期与审计需在桌面应用中验证。"],
};

function demoMessage(
  role: AssistantMessage["role"],
  content: string,
  toolCall?: AssistantMessage["toolCall"],
): AssistantMessage {
  return {
    id: `${role}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
    threadId: demoThread.id,
    role,
    content,
    providerName: role === "assistant" ? "本地编排器" : null,
    model: role === "assistant" ? "deterministic-v1" : null,
    toolCall,
    resultStatus: toolCall?.status ?? null,
    createdAt: new Date().toISOString(),
  };
}

function planDemoTool(prompt: string): AssistantMessage["toolCall"] {
  if (/账号|登录|会话/.test(prompt)) {
    return {
      callId: `proposal-${Date.now()}`,
      toolName: "account_get_info",
      permission: "read",
      targetSummary: "全部脱敏账号状态",
      parameterSummary: "无筛选参数",
      arguments: {},
      status: "proposed",
    };
  }
  if (/内容|草稿|检索/.test(prompt)) {
    return {
      callId: `proposal-${Date.now()}`,
      toolName: "content_list",
      permission: "read",
      targetSummary: "本地内容包",
      parameterSummary: "limit=20",
      arguments: { limit: 20 },
      status: "proposed",
    };
  }
  return undefined;
}

export interface McpStore {
  status: McpHostStatus | null;
  settings: McpSettings | null;
  tools: McpToolDefinition[];
  audit: McpAuditRecord[];
  security: McpSecurityEvidence | null;
  threads: AssistantThread[];
  activeThread: AssistantThreadDetail | null;
  activePreview: McpAuthorizationPreview | null;
  previewArguments: Record<string, unknown> | null;
  contextPreview: AssistantContextPreview | null;
  pendingCallId: string;
  loading: boolean;
  busyAction: string;
  error: string;
  initialized: boolean;
  initialize: (force?: boolean) => Promise<void>;
  refreshStatus: () => Promise<void>;
  startServer: () => Promise<void>;
  stopServer: () => Promise<void>;
  healthCheck: () => Promise<void>;
  loadTools: () => Promise<void>;
  loadAudit: () => Promise<void>;
  loadThreads: () => Promise<void>;
  createThread: () => Promise<void>;
  selectThread: (threadId: string) => Promise<void>;
  deleteThread: (threadId: string) => Promise<void>;
  sendPrompt: (prompt: string) => Promise<void>;
  previewToolCall: (toolName: string, args: Record<string, unknown>) => Promise<void>;
  confirmPreview: () => Promise<McpCallResult | null>;
  cancelCall: () => Promise<void>;
  clearPreview: () => void;
  saveSettings: (next: McpSettings) => Promise<void>;
  loadContextPreview: (
    providerId: string | null,
    contentId: string | null,
    scope: ModelContextScope,
  ) => Promise<void>;
  generateReply: (
    request: Omit<AssistantGenerateRequest, "threadId">,
  ) => Promise<AssistantGenerateResult | null>;
}

const createMcpStoreState: StateCreator<McpStore> = (set, get) => {
  async function reloadThread() {
    const threadId = get().activeThread?.thread.id;
    if (!threadId || !isTauri()) return;
    const detail = await invoke<AssistantThreadDetail>("get_assistant_thread", { threadId });
    set({ activeThread: detail });
  }

  async function reloadOperationalState() {
    await Promise.all([get().loadThreads(), get().loadAudit()]);
    await reloadThread();
  }

  return {
    status: null,
    settings: null,
    tools: [],
    audit: [],
    security: null,
    threads: [],
    activeThread: null,
    activePreview: null,
    previewArguments: null,
    contextPreview: null,
    pendingCallId: "",
    loading: true,
    busyAction: "",
    error: "",
    initialized: false,

    initialize: async (force = false) => {
      if (get().initialized && !force) return;
      set({ initialized: true, loading: true, error: "" });
      try {
        if (!isTauri()) {
          set({
            status: demoStatus,
            settings: demoSettings,
            tools: demoTools,
            security: demoSecurity,
            threads: [demoThread],
            activeThread: {
              thread: demoThread,
              messages: [
                demoMessage(
                  "assistant",
                  "当前为 Web 演示模式。桌面应用会启动独立 stdio MCP Server，并通过 Core 服务执行工具；写入和外部发布会在执行前展示确认页。",
                ),
              ],
            },
          });
          return;
        }
        const [status, settings, tools, audit, security, threads] = await Promise.all([
          invoke<McpHostStatus>("get_mcp_host_status"),
          invoke<McpSettings>("get_mcp_settings"),
          invoke<McpToolDefinition[]>("mcp_list_tools"),
          invoke<McpAuditRecord[]>("list_mcp_audit", { limit: 50 }),
          invoke<McpSecurityEvidence>("mcp_security_self_test"),
          invoke<AssistantThread[]>("list_assistant_threads"),
        ]);
        let nextThreads = threads;
        if (nextThreads.length === 0) {
          const created = await invoke<AssistantThread>("create_assistant_thread", {
            title: null,
            providerId: null,
          });
          nextThreads = [created];
        }
        const activeId = get().activeThread?.thread.id ?? nextThreads[0].id;
        const detail = await invoke<AssistantThreadDetail>("get_assistant_thread", {
          threadId: activeId,
        });
        set({
          status,
          settings,
          tools,
          audit,
          security,
          threads: nextThreads,
          activeThread: detail,
        });
      } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        set({ error: message });
        useUiStore.getState().setError(message);
      } finally {
        set({ loading: false });
      }
    },

    refreshStatus: async () => {
      try {
        const status = isTauri() ? await invoke<McpHostStatus>("get_mcp_host_status") : demoStatus;
        set({ status });
      } catch (cause) {
        set({ error: String(cause) });
      }
    },

    startServer: async () => {
      set({ busyAction: "start" });
      try {
        const status = isTauri() ? await invoke<McpHostStatus>("start_mcp_server") : demoStatus;
        set({ status });
        await get().loadTools();
      } catch (cause) {
        const message = String(cause);
        set({ error: message });
        useUiStore.getState().showToast(message);
      } finally {
        set({ busyAction: "" });
      }
    },

    stopServer: async () => {
      set({ busyAction: "stop" });
      try {
        const status = isTauri()
          ? await invoke<McpHostStatus>("stop_mcp_server")
          : { ...demoStatus, state: "stopped" };
        set({ status, tools: [] });
      } catch (cause) {
        set({ error: String(cause) });
      } finally {
        set({ busyAction: "" });
      }
    },

    healthCheck: async () => {
      set({ busyAction: "health" });
      try {
        const status = isTauri() ? await invoke<McpHostStatus>("mcp_health_check") : demoStatus;
        set({ status });
        await get().loadTools();
        useUiStore.getState().showToast(`MCP Host 状态：${status.state}`);
      } catch (cause) {
        set({ error: String(cause) });
        useUiStore.getState().showToast(String(cause));
      } finally {
        set({ busyAction: "" });
      }
    },

    loadTools: async () => {
      if (!isTauri()) {
        set({ tools: demoTools });
        return;
      }
      try {
        set({ tools: await invoke<McpToolDefinition[]>("mcp_list_tools") });
      } catch (cause) {
        set({ error: String(cause), tools: [] });
      }
    },

    loadAudit: async () => {
      if (!isTauri()) return;
      try {
        set({ audit: await invoke<McpAuditRecord[]>("list_mcp_audit", { limit: 80 }) });
      } catch (cause) {
        set({ error: String(cause) });
      }
    },

    loadThreads: async () => {
      if (!isTauri()) {
        set({ threads: [demoThread] });
        return;
      }
      try {
        set({ threads: await invoke<AssistantThread[]>("list_assistant_threads") });
      } catch (cause) {
        set({ error: String(cause) });
      }
    },

    createThread: async () => {
      set({ busyAction: "thread" });
      try {
        if (!isTauri()) {
          const now = new Date().toISOString();
          const created: AssistantThread = {
            ...demoThread,
            id: `demo-thread-${Date.now()}-${Math.random().toString(16).slice(2, 6)}`,
            title: "新对话",
            createdAt: now,
            updatedAt: now,
          };
          set((state) => ({
            threads: [created, ...state.threads],
            activeThread: { thread: created, messages: [] },
          }));
          return;
        }
        const thread = await invoke<AssistantThread>("create_assistant_thread", {
          title: null,
          providerId: null,
        });
        const detail = await invoke<AssistantThreadDetail>("get_assistant_thread", {
          threadId: thread.id,
        });
        set((state) => ({
          threads: [thread, ...state.threads],
          activeThread: detail,
        }));
      } catch (cause) {
        set({ error: String(cause) });
      } finally {
        set({ busyAction: "" });
      }
    },

    selectThread: async (threadId) => {
      set({ busyAction: "thread" });
      try {
        if (!isTauri()) {
          const target = get().threads.find((item) => item.id === threadId) ?? demoThread;
          set({ activeThread: { thread: target, messages: [] } });
          return;
        }
        set({
          activeThread: await invoke<AssistantThreadDetail>("get_assistant_thread", {
            threadId,
          }),
        });
      } catch (cause) {
        set({ error: String(cause) });
      } finally {
        set({ busyAction: "" });
      }
    },

    deleteThread: async (threadId) => {
      set({ busyAction: "thread", error: "" });
      try {
        const wasActive = get().activeThread?.thread.id === threadId;
        if (!isTauri()) {
          let remaining = get().threads.filter((item) => item.id !== threadId);
          if (remaining.length === 0) {
            const now = new Date().toISOString();
            remaining = [
              {
                ...demoThread,
                id: `demo-thread-${Date.now()}-${Math.random().toString(16).slice(2, 6)}`,
                title: "新对话",
                createdAt: now,
                updatedAt: now,
              },
            ];
          }
          set((state) => ({
            threads: remaining,
            activeThread:
              wasActive || !state.activeThread
                ? { thread: remaining[0], messages: [] }
                : state.activeThread,
          }));
          useUiStore.getState().showToast("已删除任务会话");
          return;
        }
        await invoke("delete_assistant_thread", { threadId });
        let remaining = await invoke<AssistantThread[]>("list_assistant_threads");
        if (remaining.length === 0) {
          const created = await invoke<AssistantThread>("create_assistant_thread", {
            title: null,
            providerId: null,
          });
          remaining = [created];
        }
        const nextActive =
          wasActive || !get().activeThread
            ? await invoke<AssistantThreadDetail>("get_assistant_thread", {
                threadId: remaining[0].id,
              })
            : get().activeThread;
        set({ threads: remaining, activeThread: nextActive });
        useUiStore.getState().showToast("已删除任务会话");
      } catch (cause) {
        const message = String(cause);
        set({ error: message });
        useUiStore.getState().showToast(message);
      } finally {
        set({ busyAction: "" });
      }
    },

    sendPrompt: async (prompt) => {
      if (!prompt.trim()) return;
      set({ busyAction: "prompt" });
      try {
        if (!isTauri()) {
          const current = get().activeThread ?? { thread: demoThread, messages: [] };
          const user = demoMessage("user", prompt);
          const toolCall = planDemoTool(prompt);
          const assistant = demoMessage(
            "assistant",
            toolCall
              ? `我建议调用 ${toolCall.toolName}。执行前会先展示目标、参数与权限。`
              : "Web 演示模式未连接桌面 Core。请在 Tauri 应用中执行真实工具调用。",
            toolCall,
          );
          set({
            activeThread: {
              ...current,
              messages: [...current.messages, user, assistant],
            },
          });
          return;
        }
        const threadId = get().activeThread?.thread.id;
        if (!threadId) return;
        const plan = await invoke<{
          thread: AssistantThread;
          userMessage: AssistantMessage;
          assistantMessage: AssistantMessage;
        }>("assistant_plan_message", { prompt, threadId });
        set((state) => ({
          activeThread: state.activeThread
            ? {
                thread: plan.thread,
                messages: [...state.activeThread.messages, plan.userMessage, plan.assistantMessage],
              }
            : state.activeThread,
        }));
        await get().loadThreads();
      } catch (cause) {
        set({ error: String(cause) });
        useUiStore.getState().showToast(String(cause));
      } finally {
        set({ busyAction: "" });
      }
    },

    previewToolCall: async (toolName, args) => {
      set({ busyAction: "preview", error: "" });
      try {
        const threadId = get().activeThread?.thread.id ?? null;
        if (!isTauri()) {
          const tool = demoTools.find((item) => item.name === toolName);
          set({
            activePreview: {
              callId: `demo-${Date.now()}`,
              toolName,
              title: tool?.title ?? toolName,
              permission: tool?.permission ?? "read",
              targetSummary: "演示目标",
              parameterSummary: JSON.stringify(args),
              effects: ["Web 演示模式不会产生真实副作用"],
              details: args,
              requiresConfirmation: tool?.permission !== "read",
              authorizationToken: null,
              fieldErrors: [],
            },
            previewArguments: args,
          });
          return;
        }
        const preview = await invoke<McpAuthorizationPreview>("mcp_preview_tool_call", {
          request: { toolName, arguments: args, threadId },
        });
        set({ activePreview: preview, previewArguments: args });
      } catch (cause) {
        set({ error: String(cause) });
        useUiStore.getState().showToast(String(cause));
      } finally {
        set({ busyAction: "" });
      }
    },

    confirmPreview: async () => {
      const preview = get().activePreview;
      const args = get().previewArguments;
      if (!preview || !args) return null;
      if (preview.fieldErrors.length > 0) return null;
      set({ busyAction: "execute", pendingCallId: preview.callId });
      try {
        if (!isTauri()) {
          const result: McpCallResult = {
            callId: preview.callId,
            auditId: `demo-audit-${Date.now()}`,
            toolName: preview.toolName,
            permission: preview.permission,
            status: "success",
            contentKind: preview.contentKind,
            targetSummary: preview.targetSummary,
            parameterSummary: preview.parameterSummary,
            result: { demo: true, arguments: args },
            durationMs: 0,
          };
          set({ activePreview: null, previewArguments: null });
          const message = demoMessage(
            "assistant",
            `演示工具 ${preview.toolName} 已完成；Web 模式未产生真实副作用。`,
          );
          set((state) => ({
            activeThread: state.activeThread
              ? { ...state.activeThread, messages: [...state.activeThread.messages, message] }
              : state.activeThread,
          }));
          return result;
        }
        const result = await invoke<McpCallResult>("mcp_call_tool", {
          request: {
            callId: preview.callId,
            toolName: preview.toolName,
            arguments: args,
            threadId: get().activeThread?.thread.id ?? null,
            authorizationToken: preview.authorizationToken ?? null,
            timeoutMs: 30_000,
          },
        });
        set({ activePreview: null, previewArguments: null });
        await reloadOperationalState();
        useUiStore
          .getState()
          .showToast(
            result.status === "success" ? "工具调用完成" : `工具调用状态：${result.status}`,
          );
        return result;
      } catch (cause) {
        const message = String(cause);
        set({ error: message });
        useUiStore.getState().showToast(message);
        return null;
      } finally {
        set({ busyAction: "", pendingCallId: "" });
      }
    },

    cancelCall: async () => {
      const callId = get().pendingCallId;
      if (!callId) return;
      try {
        if (isTauri()) await invoke("mcp_cancel_tool", { callId });
        useUiStore.getState().showToast("已发送取消请求；不会重复执行");
      } catch (cause) {
        set({ error: String(cause) });
      }
    },

    clearPreview: () => set({ activePreview: null, previewArguments: null }),

    saveSettings: async (next) => {
      set({ busyAction: "settings" });
      try {
        const saved = isTauri()
          ? await invoke<McpSettings>("save_mcp_settings", { input: next })
          : next;
        set({ settings: saved });
        await get().refreshStatus();
        useUiStore.getState().showToast("MCP 权限与生命周期策略已保存");
      } catch (cause) {
        set({ error: String(cause) });
        useUiStore.getState().showToast(String(cause));
      } finally {
        set({ busyAction: "" });
      }
    },

    loadContextPreview: async (providerId, contentId, scope) => {
      try {
        const preview = isTauri()
          ? await invoke<AssistantContextPreview>("assistant_context_preview", {
              providerId,
              contentId,
              scope,
            })
          : null;
        set({ contextPreview: preview });
      } catch (cause) {
        set({ error: String(cause), contextPreview: null });
      }
    },

    generateReply: async (request) => {
      const threadId = get().activeThread?.thread.id;
      if (!threadId) return null;
      set({ busyAction: "generate", error: "" });
      try {
        if (!isTauri()) {
          const message = demoMessage(
            "assistant",
            "Web 演示模式不会向模型服务商发送请求。桌面应用会在发送前展示服务商、模型与实际上下文。",
          );
          set((state) => ({
            activeThread: state.activeThread
              ? { ...state.activeThread, messages: [...state.activeThread.messages, message] }
              : state.activeThread,
          }));
          return {
            assistantMessage: message,
            contextPreview: {
              providerName: "本地编排器",
              model: "deterministic-v1",
              context: "",
              characterCount: 0,
              includedFields: [],
              excludedFields: ["body"],
              redactionCount: 0,
            },
          } satisfies AssistantGenerateResult;
        }
        const result = await invoke<AssistantGenerateResult>("assistant_generate_reply", {
          request: { ...request, threadId },
        });
        await reloadOperationalState();
        return result;
      } catch (cause) {
        const message = String(cause);
        set({ error: message });
        useUiStore.getState().showToast(message);
        return null;
      } finally {
        set({ busyAction: "" });
      }
    },
  };
};

export const useMcpStore = create<McpStore>()(createMcpStoreState);
export const createMcpStore = () => createStore<McpStore>()(createMcpStoreState);
