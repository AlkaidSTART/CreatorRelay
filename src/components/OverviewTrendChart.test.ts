import { describe, expect, it } from "vitest";
import { demoOverviewTrends } from "../data/demoData";
import { buildOverviewTrendOption } from "./OverviewTrendChart";

describe("buildOverviewTrendOption", () => {
  it("builds per-platform line series matching the 7d dataset", () => {
    const dataset = demoOverviewTrends["7d"];
    const option = buildOverviewTrendOption(dataset);

    expect(option.xAxis).toMatchObject({
      type: "category",
      data: dataset.dates,
    });

    const series = Array.isArray(option.series) ? option.series : [];
    expect(series).toHaveLength(dataset.series.length);
    expect(series.map((item) => item.name)).toEqual(["掘金", "Bilibili", "CSDN", "知乎"]);
    for (const [index, item] of series.entries()) {
      expect(item.type).toBe("line");
      expect(item.data).toEqual(dataset.series[index].values);
    }
  });

  it("formats tooltip with each platform's original metric label", () => {
    const dataset = demoOverviewTrends["30d"];
    const option = buildOverviewTrendOption(dataset);
    const tooltip = Array.isArray(option.tooltip) ? option.tooltip[0] : option.tooltip;
    const formatter = tooltip?.formatter;

    if (typeof formatter !== "function") {
      throw new Error("Expected tooltip formatter to be a function");
    }

    const html = formatter(
      [
        {
          axisValueLabel: "11-25",
          seriesName: "掘金",
          value: 3450,
          color: "#18181b",
        },
        {
          axisValueLabel: "11-25",
          seriesName: "Bilibili",
          value: 5120,
          color: "#2563eb",
        },
      ] as never,
      "",
      () => {},
    );
    expect(String(html)).toContain("11-25");
    expect(String(html)).toContain("文章阅读（掘金口径）");
    expect(String(html)).toContain("视频播放（Bilibili 口径）");
    expect(String(html)).toContain("3,450");
    expect(String(html)).toContain("5,120");
  });
});
