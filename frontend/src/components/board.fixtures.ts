import type {
  Card,
  CatalogCard,
  HexCoord,
  HexTile,
  MatchProgressionLoadout,
  MatchState,
  MatchCommandProjection,
  ActionTarget,
  StackItem,
  Unit,
  Hero,
} from "../types";

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

export const emberSquireCard: Card = {
  id: "story-ember-squire",
  templateId: "ember-squire",
  name: "Ember Squire",
  rarity: "basic",
  cost: 1,
  text: "1 attack / 2 armor / 2 AP.",
  kind: { type: "unit", attack: 1, armor: 2, maxAp: 2 },
};

export const sparkJoltCard: Card = {
  id: "story-spark-jolt",
  templateId: "spark-jolt",
  name: "Spark Jolt",
  rarity: "basic",
  cost: 1,
  text: "Priority 3. Range 2. Deal 1 damage to an enemy unit or hero.",
  kind: { type: "spell", range: 2, priority: 3, effect: { type: "damage", amount: 1 } },
};

export const cinderRingCard: Card = {
  id: "story-cinder-ring",
  templateId: "cinder-ring",
  name: "Cinder Ring",
  rarity: "advanced",
  cost: 2,
  text: "Area damage around a Hex.",
  kind: { type: "spell", range: 2, priority: 2, effect: { type: "areaDamage", amount: 1, radius: 1 } },
};

export const thunderRailCard: Card = {
  id: "story-thunder-rail",
  templateId: "thunder-rail",
  name: "Thunder Rail",
  rarity: "advanced",
  cost: 2,
  text: "Line damage from the caster.",
  kind: { type: "spell", range: 3, priority: 2, effect: { type: "lineDamage", amount: 1 } },
};

export const emberFlaskCard: Card = {
  id: "story-ember-flask",
  templateId: "ember-flask",
  name: "Ember Flask",
  rarity: "advanced",
  cost: 2,
  text: "Range 2. Equip to an allied unit. Passive: +1 attack. Active: spend 1 unit AP to heal carrier 2.",
  kind: {
    type: "item",
    range: 2,
    passive: { type: "statBonus", attack: 1, armor: 0, maxAp: 0 },
    active: { type: "healCarrier", amount: 2 },
  },
};

export const catalogCards: CatalogCard[] = [
  catalogCard(emberSquireCard, 12, "/card-art/ember-squire.svg"),
  catalogCard(sparkJoltCard, 12, "/card-art/spark-jolt.svg"),
  catalogCard(cinderRingCard, 3, "/card-art/cinder-ring.svg"),
  catalogCard(thunderRailCard, 3, "/card-art/thunder-rail.svg"),
  catalogCard(emberFlaskCard, 3, "/card-art/ember-flask.svg"),
];

// Explicit wire fixtures: tests supply different commands/rejections for other states.
export function storyCardProjection(
  entries: { card: Card; targets?: ActionTarget[]; rejection?: MatchCommandProjection["cards"][number]["rejection"] }[],
): MatchCommandProjection {
  return {
    viewerSide: "player",
    proactiveCardPhases: ["movement", "cardPlay"],
    legalCommands: entries.flatMap(({ card, targets = [] }) => targets.map((target) => ({
      type: "playCard" as const, cardId: card.id, target,
    }))),
    cards: entries.map(({ card, targets = [], rejection }) => ({
      cardId: card.id, allowed: targets.length > 0, rejection,
    })),
  };
}

const openingCardProjection = storyCardProjection([
  { card: emberSquireCard, targets: [
    { type: "hex", coord: { q: 0, r: 0 } },
    { type: "hex", coord: { q: -1, r: 1 } },
    { type: "hex", coord: { q: -1, r: 2 } },
    { type: "hex", coord: { q: 0, r: 2 } },
    { type: "hex", coord: { q: 1, r: 0 } },
  ] },
  { card: sparkJoltCard, targets: [{ type: "piece", pieceId: "opponent-hero" }] },
]);

export function storyMatch(options: {
  hand?: Card[];
  units?: Unit[];
  playerHero?: HexCoord;
  opponentHero?: HexCoord;
  actionStack?: StackItem[];
  prioritySide?: "player" | "opponent" | null;
  phase?: MatchState["phase"];
  commandProjection?: MatchCommandProjection;
} = {}): MatchState {
  const hand = options.hand ?? [emberSquireCard, sparkJoltCard];
  return {
    mode: "solo",
    commandProjection: options.commandProjection ?? openingCardProjection,
    round: 1,
    phase: options.phase ?? "movement",
    activeSide: "player",
    prioritySide: options.prioritySide ?? null,
    player: {
      side: "player",
      mana: 8,
      maxMana: 8,
      hero: hero("player-hero", "player", options.playerHero ?? { q: 0, r: 1 }),
      progression: emptyProgression,
      hand,
      handCount: hand.length,
      deckCount: 24,
      discardCount: 0,
    },
    opponent: {
      side: "opponent",
      mana: 8,
      maxMana: 8,
      hero: hero("opponent-hero", "opponent", options.opponentHero ?? { q: 1, r: 1 }),
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
    log: ["Loaded story match state."],
    winner: null,
  };
}

export function storyUnit(position: HexCoord, overrides: Partial<Unit> = {}): Unit {
  return {
    id: "story-player-unit",
    side: "player",
    name: "Rune Runner",
    templateId: "rune-runner",
    attack: 1,
    attackRange: 1,
    armor: 1,
    maxArmor: 1,
    position,
    apRemaining: 4,
    maxAp: 4,
    hasAttacked: false,
    items: [],
    ...overrides,
  };
}

export const pendingAttackStack: StackItem = {
  id: "story-stack-1",
  side: "opponent",
  priority: 0,
  action: {
    type: "attack",
    attackerId: "opponent-hero",
    targetId: "player-hero",
  },
};

export const pendingSpellStack: StackItem = {
  id: "story-stack-2",
  side: "player",
  priority: 1,
  action: {
    type: "castSpell",
    card: sparkJoltCard,
    targetId: "opponent-hero",
  },
};

export const pendingMoveStack: StackItem = {
  id: "story-stack-3",
  side: "player",
  priority: 2,
  action: {
    type: "movePiece",
    pieceId: "player-hero",
    from: { q: 0, r: 1 },
    to: { q: 0, r: 0 },
  },
};

function catalogCard(card: Card, copyCount: number, artPath: string): CatalogCard {
  return {
    ...card,
    id: card.templateId,
    copyCount,
    artKey: card.templateId,
    artPath,
  };
}

function hero(id: string, side: "player" | "opponent", position: HexCoord): Hero {
  return {
    id,
    side,
    heroType: side === "player" ? "runekeeper" : "pyromancer",
    hp: side === "player" ? 20 : 18,
    maxHp: side === "player" ? 20 : 18,
    attack: side === "player" ? 1 : 2,
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
