import type {
  Building,
  Card,
  CardSummary,
  CatalogCard,
  HexCoord,
  HexTile,
  Hero,
  MatchProgressionLoadout,
  MatchState,
  Side,
  StackItem,
  Unit,
} from "../types";
import type { BoardTutorialHighlight } from "../tutorial/tutorialHighlights";

export type WikiSceneId =
  | "turn-flow-refresh"
  | "mana-source"
  | "action-points-budget"
  | "cards-priority-stack"
  | "combat-range-counter"
  | "building-occupation"
  | "deck-recipe-legality";

export type WikiScene = WikiBoardScene | WikiDeckRulesScene;

export type WikiBoardScene = {
  type: "board";
  id: Exclude<WikiSceneId, "deck-recipe-legality">;
  title: string;
  summary: string;
  steps: WikiBoardSceneStep[];
};

export type WikiBoardSceneStep = {
  title: string;
  instruction: string;
  match: MatchState;
  selectedCardId?: string;
  selectedPieceId?: string;
  focusedCoord?: HexCoord;
  highlights: BoardTutorialHighlight[];
  callouts: Array<{ label: string; value: string }>;
};

export type WikiDeckRulesScene = {
  type: "deckRules";
  id: "deck-recipe-legality";
  title: string;
  summary: string;
  initialCounts: DeckRecipeCounts;
  rules: {
    minCards: 60;
    basicCopyLimit: 5;
    advancedCopyLimit: 4;
    rareCopyLimit: 3;
    advancedTotalLimit: 24;
    rareTotalLimit: 12;
  };
};

export type DeckRecipeCounts = {
  basic: number;
  advanced: number;
  rare: number;
};

export type DeckRecipeSceneLegality = {
  legal: boolean;
  messages: string[];
  totalCards: number;
};

const WIKI_PLAYER_HERO_ID = "wiki-player-hero";
const WIKI_OPPONENT_HERO_ID = "wiki-opponent-hero";
const WIKI_PLAYER_UNIT_ID = "wiki-player-unit";
const WIKI_OPPONENT_UNIT_ID = "wiki-opponent-unit";
const WIKI_SECOND_PLAYER_UNIT_ID = "wiki-player-unit-second";
const WIKI_MANA_WELL_ID = "wiki-mana-well";
const WIKI_ACTIVATED_BUILDING_ID = "wiki-signal-tower";
const WIKI_EMBER_SQUIRE_CARD_ID = "wiki-card-ember-squire";
const WIKI_SPARK_JOLT_CARD_ID = "wiki-card-spark-jolt";

export const wikiEmberSquireCard = card({
  id: WIKI_EMBER_SQUIRE_CARD_ID,
  templateId: "ember-squire",
  name: "Ember Squire",
  rarity: "basic",
  cost: 1,
  text: "1 attack / 2 armor / 2 AP.",
  kind: { type: "unit", attack: 1, armor: 2, maxAp: 2 },
});

export const wikiSparkJoltCard = card({
  id: WIKI_SPARK_JOLT_CARD_ID,
  templateId: "spark-jolt",
  name: "Spark Jolt",
  rarity: "basic",
  cost: 1,
  text: "Priority 3. Range 2. Deal 1 damage to an enemy Unit or Hero.",
  kind: { type: "spell", range: 2, priority: 3, effect: { type: "damage", amount: 1 } },
});

export const wikiSceneCatalogCards: CatalogCard[] = [
  catalogCard(wikiEmberSquireCard, 12, "/card-art/ember-squire.svg"),
  catalogCard(wikiSparkJoltCard, 12, "/card-art/spark-jolt.svg"),
];

export const WIKI_DECK_RULES = {
  minCards: 60,
  basicCopyLimit: 5,
  advancedCopyLimit: 4,
  rareCopyLimit: 3,
  advancedTotalLimit: 24,
  rareTotalLimit: 12,
} as const;

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

