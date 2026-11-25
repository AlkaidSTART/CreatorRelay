import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  KeyRound,
  Pencil,
  Plus,
  RefreshCw,
  ShieldAlert,
  Trash2,
  X,
} from "../components/Icons";
import { PageHeader } from "../components/PageHeader";
import { PlatformLogo } from "../components/PlatformLogo";
import { PLATFORM_CATEGORIES } from "../data/demoData";
import { usePlatformStore, useUiStore } from "../stores";
import type {
  AccountStatus,
  AuthChallenge,
  DeletionPreview,
  PlatformAccountItem,
  PlatformAuthMethod,
  PlatformCatalogEntry,
  PlatformCategory,
} from "../types/platform";

const accountStatusLabel: Record<AccountStatus, { label: string; className: string }> = {
  ACTIVE: { label: "会话有效", className: "bg-mint-bg text-mint-text" },
  AUTHENTICATING: { label: "验证中", className: "bg-lemon-bg text-lemon-text" },
  EXPIRED: { label: "会话已过期", className: "bg-coral-bg text-coral-text" },
  NEEDS_REAUTH: { label: "需重新授权", className: "bg-lemon-bg text-lemon-text" },
  UNAUTHORIZED: { label: "未登录", className: "bg-sidebar text-ink-secondary" },
  ERROR: { label: "校验失败", className: "bg-coral-bg text-coral-text" },
  DISABLED: { label: "已禁用", className: "bg-sidebar text-ink-muted" },
};

const capabilityLabel: Record<string, string> = {
  supported: "已验证支持",
  restricted: "受限 / 人工",
  experimental: "实验性",
  unsupported: "不支持自动化",
};

const authMethodLabel: Record<string, string> = {
  browser: "系统浏览器",
  oauth: "官方 OAuth",
  manual: "人工流程",
  embeddedWebView: "内置 WebView",
};

const categoryFilterOptions: Array<{ id: "ALL" | PlatformCategory; label: string }> = [
  { id: "ALL", label: "全部类型" },
  ...PLATFORM_CATEGORIES.map((category) => ({ id: category.id, label: category.label })),
];

type EditState = {
  accountId: string;
  alias: string;
  enabled: boolean;
  isDefault: boolean;
};

type AuthState = {
  accountId: string;
  challenge: AuthChallenge;
};

type ManualPrompt = {
  title: string;
  steps: string[];
  loginUrl?: string | null;
};

type DeleteState = {
  accountId: string;
  preview: DeletionPreview;
  credentials: boolean;
  sessions: boolean;
  historyRetainedUnderstood: boolean;
};

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function formatCheckedAt(value?: string | null) {
  if (!value) return "尚未检测";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function usableAuthMethod(platform?: PlatformCatalogEntry): PlatformAuthMethod | undefined {
  return platform?.authMethods.find((method) => method.available && method.method === "browser");
}

function ModalShell({
  children,
  onClose,
  width = "560px",
  label,
}: {
  children: React.ReactNode;
  onClose: () => void;
  width?: string;
  label: string;
}) {
  return (
    <div
      className="fixed inset-0 z-50 grid place-items-center bg-black/30 p-4 backdrop-blur-[2px]"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      role="presentation"
    >
      <section
        role="dialog"
        aria-modal="true"
        aria-label={label}
        className="relative max-h-[min(760px,calc(100vh-32px))] w-full overflow-y-auto rounded-2xl border border-line bg-surface-soft p-6 shadow-[0_20px_60px_rgba(51,43,41,0.2)]"
        style={{ maxWidth: width }}
      >
        {children}
      </section>
    </div>
  );
}

function CloseButton({ onClick, label = "关闭" }: { onClick: () => void; label?: string }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className="absolute top-4 right-4 grid h-7 w-7 cursor-pointer place-items-center rounded-lg text-ink-muted transition-colors hover:bg-sidebar hover:text-ink"
    >
      <X size={14} />
    </button>
  );
}

