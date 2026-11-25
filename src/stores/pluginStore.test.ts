import { describe, expect, it } from "vitest";
import { createPluginStore } from "./pluginStore";

function decisions(
  permissions: Array<{ id: string; required: boolean }>,
  grantRequiredOnly = true,
) {
  return permissions.map((permission) => ({
    id: permission.id,
    granted: grantRequiredOnly ? permission.required : false,
  }));
}

describe("plugin store", () => {
  it("requires discovery, installation and explicit permission review before enable", async () => {
    const store = createPluginStore();
    await store.getState().initialize();
    const catalog = store.getState().catalog[0];
    expect(catalog.installed).toBe(false);

    const installed = await store.getState().install(catalog.id);
    expect(installed.status).toBe("REVIEW_REQUIRED");
    expect(installed.permissions.every((permission) => permission.granted === null)).toBe(true);

    await store.getState().savePermissions(installed.id, decisions(installed.permissions, false));
    expect(store.getState().plugins[0].status).toBe("REVIEW_REQUIRED");
    await expect(store.getState().enable(installed.id)).rejects.toThrow("权限审查");

    const ready = await store
      .getState()
      .savePermissions(installed.id, decisions(installed.permissions));
    expect(ready.status).toBe("READY");
    const enabled = await store.getState().enable(installed.id);
    expect(enabled.status).toBe("ENABLED");
    expect(enabled.runtimeState).toBe("running");
  });

  it("shows per-content-kind probe states and isolates simulated faults", async () => {
    const store = createPluginStore();
    await store.getState().initialize();
    const installed = await store.getState().install("creatorrelay.official.test-adapter");
    await store.getState().savePermissions(installed.id, decisions(installed.permissions));
    await store.getState().enable(installed.id);

    const probe = await store.getState().probe(installed.id, "fixture-image-text");
    expect(probe.ok).toBe(true);
    const video = probe.capabilities.find((capability) => capability.kind === "video");
    const imageText = probe.capabilities.find((capability) => capability.kind === "imageText");
    expect(video?.effectivePublish).toBe("unsupported");
    expect(imageText?.effectivePublish).toBe("supported");

    const fault = await store.getState().simulateFault(installed.id, "crash");
    expect(fault.status).toBe("ERROR");
    expect(store.getState().plugins[0].status).toBe("ERROR");
    const restarted = await store.getState().restart(installed.id);
    expect(restarted.status).toBe("ENABLED");
  });

  it("never displays Manifest-supported video as available when runtime probing fails", async () => {
    const store = createPluginStore();
    await store.getState().initialize();
    const installed = await store.getState().install("creatorrelay.official.probe-failure-adapter");
    await store.getState().savePermissions(installed.id, decisions(installed.permissions));
    await expect(store.getState().enable(installed.id)).rejects.toThrow("PROBE_FAILED");

    const failed = store.getState().plugins.find((plugin) => plugin.id === installed.id);
    expect(failed?.status).toBe("ERROR");
    expect(
      failed?.capabilities.every((capability) => capability.effectivePublish === "unavailable"),
    ).toBe(true);
  });
});
