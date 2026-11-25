import type { ContentKind } from "./plugin";

export interface AiProvider {
  id: string;
  name: string;
  baseUrl: string;
  model: string;
  apiStyle: string;
  isDefault: boolean;
  configured: boolean;
  lastTestStatus?: string | null;
  lastTestError?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AiProviderInput {
  id?: string | null;
  name: string;
  baseUrl: string;
  model: string;
  apiStyle?: string;
  apiKey?: string | null;
  isDefault?: boolean;
}

export interface AiTestResult {
  providerId: string;
  ok: boolean;
  message: string;
  latencyMs: number;
  status?: string | null;
}

export interface AiMediaMetadata {
  kind: "image" | "video";
  fileName: string;
  mimeType: string;
  byteSize: number;
  caption: string;
  order: number;
}

export interface AiGenerateRequest {
  providerId: string;
  operation: string;
  contentKind: ContentKind;
  title: string;
  context: string;
  mediaMetadata: AiMediaMetadata[];
  instruction?: string | null;
}

export interface AiGenerateResult {
  requestId: string;
  providerId: string;
  providerName: string;
  model: string;
  operation: string;
  content: string;
}

export type AiOperation =
  | "title"
  | "summary"
  | "description"
  | "shortCopy"
  | "polish"
  | "structure"
  | "tone"
  | "contentCheck"
  | "custom";

export interface AiOperationMeta {
  operation: AiOperation;
  label: string;
  target: "title" | "summary" | "body";
  kinds: ContentKind[];
  description: string;
}

export const AI_OPERATIONS: AiOperationMeta[] = [
  {
    operation: "title",
    label: "生成标题",
    target: "title",
    kinds: ["video", "imageText", "imageArticle", "article"],
    description: "基于当前标题、摘要和已选素材元数据生成标题",
  },
  {
    operation: "description",
    label: "生成简介",
    target: "summary",
    kinds: ["video"],
    description: "生成视频简介，不会改动视频文件",
  },
  {
    operation: "shortCopy",
    label: "生成短文案",
    target: "summary",
    kinds: ["imageText"],
    description: "生成图集短文案，不改变图片顺序或封面",
  },
  {
    operation: "summary",
    label: "生成摘要",
    target: "summary",
    kinds: ["imageArticle", "article", "video", "imageText"],
    description: "基于用户确认发送的正文生成摘要",
  },
  {
    operation: "polish",
    label: "润色正文",
    target: "body",
    kinds: ["imageArticle", "article"],
    description: "只生成候选正文，应用后才替换编辑器内容",
  },
  {
    operation: "structure",
    label: "结构优化",
    target: "body",
    kinds: ["imageArticle", "article"],
    description: "返回重组后的 Markdown，保留图片引用",
  },
  {
    operation: "tone",
    label: "语气转换",
    target: "body",
    kinds: ["imageArticle", "article", "video", "imageText"],
    description: "按附加要求转换语气，结果需人工确认",
  },
  {
    operation: "contentCheck",
    label: "AI 内容检查",
    target: "body",
    kinds: ["video", "imageText", "imageArticle", "article"],
    description: "返回问题列表，不会自动修改内容",
  },
  {
    operation: "custom",
    label: "自定义 Prompt",
    target: "body",
    kinds: ["video", "imageText", "imageArticle", "article"],
    description: "只发送当前编辑内容与明确填写的附加要求",
  },
];
