import { describe, expect, it } from "vitest";
import type { ContentAsset } from "../types/content";
import { buildDraftTargets } from "./contentTargets";
import { evaluateContentChecks } from "./contentChecks";
import { demoPlatformAccounts } from "../data/demoData";
import type { PluginSummary } from "../types/plugin";

function image(id: string, order: number, bytes = 1024): ContentAsset {
  return {
    id,
    kind: "image",
    file: `assets/${id}.png`,
    originalName: `${id}.png`,
    mimeType: "image/png",
    byteSize: bytes,
    caption: `说明 ${order}`,
    order,
  };
}

const plugin: PluginSummary = {
  id: "creatorrelay.official.test-adapter",
  name: "CreatorRelay 测试适配器",
  description: "",
  version: "1.0.0",
  adapterContractVersion: 1,
  pluginType: "adapter",
  runtimeKind: "coreTestRunner",
  entry: "builtin://test",
  source: "BUNDLED",
  trustLevel: "OFFICIAL",
  status: "ENABLED",
  enabled: true,
  runtimeState: "running",
  sandboxed: false,
  isolation: { mode: "process-only", strong: false, network: "controlled", description: "" },
  manifestHash: "hash",
  installPath: "/tmp/plugin",
  installedAt: "2026-11-25T00:00:00Z",
  updatedAt: "2026-11-25T00:00:00Z",
  capabilities: [
    {
      kind: "video",
      label: "视频",
      declaredRead: "supported",
      declaredPublish: "supported",
      effectiveRead: "supported",
      effectivePublish: "unsupported",
      source: "probe",
      reason: "runtime probe says unsupported",
      checkedAt: "2026-11-25T00:00:00Z",
    },
    {
      kind: "imageText",
      label: "图文",
      declaredRead: "supported",
      declaredPublish: "supported",
      effectiveRead: "supported",
      effectivePublish: "supported",
      source: "probe",
      reason: null,
      checkedAt: "2026-11-25T00:00:00Z",
    },
  ],
  permissions: [],
};

describe("content targets and checks", () => {
  it("filters video out of an adapter that only supports image text", () => {
    const videoTargets = buildDraftTargets("video", [], [plugin]);
    expect(videoTargets).toHaveLength(1);
    expect(videoTargets[0].selectable).toBe(false);
    expect(videoTargets[0].capability).toBe("unsupported");
    expect(videoTargets[0].reason).toContain("未声明支持");

    const imageTargets = buildDraftTargets("imageText", [], [plugin]);
    expect(imageTargets[0].selectable).toBe(true);
    expect(imageTargets[0].capability).toBe("supported");
  });

  it("only offers active platform accounts that declare the content kind", () => {
    const targets = buildDraftTargets("video", demoPlatformAccounts, []);
    expect(targets.find((target) => target.platformId === "juejin")?.selectable).toBe(false);
    expect(targets.some((target) => target.platformId === "bilibili" && target.selectable)).toBe(
      true,
    );
  });

  it("reports hard errors, warnings and unknown checks separately", () => {
    const report = evaluateContentChecks(
      {
        kind: "imageText",
        title: "",
        summary: "",
        body: "",
        cover: null,
        media: [image("a", 1, 20 * 1024 * 1024)],
      },
      "creatorrelay.official.test-adapter",
    );
    expect(report.verified).toBe(true);
    expect(report.errorCount).toBeGreaterThanOrEqual(3);
    expect(report.warningCount).toBeGreaterThan(0);
    expect(report.uncheckedCount).toBeGreaterThan(0);
    expect(report.items.some((item) => item.id === "cover-required")).toBe(true);
  });

  it("does not turn an unknown platform rule into a pass", () => {
    const report = evaluateContentChecks(
      {
        kind: "article",
        title: "标题",
        summary: "摘要",
        body: "正文",
        cover: null,
        media: [],
      },
      "unknown-platform",
      "未知平台",
    );
    expect(report.verified).toBe(false);
    expect(report.items.every((item) => item.level === "unchecked")).toBe(true);
  });

  it("passes a complete test-adapter image text draft", () => {
    const media = [image("a", 1), image("b", 2), image("c", 3)];
    const report = evaluateContentChecks(
      {
        kind: "imageText",
        title: "三图短文案",
        summary: "短文案",
        body: "",
        cover: media[0],
        media,
      },
      "creatorrelay.official.test-adapter",
    );
    expect(report.errorCount).toBe(0);
    expect(report.items.some((item) => item.level === "unchecked")).toBe(true);
  });
});
