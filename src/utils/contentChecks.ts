import type { ContentKind } from "../types/plugin";
import type { ContentAsset } from "../types/content";

export type CheckLevel = "error" | "warning" | "unchecked";

export interface ContentCheckItem {
  id: string;
  level: CheckLevel;
  label: string;
  message: string;
}

export interface ContentCheckInput {
  kind: ContentKind;
  title: string;
  summary: string;
  body: string;
  cover?: ContentAsset | null;
  media: ContentAsset[];
}

export interface TargetRule {
  id: string;
  label: string;
  kinds: ContentKind[];
  minMedia: number;
  maxMedia: number;
  maxTitle: number;
  maxBody: number;
  requiresCover: boolean;
  requiresBody: boolean;
  mediaMaxBytes: number | null;
  unchecked: string[];
}

export interface CheckReport {
  targetId: string;
  targetLabel: string;
  verified: boolean;
  items: ContentCheckItem[];
  errorCount: number;
  warningCount: number;
  uncheckedCount: number;
}

export const TARGET_RULES: Record<string, TargetRule> = {
  "creatorrelay.official.test-adapter": {
    id: "creatorrelay.official.test-adapter",
    label: "CreatorRelay 测试适配器规则",
    kinds: ["imageText", "imageArticle", "article"],
    minMedia: 1,
    maxMedia: 9,
    maxTitle: 30,
    maxBody: 5000,
    requiresCover: true,
    requiresBody: true,
    mediaMaxBytes: 10 * 1024 * 1024,
    unchecked: ["视频时长与码率检查未在该测试适配器中提供"],
  },
  juejin: {
    id: "juejin",
    label: "掘金文章规则",
    kinds: ["article", "imageArticle"],
    minMedia: 0,
    maxMedia: 30,
    maxTitle: 80,
    maxBody: 100000,
    requiresCover: false,
    requiresBody: true,
    mediaMaxBytes: 20 * 1024 * 1024,
    unchecked: ["代码高亮兼容性未验证"],
  },
  csdn: {
    id: "csdn",
    label: "CSDN 文章规则",
    kinds: ["article", "imageArticle"],
    minMedia: 0,
    maxMedia: 30,
    maxTitle: 80,
    maxBody: 100000,
    requiresCover: false,
    requiresBody: true,
    mediaMaxBytes: 20 * 1024 * 1024,
    unchecked: ["CSDN 编辑器格式兼容性未验证"],
  },
  zhihu: {
    id: "zhihu",
    label: "知乎文章规则",
    kinds: ["article", "imageArticle"],
    minMedia: 0,
    maxMedia: 30,
    maxTitle: 80,
    maxBody: 100000,
    requiresCover: false,
    requiresBody: true,
    mediaMaxBytes: 20 * 1024 * 1024,
    unchecked: ["问答转文章适配未验证"],
  },
  xiaohongshu: {
    id: "xiaohongshu",
    label: "小红书图集规则",
    kinds: ["imageText", "video"],
    minMedia: 1,
    maxMedia: 18,
    maxTitle: 20,
    maxBody: 1000,
    requiresCover: true,
    requiresBody: false,
    mediaMaxBytes: 20 * 1024 * 1024,
    unchecked: ["视频时长、比例与音乐版权未提供可验证规则"],
  },
  douyin: {
    id: "douyin",
    label: "抖音内容规则",
    kinds: ["video", "imageText"],
    minMedia: 1,
    maxMedia: 35,
    maxTitle: 55,
    maxBody: 1000,
    requiresCover: true,
    requiresBody: false,
    mediaMaxBytes: 4 * 1024 * 1024 * 1024,
    unchecked: ["视频分辨率与帧率未由本地规则验证"],
  },
  bilibili: {
    id: "bilibili",
    label: "Bilibili 内容规则",
    kinds: ["video", "article", "imageArticle"],
    minMedia: 1,
    maxMedia: 1,
    maxTitle: 80,
    maxBody: 2000,
    requiresCover: true,
    requiresBody: false,
    mediaMaxBytes: 8 * 1024 * 1024 * 1024,
    unchecked: ["分P、字幕与合集规则未验证"],
  },
  youtube: {
    id: "youtube",
    label: "YouTube 视频规则",
    kinds: ["video"],
    minMedia: 1,
    maxMedia: 1,
    maxTitle: 100,
    maxBody: 5000,
    requiresCover: true,
    requiresBody: false,
    mediaMaxBytes: 256 * 1024 * 1024 * 1024,
    unchecked: ["视频编解码、缩略图尺寸与版权检查未验证"],
  },
  wechat: {
    id: "wechat",
    label: "微信公众号规则",
    kinds: ["article", "imageArticle", "imageText", "video"],
    minMedia: 0,
    maxMedia: 20,
    maxTitle: 64,
    maxBody: 20000,
    requiresCover: true,
    requiresBody: true,
    mediaMaxBytes: 20 * 1024 * 1024,
    unchecked: ["公众号草稿箱接口与素材上传限制未验证"],
  },
};

