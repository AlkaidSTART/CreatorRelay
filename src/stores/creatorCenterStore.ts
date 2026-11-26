import { invoke, isTauri } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { CreatorCenterLink } from "../types/creatorCenter";

export const DEFAULT_CREATOR_CENTERS: CreatorCenterLink[] = [
  {
    platformId: "douyin",
    platformName: "抖音",
    url: "https://creator.douyin.com/",
    updatedAt: "",
  },
  {
    platformId: "xiaohongshu",
    platformName: "小红书",
    url: "https://creator.xiaohongshu.com/new/home",
    updatedAt: "",
  },
  {
    platformId: "zhihu",
    platformName: "知乎",
    url: "https://www.zhihu.com/signin?next=%2F",
    updatedAt: "",
  },
  {
    platformId: "wechat",
    platformName: "微信公众号",
    url: "https://mp.weixin.qq.com/cgi-bin/home?t=home/index&lang=zh_CN",
    updatedAt: "",
  },
  {
    platformId: "kuaishou",
    platformName: "快手",
    url: "https://cp.kuaishou.com/profile",
    updatedAt: "",
  },
  {
    platformId: "bilibili",
    platformName: "哔哩哔哩",
    url: "https://member.bilibili.com/platform/home",
    updatedAt: "",
  },
  {
    platformId: "juejin",
    platformName: "稀土掘金",
    url: "https://juejin.cn/creator/home",
    updatedAt: "",
  },
  {
    platformId: "csdn",
    platformName: "CSDN",
    url: "https://mp.csdn.net/",
    updatedAt: "",
  },
];

interface CreatorCenterState {
  links: CreatorCenterLink[];
  loading: boolean;
  error: string;
  lastBrowsedPlatformId: string | null;
  initialize: () => Promise<void>;
  saveUrl: (platformId: string, url: string) => Promise<CreatorCenterLink>;
  setLastBrowsedPlatformId: (platformId: string | null) => void;
}

export const useCreatorCenterStore = create<CreatorCenterState>((set, get) => ({
  links: DEFAULT_CREATOR_CENTERS,
  loading: false,
  error: "",
  lastBrowsedPlatformId: null,
  setLastBrowsedPlatformId: (lastBrowsedPlatformId) => set({ lastBrowsedPlatformId }),
  initialize: async () => {
    if (!isTauri()) {
      set({ links: DEFAULT_CREATOR_CENTERS, loading: false, error: "" });
      return;
    }
    set({ loading: true, error: "" });
    try {
      const links = await invoke<CreatorCenterLink[]>("list_creator_center_links");
      set({ links, loading: false });
    } catch (error) {
      set({
        loading: false,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  },
  saveUrl: async (platformId, url) => {
    if (!isTauri()) {
      const existing = get().links.find((item) => item.platformId === platformId);
      if (!existing) throw new Error("未找到对应平台的创作者中心配置");
      const parsed = new URL(url.trim());
      if (parsed.protocol !== "https:") throw new Error("链接必须使用 HTTPS");
      if (parsed.username || parsed.password) throw new Error("链接不能包含用户名或密码");
      const updated = { ...existing, url: parsed.toString() };
      set((state) => ({
        links: state.links.map((item) => (item.platformId === platformId ? updated : item)),
      }));
      return updated;
    }
    const saved = await invoke<CreatorCenterLink>("save_creator_center_link", {
      platformId,
      url,
    });
    set((state) => ({
      links: state.links.map((item) => (item.platformId === platformId ? saved : item)),
    }));
    return saved;
  },
}));
