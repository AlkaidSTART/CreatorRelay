export type ThemeId = "taro" | "matcha" | "peach" | "seasalt" | "espresso";

export interface ThemeDefinition {
  id: ThemeId;
  name: string;
  enName: string;
  description: string;
  isDark: boolean;
  swatches: [string, string, string];
}

export const THEME_STORAGE_KEY = "creatorrelay:theme";

export const THEMES: ThemeDefinition[] = [
  {
    id: "taro",
    name: "石墨靛蓝",
    enName: "Studio Slate",
    description: "中性冷灰画布配利落靛蓝，高对比度专业工作台风格。",
    isDark: false,
    swatches: ["#F7F7F8", "#EEF2FF", "#4F46E5"],
  },
  {
    id: "matcha",
    name: "松烟墨绿",
    enName: "Pine Studio",
    description: "低饱和度灰绿基底配深松绿，适合长时间专注写作。",
    isDark: false,
    swatches: ["#F6F8F7", "#EAF5F0", "#1E6F50"],
  },
  {
    id: "peach",
    name: "暖石素纸",
    enName: "Editorial Paper",
    description: "温润纸张色调搭配陶土红强调色，贴近编辑部排版质感。",
    isDark: false,
    swatches: ["#F8F6F3", "#FBF0ED", "#B44C37"],
  },
  {
    id: "seasalt",
    name: "极地钴蓝",
    enName: "Nordic Blue",
    description: "冷峻蓝灰界面与清晰钴蓝，适合数据分析与多平台运维。",
    isDark: false,
    swatches: ["#F6F8FB", "#EFF6FF", "#2563EB"],
  },
  {
    id: "espresso",
    name: "深空石墨",
    enName: "Obsidian Dark",
    description: "深炭黑层级与克制微光强调，暗光环境下依然锐利清晰。",
    isDark: true,
    swatches: ["#0F1012", "#18191D", "#6E6BF0"],
  },
];

export function isThemeId(value: unknown): value is ThemeId {
  return THEMES.some((theme) => theme.id === value);
}

export function getInitialTheme(): ThemeId {
  if (typeof window === "undefined") return "taro";
  try {
    const saved = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (isThemeId(saved)) return saved;
  } catch {
    // Ignore storage access errors in restricted environments.
  }
  return "taro";
}

export function applyThemeToDocument(theme: ThemeId) {
  if (typeof document !== "undefined") {
    document.documentElement.dataset.theme = theme;
  }
  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // Ignore storage write errors in restricted environments.
    }
  }
}
