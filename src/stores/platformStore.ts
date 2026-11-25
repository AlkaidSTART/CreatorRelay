import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import { demoPlatformAccounts, PLATFORM_CATEGORIES } from "../data/demoData";
import type {
  AccountCheckResult,
  AuthChallenge,
  DeletionConfirmation,
  DeletionPreview,
  DeletionResult,
  PlatformAccountItem,
  PlatformCatalogEntry,
  PlatformCategory,
} from "../types/platform";

const bilibiliSteps = [
  "点击“打开登录页”，在当前系统浏览器完成平台登录；CreatorRelay 不读取浏览器主配置目录。",
  "登录后从平台请求中复制完整 Cookie 请求头；只复制自己账号的会话值。",
  "返回 CreatorRelay 粘贴 Cookie。后端会先调用平台身份接口校验，校验通过后才写入系统 Keychain。",
];

const demoCatalog: PlatformCatalogEntry[] = PLATFORM_CATEGORIES.flatMap((category) =>
  demoPlatformAccounts
    .filter((account) => account.category === category.id)
    .map((account) => ({
      id: account.platformId,
      name: account.platformName,
      category: account.category,
      categoryLabel: category.label,
      contentFormats: account.contentFormats,
      capabilities: account.capabilities,
      authMethods:
        account.platformId === "bilibili"
          ? [
              {
                method: "browser" as const,
                available: true,
                label: "系统浏览器 + 会话校验",
                loginUrl: "https://www.bilibili.com/",
                manualSteps: bilibiliSteps,
                reason: null,
              },
            ]
          : [
              {
                method: account.authMode,
                available: account.authMode !== "oauth",
                label: account.authMode === "oauth" ? "官方 OAuth" : "系统浏览器人工流程",
                loginUrl: null,
                manualSteps: [
                  `在系统浏览器打开 ${account.platformName} 并完成人工登录。`,
                  "平台创建、检查和发布操作在官方页面内人工完成。",
                  "人工登录不会被 CreatorRelay 标记为已验证会话。",
                ],
                reason:
                  account.authMode === "oauth"
                    ? "浏览器演示模式未配置官方 OAuth 客户端。"
                    : "桌面版才会执行系统凭据和会话校验。",
              },
            ],
    })),
);

function upsertAccount(
  accounts: PlatformAccountItem[],
  account: PlatformAccountItem,
): PlatformAccountItem[] {
  const index = accounts.findIndex((item) => item.id === account.id);
  if (index === -1) return [...accounts, account];
  return accounts.map((item) => (item.id === account.id ? account : item));
}

function demoChallenge(account: PlatformAccountItem): AuthChallenge {
  const platform = demoCatalog.find((item) => item.id === account.platformId);
  const method = platform?.authMethods.find((item) => item.method === "browser" && item.available);
  if (!method) {
    throw new Error("浏览器预览模式不执行平台身份校验，请在桌面应用中连接账号");
  }
  return {
    accountId: account.id,
    challengeId: `preview-${Date.now()}`,
    method: "browser",
    loginUrl: method.loginUrl ?? "",
    manualSteps: method.manualSteps,
    credentialLabel: "平台会话 Cookie",
    credentialHelp: "浏览器预览不会接收或保存 Cookie；请使用桌面应用完成真实校验。",
  };
}

export interface PlatformStore {
  catalog: PlatformCatalogEntry[];
  accounts: PlatformAccountItem[];
  categoryFilter: "ALL" | PlatformCategory;
  loading: boolean;
  initialized: boolean;
  error: string;
  initialize: (force?: boolean) => Promise<void>;
  setCategoryFilter: (filter: "ALL" | PlatformCategory) => void;
  createAccount: (platformId: string, alias: string) => Promise<PlatformAccountItem>;
  updateAccount: (
    accountId: string,
    changes: { alias?: string; enabled?: boolean; isDefault?: boolean },
  ) => Promise<PlatformAccountItem>;
  beginAuthentication: (accountId: string) => Promise<AuthChallenge>;
  cancelAuthentication: (accountId: string) => Promise<PlatformAccountItem>;
  completeAuthentication: (
    accountId: string,
    challengeId: string,
    credential: string,
  ) => Promise<AccountCheckResult>;
  checkStatus: (accountId: string) => Promise<AccountCheckResult>;
  ensureActive: (accountId: string) => Promise<PlatformAccountItem>;
  previewDeletion: (accountId: string) => Promise<DeletionPreview>;
  deleteAccount: (accountId: string, confirmation: DeletionConfirmation) => Promise<DeletionResult>;
}

