import { describe, expect, it } from "vitest";
import { createContentStore, createUiStore, createWorkspaceStore } from "./appStore";

describe("stores", () => {
  it("initializes workspace store with clean defaults", () => {
    const store = createWorkspaceStore();
    const state = store.getState();

    expect(state.workspace).toBeNull();
    expect(state.documents).toEqual([]);
    expect(state.selectedId).toBe("");
    expect(state.content).toBe("");
    expect(state.saveState).toBe("saved");
    expect(state.loading).toBe(true);
    expect(state.revisions.length).toBeGreaterThan(0);
  });

  it("updates editor content and marks it unsaved atomically", () => {
    const store = createWorkspaceStore();
    const observedTransitions: Array<{ content: string; saveState: string }> = [];
    store.subscribe(({ content, saveState }) => observedTransitions.push({ content, saveState }));

    store.getState().updateContent("# Draft");

    expect(store.getState().content).toBe("# Draft");
    expect(store.getState().saveState).toBe("unsaved");
    expect(observedTransitions).toEqual([{ content: "# Draft", saveState: "unsaved" }]);
  });

  it("toggles UI panels and sidebar cleanly", () => {
    const ui = createUiStore();

    expect(ui.getState().sidebarOpen).toBe(true);
    expect(ui.getState().rightPanel).toBe("none");

    ui.getState().toggleSidebar();
    expect(ui.getState().sidebarOpen).toBe(false);

    ui.getState().toggleRightPanel("info");
    expect(ui.getState().rightPanel).toBe("info");

    ui.getState().toggleRightPanel("history");
    expect(ui.getState().rightPanel).toBe("history");

    ui.getState().toggleRightPanel("history");
    expect(ui.getState().rightPanel).toBe("none");
  });

  it("manages shared content filters and assistant messages", () => {
    const content = createContentStore();
    const initialCount = content.getState().assistantMessages.length;

    content.getState().setPublishingFilter("SCHEDULED");
    expect(content.getState().publishingFilter).toBe("SCHEDULED");

    content.getState().sendAssistantPrompt("把当前主稿适配为掘金版本");
    const messages = content.getState().assistantMessages;
    expect(messages.length).toBe(initialCount + 2);
    expect(messages[messages.length - 1].toolCall?.name).toBe("studio_adapt_content");
  });
});
