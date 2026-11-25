import { describe, expect, it } from "vitest";
import { createMcpStore } from "./mcpStore";

describe("MCP assistant store", () => {
  it("initializes a visible demo host and executes a proposed read tool", async () => {
    const store = createMcpStore();
    await store.getState().initialize();

    expect(store.getState().status?.state).toBe("healthy");
    expect(store.getState().tools.length).toBeGreaterThan(0);
    expect(store.getState().activeThread?.thread.providerName).toBe("本地编排器");

    await store.getState().sendPrompt("查询所有账号状态");
    const messages = store.getState().activeThread?.messages ?? [];
    const lastMessage = messages[messages.length - 1];
    expect(lastMessage?.toolCall?.toolName).toBe("account_get_info");
    expect(lastMessage?.toolCall?.status).toBe("proposed");

    await store.getState().previewToolCall("account_get_info", {});
    expect(store.getState().activePreview?.permission).toBe("read");
    expect(store.getState().activePreview?.requiresConfirmation).toBe(false);
    expect(store.getState().activePreview?.fieldErrors).toEqual([]);

    const result = await store.getState().confirmPreview();
    expect(result?.status).toBe("success");
    expect(store.getState().activePreview).toBeNull();
  });
});
