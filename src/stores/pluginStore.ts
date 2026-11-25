import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { StateCreator } from "zustand";
import { createStore } from "zustand/vanilla";
import type {
  AdapterCatalogEntry,
  CapabilityLevel,
  CapabilityStateView,
  ContentKind,
  PluginEvent,
  PluginFaultResult,
  PluginPermissionDecision,
  PluginPermissionView,
  PluginProbeResult,
  PluginPublishValidationResult,
  PluginStatus,
  PluginSummary,
  PluginUninstallResult,
  PublishInput,
  PublishPayloadCheckResult,
  SecuritySelfTestResult,
} from "../types/plugin";

const contentKinds: Array<{ kind: ContentKind; label: string }> = [
  { kind: "video", label: "视频" },
  { kind: "imageText", label: "图文" },
  { kind: "imageArticle", label: "图文文章" },
  { kind: "article", label: "纯文章" },
];

function demoCapabilities(
  videoPublish: CapabilityLevel = "unsupported",
  imageTextPublish: CapabilityLevel = "supported",
): CapabilityStateView[] {
  return contentKinds.map(({ kind, label }) => {
    const declaredPublish =
      kind === "video"
        ? videoPublish
        : kind === "imageText"
          ? imageTextPublish
          : kind === "imageArticle"
            ? "restricted"
            : "supported";
    return {
      kind,
      label,
      declaredRead: "supported",
      declaredPublish,
      effectiveRead: "supported",
      effectivePublish: declaredPublish,
      source: "declared",
      reason: null,
      checkedAt: null,
    };
  });
}

function demoPermissions(granted?: boolean): PluginPermissionView[] {
  return [
    {
      id: "account.credentials",
      label: "账号会话代理",
      risk: "high",
      description: "仅获得 Core 签发的账号作用域引用，不能读取 Cookie、Token 或 Keychain 原文。",
      reason: "通过 Core 的账号会话代理验证发布账号作用域",
      required: true,
      granted,
    },
    {
      id: "network",
      label: "网络访问",
      risk: "medium",
      description: "仅通过 Core 网络代理发起已批准请求；插件进程默认不能直接联网。",
      reason: "通过 Core 网络代理发起测试发布请求",
      required: false,
      granted,
    },
    {
      id: "workspace.read",
      label: "读取工作区素材",
      risk: "high",
      description: "通过受控资产服务读取用户选择的文档/素材，不会获得任意工作区路径。",
      reason: "读取用户明确选择的发布素材",
      required: false,
      granted,
    },
    {
      id: "platform.publish",
      label: "平台发布",
      risk: "high",
      description: "调用已授权账号对应的发布能力；每次载荷仍需经过 Core 的形态与完整性校验。",
      reason: "执行已授权的图文/文章发布能力",
      required: true,
      granted,
    },
  ];
}

const demoCatalog: AdapterCatalogEntry[] = [
  {
    id: "creatorrelay.official.test-adapter",
    name: "CreatorRelay 测试适配器",
    description:
      "M4 公开合约测试适配器：模拟图文可用、视频不可用，并通过 Core 主机 API 验证权限边界。",
    version: "1.0.0",
    adapterContractVersion: 1,
    pluginType: "adapter",
    source: "BUNDLED",
    trustLevel: "OFFICIAL",
    permissions: demoPermissions(),
    capabilities: demoCapabilities(),
    installed: false,
    installable: true,
    blockedReason: null,
    isolation: {
      mode: "process-only",
      strong: false,
      network: "controlled",
      description: "浏览器预览不启动真实适配器进程。",
    },
  },
  {
    id: "creatorrelay.official.fault-adapter",
    name: "CreatorRelay 故障注入适配器",
    description: "用于验证崩溃、超时、错误展示与重启恢复的官方测试适配器。",
    version: "1.0.0",
    adapterContractVersion: 1,
    pluginType: "adapter",
    source: "BUNDLED",
    trustLevel: "OFFICIAL",
    permissions: demoPermissions(),
    capabilities: demoCapabilities("unsupported", "supported"),
    installed: false,
    installable: true,
    blockedReason: null,
    isolation: {
      mode: "process-only",
      strong: false,
      network: "controlled",
      description: "浏览器预览不启动真实适配器进程。",
    },
  },
  {
    id: "creatorrelay.official.probe-failure-adapter",
    name: "CreatorRelay 能力探测失败适配器",
    description:
      "Manifest 声明支持视频发布，但运行时探测失败；Core 必须显示 unavailable 而不是 supported。",
    version: "1.0.0",
    adapterContractVersion: 1,
    pluginType: "adapter",
    source: "BUNDLED",
    trustLevel: "OFFICIAL",
    permissions: demoPermissions(),
    capabilities: demoCapabilities("supported", "supported"),
    installed: false,
    installable: true,
    blockedReason: null,
    isolation: {
      mode: "process-only",
      strong: false,
      network: "controlled",
      description: "浏览器预览不启动真实适配器进程。",
    },
  },
];