const createPlatformStoreState: StateCreator<PlatformStore> = (set, get) => ({
  catalog: demoCatalog,
  accounts: demoPlatformAccounts,
  categoryFilter: "ALL",
  loading: false,
  initialized: false,
  error: "",

  initialize: async (force = false) => {
    if (get().initialized && !force) return;
    set({ loading: true, error: "" });
    if (!isTauri()) {
      set({
        catalog: demoCatalog,
        accounts: demoPlatformAccounts,
        loading: false,
        initialized: true,
      });
      return;
    }
    try {
      const [catalog, accounts] = await Promise.all([
        invoke<PlatformCatalogEntry[]>("list_platform_catalog"),
        invoke<PlatformAccountItem[]>("list_platform_accounts"),
      ]);
      set({ catalog, accounts, loading: false, initialized: true });
    } catch (error) {
      set({
        loading: false,
        initialized: true,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },

  setCategoryFilter: (categoryFilter) => set({ categoryFilter }),

  createAccount: async (platformId, alias) => {
    if (!isTauri()) {
      const platform = get().catalog.find((item) => item.id === platformId);
      if (!platform) throw new Error("不支持该平台");
      const created: PlatformAccountItem = {
        id: `preview-${Date.now()}`,
        platformId,
        platformName: platform.name,
        category: platform.category,
        contentFormats: platform.contentFormats,
        authMode:
          platform.authMethods.find((method) => method.available)?.method === "oauth"
            ? "oauth"
            : platform.id === "bilibili"
              ? "browser"
              : "manual",
        alias: alias.trim(),
        nickname: "",
        uid: "",
        status: "UNAUTHORIZED",
        enabled: true,
        isDefault: false,
        lastError: null,
        capabilities: platform.capabilities,
      };
      set((state) => ({ accounts: [...state.accounts, created] }));
      return created;
    }
    const created = await invoke<PlatformAccountItem>("create_platform_account", {
      platformId,
      alias,
    });
    set((state) => ({ accounts: [...state.accounts, created] }));
    return created;
  },

  updateAccount: async (accountId, changes) => {
    if (!isTauri()) {
      const existing = get().accounts.find((account) => account.id === accountId);
      if (!existing) throw new Error("未找到平台账号");
      const updated: PlatformAccountItem = {
        ...existing,
        alias: changes.alias ?? existing.alias,
        enabled: changes.enabled ?? existing.enabled,
        isDefault: changes.isDefault ?? existing.isDefault,
        status: changes.enabled === false ? "DISABLED" : existing.status,
      };
      set((state) => ({ accounts: upsertAccount(state.accounts, updated) }));
      return updated;
    }
    const updated = await invoke<PlatformAccountItem>("update_platform_account", {
      accountId,
      alias: changes.alias,
      enabled: changes.enabled,
      isDefault: changes.isDefault,
    });
    set((state) => ({ accounts: upsertAccount(state.accounts, updated) }));
    return updated;
  },

  beginAuthentication: async (accountId) => {
    if (!isTauri()) {
      const account = get().accounts.find((item) => item.id === accountId);
      if (!account) throw new Error("未找到平台账号");
      return demoChallenge(account);
    }
    const challenge = await invoke<AuthChallenge>("begin_platform_authentication", { accountId });
    set((state) => ({
      accounts: state.accounts.map((account) =>
        account.id === accountId
          ? { ...account, status: "AUTHENTICATING", lastError: null }
          : account,
      ),
    }));
    return challenge;
  },

  cancelAuthentication: async (accountId) => {
    if (!isTauri()) {
      const account = get().accounts.find((item) => item.id === accountId);
      if (!account) throw new Error("未找到平台账号");
      const restored: PlatformAccountItem = {
        ...account,
        status: account.uid ? "NEEDS_REAUTH" : "UNAUTHORIZED",
        lastError: null,
      };
      set((state) => ({ accounts: upsertAccount(state.accounts, restored) }));
      return restored;
    }
    const account = await invoke<PlatformAccountItem>("cancel_platform_authentication", {
      accountId,
    });
    set((state) => ({ accounts: upsertAccount(state.accounts, account) }));
    return account;
  },

  completeAuthentication: async (accountId, challengeId, credential) => {
    if (!isTauri()) {
      throw new Error("浏览器预览不会接收或保存 Cookie，请使用桌面应用完成真实身份校验");
    }
    const result = await invoke<AccountCheckResult>("complete_platform_authentication", {
      accountId,
      challengeId,
      credential,
    });
    set((state) => ({ accounts: upsertAccount(state.accounts, result.account) }));
    return result;
  },

  checkStatus: async (accountId) => {
    if (!isTauri()) {
      const account = get().accounts.find((item) => item.id === accountId);
      if (!account) throw new Error("未找到平台账号");
      return {
        account,
        message: "浏览器预览不执行真实平台校验；请在桌面应用中检测会话",
      };
    }
    const result = await invoke<AccountCheckResult>("check_platform_account_status", {
      accountId,
    });
    set((state) => ({ accounts: upsertAccount(state.accounts, result.account) }));
    return result;
  },

  ensureActive: async (accountId) => {
    if (!isTauri()) {
      const account = get().accounts.find((item) => item.id === accountId);
      if (!account || account.status !== "ACTIVE") {
        throw new Error("账号当前不是 ACTIVE 状态，无法执行依赖该账号的任务");
      }
      return account;
    }
    return invoke<PlatformAccountItem>("ensure_platform_account_active", { accountId });
  },

  previewDeletion: async (accountId) => {
    if (!isTauri()) {
      const account = get().accounts.find((item) => item.id === accountId);
      if (!account) throw new Error("未找到平台账号");
      return {
        accountId,
        platformName: account.platformName,
        alias: account.alias,
        willDeleteCredentials: true,
        willDeleteSessionProfile: `浏览器预览 · ${account.profileId ?? "独立会话容器"}`,
        retainedHistoryCount: 0,
        retainedData: ["本地主稿与运营历史不会因删除账号而被删除"],
      };
    }
    return invoke<DeletionPreview>("preview_platform_account_deletion", { accountId });
  },

  deleteAccount: async (accountId, confirmation) => {
    if (!isTauri()) {
      if (
        !confirmation.credentials ||
        !confirmation.sessions ||
        !confirmation.historyRetainedUnderstood
      ) {
        throw new Error("请确认凭据删除、会话清理和历史作品保留范围");
      }
      const account = get().accounts.find((item) => item.id === accountId);
      if (!account) throw new Error("未找到平台账号");
      set((state) => ({ accounts: state.accounts.filter((item) => item.id !== accountId) }));
      return {
        accountId,
        deletedCredentials: true,
        deletedSessionProfile: true,
        retainedHistoryCount: 0,
        message: `已删除 ${account.platformName} · ${account.alias}`,
      };
    }
    const result = await invoke<DeletionResult>("delete_platform_account", {
      accountId,
      confirmation,
    });
    set((state) => ({ accounts: state.accounts.filter((item) => item.id !== accountId) }));
    return result;
  },
});

export const usePlatformStore = create<PlatformStore>()(createPlatformStoreState);

/** Creates an isolated vanilla platform store for unit tests. */
export const createPlatformStore = () => createStore<PlatformStore>()(createPlatformStoreState);
