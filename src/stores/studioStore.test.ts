import { describe, expect, it } from "vitest";
import { createStudioStore } from "./studioStore";
import type { ContentEditorState } from "./studioStore";

const editor: ContentEditorState = {
  id: "content-1",
  kind: "article",
  title: "原标题",
  summary: "摘要",
  tags: ["M5"],
  cover: null,
  media: [],
  body: "# 正文",
  contentHash: "hash-1",
  contentVersion: "version-1",
  createdAt: "2026-11-25T00:00:00Z",
  updatedAt: "2026-11-25T00:00:00Z",
  path: "contents/content-1",
  source: null,
};

describe("studio store", () => {
  it("keeps unsaved editor content and exposes an actionable save error", async () => {
    const store = createStudioStore();
    store.setState({
      editor: { ...editor, title: "尚未保存的新标题", body: "# 新正文" },
      current: null,
      saveState: "unsaved",
      simulateNextSaveFailure: true,
    });

    const saved = await store.getState().save();

    expect(saved).toBe(false);
    expect(store.getState().saveState).toBe("error");
    expect(store.getState().saveError).toContain("模拟磁盘写入失败");
    expect(store.getState().editor?.title).toBe("尚未保存的新标题");
    expect(store.getState().editor?.body).toBe("# 新正文");
    expect(store.getState().editor?.contentHash).toBe("hash-1");
  });
});