function summaryFromCatalog(entry: AdapterCatalogEntry): PluginSummary {
  return {
    id: entry.id,
    name: entry.name,
    description: entry.description,
    version: entry.version,
    adapterContractVersion: entry.adapterContractVersion,
    pluginType: entry.pluginType,
    runtimeKind: "coreTestRunner",
    runtimeProfile: entry.id.includes("probe-failure")
      ? "probeFailure"
      : entry.id.includes("fault")
        ? "faulty"
        : "healthy",
    entry: `builtin://${entry.id}`,
    source: entry.source,
    trustLevel: entry.trustLevel,
    status: "REVIEW_REQUIRED",
    enabled: false,
    runtimeState: "stopped",
    sandboxed: false,
    isolation: entry.isolation,
    manifestHash: "browser-preview",
    installPath: "浏览器预览",
    installedAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    lastError: null,
    previousVersion: null,
    lastProbeAt: null,
    permissions: entry.permissions.map((permission) => ({ ...permission, granted: null })),
    capabilities: entry.capabilities.map((capability) => ({ ...capability })),
  };
}

function requiredPermissionsGranted(permissions: PluginPermissionView[]) {
  return permissions
    .filter((permission) => permission.required)
    .every((permission) => permission.granted === true);
}

function reviewComplete(permissions: PluginPermissionView[]) {
  return permissions.every((permission) => typeof permission.granted === "boolean");
}

function updatePlugin(
  plugins: PluginSummary[],
  pluginId: string,
  update: (plugin: PluginSummary) => PluginSummary,
) {
  return plugins.map((plugin) => (plugin.id === pluginId ? update(plugin) : plugin));
}

function demoProbe(plugin: PluginSummary, accountRef: string): PluginProbeResult {
  const probeFailure = plugin.runtimeProfile === "probeFailure";
  const checkedAt = new Date().toISOString();
  const capabilities = probeFailure
    ? plugin.capabilities.map((capability) => ({
        ...capability,
        effectiveRead: "unavailable" as const,
        effectivePublish: "unavailable" as const,
        source: "probe" as const,
        reason: "浏览器预览模拟运行时能力探测失败",
        checkedAt,
      }))
    : plugin.capabilities.map((capability) => ({
        ...capability,
        effectiveRead: capability.declaredRead,
        effectivePublish: capability.declaredPublish,
        source: "probe" as const,
        reason: capability.reason,
        checkedAt,
      }));
  return {
    pluginId: plugin.id,
    accountRef,
    ok: !probeFailure,
    error: probeFailure ? "PROBE_FAILED: 浏览器预览模拟能力探测失败" : null,
    checkedAt,
    sandboxed: false,
    hostAccess: [
      {
        method: "credentials.get",
        allowed:
          plugin.permissions.find((item) => item.id === "account.credentials")?.granted === true,
        code: null,
        detail: "浏览器预览不访问真实凭据",
      },
      {
        method: "credentials.readRaw",
        allowed: false,
        code: "FORBIDDEN_RAW_CREDENTIAL",
        detail: "插件不能读取原始凭据",
      },
      {
        method: "workspace.read",
        allowed: false,
        code: "PERMISSION_DENIED",
        detail: "浏览器预览不授予工作区访问",
      },
      {
        method: "network.request",
        allowed: false,
        code: "PERMISSION_DENIED",
        detail: "浏览器预览不发起网络请求",
      },
      {
        method: "database.query",
        allowed: false,
        code: "FORBIDDEN",
        detail: "插件不能访问 Core SQLite",
      },
    ],
    capabilities,
  };
}

export interface PluginStore {
  catalog: AdapterCatalogEntry[];
  plugins: PluginSummary[];
  loading: boolean;
  initialized: boolean;
  error: string;
  initialize: (force?: boolean) => Promise<void>;
  install: (pluginId: string) => Promise<PluginSummary>;
  savePermissions: (
    pluginId: string,
    decisions: PluginPermissionDecision[],
  ) => Promise<PluginSummary>;
  enable: (pluginId: string) => Promise<PluginSummary>;
  disable: (pluginId: string) => Promise<PluginSummary>;
  restart: (pluginId: string) => Promise<PluginSummary>;
  rollback: (pluginId: string) => Promise<PluginSummary>;
  uninstall: (pluginId: string, deletePluginData: boolean) => Promise<PluginUninstallResult>;
  probe: (pluginId: string, accountRef?: string) => Promise<PluginProbeResult>;
  simulateFault: (pluginId: string, fault: "crash" | "timeout") => Promise<PluginFaultResult>;
  runSecuritySelfTest: (pluginId: string) => Promise<SecuritySelfTestResult>;
  listEvents: (pluginId: string, limit?: number) => Promise<PluginEvent[]>;
  runPayloadSelfCheck: () => Promise<PublishPayloadCheckResult[]>;
  validatePublishInput: (
    pluginId: string,
    accountRef: string,
    input: PublishInput,
  ) => Promise<PluginPublishValidationResult>;
}

