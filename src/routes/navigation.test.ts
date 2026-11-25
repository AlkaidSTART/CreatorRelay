import { describe, expect, it } from "vitest";
import {
  ECOSYSTEM_ROUTES,
  findRouteByPathname,
  NAV_GROUPS,
  PRD_ROUTES,
  SYSTEM_ROUTES,
  WORKSPACE_ROUTES,
} from "./navigation";

describe("PRD route navigation definitions", () => {
  it("defines all 13 independent routes across 3 sidebar groups with unique paths", () => {
    const ids = PRD_ROUTES.map((route) => route.id);
    const paths = PRD_ROUTES.map((route) => route.path);

    expect(ids).toEqual([
      "dashboard",
      "studio",
      "publishing",
      "content",
      "analytics",
      "assistant",
      "platforms",
      "adapters",
      "creator-centers",
      "ecosystem",
      "settings",
      "settings-ai",
      "settings-diagnostics",
    ]);
    expect(new Set(paths).size).toBe(13);
    expect(NAV_GROUPS.map((group) => group.id)).toEqual(["workspace", "ecosystem", "system"]);
    expect(WORKSPACE_ROUTES.length + ECOSYSTEM_ROUTES.length + SYSTEM_ROUTES.length).toBe(
      PRD_ROUTES.length,
    );
  });

  it("resolves active route metadata from pathname including nested settings routes", () => {
    expect(findRouteByPathname("/dashboard").id).toBe("dashboard");
    expect(findRouteByPathname("/publishing/task-1").id).toBe("publishing");
    expect(findRouteByPathname("/adapters").id).toBe("adapters");
    expect(findRouteByPathname("/creator-centers").id).toBe("creator-centers");
    expect(findRouteByPathname("/ecosystem").id).toBe("ecosystem");
    expect(findRouteByPathname("/settings").id).toBe("settings");
    expect(findRouteByPathname("/settings/ai").id).toBe("settings-ai");
    expect(findRouteByPathname("/settings/diagnostics").id).toBe("settings-diagnostics");
    expect(findRouteByPathname("/unknown").id).toBe("studio");
  });

  it("marks route availability and exact matching for parent routes with children", () => {
    const settings = findRouteByPathname("/settings");
    const aiSettings = findRouteByPathname("/settings/ai");
    const platforms = findRouteByPathname("/platforms");
    const adapters = findRouteByPathname("/adapters");

    expect(settings.availability).toBe("available");
    expect(settings.exact).toBe(true);
    expect(aiSettings.availability).toBe("available");
    expect(platforms.availability).toBe("available");
    expect(platforms.milestone).toBe("M3");
    expect(adapters.availability).toBe("available");
    expect(adapters.milestone).toBe("M4");
  });
});
