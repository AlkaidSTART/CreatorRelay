import { describe, expect, it } from "vitest";
import { createAppStore } from "./appStore";

const firstDocument = {
  id: "article-one",
  title: "First article",
  path: "articles/first/article.md",
  updatedAt: "2026-11-25T08:00:00.000Z",
  preview: "A first draft",
  wordCount: 3,
  charCount: 13,
  tags: ["draft"],
};

describe("app store", () => {
  it("starts with a usable, clean application state", () => {
    const store = createAppStore();
    const state = store.getState();

    expect(state.workspace).toBeNull();
    expect(state.documents).toEqual([]);
    expect(state.selectedId).toBe("");
    expect(state.content).toBe("");
    expect(state.saveState).toBe("saved");
    expect(state.loading).toBe(true);
    expect(state.revisions.length).toBeGreaterThan(0);
  });

  it("supports updater callbacks for state derived from the latest value", () => {
    const store = createAppStore();
    const { setDocuments } = store.getState();

    setDocuments([firstDocument]);
    setDocuments((documents) => documents.map((document) => ({ ...document, title: "Updated title" })));

    expect(store.getState().documents).toEqual([{ ...firstDocument, title: "Updated title" }]);
  });

  it("updates editor content and marks it unsaved atomically", () => {
    const store = createAppStore();
    const observedTransitions: Array<{ content: string; saveState: string }> = [];
    store.subscribe(({ content, saveState }) => observedTransitions.push({ content, saveState }));

    store.getState().updateContent("# Draft");

    expect(store.getState().content).toBe("# Draft");
    expect(store.getState().saveState).toBe("unsaved");
    expect(observedTransitions).toEqual([{ content: "# Draft", saveState: "unsaved" }]);
  });
});