const createPluginStoreState: StateCreator<PluginStore> = (set, get) => ({
  catalog: demoCatalog,
  plugins: [],
  loading: false,
  initialized: false,
  error: "",

  initialize: async (force = false) => {
    if (get().initialized && !force) return;
    set({ loading: true, error: "" });
    if (!isTauri()) {
      set({
        catalog: demoCatalog.map((entry) => ({
          ...entry,
          installed: get().plugins.some((plugin) => plugin.id === entry.id),
        })),
        loading: false,
        initialized: true,
      });
      return;
    }
    try {
      const [catalog, plugins] = await Promise.all([
        invoke<AdapterCatalogEntry[]>("list_adapter_catalog"),
        invoke<PluginSummary[]>("list_plugins"),
      ]);
      set({ catalog, plugins, loading: false, initialized: true });
    } catch (cause) {
      set({
        loading: false,
        initialized: true,
        error: cause instanceof Error ? cause.message : String(cause),
      });
    }
  },

  install: async (pluginId) => {
    if (!isTauri()) {
      const entry = get().catalog.find((item) => item.id === pluginId);
      if (!entry) throw new Error("未找到官方适配器");
      const created = summaryFromCatalog(entry);
      set((state) => ({
        plugins: [...state.plugins, created],
        catalog: state.catalog.map((item) =>
          item.id === pluginId
            ? { ...item, installed: true, installable: false, blockedReason: "该官方适配器已安装" }
            : item,
        ),
      }));
      return created;
    }
    const created = await invoke<PluginSummary>("install_official_adapter", { pluginId });
    await get().initialize(true);
    return created;
  },

  savePermissions: async (pluginId, decisions) => {
    if (!isTauri()) {
      const plugin = get().plugins.find((item) => item.id === pluginId);
      if (!plugin) throw new Error("未找到插件");
      const nextPermissions = plugin.permissions.map((permission) => ({
        ...permission,
        granted:
          decisions.find((decision) => decision.id === permission.id)?.granted ??
          permission.granted ??
          null,
      }));
      const status: PluginStatus =
        reviewComplete(nextPermissions) && requiredPermissionsGranted(nextPermissions)
          ? "READY"
          : "REVIEW_REQUIRED";
      let updated: PluginSummary | null = null;
      set((state) => ({
        plugins: updatePlugin(state.plugins, pluginId, (item) => {
          updated = { ...item, permissions: nextPermissions, status };
          return updated;
        }),
      }));
      return updated as unknown as PluginSummary;
    }
    const updated = await invoke<PluginSummary>("save_plugin_permissions", {
      pluginId,
      decisions,
    });
    await get().initialize(true);
    return updated;
  },

  enable: async (pluginId) => {
    if (!isTauri()) {
      const plugin = get().plugins.find((item) => item.id === pluginId);
      if (!plugin) throw new Error("未找到插件");
      if (!reviewComplete(plugin.permissions) || !requiredPermissionsGranted(plugin.permissions)) {
        throw new Error("请先完成全部权限审查并授予必需权限");
      }
      if (plugin.runtimeProfile === "probeFailure") {
        const error = "PROBE_FAILED: 浏览器预览模拟能力探测失败";
        set((state) => ({
          plugins: updatePlugin(state.plugins, pluginId, (item) => ({
            ...item,
            status: "ERROR",
            lastError: error,
            capabilities: item.capabilities.map((capability) => ({
              ...capability,
              effectiveRead: "unavailable",
              effectivePublish: "unavailable",
              source: "probe",
              reason: error,
            })),
          })),
        }));
        throw new Error(error);
      }
      let updated: PluginSummary | null = null;
      set((state) => ({
        plugins: updatePlugin(state.plugins, pluginId, (item) => {
          updated = {
            ...item,
            status: "ENABLED",
            enabled: true,
            runtimeState: "running",
            lastError: null,
          };
          return updated;
        }),
      }));
      return updated as unknown as PluginSummary;
    }
    try {
      const updated = await invoke<PluginSummary>("enable_plugin", { pluginId });
      await get().initialize(true);
      return updated;
    } catch (cause) {
      await get().initialize(true);
      throw cause;
    }
  },

  disable: async (pluginId) => {
    if (!isTauri()) {
      let updated: PluginSummary | null = null;
      set((state) => ({
        plugins: updatePlugin(state.plugins, pluginId, (item) => {
          updated = { ...item, status: "DISABLED", enabled: false, runtimeState: "stopped" };
          return updated;
        }),
      }));
      return updated as unknown as PluginSummary;
    }
    const updated = await invoke<PluginSummary>("disable_plugin", { pluginId });
    await get().initialize(true);
    return updated;
  },

  restart: async (pluginId) => {
    if (!isTauri()) {
      const plugin = get().plugins.find((item) => item.id === pluginId);
      if (!plugin) throw new Error("未找到插件");
      let updated: PluginSummary | null = null;
      set((state) => ({
        plugins: updatePlugin(state.plugins, pluginId, (item) => {
          updated = {
            ...item,
            status: "ENABLED",
            enabled: true,
            runtimeState: "running",
            lastError: null,
          };
          return updated;
        }),
      }));
      return updated as unknown as PluginSummary;
    }
    try {
      const updated = await invoke<PluginSummary>("restart_plugin", { pluginId });
      await get().initialize(true);
      return updated;
    } catch (cause) {
      await get().initialize(true);
      throw cause;
    }
  },

  rollback: async (pluginId) => {
    if (!isTauri()) throw new Error("浏览器预览不执行版本回退");
    const updated = await invoke<PluginSummary>("rollback_plugin", { pluginId });
    await get().initialize(true);
    return updated;
  },

  uninstall: async (pluginId, deletePluginData) => {
    if (!isTauri()) {
      set((state) => ({
        plugins: state.plugins.filter((plugin) => plugin.id !== pluginId),
        catalog: state.catalog.map((item) =>
          item.id === pluginId
            ? { ...item, installed: false, installable: true, blockedReason: null }
            : item,
        ),
      }));
      return {
        pluginId,
        deletedPluginFiles: true,
        deletedPluginData: deletePluginData,
        retainedEvents: 0,
        message: "浏览器预览已移除适配器记录",
      };
    }
    const result = await invoke<PluginUninstallResult>("uninstall_plugin", {
      pluginId,
      deletePluginData,
    });
    await get().initialize(true);
    return result;
  },

  probe: async (pluginId, accountRef = "fixture-image-text") => {
    if (!isTauri()) {
      const plugin = get().plugins.find((item) => item.id === pluginId);
      if (!plugin) throw new Error("未找到插件");
      const result = demoProbe(plugin, accountRef);
      set((state) => ({
        plugins: updatePlugin(state.plugins, pluginId, (item) => ({
          ...item,
          capabilities: result.capabilities,
          lastProbeAt: result.checkedAt,
          status: result.ok ? item.status : "ERROR",
          lastError: result.error,
        })),
      }));
      return result;
    }
    const result = await invoke<PluginProbeResult>("probe_plugin_capabilities", {
      pluginId,
      accountRef,
    });
    await get().initialize(true);
    return result;
  },

  simulateFault: async (pluginId, fault) => {
    if (!isTauri()) {
      const error = fault === "crash" ? "Adapter 进程已退出或崩溃" : "Adapter 响应超时（预览模拟）";
      set((state) => ({
        plugins: updatePlugin(state.plugins, pluginId, (item) => ({
          ...item,
          status: "ERROR",
          lastError: error,
          runtimeState: "stopped",
        })),
      }));
      return {
        pluginId,
        fault,
        error,
        status: "ERROR",
        runtimeState: "stopped",
      };
    }
    const result = await invoke<PluginFaultResult>("simulate_plugin_fault", { pluginId, fault });
    await get().initialize(true);
    return result;
  },

  runSecuritySelfTest: async (pluginId) => {
    if (!isTauri()) {
      throw new Error("安全边界自检仅能在桌面应用运行");
    }
    return invoke<SecuritySelfTestResult>("run_plugin_security_self_test", { pluginId });
  },

  listEvents: async (pluginId, limit = 50) => {
    if (!isTauri()) return [];
    return invoke<PluginEvent[]>("list_plugin_events", { pluginId, limit });
  },

  runPayloadSelfCheck: async () => {
    if (!isTauri()) {
      return [
        {
          id: "preview",
          contentKind: "video",
          expectedOk: false,
          actualOk: false,
          passed: true,
          message: "浏览器预览",
        },
      ];
    }
    return invoke<PublishPayloadCheckResult[]>("run_publish_payload_self_check");
  },

  validatePublishInput: async (pluginId, accountRef, input) => {
    if (!isTauri()) {
      throw new Error("发布载荷预检仅能在桌面应用运行");
    }
    return invoke<PluginPublishValidationResult>("validate_plugin_publish_input", {
      pluginId,
      accountRef,
      input,
    });
  },
});

export const usePluginStore = create<PluginStore>()(createPluginStoreState);

/** Creates an isolated vanilla plugin store for unit tests. */
export const createPluginStore = () => createStore<PluginStore>()(createPluginStoreState);
