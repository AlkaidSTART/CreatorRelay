import { useMemo, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { ArrowDown, ArrowUp, Image, Plus, Star, Trash2, Video, X } from "../Icons";
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
      placeholder="输入标题"
      className="w-full border-0 bg-transparent text-3xl font-semibold tracking-tight text-ink placeholder:text-ink-faint focus:outline-none"
    />
  );

  const tagsField = (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-[11px] text-ink-muted">标签</span>
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
        className="min-w-[220px] flex-1 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs text-ink outline-none focus:border-accent"
      />
      {editor.source && (
        <span className="rounded-md bg-sky-bg px-2 py-1 text-[10.5px] text-sky-text">
          派生自 {editor.source.contentId.slice(0, 8)}
        </span>
      )}
    </div>
  );

  const mediaToolbar = (
    <div className="flex flex-wrap gap-2">
      {(editor.kind === "video" || editor.kind === "imageArticle" || editor.kind === "article") && (
        <button
          type="button"
          onClick={() => videoInput.current?.click()}
          disabled={Boolean(busy) || (editor.kind === "video" && Boolean(video))}
          className="inline-flex items-center gap-1.5 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs font-medium text-ink-secondary hover:border-accent disabled:opacity-50"
        >
          <Video size={12} />
          {busy === "video" ? "导入中..." : video ? "替换视频" : "导入视频"}
        </button>
      )}
      {(editor.kind === "imageText" ||
        editor.kind === "imageArticle" ||
        editor.kind === "article") && (
        <button
          type="button"
          onClick={() => imageInput.current?.click()}
          disabled={Boolean(busy)}
          className="inline-flex items-center gap-1.5 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs font-medium text-ink-secondary hover:border-accent disabled:opacity-50"
        >
          <Image size={12} />
          {busy === "image" ? "导入中..." : "添加图片"}
        </button>
      )}
      <button
        type="button"
        onClick={() => coverInput.current?.click()}
        disabled={Boolean(busy)}
        className="inline-flex items-center gap-1.5 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-xs font-medium text-ink-secondary hover:border-accent disabled:opacity-50"
      >
        <Star size={12} />
        {busy === "cover" ? "导入中..." : "设置封面"}
      </button>
      {editor.cover && (
        <button
          type="button"
          onClick={() => updateEditor({ cover: null })}
          className="inline-flex items-center gap-1 rounded-lg border border-line px-2 py-1.5 text-[11px] text-ink-muted hover:text-coral-text"
        >
          <X size={10} /> 清除封面
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
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold text-ink">封面</span>
        <span className="text-[10.5px] text-ink-muted">
          {editor.cover ? editor.cover.originalName : "未设置"}
        </span>
      </div>
      <MediaThumb
        source={{ kind: "content", packageId: editor.id }}
        asset={editor.cover ?? null}
        className="h-32 w-full"
      />
    </div>
  );

  return (
    <div className="mx-auto flex w-full max-w-[900px] flex-col gap-5">
      {titleField}
      {tagsField}
      <div className="flex flex-wrap items-center gap-2 border-b border-line pb-4">
        {mediaToolbar}
      </div>
      {fileInputs}

      {editor.kind === "video" && (
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_260px]">
          <div className="space-y-3">
            <span className="text-xs font-semibold text-ink">主视频</span>
            <MediaThumb
              source={{ kind: "content", packageId: editor.id }}
              asset={video}
              controls
              className="h-64 w-full bg-black"
            />
            {!video && (
              <p className="rounded-xl border border-dashed border-line bg-canvas px-4 py-6 text-center text-xs text-ink-muted">
                导入一个主视频。视频内容不要求 Markdown 正文，也不会被 SQLite 覆盖。
              </p>
            )}
            <label className="block text-xs font-semibold text-ink">
              简介
              <textarea
                value={editor.summary}
                onChange={(event) => updateEditor({ summary: event.target.value })}
                placeholder="填写视频简介、章节或发布说明"
                className="mt-2 min-h-32 w-full resize-y rounded-xl border border-line bg-surface p-3 text-sm leading-relaxed text-ink outline-none focus:border-accent"
              />
            </label>
          </div>
          {coverCard}
        </div>
      )}

      {editor.kind === "imageText" && (
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_260px]">
          <div className="space-y-3">
            <span className="text-xs font-semibold text-ink">短文案</span>
            <textarea
              value={editor.summary}
              onChange={(event) => updateEditor({ summary: event.target.value })}
              placeholder="输入图文短文案；逐图说明在下方图片卡片中维护"
              className="min-h-28 w-full resize-y rounded-xl border border-line bg-surface p-3 text-sm leading-relaxed text-ink outline-none focus:border-accent"
            />
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-semibold text-ink">图片顺序与说明</span>
                <span className="text-[10.5px] text-ink-muted">
                  {images.length} 张 · 拖拽或使用箭头排序
                </span>
              </div>
              {images.length === 0 ? (
                <p className="rounded-xl border border-dashed border-line bg-canvas px-4 py-8 text-center text-xs text-ink-muted">
                  至少导入三张图片以完成 M5 图集验收。
                </p>
              ) : (
                <div className="grid gap-3 sm:grid-cols-2">
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
                      className={`rounded-xl border bg-surface p-2.5 ${
                        editor.cover?.id === asset.id ? "border-accent" : "border-line"
                      }`}
                    >
                      <div className="relative">
                        <MediaThumb
                          source={{ kind: "content", packageId: editor.id }}
                          asset={asset}
                          className="h-32 w-full"
                        />
                        <span className="absolute top-1.5 left-1.5 rounded bg-black/65 px-1.5 py-0.5 text-[10px] font-medium text-white">
                          {index + 1}
                        </span>
                        {editor.cover?.id === asset.id && (
                          <span className="absolute top-1.5 right-1.5 rounded bg-accent px-1.5 py-0.5 text-[10px] font-medium text-accent-on">
                            封面
                          </span>
                        )}
                      </div>
                      <input
                        value={asset.caption}
                        onChange={(event) => updateMedia(asset.id, { caption: event.target.value })}
                        placeholder="逐图说明"
                        className="mt-2 w-full rounded-lg border border-line bg-canvas px-2 py-1.5 text-[11px] text-ink outline-none focus:border-accent"
                      />
                      <div className="mt-1.5 flex items-center justify-between">
                        <div className="flex gap-1">
                          <button
                            type="button"
                            aria-label="上移图片"
                            disabled={index === 0}
                            onClick={() => moveMedia(asset.id, -1)}
                            className="grid h-6 w-6 place-items-center rounded border border-line text-ink-muted disabled:opacity-30"
                          >
                            <ArrowUp size={10} />
                          </button>
                          <button
                            type="button"
                            aria-label="下移图片"
                            disabled={index === images.length - 1}
                            onClick={() => moveMedia(asset.id, 1)}
                            className="grid h-6 w-6 place-items-center rounded border border-line text-ink-muted disabled:opacity-30"
                          >
                            <ArrowDown size={10} />
                          </button>
                        </div>
                        <div className="flex gap-1">
                          <button
                            type="button"
                            onClick={() => setCover(asset)}
                            className="rounded border border-line px-2 py-1 text-[10px] text-ink-secondary hover:border-accent"
                          >
                            设为封面
                          </button>
                          <button
                            type="button"
                            aria-label="删除图片"
                            onClick={() => removeMedia(asset.id)}
                            className="grid h-6 w-6 place-items-center rounded border border-line text-ink-muted hover:text-coral-text"
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

      {(editor.kind === "imageArticle" || editor.kind === "article") && (
        <div className="grid gap-5 lg:grid-cols-[minmax(0,1fr)_240px]">
          <div className="min-w-0 space-y-3">
            <label className="block text-xs font-semibold text-ink">
              摘要
              <input
                value={editor.summary}
                onChange={(event) => updateEditor({ summary: event.target.value })}
                placeholder="用于平台稿与列表预览的摘要"
                className="mt-1.5 w-full rounded-lg border border-line bg-surface px-2.5 py-2 text-xs text-ink outline-none focus:border-accent"
              />
            </label>
            <div className="relative">
              {preview ? (
                <MarkdownPreview
                  markdown={editor.body}
                  source={{ kind: "content", packageId: editor.id }}
                  assets={editor.media}
                />
              ) : (
                <textarea
                  ref={bodyRef}
                  value={editor.body}
                  onChange={(event) =>
                    handleBodyChange(event.target.value, event.target.selectionStart)
                  }
                  aria-label="Markdown 正文"
                  spellCheck={false}
                  placeholder="开始写 Markdown；输入 / 唤起块级命令"
                  className="min-h-[calc(100vh-340px)] w-full resize-none rounded-xl border border-line bg-surface p-4 font-mono text-[13px] leading-[1.8] text-ink outline-none focus:border-accent"
                />
              )}
              {slashOpen && !preview && (
                <div className="absolute top-10 right-4 z-20 w-56 overflow-hidden rounded-xl border border-line bg-surface shadow-xl">
                  <div className="border-b border-line px-3 py-2 text-[10.5px] font-medium text-ink-muted">
                    Slash Commands
                  </div>
                  {[
                    ["二级标题", "## 标题\n"],
                    ["引用", "> 引用内容\n"],
                    ["代码块", "```\n\n```\n"],
                    ["无序列表", "- 项目\n"],
                  ].map(([label, snippet]) => (
                    <button
                      key={label}
                      type="button"
                      onClick={() => insertSlash(snippet)}
                      className="flex w-full items-center justify-between px-3 py-2 text-left text-xs text-ink-secondary hover:bg-hover hover:text-ink"
                    >
                      {label}
                      <Plus size={10} />
                    </button>
                  ))}
                </div>
              )}
            </div>
            {editor.kind === "imageArticle" && (
              <div className="space-y-2">
                <span className="text-xs font-semibold text-ink">内联媒体</span>
                {images.length === 0 ? (
                  <p className="rounded-lg bg-canvas px-3 py-3 text-[11px] text-ink-muted">
                    添加图片后可插入正文，图片说明保存在 `![说明](asset:...)` 的 alt 文本中。
                  </p>
                ) : (
                  <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
                    {images.map((asset) => (
                      <div key={asset.id} className="rounded-lg border border-line p-1.5">
                        <MediaThumb
                          source={{ kind: "content", packageId: editor.id }}
                          asset={asset}
                          className="h-20 w-full"
                        />
                        <input
                          value={asset.caption}
                          onChange={(event) =>
                            updateMedia(asset.id, { caption: event.target.value })
                          }
                          placeholder="图片说明"
                          className="mt-1 w-full rounded border border-line bg-canvas px-1.5 py-1 text-[9.5px] text-ink outline-none focus:border-accent"
                        />
                        <button
                          type="button"
                          onClick={() => insertMediaReference(asset)}
                          className="mt-1 w-full rounded bg-sidebar py-1 text-[10px] text-ink-secondary hover:text-accent"
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
