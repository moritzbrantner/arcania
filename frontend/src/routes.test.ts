import { describe, expect, it } from "vitest";
import {
  browserRoutePath,
  matchSummaryRouteFromPath,
  routePathFromBrowserLocation,
  sharedMatchRouteFromPath,
  sharedMatchSummaryRouteFromPath,
  sharedReplayRouteFromPath,
  wikiTopicSlugFromPath,
} from "./routes";

describe("route parsers", () => {
  it("maps GitHub Pages browser URLs back to application routes", () => {
    expect(routePathFromBrowserLocation("/arcania/", "", "/arcania/")).toBe("/");
    expect(routePathFromBrowserLocation("/arcania/tutorial", "?step=2", "/arcania/")).toBe("/tutorial?step=2");
    expect(routePathFromBrowserLocation("/arcaniaized/tutorial", "", "/arcania/")).toBe("/arcaniaized/tutorial");
  });

  it("maps application routes into the deployment subpath", () => {
    expect(browserRoutePath("/", "/arcania/")).toBe("/arcania/");
    expect(browserRoutePath("/tutorial?step=2#board", "/arcania/")).toBe("/arcania/tutorial?step=2#board");
    expect(browserRoutePath("/arcania/wiki", "/arcania/")).toBe("/arcania/wiki");
    expect(browserRoutePath("//example.test/wiki", "/arcania/")).toBe("//example.test/wiki");
  });

  it("parses match summary routes", () => {
    expect(matchSummaryRouteFromPath("/matches/rl-123/summary")).toBe("rl-123");
    expect(matchSummaryRouteFromPath("/matches/rl-123/replay")).toBeNull();
  });

  it("parses shared summary and replay routes before shared match routes", () => {
    expect(sharedMatchSummaryRouteFromPath("/match/rl-123/player-token/summary")).toEqual({
      matchId: "rl-123",
      seatToken: "player-token",
    });
    expect(sharedReplayRouteFromPath("/match/rl-123/player-token/replay")).toEqual({
      matchId: "rl-123",
      seatToken: "player-token",
    });
    expect(sharedMatchRouteFromPath("/match/rl-123/player-token/summary")).toBeNull();
    expect(sharedMatchRouteFromPath("/match/rl-123/player-token/replay")).toBeNull();
  });

  it("parses wiki topic routes", () => {
    expect(wikiTopicSlugFromPath("/wiki")).toBeNull();
    expect(wikiTopicSlugFromPath("/wiki/")).toBeNull();
    expect(wikiTopicSlugFromPath("/wiki/mana")).toBe("mana");
    expect(wikiTopicSlugFromPath("/wiki/mana?from=play")).toBe("mana");
    expect(wikiTopicSlugFromPath("/wiki/mana#top")).toBe("mana");
    expect(wikiTopicSlugFromPath("/wiki/cards-and-priority")).toBe("cards-and-priority");
    expect(wikiTopicSlugFromPath("/wiki/turn-flow/")).toBe("turn-flow");
    expect(wikiTopicSlugFromPath("/wiki/mana/extra")).toBeNull();
  });
});
