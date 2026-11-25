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

  const titleField = (
    <input
      value={editor.title}
      onChange={(event) => updateEditor({ title: event.target.value })}
      aria-label="内容标题"
      placeholder="输入内容标题..."
      className="w-full border-0 bg-transparent text-[26px] leading-snug font-semibold tracking-tight text-ink placeholder:text-ink-faint focus:outline-none"
    />
  );

  const tagsField = (
    <div className="flex flex-wrap items-center gap-2.5">
      <div className="flex min-w-[240px] flex-1 items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2 shadow-[0_1px_2px_rgba(51,43,41,0.03)] transition-all focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
        <Tags size={12} className="shrink-0 text-ink-muted" />
        <span className="shrink-0 text-[11px] font-medium text-ink-muted">标签</span>
        <span className="h-3 w-px bg-line" />
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
          placeholder="用逗号分隔，例如：Rust, 本地优先"
          className="min-w-0 flex-1 bg-transparent text-xs text-ink placeholder:text-ink-muted focus:outline-none"
        />
      </div>
      {editor.source && (
        <span className="inline-flex items-center rounded-xl border border-sky-border bg-sky-bg px-2.5 py-2 text-[11px] font-medium text-sky-text">
          派生自 {editor.source.contentId.slice(0, 8)}
        </span>
      )}
    </div>
  );

  const mediaToolbar = (
    <div className="flex flex-wrap items-center gap-2">
      {(editor.kind === "video" || editor.kind === "imageArticle" || editor.kind === "article") && (
        <button
          type="button"
          onClick={() => videoInput.current?.click()}
          disabled={Boolean(busy) || (editor.kind === "video" && Boolean(video))}
          className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary shadow-[0_1px_2px_rgba(51,43,41,0.03)] transition-all hover:border-accent hover:text-accent-deep disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Video size={12} />
          {busy === "video" ? "导入中..." : video ? "已导入视频" : "导入视频"}
        </button>
      )}
      {(editor.kind === "imageText" ||
        editor.kind === "imageArticle" ||
        editor.kind === "article") && (
        <button
          type="button"
          onClick={() => imageInput.current?.click()}
          disabled={Boolean(busy)}
          className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary shadow-[0_1px_2px_rgba(51,43,41,0.03)] transition-all hover:border-accent hover:text-accent-deep disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Image size={12} />
          {busy === "image" ? "导入中..." : "添加图片"}
        </button>
      )}
      <button
        type="button"
        onClick={() => coverInput.current?.click()}
        disabled={Boolean(busy)}
        className="inline-flex h-8.5 cursor-pointer items-center gap-1.5 rounded-xl border border-line bg-surface px-3 text-xs font-medium text-ink-secondary shadow-[0_1px_2px_rgba(51,43,41,0.03)] transition-all hover:border-accent hover:text-accent-deep disabled:cursor-not-allowed disabled:opacity-50"
      >
        <Star size={12} />
        {busy === "cover" ? "导入中..." : editor.cover ? "更换封面" : "设置封面"}
      </button>
      {editor.cover && (
        <button
          type="button"
          onClick={() => updateEditor({ cover: null })}
          className="inline-flex h-8.5 cursor-pointer items-center gap-1 rounded-xl border border-transparent px-2.5 text-xs text-ink-muted transition-colors hover:border-coral-border hover:bg-coral-bg hover:text-coral-text"
        >
          <X size={11} /> 清除封面
        </button>
      )}
    </div>
  );

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

  const coverCard = (
    <div className="h-fit rounded-2xl border border-line bg-surface p-4 shadow-[0_1px_4px_rgba(51,43,41,0.04)]">
      <div className="mb-3 flex items-center justify-between gap-2">
        <span className="text-xs font-semibold text-ink">封面预览</span>
        <span className="truncate text-[10.5px] text-ink-muted">
          {editor.cover ? editor.cover.originalName : "未设置"}
        </span>
      </div>
      {editor.cover ? (
        <div className="group relative overflow-hidden rounded-xl border border-line">
          <MediaThumb
            source={{ kind: "content", packageId: editor.id }}
            asset={editor.cover}
            className="aspect-[16/10] w-full"
          />
          <button
            type="button"
            onClick={() => coverInput.current?.click()}
            className="absolute right-2 bottom-2 cursor-pointer rounded-lg bg-black/60 px-2.5 py-1 text-[10.5px] font-medium text-white opacity-0 backdrop-blur-sm transition-opacity group-hover:opacity-100"
          >
            更换封面
          </button>
        </div>
      ) : (
        <button
          type="button"
          onClick={() => coverInput.current?.click()}
          disabled={Boolean(busy)}
          className="flex aspect-[16/10] w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-line-strong bg-canvas/70 px-4 text-center transition-all hover:border-accent hover:bg-accent-soft/25 disabled:cursor-not-allowed disabled:opacity-50"
        >
          <span className="grid h-8 w-8 place-items-center rounded-xl bg-surface text-ink-muted shadow-xs ring-1 ring-line">
            <Star size={13} />
          </span>
          <span className="text-[11px] font-medium text-ink-secondary">点击上传封面</span>
        </button>
      )}
    </div>
  );

  return (
    <div className="mx-auto flex w-full max-w-[840px] flex-col gap-6">
      {/* Document Header */}
      <div className="space-y-4 border-b border-line pb-5">
        {titleField}
        {tagsField}
        {mediaToolbar}
      </div>
      {fileInputs}

      {/* Video Mode */}
      {editor.kind === "video" && (
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_240px]">
          <div className="min-w-0 space-y-5">
            <div className="rounded-2xl border border-line bg-surface p-4 shadow-[0_1px_4px_rgba(51,43,41,0.04)]">
              <div className="mb-3 flex items-center justify-between gap-2">
                <span className="text-xs font-semibold text-ink">主视频</span>
                {video ? (
                  <span className="truncate text-[11px] text-ink-muted">
                    {video.originalName}
                    {video.byteSize ? ` · ${formatByteSize(video.byteSize)}` : ""}
                  </span>
                ) : (
                  <span className="text-[11px] text-ink-muted">未导入</span>
                )}
              </div>
              {video ? (
                <div className="space-y-2.5">
                  <MediaThumb
                    source={{ kind: "content", packageId: editor.id }}
                    asset={video}
                    controls
                    className="aspect-video w-full bg-black"
                  />
                  <div className="flex items-center justify-end gap-2">
                    <button
                      type="button"
                      onClick={() => removeMedia(video.id)}
                      className="inline-flex cursor-pointer items-center gap-1 rounded-lg px-2 py-1 text-[11px] text-ink-muted transition-colors hover:bg-coral-bg hover:text-coral-text"
                    >
                      <Trash2 size={11} /> 移除视频
                    </button>
                  </div>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => videoInput.current?.click()}
                  disabled={Boolean(busy)}
                  className="flex aspect-video w-full cursor-pointer flex-col items-center justify-center gap-2.5 rounded-xl border border-dashed border-line-strong bg-canvas/70 px-6 text-center transition-all hover:border-accent hover:bg-accent-soft/25 disabled:cursor-not-allowed disabled:opacity-50"
                >
                  <span className="grid h-11 w-11 place-items-center rounded-2xl bg-surface text-accent shadow-xs ring-1 ring-line">
                    <Video size={17} />
                  </span>
                  <div>
                    <strong className="block text-xs font-medium text-ink">点击导入主视频</strong>
                    <p className="mt-1 text-[11px] leading-relaxed text-ink-muted">
                      支持 MP4 / MOV / WebM · 视频主稿不要求 Markdown 正文
                    </p>
                  </div>
                </button>
              )}
            </div>

            <div className="rounded-2xl border border-line bg-surface p-4 shadow-[0_1px_4px_rgba(51,43,41,0.04)] transition-all focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
              <div className="mb-2 flex items-center justify-between">
                <label htmlFor="video-summary" className="text-xs font-semibold text-ink">
                  视频简介
                </label>
                <span className="text-[10.5px] text-ink-muted">{editor.summary.length} 字</span>
              </div>
              <textarea
                id="video-summary"
                value={editor.summary}
                onChange={(event) => updateEditor({ summary: event.target.value })}
                placeholder="填写视频简介、章节时间轴或发布说明..."
                className="min-h-36 w-full resize-y bg-transparent text-[13.5px] leading-relaxed text-ink placeholder:text-ink-muted focus:outline-none"
              />
            </div>
          </div>
          {coverCard}
        </div>
      )}

      {/* ImageText Mode */}
      {editor.kind === "imageText" && (
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_240px]">
          <div className="min-w-0 space-y-5">
            <div className="rounded-2xl border border-line bg-surface p-4 shadow-[0_1px_4px_rgba(51,43,41,0.04)] transition-all focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
              <div className="mb-2 flex items-center justify-between">
                <label htmlFor="imagetext-summary" className="text-xs font-semibold text-ink">
                  短文案
                </label>
                <span className="text-[10.5px] text-ink-muted">{editor.summary.length} 字</span>
              </div>
              <textarea
                id="imagetext-summary"
                value={editor.summary}
                onChange={(event) => updateEditor({ summary: event.target.value })}
                placeholder="输入图文短文案；逐图说明可在下方图片卡片中维护..."
                className="min-h-28 w-full resize-y bg-transparent text-[13.5px] leading-relaxed text-ink placeholder:text-ink-muted focus:outline-none"
              />
            </div>

            <div className="rounded-2xl border border-line bg-surface p-4 shadow-[0_1px_4px_rgba(51,43,41,0.04)]">
              <div className="mb-3 flex items-center justify-between">
                <span className="text-xs font-semibold text-ink">图片顺序与说明</span>
                <span className="text-[11px] text-ink-muted">
                  {images.length} 张 · 拖拽或使用箭头排序
                </span>
              </div>
              {images.length === 0 ? (
                <button
                  type="button"
                  onClick={() => imageInput.current?.click()}
                  disabled={Boolean(busy)}
                  className="flex w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-line-strong bg-canvas/70 px-6 py-10 text-center transition-all hover:border-accent hover:bg-accent-soft/25"
                >
                  <span className="grid h-10 w-10 place-items-center rounded-2xl bg-surface text-accent shadow-xs ring-1 ring-line">
                    <Image size={16} />
                  </span>
                  <strong className="text-xs font-medium text-ink">点击添加图集图片</strong>
                  <span className="text-[11px] text-ink-muted">
                    支持多选导入 PNG / JPG / WebP，支持拖拽调整展示顺序
                  </span>
                </button>
              ) : (
                <div className="grid gap-3.5 sm:grid-cols-2">
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
                      className={`rounded-xl border bg-canvas/50 p-3 transition-all ${
                        editor.cover?.id === asset.id
                          ? "border-accent ring-1 ring-accent/20"
                          : "border-line hover:border-line-strong"
                      }`}
                    >
                      <div className="relative">
                        <MediaThumb
                          source={{ kind: "content", packageId: editor.id }}
                          asset={asset}
                          className="h-36 w-full"
                        />
                        <span className="absolute top-2 left-2 rounded-md bg-black/65 px-1.5 py-0.5 text-[10px] font-medium text-white backdrop-blur-xs">
                          {index + 1}
                        </span>
                        {editor.cover?.id === asset.id && (
                          <span className="absolute top-2 right-2 rounded-md bg-accent px-1.5 py-0.5 text-[10px] font-medium text-accent-on">
                            封面
                          </span>
                        )}
                      </div>
                      <input
                        value={asset.caption}
                        onChange={(event) => updateMedia(asset.id, { caption: event.target.value })}
                        placeholder="添加图片说明..."
                        className="mt-2.5 w-full rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs text-ink outline-none focus:border-accent"
                      />
                      <div className="mt-2 flex items-center justify-between">
                        <div className="flex gap-1">
                          <button
                            type="button"
                            aria-label="上移图片"
                            disabled={index === 0}
                            onClick={() => moveMedia(asset.id, -1)}
                            className="grid h-6.5 w-6.5 cursor-pointer place-items-center rounded-lg border border-line bg-surface text-ink-muted transition-colors hover:text-ink disabled:cursor-not-allowed disabled:opacity-30"
                          >
                            <ArrowUp size={10} />
                          </button>
                          <button
                            type="button"
                            aria-label="下移图片"
                            disabled={index === images.length - 1}
                            onClick={() => moveMedia(asset.id, 1)}
                            className="grid h-6.5 w-6.5 cursor-pointer place-items-center rounded-lg border border-line bg-surface text-ink-muted transition-colors hover:text-ink disabled:cursor-not-allowed disabled:opacity-30"
                          >
                            <ArrowDown size={10} />
                          </button>
                        </div>
                        <div className="flex gap-1.5">
                          <button
                            type="button"
                            onClick={() => setCover(asset)}
                            className="cursor-pointer rounded-lg border border-line bg-surface px-2 py-1 text-[10.5px] font-medium text-ink-secondary transition-colors hover:border-accent hover:text-accent-deep"
                          >
                            设为封面
                          </button>
                          <button
                            type="button"
                            aria-label="删除图片"
                            onClick={() => removeMedia(asset.id)}
                            className="grid h-6.5 w-6.5 cursor-pointer place-items-center rounded-lg border border-line bg-surface text-ink-muted transition-colors hover:border-coral-border hover:bg-coral-bg hover:text-coral-text"
                          >
                            <Trash2 size={10} />
                          </button>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
          {coverCard}
        </div>
      )}

      {/* Article / ImageArticle Mode */}
      {(editor.kind === "imageArticle" || editor.kind === "article") && (
        <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_240px]">
          <div className="min-w-0 space-y-5">
            <div className="rounded-2xl border border-line bg-surface px-4 py-3 shadow-[0_1px_4px_rgba(51,43,41,0.04)] transition-all focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
              <label htmlFor="article-summary" className="block text-[11px] font-semibold text-ink-muted">
                摘要
              </label>
              <input
                id="article-summary"
                value={editor.summary}
                onChange={(event) => updateEditor({ summary: event.target.value })}
                placeholder="用于平台稿与列表预览的一句话摘要..."
                className="mt-1 w-full bg-transparent text-[13px] text-ink placeholder:text-ink-muted focus:outline-none"
              />
            </div>

            <div className="relative">
              {preview ? (
                <div className="rounded-2xl border border-line bg-surface p-6 shadow-[0_1px_4px_rgba(51,43,41,0.04)]">
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
                  className="min-h-[calc(100vh-380px)] w-full resize-none rounded-2xl border border-line bg-surface p-6 font-mono text-[13.5px] leading-[1.85] text-ink shadow-[0_1px_4px_rgba(51,43,41,0.04)] outline-none transition-all focus:border-accent focus:ring-2 focus:ring-accent/15"
                />
              )}
              {slashOpen && !preview && (
                <div className="absolute top-12 right-5 z-20 w-56 overflow-hidden rounded-xl border border-line bg-surface p-1 shadow-[0_12px_32px_rgba(51,43,41,0.12)]">
                  <div className="px-2.5 py-1.5 text-[10.5px] font-medium text-ink-muted">
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
                      className="flex w-full cursor-pointer items-center justify-between rounded-lg px-2.5 py-2 text-left text-xs text-ink-secondary transition-colors hover:bg-hover hover:text-ink"
                    >
                      {label}
                      <Plus size={10} />
                    </button>
                  ))}
                </div>
              )}
            </div>

            {editor.kind === "imageArticle" && (
              <div className="rounded-2xl border border-line bg-surface p-4 shadow-[0_1px_4px_rgba(51,43,41,0.04)]">
                <div className="mb-3 flex items-center justify-between">
                  <span className="text-xs font-semibold text-ink">内联媒体</span>
                  <span className="text-[11px] text-ink-muted">{images.length} 张图片</span>
                </div>
                {images.length === 0 ? (
                  <p className="rounded-xl bg-canvas px-4 py-4 text-xs text-ink-muted">
                    点击上方「添加图片」导入后可插入正文，图片说明保存在{" "}
                    <code className="rounded bg-subtle px-1 py-0.5 text-[11px]">
                      ![说明](asset:...)
                    </code>{" "}
                    中。
                  </p>
                ) : (
                  <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                    {images.map((asset) => (
                      <div key={asset.id} className="rounded-xl border border-line bg-canvas/50 p-2">
                        <MediaThumb
                          source={{ kind: "content", packageId: editor.id }}
                          asset={asset}
                          className="h-24 w-full"
                        />
                        <input
                          value={asset.caption}
                          onChange={(event) =>
                            updateMedia(asset.id, { caption: event.target.value })
                          }
                          placeholder="图片说明"
                          className="mt-2 w-full rounded-lg border border-line bg-surface px-2 py-1 text-[11px] text-ink outline-none focus:border-accent"
                        />
                        <button
                          type="button"
                          onClick={() => insertMediaReference(asset)}
                          className="mt-1.5 w-full cursor-pointer rounded-lg bg-surface py-1.5 text-[11px] font-medium text-ink-secondary ring-1 ring-line transition-colors hover:bg-accent-soft hover:text-accent-deep hover:ring-accent-border"
                        >
                          插入到光标
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
          {coverCard}
        </div>
      )}
    </div>
  );
}
