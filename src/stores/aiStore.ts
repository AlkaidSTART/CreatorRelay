import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import {
  AI_OPERATIONS,
  type AiGenerateRequest,
  type AiGenerateResult,
  type AiProvider,
  type AiProviderInput,
  type AiTestResult,
  type AiOperation,
} from "../types/ai";
import type { ContentKind } from "../types/plugin";
import type { ContentAsset } from "../types/content";
import { useUiStore } from "./uiStore";

export interface AiStore {
  providers: AiProvider[];
  loading: boolean;
  initialized: boolean;
  busyAction: string;
  initialize: (force?: boolean) => Promise<void>;
  saveProvider: (input: AiProviderInput) => Promise<AiProvider | null>;
  deleteProvider: (providerId: string) => Promise<void>;
  setDefaultProvider: (providerId: string) => Promise<void>;
  testProvider: (providerId: string) => Promise<AiTestResult | null>;
  defaultProvider: () => AiProvider | null;
  generate: (request: AiGenerateRequest) => Promise<AiGenerateResult | null>;
}

function demoProvider(): AiProvider {
  return {
    id: "demo-openai",
    name: "OpenAI Compatible",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-4.1-mini",
    apiStyle: "openai-compatible",
    isDefault: true,
    configured: true,
    lastTestStatus: "ok",
    lastTestError: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
}

function demoGenerate(request: AiGenerateRequest): AiGenerateResult {
  const input = request.context.trim() || request.title.trim();
  const excerpt = input.slice(0, 60) || "当前内容";
  let content = excerpt;
  if (request.operation === "title") content = `关于${excerpt.slice(0, 12)}的新视角`;
  if (request.operation === "description")
    content = `本期视频围绕“${excerpt.slice(0, 20)}”展开，梳理关键背景与实践取舍。`;
  if (request.operation === "shortCopy") content = `今天分享：${excerpt.slice(0, 28)}。`;
  if (request.operation === "summary")
    content = `本文围绕 ${excerpt.slice(0, 24)} 展开，总结核心观点与可执行建议。`;
  if (request.operation === "contentCheck")
    content = "1. 标题与正文一致。\n2. 建议补充素材数量说明。";
  return {
    requestId: `demo-${crypto.randomUUID()}`,
    providerId: request.providerId,
    providerName: "浏览器演示模型",
    model: "demo-model",
    operation: request.operation,
    content,
  };
}

const createAiStoreState: StateCreator<AiStore> = (set, get) => ({
  providers: [],
  loading: false,
  initialized: false,
  busyAction: "",

  initialize: async (force = false) => {
    if (get().initialized && !force) return;
    set({ initialized: true, loading: true });
    try {
      if (isTauri()) {
        const providers = await invoke<AiProvider[]>("list_ai_providers");
        set({ providers });
      } else {
        set({ providers: [demoProvider()] });
      }
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      set({ loading: false });
    }
  },

  saveProvider: async (input) => {
    set({ busyAction: "save" });
    try {
      let provider: AiProvider;
      if (isTauri()) {
        provider = await invoke<AiProvider>("save_ai_provider", { input });
      } else {
        const existing = get().providers.find((item) => item.id === input.id);
        provider = {
          id: input.id ?? `demo-${crypto.randomUUID()}`,
          name: input.name,
          baseUrl: input.baseUrl,
          model: input.model,
          apiStyle: input.apiStyle ?? "openai-compatible",
          isDefault: input.isDefault ?? existing?.isDefault ?? get().providers.length === 0,
          configured: Boolean(input.apiKey) || Boolean(existing?.configured),
          lastTestStatus: null,
          lastTestError: null,
          createdAt: existing?.createdAt ?? new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        };
      }
      await get().initialize(true);
      if (provider.isDefault) await get().setDefaultProvider(provider.id);
      useUiStore.getState().showToast("AI 供应商配置已保存");
      return get().providers.find((item) => item.id === provider.id) ?? provider;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    } finally {
      set({ busyAction: "" });
    }
  },

  deleteProvider: async (providerId) => {
    set({ busyAction: `delete:${providerId}` });
    try {
      if (isTauri()) await invoke("delete_ai_provider", { providerId });
      set((state) => ({ providers: state.providers.filter((item) => item.id !== providerId) }));
      useUiStore.getState().showToast("AI 供应商已删除，凭据已从钥匙串清理");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      set({ busyAction: "" });
    }
  },

  setDefaultProvider: async (providerId) => {
    set({ busyAction: `default:${providerId}` });
    try {
      if (isTauri()) await invoke("set_default_ai_provider", { providerId });
      set((state) => ({
        providers: state.providers.map((item) => ({ ...item, isDefault: item.id === providerId })),
      }));
      useUiStore.getState().showToast("已切换默认 AI 模型");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      set({ busyAction: "" });
    }
  },

  testProvider: async (providerId) => {
    set({ busyAction: `test:${providerId}` });
    try {
      let result: AiTestResult;
      if (isTauri()) {
        result = await invoke<AiTestResult>("test_ai_provider", { providerId });
      } else {
        result = {
          providerId,
          ok: false,
          message: "浏览器预览不连接真实模型。请在桌面应用配置 API Key 后测试。",
          latencyMs: 0,
          status: "failed",
        };
      }
      await get().initialize(true);
      return result;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    } finally {
      set({ busyAction: "" });
    }
  },

  defaultProvider: () =>
    get().providers.find((provider) => provider.isDefault) ?? get().providers[0] ?? null,

  generate: async (request) => {
    set({ busyAction: "generate" });
    try {
      const result = isTauri()
        ? await invoke<AiGenerateResult>("ai_generate", { request })
        : demoGenerate(request);
      return result;
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
      return null;
    } finally {
      set({ busyAction: "" });
    }
  },
});

export const useAiStore = create<AiStore>()(createAiStoreState);
export const createAiStore = () => createStore<AiStore>()(createAiStoreState);

export function operationsForKind(kind: ContentKind) {
  return AI_OPERATIONS.filter((operation) => operation.kinds.includes(kind));
}

export function buildAiMetadata(media: ContentAsset[]) {
  return media.map((asset) => ({
    kind: asset.kind,
    fileName: asset.originalName,
    mimeType: asset.mimeType,
    byteSize: asset.byteSize,
    caption: asset.caption,
    order: asset.order,
  }));
}

export function aiOperationTarget(operation: AiOperation): "title" | "summary" | "body" {
  return AI_OPERATIONS.find((item) => item.operation === operation)?.target ?? "body";
}
