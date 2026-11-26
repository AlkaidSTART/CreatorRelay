import { useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { ArrowDown, ArrowUp, Image, Plus, Star, Tags, Trash2, Video, X } from "../Icons";
import type { ContentAsset } from "../../types/content";
import { useStudioStore } from "../../stores/studioStore";
import { useUiStore } from "../../stores/uiStore";
import { MediaThumb } from "./MediaAsset";
import { MarkdownPreview } from "./MarkdownPreview";

const ACCEPT_IMAGE = "image/png,image/jpeg,image/gif,image/webp,image/svg+xml";
const ACCEPT_VIDEO = "video/mp4,video/quicktime,video/webm";

function fileMime(file: File, kind: "image" | "video") {
  if (file.type) return file.type;
  const extension = file.name.split(".").pop()?.toLowerCase();
  if (kind === "video") return extension === "mov" ? "video/quicktime" : "video/mp4";
  if (extension === "jpg" || extension === "jpeg") return "image/jpeg";
  if (extension === "svg") return "image/svg+xml";
  if (extension === "webp") return "image/webp";
  if (extension === "gif") return "image/gif";
  return "image/png";
}

function formatByteSize(bytes: number) {
  if (!bytes || bytes <= 0) return "";
  if (bytes < 1024 * 1024) return `${Math.max(1, Math.round(bytes / 1024))} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function ContentEditor({ preview }: { preview: boolean }) {
  const editor = useStudioStore((state) => state.editor);
  const updateEditor = useStudioStore((state) => state.updateEditor);
  const addAsset = useStudioStore((state) => state.addAsset);
  const updateMedia = useStudioStore((state) => state.updateMedia);
  const moveMedia = useStudioStore((state) => state.moveMedia);
  const reorderMedia = useStudioStore((state) => state.reorderMedia);
  const removeMedia = useStudioStore((state) => state.removeMedia);
  const setCover = useStudioStore((state) => state.setCover);
  const showToast = useUiStore((state) => state.showToast);
  const [busy, setBusy] = useState("");
  const [slashOpen, setSlashOpen] = useState(false);
  const bodyRef = useRef<HTMLTextAreaElement>(null);
  const dragAssetId = useRef("");
  const imageInput = useRef<HTMLInputElement>(null);
  const videoInput = useRef<HTMLInputElement>(null);
  const coverInput = useRef<HTMLInputElement>(null);

  const video = useMemo(
    () => editor?.media.find((asset) => asset.kind === "video") ?? null,
    [editor?.media],
  );
  const images = useMemo(
    () => editor?.media.filter((asset) => asset.kind === "image") ?? [],
    [editor?.media],
  );

  if (!editor) return null;
  const activeEditor = editor;

  async function importFiles(
    files: FileList | null,
    role: "media" | "cover",
    expectedKind: "image" | "video",
  ) {
    if (!files || files.length === 0 || !editor) return;
    setBusy(role === "cover" ? "cover" : expectedKind);
    try {
      for (const file of Array.from(files)) {
        const mimeType = fileMime(file, expectedKind);
        let asset: ContentAsset;
        if (isTauri()) {
          const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
          asset = await invoke<ContentAsset>("import_content_asset", {
            packageId: editor.id,
            role,
            sourceName: file.name,
            mimeType,
            bytes,
          });
        } else {
          asset = {
            id: crypto.randomUUID(),
            kind: expectedKind,
            file: `assets/${file.name}`,
            originalName: file.name,
            mimeType,
            byteSize: file.size,
            caption: "",
            order: editor.media.length + 1,
          };
        }
        addAsset(asset, role);
        if (role === "cover") break;
      }
      showToast(role === "cover" ? "封面已更新" : "素材已导入内容包");
    } catch (cause) {
      useUiStore.getState().setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy("");
      if (imageInput.current) imageInput.current.value = "";
      if (videoInput.current) videoInput.current.value = "";
      if (coverInput.current) coverInput.current.value = "";
    }
  }

  function insertSlash(snippet: string) {
    const textarea = bodyRef.current;
    const cursor = textarea?.selectionStart ?? activeEditor.body.length;
    const before = activeEditor.body.slice(0, cursor).replace(/\/$/u, "");
    const after = activeEditor.body.slice(cursor);
    const next = `${before}${snippet}${after}`;
    updateEditor({ body: next });
    setSlashOpen(false);
    window.setTimeout(() => {
      textarea?.focus();
      const position = before.length + snippet.length;
      textarea?.setSelectionRange(position, position);
    }, 0);
  }

  function insertMediaReference(asset: ContentAsset) {
    const textarea = bodyRef.current;
    const cursor = textarea?.selectionStart ?? activeEditor.body.length;
    const prefix = asset.kind === "image" ? "!" : "";
    const snippet = `${prefix}[${asset.caption || asset.originalName}](asset:${asset.id})`;
    const before = activeEditor.body.slice(0, cursor);
    const after = activeEditor.body.slice(cursor);
    const separator = before.endsWith("\n") || before.length === 0 ? "" : "\n\n";
    updateEditor({ body: `${before}${separator}${snippet}\n${after}` });
    showToast("已插入素材引用");
  }

  function handleBodyChange(value: string, selectionStart: number | null) {
    updateEditor({ body: value });
    const prefix = value.slice(0, selectionStart ?? value.length);
    if (/(^|\n)\/$/u.test(prefix)) setSlashOpen(true);
    else if (slashOpen) setSlashOpen(false);
  }

  const isArticleKind = editor.kind === "imageArticle" || editor.kind === "article";

  const fileInputs = (
    <>
      <input
        ref={imageInput}
        type="file"
        accept={ACCEPT_IMAGE}
        multiple={editor.kind === "imageText"}
        className="hidden"
        onChange={(event) => void importFiles(event.target.files, "media", "image")}
      />
      <input
        ref={videoInput}
        type="file"
        accept={ACCEPT_VIDEO}
        className="hidden"
        onChange={(event) => void importFiles(event.target.files, "media", "video")}
      />
      <input
        ref={coverInput}
        type="file"
        accept={ACCEPT_IMAGE}
        className="hidden"
        onChange={(event) => void importFiles(event.target.files, "cover", "image")}
      />
    </>
  );

  return (
    <div className="h-full w-full overflow-y-auto bg-surface">
      {fileInputs}

      {/* Unified Single-Column Editorial Canvas */}
      <div className="mx-auto flex min-h-full w-full max-w-[820px] flex-col px-8 py-7">
        {/* Subtle inline action strip above title (Notion/Ghost style) */}
        <div className="mb-3 flex flex-wrap items-center gap-2 text-[11.5px] text-ink-muted">
          <button
            type="button"
            onClick={() => coverInput.current?.click()}
            disabled={Boolean(busy)}
            className="inline-flex cursor-pointer items-center gap-1 rounded-sm px-1.5 py-0.5 transition-colors hover:bg-subtle hover:text-ink disabled:opacity-50"
          >
            <Star size={11} />
            {busy === "cover" ? "导入中..." : editor.cover ? "更换封面" : "添加封面"}
          </button>

          {(editor.kind === "imageText" || isArticleKind) && (
            <button
              type="button"
              onClick={() => imageInput.current?.click()}
              disabled={Boolean(busy)}
              className="inline-flex cursor-pointer items-center gap-1 rounded-sm px-1.5 py-0.5 transition-colors hover:bg-subtle hover:text-ink disabled:opacity-50"
            >
              <Image size={11} />
              {busy === "image" ? "导入中..." : "添加图片"}
            </button>
          )}

          {(editor.kind === "video" || isArticleKind) && (
            <button
              type="button"
              onClick={() => videoInput.current?.click()}
              disabled={Boolean(busy) || (editor.kind === "video" && Boolean(video))}
              className="inline-flex cursor-pointer items-center gap-1 rounded-sm px-1.5 py-0.5 transition-colors hover:bg-subtle hover:text-ink disabled:opacity-50"
            >
              <Video size={11} />
              {busy === "video" ? "导入中..." : video ? "已导入视频" : "导入视频"}
            </button>
          )}

          {editor.source && (
            <span className="ml-auto rounded-xs border border-sky-border bg-sky-bg px-1.5 py-px font-mono text-[10px] text-sky-text">
              派生自 {editor.source.contentId.slice(0, 8)}
            </span>
          )}
        </div>

        {/* Cover Banner (only takes space when a cover image is actually set) */}
        {editor.cover && (
          <div className="group relative mb-5 overflow-hidden rounded-md border border-line bg-subtle">
            <MediaThumb
              source={{ kind: "content", packageId: editor.id }}
              asset={editor.cover}
              className="max-h-[220px] w-full object-cover"
            />
            <div className="absolute right-2.5 bottom-2.5 flex items-center gap-1.5 opacity-0 transition-opacity group-hover:opacity-100">
              <button
                type="button"
                onClick={() => coverInput.current?.click()}
                className="cursor-pointer rounded-sm bg-black/70 px-2 py-1 text-[10.5px] font-medium text-white backdrop-blur-xs hover:bg-black/85"
              >
                更换封面
              </button>
              <button
                type="button"
                onClick={() => updateEditor({ cover: null })}
                aria-label="清除封面"
                className="grid h-6 w-6 cursor-pointer place-items-center rounded-sm bg-black/70 text-white backdrop-blur-xs hover:bg-coral-text"
              >
                <X size={11} />
              </button>
            </div>
          </div>
        )}

        {/* Borderless Document Title */}
        <input
          value={editor.title}
          onChange={(event) => updateEditor({ title: event.target.value })}
          aria-label="内容标题"
          placeholder="无标题内容..."
          className="w-full border-0 bg-transparent text-[26px] leading-tight font-semibold tracking-[-0.025em] text-ink placeholder:text-ink-faint focus:outline-none"
        />

        {/* Quiet Inline Metadata Row (No Nested Cards) */}
        <div className="mt-3 flex flex-col gap-2 border-b border-line pb-4 sm:flex-row sm:items-center sm:gap-5">
          <div className="flex min-w-0 flex-1 items-center gap-2 text-xs">
            <Tags size={12} className="shrink-0 text-ink-faint" />
            <input
              value={editor.tags.join(", ")}
              onChange={(event) =>
                updateEditor({
                  tags: event.target.value
                    .split(",")
                    .map((tag) => tag.trim())
                    .filter(Boolean),
                })
              }
              placeholder="添加标签（逗号分隔，如 Rust, 本地优先）"
              className="min-w-0 flex-1 bg-transparent text-[12px] text-ink-secondary placeholder:text-ink-faint focus:text-ink focus:outline-none"
            />
          </div>

          {isArticleKind && (
            <div className="flex min-w-0 flex-[1.3] items-center gap-2 border-t border-line/60 pt-2 text-xs sm:border-t-0 sm:border-l sm:pt-0 sm:pl-4">
              <label htmlFor="article-summary" className="shrink-0 text-[11px] text-ink-muted">
                摘要
              </label>
              <input
                id="article-summary"
                value={editor.summary}
                onChange={(event) => updateEditor({ summary: event.target.value })}
                placeholder="一句话摘要（用于平台稿与预览）"
                className="min-w-0 flex-1 bg-transparent text-[12px] text-ink-secondary placeholder:text-ink-faint focus:text-ink focus:outline-none"
              />
            </div>
          )}
        </div>

        {/* Mode-Specific Body — All in One Calm Flow */}
        <div className="mt-5 flex min-h-0 flex-1 flex-col">
          {/* Video Mode */}
          {editor.kind === "video" && (
            <div className="space-y-5">
              <div>
                <div className="mb-2 flex items-center justify-between text-xs">
                  <span className="font-medium text-ink">主视频</span>
                  {video ? (
                    <div className="flex items-center gap-2.5">
                      <span className="font-mono text-[11px] text-ink-muted">
                        {video.originalName}
                        {video.byteSize ? ` · ${formatByteSize(video.byteSize)}` : ""}
                      </span>
                      <button
                        type="button"
                        onClick={() => removeMedia(video.id)}
                        className="inline-flex cursor-pointer items-center gap-1 rounded-sm px-1.5 py-0.5 text-[11px] text-ink-muted transition-colors hover:bg-coral-bg hover:text-coral-text"
                      >
                        <Trash2 size={11} /> 移除
                      </button>
                    </div>
                  ) : (
                    <span className="text-[11px] text-ink-muted">MP4 / MOV / WebM</span>
                  )}
                </div>

                {video ? (
                  <div className="overflow-hidden rounded-md border border-line bg-black">
                    <MediaThumb
                      source={{ kind: "content", packageId: editor.id }}
                      asset={video}
                      controls
                      className="aspect-video max-h-[380px] w-full"
                    />
                  </div>
                ) : (
                  <button
                    type="button"
                    onClick={() => videoInput.current?.click()}
                    disabled={Boolean(busy)}
                    className="flex h-48 w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-md border border-dashed border-line-strong bg-canvas/60 text-center transition-colors hover:border-accent hover:bg-accent-soft/20 disabled:opacity-50"
                  >
                    <Video size={20} className="text-ink-muted" />
                    <strong className="text-xs font-medium text-ink">点击导入主视频</strong>
                    <span className="text-[11px] text-ink-muted">
                      视频主稿不要求 Markdown 正文
                    </span>
                  </button>
                )}
              </div>

              <div className="border-t border-line pt-4">
                <div className="mb-2 flex items-center justify-between">
                  <label htmlFor="video-summary" className="text-xs font-medium text-ink">
                    视频简介与章节
                  </label>
                  <span className="font-mono text-[10.5px] text-ink-muted">
                    {editor.summary.length} 字
                  </span>
                </div>
                <textarea
                  id="video-summary"
                  rows={8}
                  value={editor.summary}
                  onChange={(event) => updateEditor({ summary: event.target.value })}
                  placeholder="填写视频简介、章节时间轴或发布说明..."
                  className="w-full resize-y bg-transparent text-[13.5px] leading-relaxed text-ink placeholder:text-ink-faint focus:outline-none"
                />
              </div>
            </div>
          )}

          {/* ImageText Mode */}
          {editor.kind === "imageText" && (
            <div className="space-y-5">
              <div>
                <div className="mb-2.5 flex items-center justify-between text-xs">
                  <span className="font-medium text-ink">图集图片（{images.length} 张）</span>
                  <span className="text-[11px] text-ink-muted">拖拽或使用箭头调整顺序</span>
                </div>

                {images.length === 0 ? (
                  <button
                    type="button"
                    onClick={() => imageInput.current?.click()}
                    disabled={Boolean(busy)}
                    className="flex h-40 w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-md border border-dashed border-line-strong bg-canvas/60 text-center transition-colors hover:border-accent hover:bg-accent-soft/20"
                  >
                    <Image size={18} className="text-ink-muted" />
                    <strong className="text-xs font-medium text-ink">点击添加图集图片</strong>
                    <span className="text-[11px] text-ink-muted">
                      支持多选导入 PNG / JPG / WebP
                    </span>
                  </button>
                ) : (
                  <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    {images.map((asset, index) => (
                      <div
                        key={asset.id}
                        draggable
                        onDragStart={() => {
                          dragAssetId.current = asset.id;
                        }}
                        onDragOver={(event) => event.preventDefault()}
                        onDrop={() => {
                          if (dragAssetId.current) reorderMedia(dragAssetId.current, asset.id);
                          dragAssetId.current = "";
                        }}
                        className={`rounded-md border bg-canvas/50 p-2 transition-colors ${
                          editor.cover?.id === asset.id
                            ? "border-accent"
                            : "border-line hover:border-line-strong"
                        }`}
                      >
                        <div className="relative overflow-hidden rounded-sm">
                          <MediaThumb
                            source={{ kind: "content", packageId: editor.id }}
                            asset={asset}
                            className="h-28 w-full object-cover"
                          />
                          <span className="absolute top-1.5 left-1.5 rounded-xs bg-black/70 px-1.5 py-px font-mono text-[9.5px] text-white">
                            {index + 1}
                          </span>
                          {editor.cover?.id === asset.id && (
                            <span className="absolute top-1.5 right-1.5 rounded-xs bg-accent px-1.5 py-px text-[9.5px] font-medium text-accent-on">
                              封面
                            </span>
                          )}
                        </div>
                        <input
                          value={asset.caption}
                          onChange={(event) =>
                            updateMedia(asset.id, { caption: event.target.value })
                          }
                          placeholder="图片说明..."
                          className="mt-2 w-full rounded-sm border border-line bg-surface px-2 py-1 text-[11px] text-ink outline-none focus:border-accent"
                        />
                        <div className="mt-1.5 flex items-center justify-between">
                          <div className="flex gap-1">
                            <button
                              type="button"
                              aria-label="上移图片"
                              disabled={index === 0}
                              onClick={() => moveMedia(asset.id, -1)}
                              className="grid h-6 w-6 cursor-pointer place-items-center rounded-sm border border-line bg-surface text-ink-muted hover:text-ink disabled:opacity-30"
                            >
                              <ArrowUp size={9} />
                            </button>
                            <button
                              type="button"
                              aria-label="下移图片"
                              disabled={index === images.length - 1}
                              onClick={() => moveMedia(asset.id, 1)}
                              className="grid h-6 w-6 cursor-pointer place-items-center rounded-sm border border-line bg-surface text-ink-muted hover:text-ink disabled:opacity-30"
                            >
                              <ArrowDown size={9} />
                            </button>
                          </div>
                          <div className="flex gap-1">
                            <button
                              type="button"
                              onClick={() => setCover(asset)}
                              className="cursor-pointer rounded-sm border border-line bg-surface px-1.5 py-0.5 text-[10px] font-medium text-ink-secondary hover:border-line-strong hover:text-ink"
                            >
                              设为封面
                            </button>
                            <button
                              type="button"
                              aria-label="删除图片"
                              onClick={() => removeMedia(asset.id)}
                              className="grid h-6 w-6 cursor-pointer place-items-center rounded-sm border border-line bg-surface text-ink-muted hover:border-coral-border hover:bg-coral-bg hover:text-coral-text"
                            >
                              <Trash2 size={9} />
                            </button>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              <div className="border-t border-line pt-4">
                <div className="mb-2 flex items-center justify-between">
                  <label htmlFor="imagetext-summary" className="text-xs font-medium text-ink">
                    短文案
                  </label>
                  <span className="font-mono text-[10.5px] text-ink-muted">
                    {editor.summary.length} 字
                  </span>
                </div>
                <textarea
                  id="imagetext-summary"
                  rows={8}
                  value={editor.summary}
                  onChange={(event) => updateEditor({ summary: event.target.value })}
                  placeholder="输入图文短文案；逐图说明可在上方图片卡片中维护..."
                  className="w-full resize-y bg-transparent text-[13.5px] leading-relaxed text-ink placeholder:text-ink-faint focus:outline-none"
                />
              </div>
            </div>
          )}

          {/* Article / ImageArticle Mode */}
          {isArticleKind && (
            <div className="relative flex min-h-0 flex-1 flex-col">
              {/* Compact Horizontal Inline Media Strip (Only shown when imageArticle has images) */}
              {editor.kind === "imageArticle" && images.length > 0 && (
                <div className="mb-4 rounded-md border border-line bg-subtle/50 p-2.5">
                  <div className="mb-2 flex items-center justify-between text-[11px] text-ink-muted">
                    <span className="font-medium text-ink-secondary">
                      内联图片素材 ({images.length})
                    </span>
                    <span>点击可插入正文光标处</span>
                  </div>
                  <div className="flex gap-2.5 overflow-x-auto pb-1">
                    {images.map((asset) => (
                      <div
                        key={asset.id}
                        className="flex w-44 shrink-0 flex-col rounded-sm border border-line bg-surface p-1.5"
                      >
                        <MediaThumb
                          source={{ kind: "content", packageId: editor.id }}
                          asset={asset}
                          className="h-18 w-full rounded-xs object-cover"
                        />
                        <input
                          value={asset.caption}
                          onChange={(event) =>
                            updateMedia(asset.id, { caption: event.target.value })
                          }
                          placeholder="图片说明"
                          className="mt-1.5 w-full rounded-xs border border-line bg-canvas px-1.5 py-0.5 text-[10.5px] text-ink outline-none focus:border-accent"
                        />
                        <div className="mt-1.5 flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => insertMediaReference(asset)}
                            className="flex-1 cursor-pointer rounded-xs bg-subtle py-0.5 text-[10.5px] font-medium text-ink-secondary transition-colors hover:bg-accent-soft hover:text-accent-deep"
                          >
                            插入正文
                          </button>
                          <button
                            type="button"
                            aria-label="删除图片"
                            onClick={() => removeMedia(asset.id)}
                            className="grid h-5 w-5 cursor-pointer place-items-center rounded-xs text-ink-muted hover:bg-coral-bg hover:text-coral-text"
                          >
                            <Trash2 size={9} />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {preview ? (
                <div className="min-h-[420px] flex-1 py-1">
                  <MarkdownPreview
                    markdown={editor.body}
                    source={{ kind: "content", packageId: editor.id }}
                    assets={editor.media}
                  />
                </div>
              ) : (
                <textarea
                  ref={bodyRef}
                  value={editor.body}
                  onChange={(event) =>
                    handleBodyChange(event.target.value, event.target.selectionStart)
                  }
                  aria-label="Markdown 正文"
                  spellCheck={false}
                  placeholder="开始写作 Markdown 正文；输入 / 可唤起块级命令..."
                  className="min-h-[460px] w-full flex-1 resize-none border-0 bg-transparent font-mono text-[13.5px] leading-[1.85] text-ink placeholder:text-ink-faint focus:outline-none"
                />
              )}

              {slashOpen && !preview && (
                <div className="absolute top-2 right-0 z-20 w-48 overflow-hidden rounded-md border border-line-strong bg-surface p-1 shadow-lg">
                  <div className="px-2 py-1 text-[10px] font-medium text-ink-muted">
                    快捷块级命令
                  </div>
                  {[
                    ["二级标题", "## 标题\n"],
                    ["引用块", "> 引用内容\n"],
                    ["代码块", "```\n\n```\n"],
                    ["无序列表", "- 项目\n"],
                  ].map(([label, snippet]) => (
                    <button
                      key={label}
                      type="button"
                      onClick={() => insertSlash(snippet)}
                      className="flex w-full cursor-pointer items-center justify-between rounded-sm px-2 py-1.5 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                    >
                      {label}
                      <Plus size={10} />
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
