import type { ContentKind } from "./plugin";

export type { ContentKind };

export type AssetKind = "video" | "image";

export interface ContentAsset {
  id: string;
  kind: AssetKind;
  file: string;
  originalName: string;
  mimeType: string;
  byteSize: number;
  caption: string;
  order: number;
}

export interface ContentSourceRef {
  contentId: string;
  version: string;
}

export interface ContentPackage {
  schemaVersion: number;
  id: string;
  kind: ContentKind;
  title: string;
  summary: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  createdAt: string;
  updatedAt: string;
  source?: ContentSourceRef | null;
  body: string;
  path: string;
  contentHash: string;
  contentVersion: string;
}

export interface ContentPackageSummary {
  id: string;
  kind: ContentKind;
  title: string;
  summary: string;
  tags: string[];
  updatedAt: string;
  hasCover: boolean;
  mediaCount: number;
  path: string;
  contentVersion: string;
}

export interface ContentSaveInput {
  title: string;
  summary: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  body: string;
  baseHash?: string | null;
}

export interface PlatformDraft {
  id: string;
  contentId: string;
  platformId: string;
  platformName: string;
  accountId: string;
  accountAlias: string;
  adapterId?: string | null;
  title: string;
  summary: string;
  body: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  sourceVersion: string;
  generatedAt: string;
  updatedAt: string;
  manuallyEdited: boolean;
  hash: string;
  sourceChanged: boolean;
}

export interface DraftTarget {
  platformId: string;
  platformName: string;
  accountId: string;
  accountAlias: string;
  adapterId?: string | null;
}

export interface DraftSaveInput {
  title: string;
  summary: string;
  tags: string[];
  cover?: ContentAsset | null;
  media: ContentAsset[];
  body: string;
  baseHash: string;
}

export interface ContentAssetContent {
  bytes: number[];
  mimeType: string;
}

export const CONTENT_KIND_META: Array<{
  kind: ContentKind;
  label: string;
  description: string;
}> = [
  { kind: "video", label: "视频", description: "导入主视频、封面与简介，适合视频平台" },
  { kind: "imageText", label: "图文", description: "多图排序、封面、逐图说明与短文案" },
  { kind: "imageArticle", label: "图文文章", description: "Markdown 正文、内联媒体与分栏预览" },
  { kind: "article", label: "纯文章", description: "Markdown 长文、摘要、标签与阅读预览" },
];

export function contentKindLabel(kind: ContentKind): string {
  return CONTENT_KIND_META.find((item) => item.kind === kind)?.label ?? kind;
}
