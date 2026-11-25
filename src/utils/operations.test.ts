import { describe, expect, it } from "vitest";
import type { ExternalWork, PublishTask } from "../types/operations";
import {
  analyticsToTrendDataset,
  filterExternalWorks,
  filterPublishTasks,
  primaryMetric,
  workStatus,
} from "./operations";

const task = (patch: Partial<PublishTask>): PublishTask => ({
  id: "task-1",
  sourceType: "platformDraft",
  sourceId: "draft-1",
  contentId: "content-1",
  contentVersion: "v1",
  contentKind: "imageText",
  platformId: "juejin",
  platformName: "掘金",
  accountId: "account-1",
  accountAlias: "主号",
  title: "测试内容",
  body: "",
  tags: [],
  media: [],
  executionMode: "manual",
  status: "CREATED",
  paused: false,
  attemptCount: 0,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  ...patch,
});

const work = (patch: Partial<ExternalWork>): ExternalWork => ({
  id: "work-1",
  sourceKey: "sync:juejin:account-1:1",
  accountId: "account-1",
  platformId: "juejin",
  platformName: "掘金",
  accountAlias: "主号",
  externalId: "juejin-1",
  title: "测试作品",
  contentKind: "article",
  originalContentType: "文章",
  publishedAt: new Date().toISOString(),
  source: "sync",
  syncedAt: new Date().toISOString(),
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
  metrics: [
    {
      id: "metric-1",
      workId: "work-1",
      metricName: "文章阅读量",
      metricDefinition: "平台原始定义",
      unit: "次",
      value: 120,
      capturedAt: new Date().toISOString(),
      snapshotDay: "2026-11-25",
      source: "test",
      canonicalKey: null,
      comparable: false,
    },
  ],
  ...patch,
});

describe("M6 operations filters", () => {
  it("filters publish tasks by status, platform, account, kind and keyword", () => {
    const tasks = [
      task({ id: "a", status: "SUCCESS", title: "视频发布" }),
      task({ id: "b", status: "NEEDS_MANUAL_ACTION", platformId: "csdn", contentKind: "article" }),
    ];
    expect(
      filterPublishTasks(tasks, {
        status: "NEEDS_MANUAL_ACTION",
        platformId: "csdn",
        accountId: "account-1",
        contentKind: "article",
      }).map((item) => item.id),
    ).toEqual(["b"]);
    expect(filterPublishTasks(tasks, { keyword: "视频" }).map((item) => item.id)).toEqual(["a"]);
  });

  it("filters works by form, account, status and keyword", () => {
    const works = [work({}), work({ id: "manual", source: "manual", contentKind: "video" })];
    expect(filterExternalWorks(works, { contentKind: "article" })).toHaveLength(1);
    expect(filterExternalWorks(works, { status: "manual" })[0].id).toBe("manual");
    expect(workStatus(works[0])).toBe("synced");
  });

  it("uses a readable primary metric without merging platform definitions", () => {
    const item = work({});
    expect(primaryMetric(item)?.metricName).toBe("文章阅读量");
  });

  it("converts analytics series to chart data without inventing extra points", () => {
    const dataset = analyticsToTrendDataset({
      range: "7d",
      granularity: "day",
      from: "2026-11-23T00:00:00Z",
      to: "2026-11-25T00:00:00Z",
      warnings: [],
      series: [
        {
          key: "work::metric",
          workId: "work-1",
          title: "作品",
          platformId: "juejin",
          platformName: "掘金",
          accountAlias: "主号",
          contentKind: "article",
          metricName: "文章阅读量",
          metricDefinition: "原始定义",
          unit: "次",
          points: [
            { bucket: "2026-11-23", capturedAt: "2026-11-23T00:00:00Z", value: 10 },
            { bucket: "2026-11-25", capturedAt: "2026-11-25T00:00:00Z", value: 20 },
          ],
          comparable: false,
          comparabilityNote: "不聚合",
        },
      ],
    });
    expect(dataset.dates).toEqual(["2026-11-23", "2026-11-25"]);
    expect(dataset.series[0].values).toEqual([10, null]);
  });
});
