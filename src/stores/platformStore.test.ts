import { describe, expect, it } from "vitest";
import { createPlatformStore } from "./platformStore";

describe("platform store", () => {
  it("keeps two accounts on the same platform independent", async () => {
    const store = createPlatformStore();
    await store.getState().initialize();
    const baseline = store.getState().accounts.length;
    const first = await store.getState().createAccount("bilibili", "主账号");
    const second = await store.getState().createAccount("bilibili", "副账号");

    expect(store.getState().accounts).toHaveLength(baseline + 2);
    await store.getState().updateAccount(first.id, { alias: "修改后的主账号", enabled: false });

    const updatedFirst = store.getState().accounts.find((account) => account.id === first.id);
    const untouchedSecond = store.getState().accounts.find((account) => account.id === second.id);
    expect(updatedFirst?.alias).toBe("修改后的主账号");
    expect(updatedFirst?.status).toBe("DISABLED");
    expect(untouchedSecond?.alias).toBe("副账号");
    expect(untouchedSecond?.status).toBe("UNAUTHORIZED");
  });

  it("never turns a session check into ACTIVE in browser preview", async () => {
    const store = createPlatformStore();
    await store.getState().initialize();
    const account = await store.getState().createAccount("bilibili", "状态测试");
    const result = await store.getState().checkStatus(account.id);

    expect(result.message).toContain("不执行真实平台校验");
    expect(result.account.status).toBe("UNAUTHORIZED");
    expect(store.getState().accounts.find((item) => item.id === account.id)?.status).not.toBe(
      "ACTIVE",
    );
  });

  it("exposes browser fallback steps and requires explicit deletion scopes", async () => {
    const store = createPlatformStore();
    await store.getState().initialize();
    const account = await store.getState().createAccount("bilibili", "浏览器回退");
    const challenge = await store.getState().beginAuthentication(account.id);
    expect(challenge.loginUrl).toContain("bilibili.com");
    expect(challenge.manualSteps.length).toBeGreaterThan(0);

    await expect(
      store.getState().completeAuthentication(account.id, challenge.challengeId, "secret-cookie"),
    ).rejects.toThrow("浏览器预览不会接收或保存 Cookie");

    const preview = await store.getState().previewDeletion(account.id);
    expect(preview.willDeleteCredentials).toBe(true);
    await expect(
      store.getState().deleteAccount(account.id, {
        credentials: true,
        sessions: false,
        historyRetainedUnderstood: true,
      }),
    ).rejects.toThrow("请确认凭据删除、会话清理和历史作品保留范围");
  });
});
