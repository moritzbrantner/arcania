import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { WIKI_TOPICS } from "./wikiContent";
import { WIKI_DECK_RULES, WIKI_SCENES, wikiSceneById, type WikiSceneId } from "./wikiScenes";

describe("wiki scenes", () => {
  it("has unique scene IDs", () => {
    const ids = WIKI_SCENES.map((scene) => scene.id);

    expect(new Set(ids).size).toBe(ids.length);
  });

  it("resolves every topic scene ID", () => {
    for (const topic of WIKI_TOPICS) {
      expect(wikiSceneById(topic.sceneId)).not.toBeNull();
    }
  });

  it("gives every board scene multiple complete steps", () => {
    for (const scene of WIKI_SCENES) {
      if (scene.type !== "board") {
        continue;
      }

      expect(scene.steps.length).toBeGreaterThanOrEqual(2);
      for (const step of scene.steps) {
        expect(step.title.trim().length).toBeGreaterThan(0);
        expect(step.instruction.trim().length).toBeGreaterThan(0);
        expect(["solo", "shared"]).toContain(step.match.mode);
        expect(step.match.board).toEqual(expect.any(Object));
        expect(step.highlights.length + step.callouts.length).toBeGreaterThan(0);
      }
    }
  });

  it("exposes the current Deck recipe values", () => {
    expect(WIKI_DECK_RULES).toEqual({
      minCards: 60,
      basicCopyLimit: 5,
      advancedCopyLimit: 4,
      rareCopyLimit: 3,
      advancedTotalLimit: 24,
      rareTotalLimit: 12,
    });
  });

  it("does not import backend Match scenario APIs", () => {
    const source = readFileSync(new URL("./wikiScenes.ts", import.meta.url), "utf8");

    expect(source).not.toContain("createMatchScenario");
    expect(source).not.toContain("loadMatchScenarios");
    expect(source).not.toContain("/api/dev/match-scenarios");
  });

  it("interleaves a Movement summon and entry move before Attack and final Card Play", () => {
    const steps = boardScene("action-points-budget").steps;

    expect(steps.map((step) => step.match.phase)).toEqual([
      "movement", "movement", "movement", "attack", "cardPlay", "cardPlay",
    ]);
    expect(steps.map((step) => step.match.player.mana)).toEqual([3, 2, 2, 2, 2, 1]);
    expect(steps.map((step) => step.match.player.hero.apRemaining)).toEqual([3, 3, 3, 3, 3, 3]);
    expect(steps[0].match.board.units).toEqual([]);
    expect(steps[1].match.board.units[0]).toMatchObject({
      position: { q: 0, r: 0 }, apRemaining: 1, maxAp: 2,
    });
    expect(steps[2].match.board.units[0]).toMatchObject({
      position: { q: 0, r: 1 }, apRemaining: 0, maxAp: 2,
    });
    expect(steps[1].match.player.hand).not.toEqual(
      expect.arrayContaining([expect.objectContaining({ templateId: "ember-squire" })]),
    );
    expect(steps[3].selectedCardId).toBeUndefined();
    expect(steps[3].callouts).toContainEqual({ label: "Proactive Cards", value: "Unavailable" });
    expect(steps[4].selectedCardId).toBe("wiki-card-spark-jolt");
    expect(steps[5].match.player.hand).toEqual([]);
    expect(steps[5].match.opponent.hero.hp).toBe(17);
    for (const step of steps) {
      expect(step.callouts).toContainEqual({ label: "Hero AP", value: "3/3" });
    }
  });

  it("keeps the Movement summon pending until its higher-priority response resolves", () => {
    const steps = boardScene("cards-priority-stack").steps;

    expect(steps.map((step) => step.match.mode)).toEqual(["shared", "shared", "shared", "shared"]);
    expect(steps.map((step) => step.match.phase)).toEqual(["movement", "movement", "movement", "movement"]);
    expect(steps.map((step) => step.match.actionStack.length)).toEqual([1, 2, 1, 0]);
    expect(steps.map((step) => step.match.prioritySide)).toEqual(["opponent", "player", "opponent", null]);
    expect(steps.map((step) => step.match.player.hero.hp)).toEqual([20, 20, 19, 19]);
    expect(steps.map((step) => step.match.player.hero.apRemaining)).toEqual([3, 3, 3, 3]);
    expect(steps.map((step) => step.match.player.mana)).toEqual([2, 2, 2, 2]);
    expect(steps.map((step) => step.match.opponent.mana)).toEqual([3, 2, 2, 2]);
    expect(steps[1].match.actionStack.map((item) => item.action.type)).toEqual(["playUnit", "castSpell"]);
    expect(steps.slice(0, 3).every((step) => step.match.board.units.length === 0)).toBe(true);
    expect(steps[3].match.board.units[0]).toMatchObject({ apRemaining: 1, maxAp: 2 });
    expect(steps[3].match.prioritySide).toBeNull();
  });

  it("labels turn-ending and combat examples with their authored phases", () => {
    expect(boardScene("turn-flow-refresh").steps[0].match.phase).toBe("cardPlay");
    expect(boardScene("combat-range-counter").steps.every((step) => step.match.phase === "attack")).toBe(true);
  });
});

function boardScene(id: WikiSceneId) {
  const scene = wikiSceneById(id);
  if (scene?.type !== "board") {
    throw new Error(`Missing board scene ${id}`);
  }
  return scene;
}
