import { describe, expect, it } from "vitest";

import { resolvePlatformLogoId } from "./PlatformLogo";

describe("resolvePlatformLogoId", () => {
  it.each([
    ["juejin", "掘金", "juejin"],
    ["csdn", "CSDN", "csdn"],
    ["zhihu", "知乎", "zhihu"],
    ["wechat", "微信公众号", "wechat"],
    ["bilibili", "Bilibili", "bilibili"],
    ["youtube", "YouTube", "youtube"],
    ["douyin", "抖音", "douyin"],
    ["kuaishou", "快手", "kuaishou"],
    ["xiaohongshu", "小红书", "xiaohongshu"],
    ["weibo", "微博", "weibo"],
  ])("resolves %s from platform id", (platformId, platformName, expected) => {
    expect(resolvePlatformLogoId(platformId, platformName)).toBe(expected);
  });

  it("resolves logos when only a display name is available", () => {
    expect(resolvePlatformLogoId(undefined, "微博")).toBe("weibo");
    expect(resolvePlatformLogoId(undefined, "微信公众号")).toBe("wechat");
    expect(resolvePlatformLogoId(undefined, "Bilibili")).toBe("bilibili");
  });

  it("returns undefined for an unknown platform", () => {
    expect(resolvePlatformLogoId("unknown", "未知平台")).toBeUndefined();
  });
});
