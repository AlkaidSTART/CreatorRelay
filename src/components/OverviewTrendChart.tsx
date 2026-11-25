import { useEffect, useRef } from "react";
import * as echarts from "echarts/core";
import { LineChart, type LineSeriesOption } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  TooltipComponent,
  type GridComponentOption,
  type LegendComponentOption,
  type TooltipComponentOption,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { OverviewTrendDataset } from "../data/demoData";
import { useUiStore } from "../stores";

echarts.use([LineChart, GridComponent, LegendComponent, TooltipComponent, CanvasRenderer]);

export type OverviewTrendOption = echarts.ComposeOption<
  LineSeriesOption | GridComponentOption | TooltipComponentOption | LegendComponentOption
>;

interface TooltipDatum {
  axisValueLabel?: string;
  color?: string;
  seriesName?: string;
  value?: number | string;
}

interface ChartThemeColors {
  surface: string;
  line: string;
  lineStrong: string;
  splitLine: string;
  ink: string;
  inkSecondary: string;
  inkMuted: string;
}

function readChartThemeColors(): ChartThemeColors {
  if (typeof document === "undefined" || typeof getComputedStyle === "undefined") {
    return {
      surface: "#ffffff",
      line: "#EAE2D9",
      lineStrong: "#DCD2C7",
      splitLine: "#F4EFEA",
      ink: "#332B29",
      inkSecondary: "#6F6561",
      inkMuted: "#9A8F8A",
    };
  }
  const styles = getComputedStyle(document.documentElement);
  const pick = (name: string, fallback: string) => styles.getPropertyValue(name).trim() || fallback;
  return {
    surface: pick("--bg-surface", "#ffffff"),
    line: pick("--border-hairline", "#EAE2D9"),
    lineStrong: pick("--border-strong", "#DCD2C7"),
    splitLine: pick("--bg-subtle", "#F4EFEA"),
    ink: pick("--ink-primary", "#332B29"),
    inkSecondary: pick("--ink-secondary", "#6F6561"),
    inkMuted: pick("--ink-muted", "#9A8F8A"),
  };
}

export function buildOverviewTrendOption(dataset: OverviewTrendDataset): OverviewTrendOption {
  const colors = readChartThemeColors();
  const metricByPlatform = new Map(
    dataset.series.map((item) => [item.platformName, item.metricLabel]),
  );

  return {
    animationDuration: 450,
    color: dataset.series.map((item) => item.color),
    grid: {
      top: 40,
      right: 16,
      bottom: 24,
      left: 12,
      containLabel: true,
    },
    legend: {
      top: 0,
      left: 0,
      icon: "circle",
      itemWidth: 8,
      itemHeight: 8,
      itemGap: 16,
      textStyle: {
        color: colors.inkSecondary,
        fontSize: 12,
      },
    },
    tooltip: {
      trigger: "axis",
      backgroundColor: colors.surface,
      borderColor: colors.line,
      borderWidth: 1,
      padding: [10, 12],
      textStyle: {
        color: colors.ink,
        fontSize: 12,
      },
      axisPointer: {
        type: "line",
        lineStyle: {
          color: colors.lineStrong,
          type: "dashed",
        },
      },
      formatter: (params: unknown) => {
        const items = (Array.isArray(params) ? params : [params]) as TooltipDatum[];
        if (items.length === 0) return "";
        const dateLabel = items[0]?.axisValueLabel ?? "";
        const rows = items.map((item) => {
          const name = item.seriesName ?? "";
          const metricLabel = metricByPlatform.get(name) ?? name;
          const numericValue =
            typeof item.value === "number" ? item.value : Number(item.value ?? 0);
          const formatted = Number.isNaN(numericValue)
            ? String(item.value ?? "-")
            : numericValue.toLocaleString("zh-CN");
          const dotColor = typeof item.color === "string" ? item.color : colors.ink;
          return [
            `<div style="display:flex;align-items:center;justify-content:space-between;gap:16px;margin-top:6px;">`,
            `<span style="display:inline-flex;align-items:center;gap:6px;color:${colors.inkSecondary};">`,
            `<span style="width:7px;height:7px;border-radius:999px;background:${dotColor};display:inline-block;"></span>`,
            `${name} · ${metricLabel}`,
            `</span>`,
            `<strong style="color:${colors.ink};font-variant-numeric:tabular-nums;">${formatted}</strong>`,
            `</div>`,
          ].join("");
        });
        return `<div style="font-weight:600;color:${colors.ink};margin-bottom:4px;">${dateLabel}</div>${rows.join("")}`;
      },
    },
    xAxis: {
      type: "category",
      boundaryGap: false,
      data: dataset.dates,
      axisLine: {
        lineStyle: {
          color: colors.line,
        },
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        color: colors.inkMuted,
        fontSize: 11,
        margin: 10,
      },
    },
    yAxis: {
      type: "value",
      splitNumber: 4,
      axisLine: {
        show: false,
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        color: colors.inkMuted,
        fontSize: 11,
      },
      splitLine: {
        lineStyle: {
          color: colors.splitLine,
        },
      },
    },
    series: dataset.series.map((item) => ({
      name: item.platformName,
      type: "line",
      smooth: 0.3,
      showSymbol: false,
      symbol: "circle",
      symbolSize: 6,
      data: item.values,
      lineStyle: {
        width: 2,
        color: item.color,
      },
      itemStyle: {
        color: item.color,
      },
      areaStyle: {
        color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
          { offset: 0, color: `${item.color}1f` },
          { offset: 1, color: `${item.color}00` },
        ]),
      },
      emphasis: {
        focus: "series",
      },
    })),
  };
}

interface OverviewTrendChartProps {
  dataset: OverviewTrendDataset;
  height?: number;
}

export function OverviewTrendChart({ dataset, height = 280 }: OverviewTrendChartProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<echarts.ECharts | null>(null);
  const theme = useUiStore((state) => state.theme);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const chart = echarts.init(el);
    chartRef.current = chart;

    const handleResize = () => {
      chart.resize();
    };

    let observer: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined") {
      observer = new ResizeObserver(handleResize);
      observer.observe(el);
    } else {
      window.addEventListener("resize", handleResize);
    }

    return () => {
      if (observer) {
        observer.disconnect();
      } else {
        window.removeEventListener("resize", handleResize);
      }
      chart.dispose();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!chartRef.current) return;
    chartRef.current.setOption(buildOverviewTrendOption(dataset), true);
  }, [dataset, theme]);

  return (
    <div
      ref={containerRef}
      className="w-full"
      style={{ height }}
      role="img"
      aria-label="各平台作品趋势折线图"
    />
  );
}
