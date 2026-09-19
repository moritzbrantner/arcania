import type { HexCoord, MatchState, Unit } from "../types";
import {
  TUTORIAL_EMBER_SQUIRE_CARD_ID,
  TUTORIAL_MOVE_COORD,
  TUTORIAL_OPPONENT_UNIT_ID,
  TUTORIAL_PLAYER_HERO_ID,
  TUTORIAL_PLAYER_UNIT_ID,
  TUTORIAL_SPARK_JOLT_CARD_ID,
  TUTORIAL_SUMMON_COORD,
  initialTutorialMatch,
  pendingTutorialAttack,
  sparkJoltStackItem,
  tutorialEmberSquireCard,
  tutorialSparkJoltCard,
  tutorialUnit,
} from "./tutorialFixtures";
import type {
  TutorialAction,
  TutorialHighlight,
  TutorialInteraction,
  TutorialState,
  TutorialStep,
} from "./tutorialTypes";
import { sameTutorialCoord } from "./tutorialHighlights";

export const TUTORIAL_COMPLETION_STORAGE_KEY = "arcania-tutorial-completed";

export const TUTORIAL_STEPS: TutorialStep[] = [
  {
    id: "board-goal",
    title: "Board and Goal",
    intro: "Your Hero is your avatar on the board and the defeat condition. Protect yours while pressuring the enemy Hero.",
    objective: "Inspect your Hero.",
    highlights: [
      { kind: "ui", targetId: "tutorial-board", tone: "secondary" },
      { kind: "piece", pieceId: TUTORIAL_PLAYER_HERO_ID, tone: "primary" },
      { kind: "piece", pieceId: "tutorial-opponent-hero", tone: "danger" },
      { kind: "ui", targetId: "tutorial-hand", tone: "secondary" },
    ],
  },
  {
    id: "mana-ap",
    title: "Mana and Action Points",
    intro: "Mana pays for cards. Hero action points pay for moving and attacking during your turn.",
    objective: "Inspect the highlighted card cost.",
    highlights: [
      { kind: "ui", targetId: "tutorial-player-badge", tone: "primary" },
      { kind: "ui", targetId: "tutorial-card-ember-squire", tone: "primary" },
    ],
  },
  {
    id: "play-unit",
    title: "Play a Unit Card",
    intro: "Unit cards create board pieces. Select the unit card, then choose a legal adjacent Hex.",
    objective: "Play Ember Squire onto the highlighted Hex.",
    highlights: [
      { kind: "ui", targetId: "tutorial-card-ember-squire", tone: "primary" },
      { kind: "coord", coord: TUTORIAL_SUMMON_COORD, tone: "primary" },
    ],
  },
  {
    id: "move-unit",
    title: "Move a Unit",
    intro: "Units have their own action points. Select your Unit, then choose a nearby empty Hex.",
    objective: "Move Ember Squire to the highlighted Hex.",
    highlights: [
      { kind: "piece", pieceId: TUTORIAL_PLAYER_UNIT_ID, tone: "primary" },
      { kind: "coord", coord: TUTORIAL_MOVE_COORD, tone: "primary" },
    ],
  },
  {
    id: "attack",
    title: "Attack",
    intro: "Adjacent enemies can be attacked. Select your Unit, then target the enemy Unit.",
    objective: "Attack the highlighted enemy Unit.",
    highlights: [
      { kind: "piece", pieceId: TUTORIAL_PLAYER_UNIT_ID, tone: "primary" },
      { kind: "piece", pieceId: TUTORIAL_OPPONENT_UNIT_ID, tone: "danger" },
    ],
  },
  {
    id: "end-turn",
    title: "End Turn",
    intro: "When you are done spending actions, end your turn so the opponent can act.",
    objective: "End your turn.",
    highlights: [
      { kind: "ui", targetId: "tutorial-phase", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-end-turn", tone: "primary" },
    ],
  },
  {
    id: "priority-response",
    title: "Stack and Priority",
    intro: "Some actions wait on the stack. When you have priority, you can answer with a higher-priority spell.",
    objective: "Play Spark Jolt as a response.",
    highlights: [
      { kind: "ui", targetId: "tutorial-stack", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-card-spark-jolt", tone: "primary" },
      { kind: "piece", pieceId: TUTORIAL_OPPONENT_UNIT_ID, tone: "danger" },
    ],
  },
];

export function createInitialTutorialState(): TutorialState {
  return {
    stepIndex: 0,
    phase: "intro",
    match: initialTutorialMatch(),
    selection: null,
    hint: null,
  };
}

export function currentTutorialStep(state: TutorialState) {
  return TUTORIAL_STEPS[state.stepIndex];
}

export function currentTutorialHighlights(state: TutorialState): TutorialHighlight[] {
  if (state.phase === "completed") {
    return [];
  }
  return currentTutorialStep(state).highlights;
}

export function tutorialReducer(state: TutorialState, action: TutorialAction): TutorialState {
  switch (action.type) {
    case "continue":
      return state.phase === "intro" ? { ...state, phase: "practice", hint: null } : state;
    case "back":
      return state.stepIndex === 0
        ? { ...state, phase: "intro", hint: null, selection: null }
        : sceneForStep(state.stepIndex - 1);
    case "restart":
      return createInitialTutorialState();
    case "interact":
      if (state.phase !== "practice") {
        return state;
      }
      return applyInteraction(state, action.interaction);
  }
}

function applyInteraction(state: TutorialState, interaction: TutorialInteraction): TutorialState {
  switch (currentTutorialStep(state).id) {
    case "board-goal":
      return interaction.type === "pieceClick" && interaction.pieceId === TUTORIAL_PLAYER_HERO_ID
        ? sceneForStep(1)
        : withHint(state, "Start by inspecting your own Hero.");
    case "mana-ap":
      return interaction.type === "cardClick" && interaction.cardId === TUTORIAL_EMBER_SQUIRE_CARD_ID
        ? sceneForStep(2)
        : withHint(state, "The highlighted card shows the Mana cost in its corner.");
    case "play-unit":
      return playUnitInteraction(state, interaction);
    case "move-unit":
      return moveUnitInteraction(state, interaction);
    case "attack":
      return attackInteraction(state, interaction);
    case "end-turn":
      return interaction.type === "endTurn"
        ? sceneForStep(6)
        : withHint(state, "Use End Turn when you are finished acting.");
    case "priority-response":
      return priorityResponseInteraction(state, interaction);
  }
}

function playUnitInteraction(state: TutorialState, interaction: TutorialInteraction): TutorialState {
  if (interaction.type === "cardClick" && interaction.cardId === TUTORIAL_EMBER_SQUIRE_CARD_ID) {
    return { ...state, selection: { type: "card", cardId: interaction.cardId }, hint: null };
  }

  const playedByDrop =
    interaction.type === "cardDrop" &&
    interaction.cardId === TUTORIAL_EMBER_SQUIRE_CARD_ID &&
    sameTutorialCoord(interaction.coord, TUTORIAL_SUMMON_COORD);
  const playedByClick =
    interaction.type === "tileClick" &&
    state.selection?.type === "card" &&
    state.selection.cardId === TUTORIAL_EMBER_SQUIRE_CARD_ID &&
    sameTutorialCoord(interaction.coord, TUTORIAL_SUMMON_COORD);

  if (playedByDrop || playedByClick) {
    return sceneForStep(3);
  }

  return withHint(state, "Select Ember Squire, then choose the highlighted Hex.");
}

function moveUnitInteraction(state: TutorialState, interaction: TutorialInteraction): TutorialState {
  if (interaction.type === "pieceClick" && interaction.pieceId === TUTORIAL_PLAYER_UNIT_ID) {
    return { ...state, selection: { type: "piece", pieceId: interaction.pieceId }, hint: null };
  }

  if (
    interaction.type === "tileClick" &&
    state.selection?.type === "piece" &&
    state.selection.pieceId === TUTORIAL_PLAYER_UNIT_ID &&
    sameTutorialCoord(interaction.coord, TUTORIAL_MOVE_COORD)
  ) {
    return sceneForStep(4);
  }

  return withHint(state, "Select your Unit first, then move it to the highlighted Hex.");
}

function attackInteraction(state: TutorialState, interaction: TutorialInteraction): TutorialState {
  if (interaction.type === "pieceClick" && interaction.pieceId === TUTORIAL_PLAYER_UNIT_ID) {
    return { ...state, selection: { type: "piece", pieceId: interaction.pieceId }, hint: null };
  }

  if (
    interaction.type === "pieceClick" &&
    interaction.pieceId === TUTORIAL_OPPONENT_UNIT_ID &&
    state.selection?.type === "piece" &&
    state.selection.pieceId === TUTORIAL_PLAYER_UNIT_ID
  ) {
    return sceneForStep(5);
  }

  return withHint(state, "Select your Unit, then click the highlighted enemy Unit.");
}

function priorityResponseInteraction(state: TutorialState, interaction: TutorialInteraction): TutorialState {
  if (interaction.type === "cardClick" && interaction.cardId === TUTORIAL_SPARK_JOLT_CARD_ID) {
    return { ...state, selection: { type: "card", cardId: interaction.cardId }, hint: null };
  }

  if (
    interaction.type === "pieceClick" &&
    interaction.pieceId === TUTORIAL_OPPONENT_UNIT_ID &&
    state.selection?.type === "card" &&
    state.selection.cardId === TUTORIAL_SPARK_JOLT_CARD_ID
  ) {
    return completeTutorialStack(state);
  }

  return withHint(state, "Select Spark Jolt, then target the highlighted enemy Unit.");
}

function completeTutorialStack(state: TutorialState): TutorialState {
  return {
    ...state,
    phase: "completed",
    selection: null,
    hint: "Tutorial complete.",
    match: {
      ...state.match,
      prioritySide: null,
      actionStack: [],
      log: [
        "Spark Jolt resolves first.",
        "Ash Hound is defeated.",
        "The pending attack fizzles.",
        "Tutorial complete.",
      ],
      board: {
        ...state.match.board,
        units: state.match.board.units.filter((unit) => unit.id !== TUTORIAL_OPPONENT_UNIT_ID),
      },
    },
  };
}

function withHint(state: TutorialState, hint: string): TutorialState {
  return { ...state, hint };
}

function sceneForStep(stepIndex: number): TutorialState {
  return {
    stepIndex,
    phase: "intro",
    match: matchForStep(stepIndex),
    selection: null,
    hint: null,
  };
}

function matchForStep(stepIndex: number): MatchState {
  if (stepIndex === 0) {
    return initialTutorialMatch();
  }

  if (stepIndex <= 2) {
    return {
      ...initialTutorialMatch(),
      phase: "cardPlay",
    };
  }

  if (stepIndex === 3) {
    return withUnits({
      phase: "movement",
      units: [tutorialUnit(TUTORIAL_SUMMON_COORD)],
      hand: [tutorialSparkJoltCard],
      log: ["Ember Squire joins the board."],
      mana: 2,
      heroAp: 2,
    });
  }

  if (stepIndex === 4) {
    return withUnits({
      phase: "attack",
      units: [
        tutorialUnit(TUTORIAL_MOVE_COORD, { apRemaining: 1 }),
        tutorialUnit(
          { q: 1, r: -1 },
          {
            id: TUTORIAL_OPPONENT_UNIT_ID,
            side: "opponent",
            name: "Ash Hound",
            templateId: "ash-hound",
            attack: 1,
            armor: 2,
            maxArmor: 2,
          },
        ),
      ],
      hand: [tutorialSparkJoltCard],
      log: ["Ember Squire moved into position."],
      mana: 2,
      heroAp: 2,
    });
  }

  if (stepIndex === 5) {
    return withUnits({
      phase: "cardPlay",
      units: [
        tutorialUnit(TUTORIAL_MOVE_COORD, { apRemaining: 1, hasAttacked: true }),
        tutorialUnit(
          { q: 1, r: -1 },
          {
            id: TUTORIAL_OPPONENT_UNIT_ID,
            side: "opponent",
            name: "Ash Hound",
            templateId: "ash-hound",
            attack: 1,
            armor: 1,
            maxArmor: 2,
          },
        ),
      ],
      hand: [tutorialSparkJoltCard],
      log: ["Ember Squire strikes Ash Hound for 1 damage."],
      mana: 2,
      heroAp: 2,
    });
  }

  if (stepIndex === 6) {
    return withUnits({
      phase: "attack",
      units: stackUnits(),
      hand: [tutorialSparkJoltCard],
      activeSide: "opponent",
      prioritySide: "player",
      actionStack: [pendingTutorialAttack()],
      log: ["Opponent attacks. You have priority."],
      mana: 2,
      heroAp: 2,
    });
  }

  return withUnits({
    phase: "attack",
    units: stackUnits(),
    hand: [],
    activeSide: "opponent",
    prioritySide: "player",
    actionStack: [pendingTutorialAttack(), sparkJoltStackItem()],
    log: ["Spark Jolt is added above the attack."],
    mana: 1,
    heroAp: 1,
  });
}

function withUnits(options: {
  units: Unit[];
  hand: MatchState["player"]["hand"];
  log: string[];
  mana: number;
  heroAp: number;
  activeSide?: "player" | "opponent";
  phase?: MatchState["phase"];
  prioritySide?: "player" | "opponent" | null;
  actionStack?: MatchState["actionStack"];
}): MatchState {
  const match = initialTutorialMatch();
  return {
    ...match,
    phase: options.phase ?? "movement",
    activeSide: options.activeSide ?? "player",
    prioritySide: options.prioritySide ?? null,
    actionStack: options.actionStack ?? [],
    log: options.log,
    player: {
      ...match.player,
      mana: options.mana,
      hero: {
        ...match.player.hero,
        apRemaining: options.heroAp,
      },
      hand: options.hand,
      handCount: options.hand.length,
    },
    opponent: {
      ...match.opponent,
      hero: {
        ...match.opponent.hero,
        position: { q: 2, r: -2 },
      },
    },
    board: {
      ...match.board,
      units: options.units,
    },
  };
}

function stackUnits(): Unit[] {
  return [
    tutorialUnit(TUTORIAL_MOVE_COORD, { apRemaining: 1, hasAttacked: true }),
    tutorialUnit(
      { q: 1, r: -1 },
      {
        id: TUTORIAL_OPPONENT_UNIT_ID,
        side: "opponent",
        name: "Ash Hound",
        templateId: "ash-hound",
        attack: 1,
        armor: 1,
        maxArmor: 2,
      },
    ),
  ];
}
