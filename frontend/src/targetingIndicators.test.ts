import { describe, expect, it } from "vitest";
import type { Card, StackItem } from "./types";
import { pieceById } from "./matchBoardHelpers";
import { storyMatch, storyUnit, storyCardProjection } from "./components/board.fixtures";
import { selectedTargetingIndicators, stackTargetingIndicators } from "./targetingIndicators";

describe("targeting indicators", () => {
  it("derives selected attack indicators only for legal attack targets", () => {
    const match = storyMatch({
      phase: "attack",
      units: [
        storyUnit({ q: 0, r: 0 }),
        storyUnit({ q: 1, r: 0 }, { id: "enemy-adjacent", side: "opponent" }),
        storyUnit({ q: -2, r: 0 }, { id: "enemy-distant", side: "opponent" }),
      ],
    });
    const selectedPiece = pieceById(match, "story-player-unit");

    const indicators = selectedTargetingIndicators({
      match,
      viewerSide: "player",
      selectedCard: null,
      selectedPiece,
      focusedCoord: null,
      hoveredCoord: null,
    });

    expect(indicators).toHaveLength(1);
    expect(indicators[0]).toMatchObject({
      actionType: "attack",
      tone: "attack",
      sourcePieceId: "story-player-unit",
      primaryTargetPieceId: "enemy-adjacent",
    });
  });

  it("derives selected spell indicators for legal primary targets", () => {
    const match = storyMatch({
      phase: "cardPlay",
      hand: [sparkJolt()],
      commandProjection: storyCardProjection([{ card: sparkJolt(), targets: [{ type: "piece", pieceId: "enemy-unit" }, { type: "piece", pieceId: "opponent-hero" }] }]),
      units: [
        storyUnit({ q: 1, r: 0 }, { id: "enemy-unit", side: "opponent" }),
        storyUnit({ q: -1, r: 1 }, { id: "friendly-unit", side: "player" }),
      ],
    });

    const indicators = selectedTargetingIndicators({
      match,
      viewerSide: "player",
      selectedCard: sparkJolt(),
      selectedPiece: null,
      focusedCoord: null,
      hoveredCoord: null,
    });

    expect(indicators.map((indicator) => indicator.primaryTargetPieceId).sort()).toEqual([
      "enemy-unit",
      "opponent-hero",
    ]);
    expect(indicators.every((indicator) => indicator.tone === "damageSpell")).toBe(true);
    expect(indicators.every((indicator) => indicator.secondaryFootprintCoords.length === 0)).toBe(true);
  });

  it("adds selected area spell footprint only for the hovered or focused target", () => {
    const match = storyMatch({
      phase: "cardPlay",
      hand: [cinderRing()],
      commandProjection: storyCardProjection([{ card: cinderRing(), targets: [{ type: "piece", pieceId: "enemy-unit" }, { type: "piece", pieceId: "opponent-hero" }] }]),
      units: [storyUnit({ q: 1, r: 0 }, { id: "enemy-unit", side: "opponent" })],
    });

    expect(
      selectedTargetingIndicators({
        match,
        viewerSide: "player",
        selectedCard: cinderRing(),
        selectedPiece: null,
        focusedCoord: null,
        hoveredCoord: null,
      }).find((indicator) => indicator.primaryTargetPieceId === "enemy-unit")?.secondaryFootprintCoords,
    ).toEqual([]);

    const hovered = selectedTargetingIndicators({
      match,
      viewerSide: "player",
      selectedCard: cinderRing(),
      selectedPiece: null,
      focusedCoord: null,
      hoveredCoord: { q: 1, r: 0 },
    }).find((indicator) => indicator.primaryTargetPieceId === "enemy-unit");

    expect(hovered?.secondaryFootprintCoords).toEqual(
      expect.arrayContaining([{ q: 0, r: 0 }, { q: 1, r: -1 }, { q: 2, r: -1 }]),
    );
    expect(hovered?.secondaryFootprintCoords).not.toContainEqual({ q: 1, r: 0 });
  });

  it("derives queued stack indicators and ignores non-attack or non-spell stack actions", () => {
    const match = storyMatch({
      actionStack: [
        attackStack(),
        castSpellStack(),
        {
          id: "move-stack",
          side: "player",
          priority: 2,
          action: {
            type: "movePiece",
            pieceId: "player-hero",
            from: { q: 0, r: 1 },
            to: { q: 0, r: 0 },
          },
        },
      ],
      prioritySide: "player",
    });

    const indicators = stackTargetingIndicators(match);

    expect(indicators.map((indicator) => indicator.id)).toEqual([
      "stack-attack-stack-attack",
      "stack-spell-stack-spell",
    ]);
  });

  it("derives line spell footprints from caster through target within range", () => {
    const match = storyMatch({
      playerHero: { q: 0, r: 1 },
      opponentHero: { q: 0, r: -2 },
      actionStack: [castSpellStack(thunderRail())],
    });

    const [indicator] = stackTargetingIndicators(match);

    expect(indicator.secondaryFootprintCoords).toEqual(
      expect.arrayContaining([{ q: 0, r: 0 }, { q: 0, r: -1 }]),
    );
  });

  it("skips queued indicators with missing source or target pieces", () => {
    const match = storyMatch({
      actionStack: [
        {
          id: "missing-attacker-stack",
          side: "opponent",
          priority: 0,
          action: { type: "attack", attackerId: "missing-attacker", targetId: "opponent-hero" },
        },
        {
          id: "missing-target-stack",
          side: "player",
          priority: 1,
          action: { type: "castSpell", card: sparkJolt(), targetId: "missing-target" },
        },
      ],
    });

    expect(stackTargetingIndicators(match)).toEqual([]);
  });
});

function sparkJolt(): Card {
  return {
    id: "spark-jolt-card",
    templateId: "spark-jolt",
    name: "Spark Jolt",
    rarity: "basic",
    cost: 1,
    text: "Deal damage.",
    kind: { type: "spell", range: 2, priority: 1, effect: { type: "damage", amount: 1 } },
  };
}

function cinderRing(): Card {
  return {
    id: "cinder-ring-card",
    templateId: "cinder-ring",
    name: "Cinder Ring",
    rarity: "advanced",
    cost: 2,
    text: "Area damage.",
    kind: { type: "spell", range: 2, priority: 1, effect: { type: "areaDamage", amount: 1, radius: 1 } },
  };
}

function thunderRail(): Card {
  return {
    id: "thunder-rail-card",
    templateId: "thunder-rail",
    name: "Thunder Rail",
    rarity: "advanced",
    cost: 2,
    text: "Line damage.",
    kind: { type: "spell", range: 3, priority: 1, effect: { type: "lineDamage", amount: 1 } },
  };
}

function attackStack(): StackItem {
  return {
    id: "attack-stack",
    side: "opponent",
    priority: 0,
    action: {
      type: "attack",
      attackerId: "opponent-hero",
      targetId: "player-hero",
    },
  };
}

function castSpellStack(card: Card = sparkJolt()): StackItem {
  return {
    id: "spell-stack",
    side: "player",
    priority: 1,
    action: {
      type: "castSpell",
      card,
      targetId: "opponent-hero",
    },
  };
}
