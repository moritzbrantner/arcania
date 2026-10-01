import type {
  Card,
  CardSummary,
  CatalogCard,
  HexCoord,
  HexTile,
  Hero,
  MatchProgressionLoadout,
  MatchState,
  StackItem,
  Unit,
} from "../types";

export const TUTORIAL_PLAYER_HERO_ID = "tutorial-player-hero";
export const TUTORIAL_OPPONENT_HERO_ID = "tutorial-opponent-hero";
export const TUTORIAL_EMBER_SQUIRE_CARD_ID = "tutorial-card-ember-squire";
export const TUTORIAL_SPARK_JOLT_CARD_ID = "tutorial-card-spark-jolt";
export const TUTORIAL_PLAYER_UNIT_ID = "tutorial-player-unit";
export const TUTORIAL_OPPONENT_UNIT_ID = "tutorial-opponent-unit";

export const TUTORIAL_SUMMON_COORD: HexCoord = { q: 0, r: 0 };
export const TUTORIAL_MOVE_COORD: HexCoord = { q: 1, r: 0 };

const emptyProgression: MatchProgressionLoadout = {
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
};

export const tutorialEmberSquireCard: Card = {
  id: TUTORIAL_EMBER_SQUIRE_CARD_ID,
  templateId: "ember-squire",
  name: "Ember Squire",
  rarity: "basic",
  cost: 1,
  text: "1 attack / 2 armor / 2 AP.",
  kind: { type: "unit", attack: 1, armor: 2, maxAp: 2 },
};

export const tutorialSparkJoltCard: Card = {
  id: TUTORIAL_SPARK_JOLT_CARD_ID,
  templateId: "spark-jolt",
  name: "Spark Jolt",
  rarity: "basic",
  cost: 1,
  text: "Priority 3. Range 2. Deal 1 damage to an enemy unit or hero.",
  kind: { type: "spell", range: 2, priority: 3, effect: { type: "damage", amount: 1 } },
};

export const tutorialCatalogCards: CatalogCard[] = [
  catalogCard(tutorialEmberSquireCard, 12, "/card-art/ember-squire.svg"),
  catalogCard(tutorialSparkJoltCard, 12, "/card-art/spark-jolt.svg"),
];

export function initialTutorialMatch(): MatchState {
  return baseTutorialMatch({
    hand: [tutorialEmberSquireCard, tutorialSparkJoltCard],
    units: [tutorialOpponentUnit()],
    log: ["Tutorial started."],
  });
}

export function tutorialUnit(position: HexCoord, overrides: Partial<Unit> = {}): Unit {
  return {
    id: TUTORIAL_PLAYER_UNIT_ID,
    side: "player",
    name: "Ember Squire",
    templateId: "ember-squire",
    attack: 1,
    attackRange: 1,
    armor: 2,
    maxArmor: 2,
    position,
    apRemaining: 1,
    maxAp: 2,
    hasAttacked: false,
    items: [],
    ...overrides,
  };
}

export function tutorialOpponentUnit(overrides: Partial<Unit> = {}): Unit {
  return tutorialUnit({ q: 1, r: -1 }, {
    id: TUTORIAL_OPPONENT_UNIT_ID,
    side: "opponent",
    name: "Ash Hound",
    templateId: "ash-hound",
    attack: 2,
    armor: 1,
    maxArmor: 1,
    apRemaining: 3,
    maxAp: 3,
    ...overrides,
  });
}

export function tutorialSummonStackItem(): StackItem {
  return {
    id: "tutorial-stack-summon",
    side: "player",
    priority: 0,
    action: { type: "playUnit", card: cardSummary(tutorialEmberSquireCard), coord: TUTORIAL_SUMMON_COORD },
  };
}

export function tutorialMoveStackItem(): StackItem {
  return {
    id: "tutorial-stack-move",
    side: "player",
    priority: 0,
    action: {
      type: "movePiece",
      pieceId: TUTORIAL_PLAYER_UNIT_ID,
      from: TUTORIAL_SUMMON_COORD,
      to: TUTORIAL_MOVE_COORD,
    },
  };
}

