import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import type { ContentAsset } from "../../types/content";
import type { AssetSource } from "./MediaAsset";

export function MarkdownPreview({
  markdown,
  source,
  assets,
}: {
  markdown: string;
  source: AssetSource | null;
  assets: ContentAsset[];
}) {
  const [urls, setUrls] = useState<Record<string, string>>({});
  useEffect(() => {
    if (!source || !isTauri() || assets.length === 0) {
      setUrls({});
      return () => undefined;
    }
    let active = true;
    const created: string[] = [];
    const command = source.kind === "content" ? "read_content_asset" : "read_platform_draft_asset";
    void Promise.all(
      assets
        .filter((asset) => asset.kind === "image")
        .map(async (asset) => {
          const args =
            source.kind === "content"
              ? { packageId: source.packageId, assetId: asset.id }
              : { draftId: source.draftId, assetId: asset.id };
          const result = await invoke<{ bytes: number[]; mimeType: string }>(command, args);
          const url = URL.createObjectURL(
            new Blob([new Uint8Array(result.bytes)], { type: result.mimeType }),
          );
          created.push(url);
          return [asset.id, url] as const;
        }),
    )
      .then((entries) => {
        if (active) setUrls(Object.fromEntries(entries));
      })
      .catch(() => {
        if (active) setUrls({});
      });
    return () => {
      active = false;
      created.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [source, assets]);

  const rendered = markdown.replace(/asset:([a-zA-Z0-9-]+)/gu, (match, assetId: string) => {
    return urls[assetId] ?? match;
  });

  return (
    <article className="studio-markdown-preview pb-12">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          img: ({ src, alt }) =>
            src?.startsWith("blob:") ? (
              <img
                src={src}
                alt={alt ?? ""}
                className="my-4 max-h-[520px] w-full rounded-xl border border-line object-contain"
              />
            ) : (
              <span className="my-2 block rounded-lg border border-dashed border-line bg-sidebar px-3 py-2 text-xs text-ink-muted">
                内联素材待载入：{alt || "图片"}
              </span>
            ),
        }}
      >
        {rendered}
      </ReactMarkdown>
    </article>
  );
}