export function PlatformsPage() {
  const showToast = useUiStore((state) => state.showToast);
  const catalog = usePlatformStore((state) => state.catalog);
  const accounts = usePlatformStore((state) => state.accounts);
  const categoryFilter = usePlatformStore((state) => state.categoryFilter);
  const setCategoryFilter = usePlatformStore((state) => state.setCategoryFilter);
  const initialize = usePlatformStore((state) => state.initialize);
  const createAccount = usePlatformStore((state) => state.createAccount);
  const updateAccount = usePlatformStore((state) => state.updateAccount);
  const beginAuthentication = usePlatformStore((state) => state.beginAuthentication);
  const cancelAuthentication = usePlatformStore((state) => state.cancelAuthentication);
  const completeAuthentication = usePlatformStore((state) => state.completeAuthentication);
  const checkStatus = usePlatformStore((state) => state.checkStatus);
  const previewDeletion = usePlatformStore((state) => state.previewDeletion);
  const deleteAccount = usePlatformStore((state) => state.deleteAccount);

  const [addPlatformId, setAddPlatformId] = useState<string | null>(null);
  const [addAlias, setAddAlias] = useState("");
  const [editState, setEditState] = useState<EditState | null>(null);
  const [authState, setAuthState] = useState<AuthState | null>(null);
  const [credential, setCredential] = useState("");
  const [authError, setAuthError] = useState("");
  const [authBusy, setAuthBusy] = useState(false);
  const [manualPrompt, setManualPrompt] = useState<ManualPrompt | null>(null);
  const [deleteState, setDeleteState] = useState<DeleteState | null>(null);
  const [busyAccountId, setBusyAccountId] = useState("");

  useEffect(() => {
    void initialize();
  }, [initialize]);

  const visibleGroups = useMemo(
    () =>
      PLATFORM_CATEGORIES.filter(
        (category) => categoryFilter === "ALL" || category.id === categoryFilter,
      )
        .map((category) => ({
          ...category,
          accounts: accounts.filter((account) => account.category === category.id),
        }))
        .filter((group) => group.accounts.length > 0),
    [accounts, categoryFilter],
  );

  const visibleCatalog = useMemo(
    () =>
      catalog.filter(
        (platform) => categoryFilter === "ALL" || platform.category === categoryFilter,
      ),
    [catalog, categoryFilter],
  );

  const selectedPlatform = catalog.find((platform) => platform.id === addPlatformId);
  const authAccount = accounts.find((account) => account.id === authState?.accountId);

  async function handleOpenExternal(url?: string | null) {
    if (!url) return;
    try {
      if (isTauri()) {
        await openUrl(url);
      } else {
        window.open(url, "_blank", "noopener,noreferrer");
      }
    } catch (error) {
      showToast(`无法打开登录页：${errorMessage(error)}`);
    }
  }

  async function handleCreate(event: React.FormEvent) {
    event.preventDefault();
    if (!selectedPlatform || !addAlias.trim()) return;
    try {
      await createAccount(selectedPlatform.id, addAlias);
      showToast(`已添加 ${selectedPlatform.name} 账号，请完成身份校验`);
      setAddPlatformId(null);
      setAddAlias("");
    } catch (error) {
      showToast(errorMessage(error));
    }
  }

  function openEdit(account: PlatformAccountItem) {
    setEditState({
      accountId: account.id,
      alias: account.alias,
      enabled: account.enabled !== false,
      isDefault: account.isDefault === true,
    });
  }

  async function handleUpdate(event: React.FormEvent) {
    event.preventDefault();
    if (!editState) return;
    setBusyAccountId(editState.accountId);
    try {
      await updateAccount(editState.accountId, {
        alias: editState.alias,
        enabled: editState.enabled,
        isDefault: editState.isDefault,
      });
      showToast("账号设置已更新");
      setEditState(null);
    } catch (error) {
      showToast(errorMessage(error));
    } finally {
      setBusyAccountId("");
    }
  }

  async function handleCheck(account: PlatformAccountItem) {
    setBusyAccountId(account.id);
    try {
      const result = await checkStatus(account.id);
      showToast(result.message);
    } catch (error) {
      showToast(errorMessage(error));
    } finally {
      setBusyAccountId("");
    }
  }

  async function handleBeginAuth(account: PlatformAccountItem) {
    const platform = catalog.find((item) => item.id === account.platformId);
    const method = usableAuthMethod(platform);
    if (!method) {
      const manual = platform?.authMethods.find(
        (item) => item.available && item.method === "manual",
      );
      setManualPrompt({
        title: `${account.platformName} · 人工流程`,
        steps: manual?.manualSteps ??
          platform?.authMethods.flatMap((item) => item.manualSteps) ?? [
            "该平台尚未提供可验证的会话流程。",
          ],
        loginUrl: manual?.loginUrl,
      });
      return;
    }
    setBusyAccountId(account.id);
    try {
      const challenge = await beginAuthentication(account.id);
      setCredential("");
      setAuthError("");
      setAuthState({ accountId: account.id, challenge });
    } catch (error) {
      showToast(errorMessage(error));
    } finally {
      setBusyAccountId("");
    }
  }

  async function closeAuth() {
    const current = authState;
    setAuthState(null);
    setCredential("");
    setAuthError("");
    if (!current) return;
    try {
      await cancelAuthentication(current.accountId);
    } catch {
      await initialize(true);
    }
  }

  async function handleCompleteAuth(event: React.FormEvent) {
    event.preventDefault();
    if (!authState || !credential.trim()) return;
    setAuthBusy(true);
    setAuthError("");
    try {
      const result = await completeAuthentication(
        authState.accountId,
        authState.challenge.challengeId,
        credential,
      );
      setCredential("");
      setAuthState(null);
      showToast(result.message);
    } catch (error) {
      setCredential("");
      setAuthError(errorMessage(error));
      await initialize(true);
    } finally {
      setAuthBusy(false);
    }
  }

  async function handlePreviewDelete(account: PlatformAccountItem) {
    setBusyAccountId(account.id);
    try {
      const preview = await previewDeletion(account.id);
      setDeleteState({
        accountId: account.id,
        preview,
        credentials: false,
        sessions: false,
        historyRetainedUnderstood: false,
      });
    } catch (error) {
      showToast(errorMessage(error));
    } finally {
      setBusyAccountId("");
    }
  }

  async function handleDelete() {
    if (!deleteState) return;
    setBusyAccountId(deleteState.accountId);
    try {
      const result = await deleteAccount(deleteState.accountId, {
        credentials: deleteState.credentials,
        sessions: deleteState.sessions,
        historyRetainedUnderstood: deleteState.historyRetainedUnderstood,
      });
      showToast(result.message);
      setDeleteState(null);
    } catch (error) {
      showToast(errorMessage(error));
    } finally {
      setBusyAccountId("");
    }
  }

  return (
    <main className="flex h-full min-w-0 flex-col bg-canvas">
      <PageHeader
        eyebrow="PLATFORMS"
        title="平台账号"
        description="管理平台账号、真实会话校验、系统凭据与隔离会话容器；未验证身份不会显示为已连接。"
        actions={
          <button
            className="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3 text-xs font-medium text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.24)] transition-colors hover:bg-accent-hover"
            onClick={() => {
              setAddPlatformId(catalog[0]?.id ?? null);
              setAddAlias("");
            }}
          >
            <Plus size={13} />
            接入平台账号
          </button>
        }
      />

      <div className="min-h-0 flex-1 space-y-7 overflow-y-auto px-7 py-6">
        <div className="flex flex-wrap gap-1.5" role="tablist" aria-label="平台类型筛选">
          {categoryFilterOptions.map((option) => {
            const count =
              option.id === "ALL"
                ? accounts.length
                : accounts.filter((account) => account.category === option.id).length;
            const selected = categoryFilter === option.id;
            return (
              <button
                key={option.id}
                role="tab"
                aria-selected={selected}
                onClick={() => setCategoryFilter(option.id)}
                className={`inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border px-3 text-xs font-medium transition-colors ${
                  selected
                    ? "border-accent bg-accent text-accent-on shadow-[0_2px_6px_rgba(123,104,168,0.22)]"
                    : "border-line bg-surface text-ink-secondary hover:bg-subtle"
                }`}
              >
                <span>{option.label}</span>
                <span
                  className={`rounded-full px-1.5 text-[10px] ${
                    selected ? "bg-surface/20 text-white" : "bg-sidebar text-ink-muted"
                  }`}
                >
                  {count}
                </span>
              </button>
            );
          })}
        </div>

        {accounts.length === 0 ? (
          <section className="rounded-2xl border border-dashed border-line-strong bg-surface p-10 text-center">
            <div className="mx-auto grid h-11 w-11 place-items-center rounded-2xl bg-accent-soft text-accent-deep">
              <KeyRound size={19} />
            </div>
            <h2 className="mt-3 text-sm font-semibold text-ink">还没有平台账号</h2>
            <p className="mx-auto mt-1 max-w-lg text-xs leading-relaxed text-ink-secondary">
              添加账号后，CreatorRelay 会先验证平台返回的真实 UID/昵称，再把会话写入系统 Keychain。
            </p>
          </section>
        ) : (
          <div className="space-y-6">
            {visibleGroups.map((group) => (
              <section key={group.id} className="space-y-3">
                <div className="flex flex-wrap items-baseline justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <h2 className="text-sm font-semibold text-ink">{group.label}</h2>
                    <span className="rounded-md border border-line bg-surface px-2 py-0.5 text-[11px] font-medium text-ink-secondary">
                      {group.accounts.length} 个账号
                    </span>
                  </div>
                  <p className="text-xs text-ink-muted">{group.description}</p>
                </div>

                <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
                  {group.accounts.map((account) => {
                    const status = accountStatusLabel[account.status];
                    const disabled = account.enabled === false;
                    const busy = busyAccountId === account.id;
                    const verifiedIdentity = account.nickname || account.uid;
                    return (
                      <article
                        key={account.id}
                        className="flex flex-col gap-4 rounded-2xl border border-line bg-surface p-5 shadow-[0_2px_12px_rgba(74,58,48,0.03)]"
                      >
                        <div className="flex items-start justify-between gap-3">
                          <div className="flex min-w-0 items-center gap-3">
                            <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl border border-line bg-white shadow-[0_1px_2px_rgba(35,28,25,0.04)]">
                              <PlatformLogo
                                platformId={account.platformId}
                                platformName={account.platformName}
                                size={28}
                              />
                            </div>
                            <div className="min-w-0">
                              <div className="flex flex-wrap items-center gap-1.5">
                                <h3 className="truncate text-sm font-semibold text-ink">
                                  {account.platformName} · {account.alias}
                                </h3>
                                {account.isDefault && (
                                  <span className="rounded-md bg-accent-soft px-1.5 py-0.5 text-[10px] font-medium text-accent-deep">
                                    默认
                                  </span>
                                )}
                              </div>
                              <small className="block truncate text-[11px] text-ink-muted">
                                {verifiedIdentity
                                  ? `已验证：${account.nickname || "未提供昵称"} · UID ${account.uid}`
                                  : "尚未验证平台身份"}
                              </small>
                              <small className="block truncate text-[11px] text-ink-muted">
                                认证方式：{authMethodLabel[account.authMode] ?? account.authMode} ·
                                检测：{formatCheckedAt(account.lastCheckedAt)}
                              </small>
                            </div>
                          </div>
                          <span
                            className={`inline-flex shrink-0 items-center gap-1 rounded-lg px-2 py-0.5 text-[11px] font-medium ${status.className}`}
                          >
                            {account.status === "ACTIVE" ? (
                              <CheckCircle2 size={11} />
                            ) : (
                              <ShieldAlert size={11} />
                            )}
                            {status.label}
                          </span>
                        </div>

                        {account.lastError && (
                          <p className="rounded-xl bg-coral-bg px-3 py-2 text-[11px] leading-relaxed text-coral-text">
                            {account.lastError}
                          </p>
                        )}

                        <div className="flex flex-wrap gap-1.5">
                          <span className="rounded-lg border border-line bg-canvas px-2 py-0.5 text-[11px] text-ink-secondary">
                            作品读取：{capabilityLabel[account.capabilities.contentRead]}
                          </span>
                          <span className="rounded-lg border border-line bg-canvas px-2 py-0.5 text-[11px] text-ink-secondary">
                            发布能力：{capabilityLabel[account.capabilities.contentPublish]}
                          </span>
                          <span className="rounded-lg border border-line bg-canvas px-2 py-0.5 text-[11px] text-ink-secondary">
                            数据分析：{capabilityLabel[account.capabilities.analytics]}
                          </span>
                        </div>

                        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line pt-3">
                          <div className="flex items-center gap-2 text-[11px] text-ink-muted">
                            <span className="font-mono">
                              profile: {account.profileId?.slice(0, 18) ?? "独立容器"}
                            </span>
                          </div>
                          <div className="flex flex-wrap items-center justify-end gap-2">
                            <button
                              type="button"
                              disabled={busy || disabled}
                              onClick={() => void handleCheck(account)}
                              className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-lg border border-line-strong bg-surface px-2.5 text-xs font-medium text-ink-secondary hover:bg-subtle disabled:cursor-not-allowed disabled:opacity-50"
                            >
                              <RefreshCw size={11} />
                              检测状态
                            </button>
                            {!disabled && account.status !== "ACTIVE" && (
                              <button
                                type="button"
                                disabled={busy}
                                onClick={() => void handleBeginAuth(account)}
                                className="inline-flex h-7 cursor-pointer items-center rounded-lg bg-accent px-2.5 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:cursor-not-allowed disabled:opacity-50"
                              >
                                {account.status === "UNAUTHORIZED" ? "验证身份" : "重新授权"}
                              </button>
                            )}
                            <button
                              type="button"
                              onClick={() => openEdit(account)}
                              aria-label={`编辑 ${account.platformName} ${account.alias}`}
                              className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg border border-line text-ink-secondary hover:bg-subtle"
                            >
                              <Pencil size={11} />
                            </button>
                            <button
                              type="button"
                              disabled={busy}
                              onClick={() => void handlePreviewDelete(account)}
                              aria-label={`删除 ${account.platformName} ${account.alias}`}
                              className="grid h-7 w-7 cursor-pointer place-items-center rounded-lg border border-line text-coral-text hover:bg-coral-bg disabled:cursor-not-allowed disabled:opacity-50"
                            >
                              <Trash2 size={11} />
                            </button>
                          </div>
                        </div>
                      </article>
                    );
                  })}
                </div>
              </section>
            ))}
          </div>
        )}

        <section className="space-y-3">
          <div>
            <h2 className="text-sm font-semibold text-ink">平台目录与认证能力</h2>
            <p className="mt-1 text-xs text-ink-muted">
              只有实际完成身份校验的方式标记为可用；OAuth、WebView 和人工流程不会互相伪装。
            </p>
          </div>
          <div className="grid grid-cols-1 gap-3 lg:grid-cols-2 xl:grid-cols-3">
            {visibleCatalog.map((platform) => (
              <article key={platform.id} className="rounded-2xl border border-line bg-surface p-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex items-center gap-2.5">
                    <PlatformLogo platformId={platform.id} platformName={platform.name} size={24} />
                    <div>
                      <h3 className="text-xs font-semibold text-ink">{platform.name}</h3>
                      <p className="text-[10.5px] text-ink-muted">
                        {platform.categoryLabel} · {platform.contentFormats}
                      </p>
                    </div>
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      setAddPlatformId(platform.id);
                      setAddAlias("");
                    }}
                    className="inline-flex h-6 cursor-pointer items-center gap-1 rounded-lg border border-line px-2 text-[10.5px] font-medium text-ink-secondary hover:bg-subtle"
                  >
                    <Plus size={9} />
                    添加
                  </button>
                </div>
                <div className="mt-3 space-y-1.5">
                  {platform.authMethods.map((method) => (
                    <div
                      key={method.method}
                      className="flex items-start justify-between gap-2 rounded-lg bg-canvas px-2.5 py-1.5"
                    >
                      <div className="min-w-0">
                        <strong className="block text-[10.5px] font-medium text-ink-secondary">
                          {method.label}
                        </strong>
                        {method.reason && (
                          <small className="block text-[10px] leading-relaxed text-ink-muted">
                            {method.reason}
                          </small>
                        )}
                      </div>
                      <span
                        className={`shrink-0 rounded-md px-1.5 py-0.5 text-[9.5px] font-medium ${
                          method.available
                            ? "bg-mint-bg text-mint-text"
                            : "bg-sidebar text-ink-muted"
                        }`}
                      >
                        {method.available ? "可用" : "未开放"}
                      </span>
                    </div>
                  ))}
                </div>
              </article>
            ))}
          </div>
        </section>
      </div>

      {addPlatformId && selectedPlatform && (
        <ModalShell label="添加平台账号" onClose={() => setAddPlatformId(null)}>
          <CloseButton onClick={() => setAddPlatformId(null)} />
          <h2 className="text-lg font-semibold tracking-tight text-ink">添加平台账号</h2>
          <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
            同一平台可以添加多个账号；每个账号拥有独立凭据引用和会话容器。
          </p>
          <form className="mt-5 space-y-4" onSubmit={handleCreate}>
            <label className="block">
              <span className="mb-1.5 block text-xs font-medium text-ink">平台</span>
              <select
                value={addPlatformId}
                onChange={(event) => setAddPlatformId(event.target.value)}
                className="h-9.5 w-full rounded-xl border border-line-strong bg-surface px-3 text-sm text-ink focus:border-accent focus:outline-none"
              >
                {catalog.map((platform) => (
                  <option key={platform.id} value={platform.id}>
                    {platform.name} · {platform.categoryLabel}
                  </option>
                ))}
              </select>
            </label>
            <label className="block">
              <span className="mb-1.5 block text-xs font-medium text-ink">账号别名</span>
              <input
                autoFocus
                value={addAlias}
                maxLength={80}
                onChange={(event) => setAddAlias(event.target.value)}
                placeholder="例如：技术主号"
                className="h-9.5 w-full rounded-xl border border-line-strong bg-surface px-3 text-sm text-ink placeholder:text-ink-muted focus:border-accent focus:outline-none"
              />
            </label>
            <div className="rounded-xl border border-line bg-canvas p-3">
              <span className="text-[11px] font-medium text-ink-secondary">可用认证方式</span>
              <div className="mt-1.5 space-y-1">
                {selectedPlatform.authMethods.map((method) => (
                  <p key={method.method} className="text-[10.5px] leading-relaxed text-ink-muted">
                    {method.available ? "✓" : "—"} {method.label}
                    {!method.available && method.reason ? `：${method.reason}` : ""}
                  </p>
                ))}
              </div>
            </div>
            <div className="flex justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => setAddPlatformId(null)}
                className="h-8.5 cursor-pointer rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary hover:bg-sidebar"
              >
                取消
              </button>
              <button
                type="submit"
                disabled={!addAlias.trim()}
                className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
              >
                <Plus size={12} />
                添加账号
              </button>
            </div>
          </form>
        </ModalShell>
      )}

      {editState && (
        <ModalShell label="编辑平台账号" onClose={() => setEditState(null)} width="470px">
          <CloseButton onClick={() => setEditState(null)} />
          <h2 className="text-lg font-semibold tracking-tight text-ink">编辑账号设置</h2>
          <form className="mt-5 space-y-4" onSubmit={handleUpdate}>
            <label className="block">
              <span className="mb-1.5 block text-xs font-medium text-ink">账号别名</span>
              <input
                autoFocus
                value={editState.alias}
                maxLength={80}
                onChange={(event) => setEditState({ ...editState, alias: event.target.value })}
                className="h-9.5 w-full rounded-xl border border-line-strong bg-surface px-3 text-sm text-ink focus:border-accent focus:outline-none"
              />
            </label>
            <label className="flex cursor-pointer items-center justify-between rounded-xl border border-line bg-canvas px-3 py-2.5">
              <span>
                <strong className="block text-xs font-medium text-ink">启用账号</strong>
                <small className="text-[10.5px] text-ink-muted">
                  禁用会阻止依赖该账号的任务，但不会删除凭据。
                </small>
              </span>
              <input
                type="checkbox"
                checked={editState.enabled}
                onChange={(event) => setEditState({ ...editState, enabled: event.target.checked })}
                className="h-4 w-4 accent-[var(--accent)]"
              />
            </label>
            <label className="flex cursor-pointer items-center justify-between rounded-xl border border-line bg-canvas px-3 py-2.5">
              <span>
                <strong className="block text-xs font-medium text-ink">设为该平台默认账号</strong>
                <small className="text-[10.5px] text-ink-muted">同一平台只保留一个默认账号。</small>
              </span>
              <input
                type="checkbox"
                disabled={!editState.enabled}
                checked={editState.isDefault}
                onChange={(event) =>
                  setEditState({ ...editState, isDefault: event.target.checked })
                }
                className="h-4 w-4 accent-[var(--accent)]"
              />
            </label>
            <div className="flex justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => setEditState(null)}
                className="h-8.5 cursor-pointer rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary hover:bg-sidebar"
              >
                取消
              </button>
              <button
                type="submit"
                disabled={!editState.alias.trim() || busyAccountId === editState.accountId}
                className="h-8.5 cursor-pointer rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
              >
                保存设置
              </button>
            </div>
          </form>
        </ModalShell>
      )}

      {authState && authAccount && (
        <ModalShell label="验证平台身份" onClose={() => void closeAuth()} width="590px">
          <CloseButton onClick={() => void closeAuth()} />
          <div className="mb-4 grid h-10 w-10 place-items-center rounded-xl border border-accent-border bg-accent-soft text-accent-deep">
            <KeyRound size={18} />
          </div>
          <h2 className="text-lg font-semibold tracking-tight text-ink">
            验证 {authAccount.platformName} · {authAccount.alias}
          </h2>
          <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
            CreatorRelay 会先调用平台身份接口，只有拿到真实 UID 和昵称后才会把账号标记为 ACTIVE。
          </p>
          <ol className="mt-4 space-y-2 rounded-xl border border-line bg-canvas p-4">
            {authState.challenge.manualSteps.map((step, index) => (
              <li key={step} className="flex gap-2 text-[11px] leading-relaxed text-ink-secondary">
                <span className="grid h-5 w-5 shrink-0 place-items-center rounded-full bg-surface text-[10px] font-semibold text-accent-deep">
                  {index + 1}
                </span>
                <span>{step}</span>
              </li>
            ))}
          </ol>
          <button
            type="button"
            onClick={() => void handleOpenExternal(authState.challenge.loginUrl)}
            className="mt-3 inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-xl border border-line-strong bg-surface px-3 text-xs font-medium text-ink-secondary hover:bg-subtle"
          >
            打开平台登录页
          </button>
          <form className="mt-4 space-y-3" onSubmit={handleCompleteAuth}>
            <label className="block">
              <span className="mb-1.5 block text-xs font-medium text-ink">
                {authState.challenge.credentialLabel}
              </span>
              <input
                type="password"
                autoComplete="off"
                spellCheck={false}
                value={credential}
                onChange={(event) => setCredential(event.target.value)}
                placeholder="粘贴完整 Cookie，例如 SESSDATA=..."
                className="h-9.5 w-full rounded-xl border border-line-strong bg-surface px-3 font-mono text-xs text-ink placeholder:font-sans placeholder:text-ink-muted focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20"
              />
            </label>
            <p className="text-[10.5px] leading-relaxed text-ink-muted">
              {authState.challenge.credentialHelp}
            </p>
            {authError && (
              <p className="rounded-xl bg-coral-bg px-3 py-2 text-[11px] leading-relaxed text-coral-text">
                {authError}
              </p>
            )}
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => void closeAuth()}
                className="h-8.5 cursor-pointer rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary hover:bg-sidebar"
              >
                取消
              </button>
              <button
                type="submit"
                disabled={!credential.trim() || authBusy}
                className="h-8.5 cursor-pointer rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on hover:bg-accent-hover disabled:cursor-not-allowed disabled:bg-line-strong"
              >
                {authBusy ? "正在校验..." : "校验身份并保存会话"}
              </button>
            </div>
          </form>
        </ModalShell>
      )}

      {manualPrompt && (
        <ModalShell label="平台人工流程" onClose={() => setManualPrompt(null)} width="520px">
          <CloseButton onClick={() => setManualPrompt(null)} />
          <h2 className="text-lg font-semibold tracking-tight text-ink">{manualPrompt.title}</h2>
          <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
            该平台没有经过验证的自动身份接口，CreatorRelay 不会用本地文件存在或人工输入冒充 ACTIVE
            会话。
          </p>
          <ol className="mt-4 space-y-2 rounded-xl border border-line bg-canvas p-4">
            {manualPrompt.steps.map((step, index) => (
              <li key={step} className="flex gap-2 text-[11px] leading-relaxed text-ink-secondary">
                <span className="grid h-5 w-5 shrink-0 place-items-center rounded-full bg-surface text-[10px] font-semibold text-accent-deep">
                  {index + 1}
                </span>
                <span>{step}</span>
              </li>
            ))}
          </ol>
          <div className="mt-4 flex justify-end gap-2">
            {manualPrompt.loginUrl && (
              <button
                type="button"
                onClick={() => void handleOpenExternal(manualPrompt.loginUrl)}
                className="h-8.5 cursor-pointer rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary hover:bg-subtle"
              >
                打开平台页面
              </button>
            )}
            <button
              type="button"
              onClick={() => setManualPrompt(null)}
              className="h-8.5 cursor-pointer rounded-xl bg-accent px-3.5 text-xs font-medium text-accent-on hover:bg-accent-hover"
            >
              我知道了
            </button>
          </div>
        </ModalShell>
      )}

      {deleteState && (
        <ModalShell label="删除平台账号" onClose={() => setDeleteState(null)} width="570px">
          <CloseButton onClick={() => setDeleteState(null)} />
          <div className="mb-4 grid h-10 w-10 place-items-center rounded-xl border border-coral-text/20 bg-coral-bg text-coral-text">
            <Trash2 size={17} />
          </div>
          <h2 className="text-lg font-semibold tracking-tight text-ink">
            删除 {deleteState.preview.platformName} · {deleteState.preview.alias}
          </h2>
          <p className="mt-1 text-xs leading-relaxed text-ink-secondary">
            删除范围需要逐项确认。账号删除是不可逆操作，历史作品和本地主稿不会连带删除。
          </p>
          <div className="mt-4 space-y-2">
            <label className="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-canvas px-3 py-2.5">
              <input
                type="checkbox"
                checked={deleteState.credentials}
                onChange={(event) =>
                  setDeleteState({ ...deleteState, credentials: event.target.checked })
                }
                className="mt-0.5 h-4 w-4 accent-[var(--accent)]"
              />
              <span>
                <strong className="block text-xs font-medium text-ink">
                  删除系统 Keychain 中的会话凭据
                </strong>
                <small className="text-[10.5px] text-ink-muted">
                  不会删除其他账号的同平台凭据。
                </small>
              </span>
            </label>
            <label className="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-canvas px-3 py-2.5">
              <input
                type="checkbox"
                checked={deleteState.sessions}
                onChange={(event) =>
                  setDeleteState({ ...deleteState, sessions: event.target.checked })
                }
                className="mt-0.5 h-4 w-4 accent-[var(--accent)]"
              />
              <span className="min-w-0">
                <strong className="block text-xs font-medium text-ink">
                  删除该账号的隔离会话容器
                </strong>
                <small className="block break-all font-mono text-[10px] text-ink-muted">
                  {deleteState.preview.willDeleteSessionProfile}
                </small>
              </span>
            </label>
            <label className="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-canvas px-3 py-2.5">
              <input
                type="checkbox"
                checked={deleteState.historyRetainedUnderstood}
                onChange={(event) =>
                  setDeleteState({
                    ...deleteState,
                    historyRetainedUnderstood: event.target.checked,
                  })
                }
                className="mt-0.5 h-4 w-4 accent-[var(--accent)]"
              />
              <span>
                <strong className="block text-xs font-medium text-ink">
                  我了解历史作品会被保留
                </strong>
                <small className="text-[10.5px] text-ink-muted">
                  将保留 {deleteState.preview.retainedHistoryCount} 条作品记录，并解除其账号关联。
                </small>
              </span>
            </label>
          </div>
          <ul className="mt-3 space-y-1 rounded-xl bg-sidebar px-3 py-2.5">
            {deleteState.preview.retainedData.map((item) => (
              <li key={item} className="text-[10.5px] leading-relaxed text-ink-secondary">
                保留：{item}
              </li>
            ))}
          </ul>
          <div className="mt-4 flex justify-end gap-2">
            <button
              type="button"
              onClick={() => setDeleteState(null)}
              className="h-8.5 cursor-pointer rounded-xl border border-line-strong bg-surface px-3.5 text-xs font-medium text-ink-secondary hover:bg-sidebar"
            >
              取消
            </button>
            <button
              type="button"
              disabled={
                !deleteState.credentials ||
                !deleteState.sessions ||
                !deleteState.historyRetainedUnderstood ||
                busyAccountId === deleteState.accountId
              }
              onClick={() => void handleDelete()}
              className="h-8.5 cursor-pointer rounded-xl bg-coral-text px-3.5 text-xs font-medium text-white hover:opacity-90 disabled:cursor-not-allowed disabled:opacity-40"
            >
              确认删除账号
            </button>
          </div>
        </ModalShell>
      )}
    </main>
  );
}