export function evaluateDeckRecipeScene(
  counts: DeckRecipeCounts,
  rules: WikiDeckRulesScene["rules"],
): DeckRecipeSceneLegality {
  const totalCards = counts.basic + counts.advanced + counts.rare;
  const messages: string[] = [];

  if (totalCards < rules.minCards) {
    messages.push("Deck recipe needs at least 60 cards.");
  }
  if (counts.advanced > rules.advancedTotalLimit) {
    messages.push("Advanced card total must be 24 or fewer.");
  }
  if (counts.rare > rules.rareTotalLimit) {
    messages.push("Rare card total must be 12 or fewer.");
  }

  return {
    legal: messages.length === 0,
    messages,
    totalCards,
  };
}

export const WIKI_SCENES = [
  {
    type: "board",
    id: "turn-flow-refresh",
    title: "Turn Flow Refresh",
    summary: "Step through how turns pass, resources refresh, and rounds advance.",
    steps: [
      {
        title: "End of your turn",
        instruction:
          "The Player has spent Hero action points, a friendly Unit is damaged, and a Mana Well is occupied before control passes.",
        match: matchState({
          phase: "cardPlay",
          round: 1,
          activeSide: "player",
          player: { mana: 1, maxMana: 3, hero: hero("player", { q: -1, r: 1 }, { apRemaining: 0 }) },
          units: [unit("player", { q: 0, r: 0 }, { armor: 1, maxArmor: 2 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
          hand: [wikiEmberSquireCard],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "danger" },
        ],
        callouts: [
          { label: "Active side", value: "Player" },
          { label: "Round", value: "1" },
          { label: "Unit armor", value: "1/2" },
        ],
      },
      {
        title: "Opponent acts",
        instruction: "Ending the turn makes Opponent the active side. Player resources wait until Player's next turn starts.",
        match: matchState({
          round: 1,
          activeSide: "opponent",
          player: { mana: 1, maxMana: 3, hero: hero("player", { q: -1, r: 1 }, { apRemaining: 0 }) },
          opponent: { hero: hero("opponent", { q: 1, r: -1 }, { apRemaining: 3 }) },
          units: [unit("player", { q: 0, r: 0 }, { armor: 1, maxArmor: 2 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
        }),
        selectedPieceId: WIKI_OPPONENT_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_OPPONENT_HERO_ID, tone: "secondary" }],
        callouts: [
          { label: "Active side", value: "Opponent" },
          { label: "Priority", value: "None" },
        ],
      },
      {
        title: "Your next turn starts",
        instruction:
          "Player becomes active again. Mana, Hero action points, Unit action points, and Unit armor refresh at turn start.",
        match: matchState({
          round: 2,
          activeSide: "player",
          player: { mana: 4, maxMana: 4, hero: hero("player", { q: -1, r: 1 }, { apRemaining: 3 }) },
          units: [unit("player", { q: 0, r: 0 }, { armor: 2, maxArmor: 2, apRemaining: 2 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
          hand: [wikiEmberSquireCard],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" },
        ],
        callouts: [
          { label: "Mana", value: "4/4" },
          { label: "Hero AP", value: "3/3" },
          { label: "Unit armor", value: "2/2" },
        ],
      },
      {
        title: "Round advances",
        instruction:
          "The round number advances after both sides have acted and control returns to Player.",
        match: matchState({
          round: 2,
          activeSide: "player",
          player: { mana: 4, maxMana: 4, hero: hero("player", { q: -1, r: 1 }) },
          units: [unit("player", { q: 0, r: 0 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
          hand: [wikiEmberSquireCard, wikiSparkJoltCard],
        }),
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" }],
        callouts: [
          { label: "Active side", value: "Player" },
          { label: "Round", value: "2" },
          { label: "Hand count", value: "2" },
        ],
      },
    ],
  },
  {
    type: "board",
    id: "mana-source",
    title: "Mana Source",
    summary: "Compare base Mana with the bonus from occupying a Mana Well.",
    steps: [
      {
        title: "Base Mana",
        instruction: "A Hero currently provides 3 base Mana at turn start.",
        match: matchState({
          player: { mana: 3, maxMana: 3 },
        }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" }],
        callouts: [{ label: "Base Mana", value: "3" }],
      },
      {
        title: "Occupy a Mana Well",
        instruction: "A friendly Unit stands on the Mana Well. The Building is neutral, but the occupant gets the next refresh benefit.",
        match: matchState({
          player: { mana: 3, maxMana: 3 },
          units: [unit("player", { q: 0, r: 0 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 0 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" },
          { kind: "coord", coord: { q: 0, r: 0 }, tone: "secondary" },
        ],
        callouts: [{ label: "Mana Well", value: "Occupied" }],
      },
      {
        title: "Next turn refresh",
        instruction: "At Player's next turn start, Mana refreshes to 3 base plus 1 from the occupied Mana Well.",
        match: matchState({
          player: { mana: 4, maxMana: 4 },
          units: [unit("player", { q: 0, r: 0 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 0 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" },
          { kind: "coord", coord: { q: 0, r: 0 }, tone: "secondary" },
        ],
        callouts: [{ label: "Mana", value: "4/4 = 3 base + 1 Mana Well" }],
      },
    ],
  },
  {
    type: "board",
    id: "action-points-budget",
    title: "Action Point Budgets",
    summary: "Use Mana for Cards and separate action points for positioning before committing to Attack.",
    steps: [
      {
        title: "Hero and Mana budgets",
        instruction: "Start Movement with 3 Mana and 3 Hero action points. Cards spend Mana; movement spends the moving piece's action points.",
        match: matchState({ hand: [wikiEmberSquireCard, wikiSparkJoltCard] }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" }],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Mana", value: "3/3" },
          { label: "Hero AP", value: "3/3" },
        ],
      },
      {
        title: "Summon during Movement",
        instruction: "Ember Squire has resolved on the adjacent empty Hex. It spends 1 Mana, preserves Hero AP, and enters with 1/2 Unit action points.",
        match: matchState({
          player: { mana: 2, discardCount: 1 },
          hand: [wikiSparkJoltCard],
          units: [unit("player", { q: 0, r: 0 }, { apRemaining: 1 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 0 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "secondary" },
        ],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Mana", value: "2/3" },
          { label: "Hero AP", value: "3/3" },
          { label: "Unit AP", value: "1/2" },
        ],
      },
      {
        title: "Move the summoned Unit",
        instruction: "Ember Squire moves one adjacent Hex using its entry action point. Hero AP and Mana stay unchanged.",
        match: matchState({
          player: { mana: 2, discardCount: 1 },
          hand: [wikiSparkJoltCard],
          units: [unit("player", { q: 0, r: 1 }, { apRemaining: 0 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 1 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" },
          { kind: "coord", coord: { q: 0, r: 1 }, tone: "secondary" },
        ],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Mana", value: "2/3" },
          { label: "Hero AP", value: "3/3" },
          { label: "Unit AP", value: "0/2" },
        ],
      },
      {
        title: "Commit to Attack",
        instruction: "Starting Attack closes Movement. Proactive Cards are unavailable; pieces need action points and an enemy in range to attack. Ember Squire has spent its entry budget.",
        match: matchState({
          phase: "attack",
          player: { mana: 2, discardCount: 1 },
          hand: [wikiSparkJoltCard],
          units: [unit("player", { q: 0, r: 1 }, { apRemaining: 0 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" }],
        callouts: [
          { label: "Phase", value: "Attack" },
          { label: "Proactive Cards", value: "Unavailable" },
          { label: "Hero AP", value: "3/3" },
          { label: "Unit AP", value: "0/2" },
        ],
      },
      {
        title: "Final Card Play",
        instruction: "Finish attacks to reach final Card Play. Remaining Mana can pay for Spark Jolt, but movement and attacks cannot resume.",
        match: matchState({
          phase: "cardPlay",
          player: { mana: 2, discardCount: 1 },
          hand: [wikiSparkJoltCard],
          units: [unit("player", { q: 0, r: 1 }, { apRemaining: 0 })],
        }),
        selectedCardId: WIKI_SPARK_JOLT_CARD_ID,
        selectedPieceId: WIKI_OPPONENT_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_OPPONENT_HERO_ID, tone: "danger" }],
        callouts: [
          { label: "Phase", value: "Card Play" },
          { label: "Mana", value: "2/3" },
          { label: "Hero AP", value: "3/3" },
        ],
      },
      {
        title: "Spend remaining Mana",
        instruction: "Spark Jolt resolves for 1 damage after spending 1 Mana. Hero AP stays unchanged. End Turn is available once this final Card Play stack is empty.",
        match: matchState({
          phase: "cardPlay",
          player: { mana: 1, discardCount: 2 },
          opponent: { hero: hero("opponent", { q: 1, r: -1 }, { hp: 17 }) },
          hand: [],
          units: [unit("player", { q: 0, r: 1 }, { apRemaining: 0 })],
        }),
        selectedPieceId: WIKI_OPPONENT_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_OPPONENT_HERO_ID, tone: "danger" }],
        callouts: [
          { label: "Phase", value: "Card Play" },
          { label: "Mana", value: "1/3" },
          { label: "Hero AP", value: "3/3" },
          { label: "Opponent Hero HP", value: "17/18" },
        ],
      },
    ],
  },
  {
    type: "board",
    id: "cards-priority-stack",
    title: "Cards and Priority",
    summary: "A Movement summon waits for a higher-priority Spell response, then Movement resumes.",
    steps: [
      {
        title: "Movement Card enters the stack",
        instruction: "Playing Ember Squire spends 1 Mana and no Hero AP. The summon waits on the stack; ordinary board actions and phase changes are frozen.",
        match: matchState({
          mode: "shared",
          prioritySide: "opponent",
          player: { mana: 2, discardCount: 1 },
          hand: [],
          actionStack: [emberSquireStackItem()],
        }),
        focusedCoord: { q: 0, r: 0 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "coord", coord: { q: 0, r: 0 }, tone: "secondary" },
        ],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Mana", value: "2/3" },
          { label: "Hero AP", value: "3/3" },
          { label: "Stack items", value: "1" },
          { label: "Priority", value: "Opponent" },
        ],
      },
      {
        title: "Opponent responds with a Spell",
        instruction: "Opponent spends 1 Mana on Spark Jolt, a priority-3 response above the priority-0 summon. Player receives priority.",
        match: matchState({
          mode: "shared",
          prioritySide: "player",
          player: { mana: 2, discardCount: 1 },
          opponent: { mana: 2, discardCount: 1, handCount: 2 },
          hand: [],
          actionStack: [emberSquireStackItem(), sparkJoltStackItem()],
        }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "danger" }],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Top of stack", value: "Spark Jolt" },
          { label: "Below it", value: "Ember Squire summon" },
          { label: "Priority", value: "Player" },
        ],
      },
      {
        title: "Top resolves first",
        instruction: "Player passes. Spark Jolt resolves for 1 Hero damage while the summon stays pending and board actions stay frozen.",
        match: matchState({
          mode: "shared",
          prioritySide: "opponent",
          player: { mana: 2, discardCount: 1, hero: hero("player", { q: -1, r: 1 }, { hp: 19 }) },
          opponent: { mana: 2, discardCount: 1, handCount: 2 },
          hand: [],
          actionStack: [emberSquireStackItem()],
        }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "danger" }],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Hero HP", value: "19/20" },
          { label: "Hero AP", value: "3/3" },
          { label: "Stack items", value: "1" },
        ],
      },
      {
        title: "Movement resumes",
        instruction: "Opponent passes and the summon resolves. The stack empties in Movement; Ember Squire may now move with its 1/2 entry action points.",
        match: matchState({
          mode: "shared",
          player: { mana: 2, discardCount: 1, hero: hero("player", { q: -1, r: 1 }, { hp: 19 }) },
          opponent: { mana: 2, discardCount: 1, handCount: 2 },
          hand: [],
          units: [unit("player", { q: 0, r: 0 }, { apRemaining: 1 })],
          actionStack: [],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" }],
        callouts: [
          { label: "Phase", value: "Movement" },
          { label: "Mana", value: "2/3" },
          { label: "Hero AP", value: "3/3" },
          { label: "Unit AP", value: "1/2" },
          { label: "Stack items", value: "0" },
        ],
      },
    ],
  },
  {
    type: "board",
    id: "combat-range-counter",
    title: "Combat Range and Counterdamage",
    summary: "Range is measured by hex distance, and counterdamage requires the defender to reach back.",
    steps: [
      {
        title: "Range check",
        instruction: "An Archer-style Hero with range 2 can attack an enemy Unit two Hexes away.",
        match: matchState({
          phase: "attack",
          player: { hero: hero("player", { q: -1, r: 1 }, { heroType: "archer", attack: 2, attackRange: 2 }) },
          units: [unit("opponent", { q: 1, r: 1 }, { id: WIKI_OPPONENT_UNIT_ID })],
        }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        focusedCoord: { q: 1, r: 1 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "piece", pieceId: WIKI_OPPONENT_UNIT_ID, tone: "danger" },
        ],
        callouts: [{ label: "Hero range", value: "2" }],
      },
      {
        title: "No blocker rule",
        instruction: "Occupied Hexes between attacker and target do not block basic attacks. Range uses hex distance, not line of sight.",
        match: matchState({
          phase: "attack",
          player: { hero: hero("player", { q: -1, r: 1 }, { heroType: "archer", attack: 2, attackRange: 2 }) },
          units: [
            unit("player", { q: 0, r: 1 }, { id: WIKI_SECOND_PLAYER_UNIT_ID }),
            unit("opponent", { q: 1, r: 1 }, { id: WIKI_OPPONENT_UNIT_ID }),
          ],
        }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "piece", pieceId: WIKI_SECOND_PLAYER_UNIT_ID, tone: "secondary" },
          { kind: "piece", pieceId: WIKI_OPPONENT_UNIT_ID, tone: "danger" },
        ],
        callouts: [{ label: "Blockers", value: "Ignored for range checks" }],
      },
      {
        title: "No counterdamage",
        instruction: "The defender has range 1, so it cannot counterdamage an attacker two Hexes away.",
        match: matchState({
          phase: "attack",
          player: { hero: hero("player", { q: -1, r: 1 }, { heroType: "archer", attack: 2, attackRange: 2 }) },
          units: [unit("opponent", { q: 1, r: 1 }, { id: WIKI_OPPONENT_UNIT_ID, attackRange: 1 })],
        }),
        selectedPieceId: WIKI_OPPONENT_UNIT_ID,
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "primary" },
          { kind: "piece", pieceId: WIKI_OPPONENT_UNIT_ID, tone: "danger" },
        ],
        callouts: [
          { label: "Distance", value: "2" },
          { label: "Defender range", value: "1" },
        ],
      },
      {
        title: "Adjacent counterdamage",
        instruction: "When the attacker is adjacent, a range-1 defender can reach back and deal counterdamage.",
        match: matchState({
          phase: "attack",
          player: { hero: hero("player", { q: 0, r: 1 }, { heroType: "archer", attack: 2, attackRange: 2, hp: 14, maxHp: 16 }) },
          units: [unit("opponent", { q: 1, r: 1 }, { id: WIKI_OPPONENT_UNIT_ID, attackRange: 1 })],
        }),
        selectedPieceId: WIKI_PLAYER_HERO_ID,
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_HERO_ID, tone: "danger" },
          { kind: "piece", pieceId: WIKI_OPPONENT_UNIT_ID, tone: "secondary" },
        ],
        callouts: [
          { label: "Distance", value: "1" },
          { label: "Counterdamage", value: "Defender can reach" },
        ],
      },
    ],
  },
  {
    type: "board",
    id: "building-occupation",
    title: "Building Occupation",
    summary: "Buildings are neutral board features; occupation decides who benefits right now.",
    steps: [
      {
        title: "Neutral Building",
        instruction: "A Mana Well sits on the Hex without a permanent owner.",
        match: matchState({
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
        }),
        focusedCoord: { q: 0, r: 0 },
        highlights: [{ kind: "coord", coord: { q: 0, r: 0 }, tone: "secondary" }],
        callouts: [{ label: "Owner", value: "None" }],
      },
      {
        title: "Occupy the Building",
        instruction: "Either side can occupy the Building. The current occupant is the side that benefits.",
        match: matchState({
          units: [unit("player", { q: 0, r: 0 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 0 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" },
          { kind: "coord", coord: { q: 0, r: 0 }, tone: "secondary" },
        ],
        callouts: [{ label: "Occupant", value: "Player Unit" }],
      },
      {
        title: "Turn-start benefit",
        instruction: "The occupied Mana Well increases Player's next turn-start Mana.",
        match: matchState({
          player: { mana: 4, maxMana: 4 },
          units: [unit("player", { q: 0, r: 0 })],
          buildings: [manaWellBuilding({ q: 0, r: 0 })],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        highlights: [{ kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" }],
        callouts: [{ label: "Turn-start Mana", value: "+1 from occupation" }],
      },
      {
        title: "Activated Building",
        instruction: "Some Buildings have activated effects. The occupying Unit spends an action point and the activation goes on the stack.",
        match: matchState({
          units: [unit("player", { q: 0, r: 0 }, { apRemaining: 1 })],
          buildings: [activatedBuilding({ q: 0, r: 0 }, false)],
          actionStack: [buildingActivationStackItem()],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 0 },
        highlights: [
          { kind: "piece", pieceId: WIKI_PLAYER_UNIT_ID, tone: "primary" },
          { kind: "coord", coord: { q: 0, r: 0 }, tone: "secondary" },
        ],
        callouts: [
          { label: "Activation cost", value: "1 Unit AP" },
          { label: "Stack items", value: "1" },
        ],
      },
      {
        title: "Once per turn",
        instruction: "After activation, the Building is marked used and cannot activate again this turn.",
        match: matchState({
          units: [unit("player", { q: 0, r: 0 }, { apRemaining: 1 })],
          buildings: [activatedBuilding({ q: 0, r: 0 }, true)],
        }),
        selectedPieceId: WIKI_PLAYER_UNIT_ID,
        focusedCoord: { q: 0, r: 0 },
        highlights: [{ kind: "coord", coord: { q: 0, r: 0 }, tone: "danger" }],
        callouts: [{ label: "Building use", value: "Already activated this turn" }],
      },
    ],
  },
  {
    type: "deckRules",
    id: "deck-recipe-legality",
    title: "Deck Recipe Legality",
    summary: "Adjust rarity totals to see when a Draft deck recipe becomes Legal.",
    initialCounts: { basic: 33, advanced: 18, rare: 8 },
    rules: WIKI_DECK_RULES,
  },
] satisfies WikiScene[];

export function wikiSceneById(id: string): WikiScene | null {
  return WIKI_SCENES.find((scene) => scene.id === id) ?? null;
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

function hero(side: Side, position: HexCoord, overrides: Partial<Hero> = {}): Hero {
  return {
    id: side === "player" ? WIKI_PLAYER_HERO_ID : WIKI_OPPONENT_HERO_ID,
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
    ...overrides,
  };
}

function unit(side: Side, position: HexCoord, overrides: Partial<Unit> = {}): Unit {
  return {
    id: side === "player" ? WIKI_PLAYER_UNIT_ID : WIKI_OPPONENT_UNIT_ID,
    side,
    name: "Ember Squire",
    templateId: "ember-squire",
    attack: 1,
    attackRange: 1,
    armor: 2,
    maxArmor: 2,
    position,
    apRemaining: 2,
    maxAp: 2,
    hasAttacked: false,
    items: [],
    ...overrides,
  };
}

function card(cardValue: Card): Card {
  return cardValue;
}

function catalogCard(cardValue: Card, copyCount: number, artPath: string): CatalogCard {
  return {
    ...cardValue,
    id: cardValue.templateId,
    copyCount,
    artKey: cardValue.templateId,
    artPath,
  };
}

function matchState(options: {
  mode?: MatchState["mode"];
  phase?: MatchState["phase"];
  round?: number;
  activeSide?: Side;
  prioritySide?: Side | null;
  player?: Partial<Omit<MatchState["player"], "side" | "progression" | "hand">> & { hand?: Card[]; hero?: Hero };
  opponent?: Partial<Omit<MatchState["opponent"], "side" | "progression">> & { hero?: Hero };
  units?: Unit[];
  buildings?: Building[];
  manaSources?: HexCoord[];
  hand?: Card[];
  actionStack?: StackItem[];
  log?: string[];
} = {}): MatchState {
  const hand = options.hand ?? options.player?.hand ?? [wikiEmberSquireCard, wikiSparkJoltCard];

  return {
    mode: options.mode ?? "solo",
    round: options.round ?? 1,
    phase: options.phase ?? "movement",
    activeSide: options.activeSide ?? "player",
    prioritySide: options.prioritySide ?? null,
    player: {
      side: "player",
      mana: 3,
      maxMana: 3,
      hero: hero("player", { q: -1, r: 1 }),
      progression: emptyProgression,
      hand,
      handCount: hand.length,
      deckCount: 24,
      discardCount: 0,
      ...options.player,
    },
    opponent: {
      side: "opponent",
      mana: 3,
      maxMana: 3,
      hero: hero("opponent", { q: 1, r: -1 }),
      progression: emptyProgression,
      handCount: 3,
      deckCount: 25,
      discardCount: 0,
      ...options.opponent,
    },
    board: {
      radius: 3,
      tiles: radiusThreeTiles(),
      manaSources: options.manaSources ?? [],
      buildings: options.buildings ?? [],
      units: options.units ?? [],
      droppedItems: [],
    },
    actionStack: options.actionStack ?? [],
    log: options.log ?? ["Wiki scene state loaded."],
    winner: null,
  };
}

function manaWellBuilding(position: HexCoord): Building {
  return {
    id: WIKI_MANA_WELL_ID,
    templateId: "mana-well",
    name: "Mana Well",
    position,
    effect: { type: "turnStartMana", amount: 1 },
    activatedThisTurn: false,
  };
}

function activatedBuilding(position: HexCoord, activatedThisTurn: boolean): Building {
  return {
    id: WIKI_ACTIVATED_BUILDING_ID,
    templateId: "signal-tower",
    name: "Signal Tower",
    position,
    effect: { type: "activatedDamageLine", range: 3, amount: 1 },
    activatedThisTurn,
  };
}

function emberSquireStackItem(): StackItem {
  return {
    id: "wiki-stack-ember-squire",
    side: "player",
    priority: 0,
    action: {
      type: "playUnit",
      card: cardSummary(wikiEmberSquireCard),
      coord: { q: 0, r: 0 },
    },
  };
}

function sparkJoltStackItem(): StackItem {
  return {
    id: "wiki-stack-spark-jolt",
    side: "opponent",
    priority: 3,
    action: {
      type: "castSpell",
      card: cardSummary(wikiSparkJoltCard),
      targetId: WIKI_PLAYER_HERO_ID,
    },
  };
}

function buildingActivationStackItem(): StackItem {
  return {
    id: "wiki-stack-building",
    side: "player",
    priority: 1,
    action: {
      type: "activateBuilding",
      buildingId: WIKI_ACTIVATED_BUILDING_ID,
      occupantId: WIKI_PLAYER_UNIT_ID,
    },
  };
}

function cardSummary(cardValue: Card): CardSummary {
  return {
    templateId: cardValue.templateId,
    name: cardValue.name,
    rarity: cardValue.rarity,
    cost: cardValue.cost,
    kind: cardValue.kind,
  };
}

function distance(a: HexCoord, b: HexCoord) {
  const dq = a.q - b.q;
  const dr = a.r - b.r;
  const ds = -a.q - a.r - (-b.q - b.r);
  return Math.max(Math.abs(dq), Math.abs(dr), Math.abs(ds));
}
