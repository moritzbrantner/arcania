import { describe, expect, it } from "vitest";
import { isLegalAttack, isLegalCardTarget, pieceStatLabel, tileTitle } from "./matchBoardHelpers";
import { storyCardProjection } from "./components/board.fixtures";
import type { BoardPiece, BoardUnit } from "./appTypes";
import type { Card, MatchState, Side } from "./types";

type BoardHero = Extract<BoardPiece, { pieceType: "hero" }>;

describe("match board helpers", () => {
  it("accepts range-2 attacks and rejects range-2 melee attacks", () => {
    const match = baseMatch();
    match.phase = "attack";
    const target = unit("opponent-unit", "opponent", { q: 0, r: 1 }, 1);

    expect(isLegalAttack(match, "player", hero("archer", "player", { q: 0, r: 3 }, 2), target)).toBe(
      true,
    );
    expect(isLegalAttack(match, "player", hero("runekeeper", "player", { q: 0, r: 3 }, 1), target)).toBe(
      false,
    );
  });

  it("shows attack range in compact piece stats when range is greater than one", () => {
    expect(pieceStatLabel(hero("archer", "player", { q: 0, r: 3 }, 2))).toContain("R2");
    expect(pieceStatLabel(hero("runekeeper", "player", { q: 0, r: 3 }, 1))).not.toContain("R1");
  });

  it("allows mana source cards on adjacent empty non-source hexes only", () => {
    const match = baseMatch();
    match.phase = "cardPlay";
    match.commandProjection = storyCardProjection([{ card: manaSourceCard(), targets: [{ type: "hex", coord: { q: -1, r: 3 } }] }]);
    match.board.manaSources = [{ q: 1, r: 2 }];
    const blocker = unit("blocker", "player", { q: 0, r: 2 }, 1);
    match.board.units = [blocker];

    expect(isLegalCardTarget(match, "player", manaSourceCard(), { q: -1, r: 3 }, null)).toBe(true);
    expect(isLegalCardTarget(match, "player", manaSourceCard(), { q: 0, r: 2 }, blocker)).toBe(false);
    expect(isLegalCardTarget(match, "player", manaSourceCard(), { q: 1, r: 2 }, null)).toBe(false);
    expect(isLegalCardTarget(match, "player", manaSourceCard(), { q: 0, r: 1 }, null)).toBe(false);
  });

  it("labels mana source tiles", () => {
    expect(
      tileTitle({ coord: { q: 0, r: 0 } }, null, "player", 0, true),
    ).toContain("mana source");
  });
});

function baseMatch(): MatchState {
  return {
    mode: "solo",
    round: 1,
    phase: "movement",
    activeSide: "player",
    prioritySide: null,
    player: participant("player"),
    opponent: participant("opponent"),
    board: { radius: 3, tiles: [], manaSources: [], units: [], droppedItems: [] },
    actionStack: [],
    log: [],
    winner: null,
  };
}

function participant(side: Side): MatchState["player"] {
  return {
    side,
    mana: 5,
    maxMana: 5,
    hero: hero(`${side}-hero`, side, { q: 0, r: side === "player" ? 3 : -3 }, 1),
    hand: [],
    handCount: 0,
    deckCount: 40,
    discardCount: 0,
    progression: {
      runeIds: [],
      skillIds: [],
      effects: {
        maxHpDelta: 0,
        attackDelta: 0,
        maxApDelta: 0,
        manaDelta: 0,
        openingHandDelta: 0,
        summonedUnitArmorDelta: 0,
        firstSummonedUnitArmorDelta: 0,
        spellDamageDelta: 0,
      },
    },
  };
}

function hero(
  id: string,
  side: Side,
  position: { q: number; r: number },
  attackRange: number,
): BoardHero {
  return {
    pieceType: "hero",
    id,
    side,
    name: id,
    heroType: "runekeeper",
    hp: 20,
    maxHp: 20,
    attack: 1,
    attackRange,
    position,
    apRemaining: 3,
    maxAp: 3,
    hasAttacked: false,
  };
}

function unit(
  id: string,
  side: Side,
  position: { q: number; r: number },
  attackRange: number,
): BoardUnit {
  return {
    pieceType: "unit",
    id,
    side,
    name: id,
    templateId: id,
    attack: 1,
    attackRange,
    armor: 2,
    maxArmor: 2,
    position,
    apRemaining: 2,
    maxAp: 2,
    hasAttacked: false,
    items: [],
  };
}

function manaSourceCard(): Card {
  return {
    id: "mana-well",
    templateId: "mana-well",
    name: "Mana Well",
    rarity: "basic",
    cost: 2,
    text: "",
    kind: { type: "manaSource" },
  };
}