export function pendingTutorialAttack(): StackItem {
  return {
    id: "tutorial-stack-attack",
    side: "opponent",
    priority: 0,
    action: {
      type: "attack",
      attackerId: TUTORIAL_OPPONENT_UNIT_ID,
      targetId: TUTORIAL_PLAYER_UNIT_ID,
    },
  };
}

export function sparkJoltStackItem(): StackItem {
  return {
    id: "tutorial-stack-spark-jolt",
    side: "player",
    priority: 3,
    action: {
      type: "castSpell",
      card: cardSummary(tutorialSparkJoltCard),
      targetId: TUTORIAL_OPPONENT_UNIT_ID,
    },
  };
}

export function baseTutorialMatch(options: {
  hand?: Card[];
  units?: Unit[];
  activeSide?: "player" | "opponent";
  phase?: MatchState["phase"];
  prioritySide?: "player" | "opponent" | null;
  actionStack?: StackItem[];
  log?: string[];
} = {}): MatchState {
  const hand = options.hand ?? [tutorialEmberSquireCard, tutorialSparkJoltCard];
  return {
    mode: "shared",
    round: 1,
    phase: options.phase ?? "movement",
    activeSide: options.activeSide ?? "player",
    prioritySide: options.prioritySide ?? null,
    player: {
      side: "player",
      mana: 3,
      maxMana: 3,
      hero: hero(TUTORIAL_PLAYER_HERO_ID, "player", { q: 0, r: 1 }),
      progression: emptyProgression,
      hand,
      handCount: hand.length,
      deckCount: 24,
      discardCount: 0,
    },
    opponent: {
      side: "opponent",
      mana: 3,
      maxMana: 3,
      hero: hero(TUTORIAL_OPPONENT_HERO_ID, "opponent", { q: 2, r: -2 }),
      progression: emptyProgression,
      handCount: 3,
      deckCount: 25,
      discardCount: 0,
    },
    board: {
      radius: 3,
      tiles: radiusThreeTiles(),
      manaSources: [],
      units: options.units ?? [],
      droppedItems: [],
    },
    actionStack: options.actionStack ?? [],
    log: options.log ?? ["Tutorial board ready."],
    winner: null,
  };
}

function catalogCard(card: Card, copyCount: number, artPath: string): CatalogCard {
  return {
    ...card,
    id: card.templateId,
    copyCount,
    artKey: card.templateId,
    artPath,
  };
}

function cardSummary(card: Card): CardSummary {
  return {
    templateId: card.templateId,
    name: card.name,
    rarity: card.rarity,
    cost: card.cost,
    kind: card.kind,
  };
}

function hero(id: string, side: "player" | "opponent", position: HexCoord): Hero {
  return {
    id,
    side,
    heroType: side === "player" ? "runekeeper" : "pyromancer",
    hp: side === "player" ? 20 : 18,
    maxHp: side === "player" ? 20 : 18,
    attack: 1,
    attackRange: 1,
    position,
    apRemaining: 3,
    maxAp: 3,
    hasAttacked: false,
  };
}

function radiusThreeTiles(): HexTile[] {
  const tiles: HexTile[] = [];
  for (let q = -3; q <= 3; q += 1) {
    for (let r = -3; r <= 3; r += 1) {
      const coord = { q, r };
      if (distance(coord, { q: 0, r: 0 }) <= 3) {
        tiles.push({ coord });
      }
    }
  }
  return tiles.sort((left, right) => left.coord.r - right.coord.r || left.coord.q - right.coord.q);
}

function distance(a: HexCoord, b: HexCoord) {
  const dq = a.q - b.q;
  const dr = a.r - b.r;
  const ds = -a.q - a.r - (-b.q - b.r);
  return Math.max(Math.abs(dq), Math.abs(dr), Math.abs(ds));
}
