import { describe, expect, it } from "vitest";
import { DEFAULT_CREATOR_CENTERS, useCreatorCenterStore } from "./creatorCenterStore";

describe("creator center links", () => {
  it("starts with the requested creator centers in stable order", () => {
    expect(DEFAULT_CREATOR_CENTERS.map((item) => item.platformId)).toEqual([
      "douyin",
      "xiaohongshu",
      "zhihu",
      "wechat",
      "kuaishou",
      "bilibili",
      "juejin",
      "csdn",
    ]);
    expect(DEFAULT_CREATOR_CENTERS.find((item) => item.platformId === "wechat")?.url).toContain(
      "lang=zh_CN",
    );
  });

  it("allows changing a URL in preview and refuses non-HTTPS URLs", async () => {
    const store = useCreatorCenterStore;
    await store.getState().initialize();
    const updated = await store
      .getState()
      .saveUrl("douyin", "https://creator.douyin.com/workbench");
    expect(updated.url).toBe("https://creator.douyin.com/workbench");
    expect(store.getState().links.find((item) => item.platformId === "douyin")?.url).toBe(
      updated.url,
    );
    await expect(store.getState().saveUrl("douyin", "http://example.com")).rejects.toThrow("HTTPS");
  });
});