function structuralChecks(input: ContentCheckInput, rule: TargetRule): ContentCheckItem[] {
  const items: ContentCheckItem[] = [];
  const title = input.title.trim();
  const images = input.media.filter((asset) => asset.kind === "image");
  const videos = input.media.filter((asset) => asset.kind === "video");
  if (!title) {
    items.push({ id: "title-required", level: "error", label: "标题", message: "标题不能为空" });
  } else if (title.length > rule.maxTitle) {
    items.push({
      id: "title-length",
      level: "error",
      label: "标题长度",
      message: `标题 ${title.length} 字符，超过 ${rule.label} 上限 ${rule.maxTitle}`,
    });
  }
  const bodyRequired =
    rule.requiresBody && (input.kind === "article" || input.kind === "imageArticle");
  if (bodyRequired && !input.body.trim()) {
    items.push({ id: "body-required", level: "error", label: "正文", message: "该平台要求正文" });
  } else if (input.body.length > rule.maxBody) {
    items.push({
      id: "body-length",
      level: "error",
      label: "正文长度",
      message: `正文 ${input.body.length} 字符，超过 ${rule.maxBody}`,
    });
  }
  if (input.media.length < rule.minMedia) {
    items.push({
      id: "media-min",
      level: "error",
      label: "媒体数量",
      message: `至少需要 ${rule.minMedia} 个媒体，当前 ${input.media.length}`,
    });
  }
  if (input.media.length > rule.maxMedia) {
    items.push({
      id: "media-max",
      level: "error",
      label: "媒体数量",
      message: `最多允许 ${rule.maxMedia} 个媒体，当前 ${input.media.length}`,
    });
  }
  if (input.kind === "video" && videos.length !== 1) {
    items.push({
      id: "video-count",
      level: "error",
      label: "视频文件",
      message: "视频主稿必须包含且仅包含 1 个主视频",
    });
  }
  if (input.kind === "imageText" && images.length === 0) {
    items.push({
      id: "image-count",
      level: "error",
      label: "图片数量",
      message: "图集至少需要一张图片",
    });
  }
  if (rule.requiresCover && !input.cover) {
    items.push({ id: "cover-required", level: "error", label: "封面", message: "该平台要求设置封面" });
  }
  if (input.media.some((asset) => rule.mediaMaxBytes && asset.byteSize > rule.mediaMaxBytes)) {
    items.push({
      id: "media-size",
      level: "error",
      label: "素材大小",
      message: `存在超过 ${rule.mediaMaxBytes} bytes 的素材`,
    });
  }
  if (input.kind === "imageText" && images.some((asset) => !asset.caption.trim())) {
    items.push({
      id: "caption-completeness",
      level: "warning",
      label: "逐图说明",
      message: "部分图片没有说明；多数平台允许，但会影响可访问性",
    });
  }
  if (!input.summary.trim() && (input.kind === "video" || input.kind === "imageText")) {
    items.push({
      id: "summary-empty",
      level: "warning",
      label: "简介/短文案",
      message: "简介为空，平台稿将只使用标题",
    });
  }
  return items;
}

export function evaluateContentChecks(
  input: ContentCheckInput,
  targetId: string,
  targetLabel?: string,
): CheckReport {
  const rule = TARGET_RULES[targetId];
  if (!rule || !rule.kinds.includes(input.kind)) {
    const items: ContentCheckItem[] = [
      {
        id: "rule-missing",
        level: "unchecked",
        label: "平台规则",
        message: rule
          ? `${targetLabel ?? targetId} 未声明支持 ${input.kind}，不能把该形态显示为通过`
          : `${targetLabel ?? targetId} 未提供可验证的形态规则`,
      },
      {
        id: "media-format-unchecked",
        level: "unchecked",
        label: "媒体格式",
        message: "未验证该平台的媒体格式、尺寸与时长要求",
      },
      {
        id: "required-fields-unchecked",
        level: "unchecked",
        label: "必填字段",
        message: "未验证该平台的标题、封面和必填字段要求",
      },
    ];
    return {
      targetId,
      targetLabel: targetLabel ?? targetId,
      verified: false,
      items,
      errorCount: 0,
      warningCount: 0,
      uncheckedCount: items.length,
    };
  }
  const items = structuralChecks(input, rule);
  for (const reason of rule.unchecked) {
    items.push({
      id: `unchecked-${reason}`,
      level: "unchecked",
      label: "未验证检查",
      message: reason,
    });
  }
  const errorCount = items.filter((item) => item.level === "error").length;
  const warningCount = items.filter((item) => item.level === "warning").length;
  const uncheckedCount = items.filter((item) => item.level === "unchecked").length;
  return {
    targetId,
    targetLabel: targetLabel ?? rule.label,
    verified: true,
    items,
    errorCount,
    warningCount,
    uncheckedCount,
  };
}
