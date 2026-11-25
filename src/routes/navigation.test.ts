import { describe, expect, it } from "vitest";
import { findRouteByPathname, PRD_ROUTES, PRIMARY_ROUTES, SECONDARY_ROUTES } from "./navigation";

describe("PRD route navigation definitions", () => {
  it("defines all 8 independent PRD modules with unique paths", () => {
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
      "settings",
    ]);
    expect(new Set(paths).size).toBe(8);
    expect(PRIMARY_ROUTES.length + SECONDARY_ROUTES.length).toBe(PRD_ROUTES.length);
  });

  it("resolves active route metadata from pathname and falls back to studio", () => {
    expect(findRouteByPathname("/dashboard").id).toBe("dashboard");
    expect(findRouteByPathname("/publishing/task-1").id).toBe("publishing");
    expect(findRouteByPathname("/unknown").id).toBe("studio");
  });

  it("marks M1-open and later-stage modules instead of presenting all as complete", () => {
    const settings = findRouteByPathname("/settings");
    const studio = findRouteByPathname("/studio");
    const analytics = findRouteByPathname("/analytics");

    expect(settings.availability).toBe("available");
    expect(findRouteByPathname("/platforms").availability).toBe("available");
    expect(findRouteByPathname("/platforms").milestone).toBe("M3 / M4");
    expect(studio.availability).toBe("available");
    expect(studio.label).toBe("内容工作室");
    expect(analytics.availability).toBe("available");
    expect(analytics.milestone).toBe("M6");
  });
});
