import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { Image as ImageIcon, Video } from "../Icons";
import type { ContentAsset } from "../../types/content";

export type AssetSource =
  { kind: "content"; packageId: string } | { kind: "draft"; draftId: string };

export function useAssetObjectUrl(source: AssetSource | null, asset: ContentAsset | null) {
  const [url, setUrl] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    let active = true;
    let objectUrl = "";
    setUrl("");
    setError("");
    if (!source || !asset || !isTauri()) return () => undefined;
    const command = source.kind === "content" ? "read_content_asset" : "read_platform_draft_asset";
    const args =
      source.kind === "content"
        ? { packageId: source.packageId, assetId: asset.id }
        : { draftId: source.draftId, assetId: asset.id };
    void invoke<{ bytes: number[]; mimeType: string }>(command, args)
      .then((result) => {
        if (!active) return;
        const blob = new Blob([new Uint8Array(result.bytes)], { type: result.mimeType });
        objectUrl = URL.createObjectURL(blob);
        setUrl(objectUrl);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    return () => {
      active = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [source, asset]);
  return { url, error };
}

export function MediaThumb({
  source,
  asset,
  className = "",
  controls = false,
}: {
  source: AssetSource | null;
  asset: ContentAsset | null;
  className?: string;
  controls?: boolean;
}) {
  const { url, error } = useAssetObjectUrl(source, asset);
  if (!asset) {
    return (
      <div className={`grid place-items-center rounded-lg bg-sidebar text-ink-faint ${className}`}>
        <ImageIcon size={18} />
      </div>
    );
  }
  if (error) {
    return (
      <div
        className={`grid place-items-center rounded-lg border border-coral-border bg-coral-bg p-2 text-center text-[10px] text-coral-text ${className}`}
      >
        素材读取失败
      </div>
    );
  }
  if (!url) {
    return (
      <div className={`grid place-items-center rounded-lg bg-sidebar text-ink-faint ${className}`}>
        {asset.kind === "video" ? <Video size={18} /> : <ImageIcon size={18} />}
      </div>
    );
  }
  if (asset.kind === "video") {
    return (
      <video
        src={url}
        controls={controls}
        className={`rounded-lg bg-black object-cover ${className}`}
      />
    );
  }
  return (
    <img
      src={url}
      alt={asset.caption || asset.originalName}
      className={`rounded-lg object-cover ${className}`}
    />
  );
}
