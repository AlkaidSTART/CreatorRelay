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
    name: "芋泥奶油",
    enName: "Taro Cream",
    description: "杏仁奶白画布搭配柔雾薰衣草，温柔耐看。",
    isDark: false,
    swatches: ["#FAF7F4", "#F2ECF9", "#7B68A8"],
  },
  {
    id: "matcha",
    name: "开心果抹茶",
    enName: "Pistachio Mint",
    description: "晨露浅绿与鼠尾草绿，清新护眼、专注写作。",
    isDark: false,
    swatches: ["#F4F8F5", "#E5F3EC", "#4D8B6E"],
  },
  {
    id: "peach",
    name: "蜜桃落日",
    enName: "Rose Peach",
    description: "暖粉米白配柔雾玫瑰，柔和优雅的创作者格调。",
    isDark: false,
    swatches: ["#FCF7F6", "#FCECEB", "#C26474"],
  },
  {
    id: "seasalt",
    name: "海盐晨光",
    enName: "Sea Salt Sky",
    description: "雾蓝奶白配低饱和海盐蓝，清爽通透。",
    isDark: false,
    swatches: ["#F5F8FC", "#E9F2FA", "#4F81B5"],
  },
  {
    id: "espresso",
    name: "可可暗夜",
    enName: "Cocoa Dusk",
    description: "深可可灰紫基底与柔光薰衣草，夜间写作不刺眼。",
    isDark: true,
    swatches: ["#1B171A", "#2B242D", "#A993DB"],
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
