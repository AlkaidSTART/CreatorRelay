import { useMemo } from "react";
import { Clock3, Send, X } from "../Icons";
import type { AiProvider } from "../../types/ai";
import type { ContentPackageSummary } from "../../types/content";
import type {
  AssistantContextPreview,
  McpAuditRecord,
  McpHostStatus,
  McpSecurityEvidence,
  McpSettings,
  McpToolDefinition,
  ModelContextScope,
} from "../../types/mcp";
import { PERMISSION_META, STATUS_META } from "./AssistantMessageStream";

export type InspectorTab = "mcp" | "privacy" | "audit";

function timeLabel(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

interface AssistantInspectorDrawerProps {
  tab: InspectorTab;
  onTabChange: (tab: InspectorTab) => void;
  onClose: () => void;
  status: McpHostStatus | null;
  tools: McpToolDefinition[];
  security: McpSecurityEvidence | null;
  audit: McpAuditRecord[];
  settingsDraft: McpSettings | null;
  onSettingsDraftChange: (next: McpSettings) => void;
  onSaveSettings: (next: McpSettings) => void;
  onStartServer: () => void;
  onStopServer: () => void;
  onRunHangCheck: () => void;
  providers: AiProvider[];
  providerId: string;
  onProviderChange: (id: string) => void;
  packages: ContentPackageSummary[];
  contentId: string;
  onContentChange: (id: string) => void;
  scope: ModelContextScope;
  onScopeChange: (next: ModelContextScope) => void;
  contextPreview: AssistantContextPreview | null;
  onGenerateReply: () => void;
  busyAction: string;
}

export function AssistantInspectorDrawer({
  tab,
  onTabChange,
  onClose,
  status,
  tools,
  security,
  audit,
  settingsDraft,
  onSettingsDraftChange,
  onSaveSettings,
  onStartServer,
  onStopServer,
  onRunHangCheck,
  providers,
  providerId,
  onProviderChange,
  packages,
  contentId,
  onContentChange,
  scope,
  onScopeChange,
  contextPreview,
  onGenerateReply,
  busyAction,
}: AssistantInspectorDrawerProps) {
  const groupedTools = useMemo(
    () => ({
      read: tools.filter((tool) => tool.permission === "read"),
      write: tools.filter((tool) => tool.permission === "write"),
      externalAction: tools.filter((tool) => tool.permission === "externalAction"),
    }),
    [tools],
  );

  return (
    <aside
      className="flex h-full w-[336px] shrink-0 flex-col border-l border-line bg-surface-soft"
      aria-label="运行环境与审计面板"
    >
      {/* Clean Underline Tab Header */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-line bg-surface px-3">
        <div className="flex h-full items-center gap-3">
          {(
            [
              ["mcp", "MCP Host"],
              ["privacy", "发送披露"],
              ["audit", `审计 (${audit.length})`],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              onClick={() => onTabChange(id)}
              className={`relative flex h-full cursor-pointer items-center text-[11.5px] font-medium transition-colors ${
                tab === id ? "text-ink" : "text-ink-muted hover:text-ink-secondary"
              }`}
            >
              {label}
              {tab === id && (
                <span className="absolute right-0 bottom-0 left-0 h-[2px] bg-accent" />
              )}
            </button>
          ))}
        </div>
        <button
          type="button"
          onClick={onClose}
          className="grid h-6.5 w-6.5 shrink-0 cursor-pointer place-items-center rounded-md text-ink-muted transition-colors hover:bg-hover hover:text-ink"
          title="收起检查面板"
          aria-label="收起检查面板"
        >
          <X size={12} />
        </button>
      </div>

      {/* Hairline-Divided Inspector Body */}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {tab === "mcp" && (
          <div className="divide-y divide-line">
            {/* Host Status */}
            <section className="p-4">
              <div className="flex items-center gap-2">
                <span
                  className={`h-1.5 w-1.5 rounded-full ${
                    status?.state === "healthy" ? "bg-mint-text" : "bg-coral-text"
                  }`}
                />
                <strong className="text-xs font-semibold text-ink">
                  {status?.state === "healthy" ? "MCP Host 运行正常" : "MCP Host 未就绪"}
                </strong>
                <span className="ml-auto font-mono text-[10px] text-ink-muted">
                  PID {status?.pid ?? "—"}
                </span>
              </div>
              <dl className="mt-2.5 grid grid-cols-2 gap-y-1.5 text-[11px] text-ink-secondary">
                <dt className="text-ink-muted">Server</dt>
                <dd className="truncate text-right font-medium text-ink">
                  {status?.serverName ?? "—"}
                </dd>
                <dt className="text-ink-muted">协议版本</dt>
                <dd className="text-right font-mono">{status?.protocolVersion ?? "—"}</dd>
                <dt className="text-ink-muted">重启次数</dt>
                <dd className="text-right tabular-nums">{status?.restartCount ?? 0} 次</dd>
                <dt className="text-ink-muted">已发现工具</dt>
                <dd className="text-right tabular-nums">{tools.length} 个</dd>
              </dl>
              {status?.lastError && (
                <p className="mt-2.5 rounded-sm border border-coral-border bg-coral-bg px-2.5 py-1.5 text-[10.5px] text-coral-text">
                  {status.lastError}
                </p>
              )}
              <div className="mt-3 grid grid-cols-2 gap-1.5">
                <button
                  type="button"
                  onClick={onStartServer}
                  disabled={Boolean(busyAction)}
                  className="h-7 cursor-pointer rounded-md border border-line-strong bg-surface text-[11px] font-medium text-ink-secondary transition-colors hover:bg-hover hover:text-ink disabled:opacity-50"
                >
                  启动 / 重启
                </button>
                <button
                  type="button"
                  onClick={onStopServer}
                  disabled={Boolean(busyAction)}
                  className="h-7 cursor-pointer rounded-md border border-line bg-surface text-[11px] font-medium text-ink-secondary transition-colors hover:border-coral-border hover:text-coral-text disabled:opacity-50"
                >
                  停止 Server
                </button>
              </div>
            </section>

            {/* Permission Policy */}
            <section className="p-4">
              <h3 className="text-xs font-semibold text-ink">权限策略</h3>
              {settingsDraft && (
                <div className="mt-2.5 space-y-2.5 text-[11.5px] text-ink-secondary">
                  <label className="flex cursor-pointer items-center justify-between gap-2">
                    允许只读工具
                    <input
                      type="checkbox"
                      checked={settingsDraft.readEnabled}
                      onChange={(event) =>
                        onSettingsDraftChange({
                          ...settingsDraft,
                          readEnabled: event.target.checked,
                        })
                      }
                    />
                  </label>
                  <label className="flex cursor-pointer items-center justify-between gap-2">
                    异常自动重启
                    <input
                      type="checkbox"
                      checked={settingsDraft.autoRestart}
                      onChange={(event) =>
                        onSettingsDraftChange({
                          ...settingsDraft,
                          autoRestart: event.target.checked,
                        })
                      }
                    />
                  </label>
                  <label className="block">
                    <span className="text-[10.5px] text-ink-muted">写操作授权</span>
                    <select
                      value={settingsDraft.writePolicy}
                      onChange={(event) =>
                        onSettingsDraftChange({
                          ...settingsDraft,
                          writePolicy: event.target.value as "confirm" | "allow",
                        })
                      }
                      className="mt-1 w-full rounded-sm border border-line bg-surface px-2 py-1.5 text-[11px] text-ink outline-none focus:border-accent"
                    >
                      <option value="confirm">每次确认</option>
                      <option value="allow">允许已授权写入</option>
                    </select>
                  </label>
                  <p className="text-[10.5px] leading-relaxed text-ink-muted">
                    外部平台操作每次都需确认；浏览器中转只读页面可见文本，不会代替发布。
                  </p>
                  <button
                    type="button"
                    onClick={() => onSaveSettings(settingsDraft)}
                    disabled={Boolean(busyAction)}
                    className="h-7.5 w-full cursor-pointer rounded-md bg-accent text-[11.5px] font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:opacity-50"
                  >
                    保存策略
                  </button>
                </div>
              )}
            </section>

            {/* Security Isolation */}
            {security && (
              <section className="p-4">
                <h3 className="text-xs font-semibold text-ink">安全隔离边界</h3>
                <p className="mt-1 font-mono text-[10.5px] text-ink-muted">{security.transport}</p>
                <div className="mt-2 flex flex-wrap gap-1.5">
                  {security.processIsolated && (
                    <span className="rounded-xs bg-mint-bg px-1.5 py-0.5 text-[10px] text-mint-text">
                      进程隔离
                    </span>
                  )}
                  {!security.credentialReferencePassed && (
                    <span className="rounded-xs bg-mint-bg px-1.5 py-0.5 text-[10px] text-mint-text">
                      无凭据透传
                    </span>
                  )}
                  {security.businessToolsForwardToCore && (
                    <span className="rounded-xs bg-sky-bg px-1.5 py-0.5 text-[10px] text-sky-text">
                      Core 转发校验
                    </span>
                  )}
                </div>
              </section>
            )}

            {/* Discovered Tools */}
            <section className="p-4">
              <div className="mb-2.5 flex items-center justify-between">
                <h3 className="text-xs font-semibold text-ink">已发现工具</h3>
                <span className="font-mono text-[10.5px] text-ink-muted">{tools.length}</span>
              </div>
              {tools.length > 0 ? (
                <div className="divide-y divide-line overflow-hidden rounded-md border border-line bg-surface">
                  {(["read", "write", "externalAction"] as const).map((permission) =>
                    groupedTools[permission].map((tool) => (
                      <div key={tool.name} className="px-2.5 py-2">
                        <div className="flex items-center gap-1.5">
                          <code className="min-w-0 flex-1 truncate font-mono text-[11px] font-medium text-ink">
                            {tool.name}
                          </code>
                          <span
                            className={`rounded-xs border px-1 py-px text-[9px] font-medium ${PERMISSION_META[permission].className}`}
                          >
                            {PERMISSION_META[permission].label}
                          </span>
                        </div>
                        <p className="mt-0.5 line-clamp-2 text-[10.5px] leading-relaxed text-ink-muted">
                          {tool.description}
                        </p>
                      </div>
                    )),
                  )}
                </div>
              ) : (
                <p className="text-center text-[11px] text-ink-muted">
                  启动 MCP Server 后显示工具目录。
                </p>
              )}

              <button
                type="button"
                onClick={onRunHangCheck}
                disabled={Boolean(busyAction)}
                className="mt-3 flex h-7.5 w-full cursor-pointer items-center justify-center gap-1.5 rounded-md border border-line-strong bg-surface text-[11px] text-ink-secondary transition-colors hover:bg-hover hover:text-ink disabled:opacity-50"
              >
                <Clock3 size={11} />
                运行超时 / 取消自检
              </button>
            </section>
          </div>
        )}

        {tab === "privacy" && (
          <div className="divide-y divide-line">
            <section className="p-4">
              <h3 className="text-xs font-semibold text-ink">发送前披露</h3>
              <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
                工具调用不经过云端模型。只有点击确认发送时，选定范围才会发送给模型服务商。
              </p>

              <label className="mt-3 block text-[11px] text-ink-secondary">
                模型服务商
                <select
                  value={providerId}
                  onChange={(event) => onProviderChange(event.target.value)}
                  className="mt-1 w-full rounded-sm border border-line bg-surface px-2 py-1.5 text-[11.5px] text-ink outline-none focus:border-accent"
                >
                  <option value="">本地编排器</option>
                  {providers.map((provider) => (
                    <option key={provider.id} value={provider.id}>
                      {provider.name} · {provider.model}
                    </option>
                  ))}
                </select>
              </label>

              <label className="mt-2.5 block text-[11px] text-ink-secondary">
                内容来源
                <select
                  value={contentId}
                  onChange={(event) => onContentChange(event.target.value)}
                  className="mt-1 w-full rounded-sm border border-line bg-surface px-2 py-1.5 text-[11.5px] text-ink outline-none focus:border-accent"
                >
                  <option value="">不发送内容包</option>
                  {packages.map((item) => (
                    <option key={item.id} value={item.id}>
                      {item.title} · {item.kind}
                    </option>
                  ))}
                </select>
              </label>

              <div className="mt-3 grid grid-cols-2 gap-1.5 text-[11px]">
                {(
                  [
                    ["标题", "includeTitle"],
                    ["摘要与标签", "includeSummary"],
                    ["正文", "includeBody"],
                    ["素材元数据", "includeMediaMetadata"],
                  ] as const
                ).map(([label, key]) => (
                  <label
                    key={key}
                    className="flex cursor-pointer items-center gap-1.5 rounded-sm border border-line bg-surface px-2 py-1.5 text-ink-secondary"
                  >
                    <input
                      type="checkbox"
                      checked={Boolean(scope[key])}
                      onChange={(event) => onScopeChange({ ...scope, [key]: event.target.checked })}
                    />
                    {label}
                  </label>
                ))}
              </div>
            </section>

            <section className="p-4">
              <div className="flex items-center justify-between gap-2">
                <h3 className="text-xs font-semibold text-ink">实际发送上下文</h3>
                {contextPreview && (
                  <span className="font-mono text-[10.5px] text-ink-muted">
                    {contextPreview.characterCount} 字
                  </span>
                )}
              </div>

              {contextPreview ? (
                <>
                  <p className="mt-1 font-mono text-[10.5px] text-ink-muted">
                    {contextPreview.providerName} · {contextPreview.model}
                  </p>
                  <div className="mt-2 flex flex-wrap gap-1">
                    {contextPreview.includedFields.map((field) => (
                      <span
                        key={field}
                        className="rounded-xs bg-mint-bg px-1.5 py-px text-[9.5px] text-mint-text"
                      >
                        包含 {field}
                      </span>
                    ))}
                    {contextPreview.excludedFields.map((field) => (
                      <span
                        key={field}
                        className="rounded-xs bg-subtle px-1.5 py-px text-[9.5px] text-ink-muted"
                      >
                        排除 {field}
                      </span>
                    ))}
                  </div>
                  <pre className="mt-2.5 max-h-56 overflow-auto whitespace-pre-wrap rounded-sm border border-line bg-surface p-2.5 font-mono text-[10.5px] leading-relaxed text-ink-secondary">
                    {contextPreview.context || "未选择内容包；仅发送用户请求。"}
                  </pre>
                </>
              ) : (
                <p className="mt-2 text-[11px] text-ink-muted">
                  选择内容与范围后显示实际发送载荷（桌面端实时计算脱敏上下文）。
                </p>
              )}

              <button
                type="button"
                onClick={onGenerateReply}
                disabled={Boolean(busyAction)}
                className="mt-3 flex h-8 w-full cursor-pointer items-center justify-center gap-1.5 rounded-md bg-accent text-[11.5px] font-medium text-accent-on transition-colors hover:bg-accent-hover disabled:opacity-50"
              >
                <Send size={11} />
                确认并发送到模型
              </button>
            </section>
          </div>
        )}

        {tab === "audit" && (
          <div>
            <div className="border-b border-line p-4">
              <h3 className="text-xs font-semibold text-ink">调用审计日志</h3>
              <p className="mt-1 text-[10.5px] leading-relaxed text-ink-muted">
                参数仅保存脱敏摘要；正文以字符数代替，不保存 API Key、Cookie、Token 或凭据引用。
              </p>
            </div>
            {audit.length > 0 ? (
              <div className="divide-y divide-line">
                {audit.map((record) => {
                  const statusBadge = STATUS_META[record.status];
                  return (
                    <div key={record.id} className="px-4 py-3">
                      <div className="flex items-center gap-1.5">
                        <code className="min-w-0 flex-1 truncate font-mono text-[11px] font-medium text-ink">
                          {record.toolName}
                        </code>
                        <span
                          className={`rounded-xs px-1.5 py-px text-[9.5px] font-medium ${
                            statusBadge?.className ?? "bg-subtle text-ink-secondary"
                          }`}
                        >
                          {statusBadge?.label ?? record.status}
                        </span>
                      </div>
                      <p className="mt-1 text-[10.5px] text-ink-muted">
                        <span className="font-mono">{timeLabel(record.startedAt)}</span> ·{" "}
                        {record.targetSummary}
                      </p>
                      <p className="mt-1 line-clamp-3 break-words font-mono text-[10px] text-ink-secondary">
                        {record.parameterSummary || "无参数摘要"}
                      </p>
                      {record.errorMessage && (
                        <p className="mt-1 text-[10px] text-coral-text">{record.errorMessage}</p>
                      )}
                    </div>
                  );
                })}
              </div>
            ) : (
              <p className="p-6 text-center text-[11px] text-ink-muted">暂无工具调用记录。</p>
            )}
          </div>
        )}
      </div>
    </aside>
  );
}
