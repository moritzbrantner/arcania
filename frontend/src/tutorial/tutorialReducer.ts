import type { HexCoord, MatchCommandProjection, MatchState } from "../types";
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
  tutorialSparkJoltCard,
  tutorialUnit,
  tutorialOpponentUnit,
  tutorialSummonStackItem,
  tutorialMoveStackItem,
} from "./tutorialFixtures";
import type {
  TutorialAction,
  TutorialHighlight,
  TutorialInteraction,
  TutorialState,
  TutorialStep,
  TutorialStepId,
} from "./tutorialTypes";
import { sameTutorialCoord } from "./tutorialHighlights";

export const TUTORIAL_COMPLETION_STORAGE_KEY = "rune-lanes-tutorial-completed";

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
    intro: "Cards spend Mana only, leaving Hero AP unchanged. You can play proactive Cards during Movement and final Card Play. Moving, attacking and activating Items use the acting piece’s AP.",
    objective: "Inspect the highlighted card cost.",
    highlights: [
      { kind: "ui", targetId: "tutorial-player-badge", tone: "primary" },
      { kind: "ui", targetId: "tutorial-card-ember-squire", tone: "primary" },
    ],
  },
  {
    id: "play-unit",
    title: "Play a Unit Card",
    intro: "Play Ember Squire during Movement for 1 Mana. In this Shared lesson the summon waits on the stack; Hero AP stays at 3.",
    objective: "Play Ember Squire onto the highlighted Hex.",
    highlights: [
      { kind: "ui", targetId: "tutorial-card-ember-squire", tone: "primary" },
      { kind: "coord", coord: TUTORIAL_SUMMON_COORD, tone: "primary" },
    ],
  },
  {
    id: "resolve-summon",
    title: "Resolve the Summon",
    intro: "While the summon is pending, normal actions pause. Let the scripted opponent pass: Ember Squire enters with 1 of its 2 AP, and Movement resumes.",
    objective: "Let the opponent pass to resolve Ember Squire.",
    highlights: [
      { kind: "ui", targetId: "tutorial-stack", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-opponent-pass", tone: "primary" },
    ],
  },
  {
    id: "move-unit",
    title: "Move a Unit",
    intro: "A summoned Unit enters with half its maximum AP, rounded down. Ember Squire has 1 of its 2 AP. Spend that entry AP to move one Hex during this same Movement phase; Hero AP stays at 3.",
    objective: "Move Ember Squire to the highlighted Hex.",
    highlights: [
      { kind: "piece", pieceId: TUTORIAL_PLAYER_UNIT_ID, tone: "primary" },
      { kind: "coord", coord: TUTORIAL_MOVE_COORD, tone: "primary" },
    ],
  },
  {
    id: "resolve-move",
    title: "Resolve the Move",
    intro: "The move spends Ember Squire’s last AP when queued. Its position changes when the opponent passes, then Movement resumes.",
    objective: "Let the opponent pass to resolve the move.",
    highlights: [
      { kind: "ui", targetId: "tutorial-stack", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-opponent-pass", tone: "primary" },
    ],
  },
  {
    id: "start-attack",
    title: "Start Attack",
    intro: "Movement and proactive Cards can interleave. Start Attack when you are ready; you cannot return to Movement this turn.",
    objective: "Start the Attack phase.",
    highlights: [{ kind: "ui", targetId: "tutorial-start-attack", tone: "primary" }],
  },
  {
    id: "finish-attack",
    title: "Finish Attacks",
    intro: "Ember Squire has 0 AP, so it cannot attack. Your Hero has no enemy in range. Finish Attacks to reach final Card Play, where you can spend remaining Mana but cannot move or return to Attack.",
    objective: "Finish Attacks to enter final Card Play.",
    highlights: [
      { kind: "piece", pieceId: TUTORIAL_PLAYER_UNIT_ID, tone: "secondary" },
      { kind: "ui", targetId: "tutorial-finish-attack", tone: "primary" },
    ],
  },
  {
    id: "end-turn",
    title: "End Turn",
    intro: "Final Card Play is the second proactive Card window. You still have 2 Mana and 3 Hero AP. Keep Spark Jolt for a response and end your turn; the scripted opponent then queues an attack.",
    objective: "End your turn.",
    highlights: [
      { kind: "ui", targetId: "tutorial-phase", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-end-turn", tone: "primary" },
    ],
  },
  {
    id: "priority-response",
    title: "Stack and Priority",
    intro: "The opponent’s attack is pending at priority 0. Answer with priority-3 Spark Jolt for 1 Mana. Hero AP stays at 3, and the Spell resolves before the attack.",
    objective: "Play Spark Jolt as a response.",
    highlights: [
      { kind: "ui", targetId: "tutorial-stack", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-card-spark-jolt", tone: "primary" },
      { kind: "piece", pieceId: TUTORIAL_OPPONENT_UNIT_ID, tone: "danger" },
    ],
  },
  {
    id: "resolve-response",
    title: "Resolve the Response",
    intro: "Spark Jolt sits above the pending attack. Let the opponent pass so its 1 damage defeats Ash Hound before the attack resolves.",
    objective: "Let the opponent pass to resolve Spark Jolt.",
    highlights: [
      { kind: "ui", targetId: "tutorial-stack", tone: "secondary" },
      { kind: "ui", targetId: "tutorial-opponent-pass", tone: "primary" },
    ],
  },
  {
    id: "pass-priority",
    title: "Clear the Pending Attack",
    intro: "Ash Hound is gone, but its attack is still on the stack. Pass priority: the attack has no attacker and fizzles. The stack empties and the opponent advances to final Card Play.",
    objective: "Pass priority to clear the attack.",
    highlights: [{ kind: "ui", targetId: "tutorial-pass-priority", tone: "primary" }],
  },
];

export function createInitialTutorialState(): TutorialState {
  return sceneForStep(0);
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
        ? nextScene(state)
        : withHint(state, "Start by inspecting your own Hero.");
    case "mana-ap":
      return interaction.type === "cardClick" && interaction.cardId === TUTORIAL_EMBER_SQUIRE_CARD_ID
        ? nextScene(state)
        : withHint(state, "The highlighted card shows the Mana cost in its corner.");
    case "play-unit":
      return playUnitInteraction(state, interaction);
    case "move-unit":
      return moveUnitInteraction(state, interaction);
    case "resolve-summon":
    case "resolve-move":
    case "resolve-response":
      return interaction.type === "opponentPass"
        ? nextScene(state)
        : withHint(state, "Let the scripted opponent pass to resolve the top stack item.");
    case "start-attack":
      return interaction.type === "startAttackPhase"
        ? nextScene(state)
        : withHint(state, "Use Start Attack to leave Movement.");
    case "finish-attack":
      return interaction.type === "startCardPlayPhase"
        ? nextScene(state)
        : withHint(state, "Use Finish Attacks to enter final Card Play.");
    case "end-turn":
      return interaction.type === "endTurn"
        ? nextScene(state)
        : withHint(state, "Use End Turn when you are finished acting.");
    case "priority-response":
      return priorityResponseInteraction(state, interaction);
    case "pass-priority":
      return interaction.type === "passPriority"
        ? completeTutorialStack(state)
        : withHint(state, "Pass priority to clear the attack.");
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
    return nextScene(state);
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
    return nextScene(state);
  }

  return withHint(state, "Select your Unit first, then move it to the highlighted Hex.");
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
    return nextScene(state);
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
      phase: "cardPlay",
      prioritySide: null,
      actionStack: [],
      log: [
        "Spark Jolt resolves first.",
        "Ash Hound is defeated.",
        "The pending attack fizzles.",
        "Tutorial complete.",
      ],
    },
  };
}

function withHint(state: TutorialState, hint: string): TutorialState {
  return { ...state, hint };
}

function sceneForStep(stepIndex: number): TutorialState {
  const match = matchForStep(stepIndex);
  const legalCommands = lessonTargets[TUTORIAL_STEPS[stepIndex].id] ?? [];
  return {
    stepIndex,
    phase: "intro",
    match: {
      ...match,
      legalCommands,
      commandProjection: {
        viewerSide: "player",
        proactiveCardPhases: ["movement", "cardPlay"],
        legalCommands,
        cards: (match.player.hand ?? []).map((card) => ({
          cardId: card.id,
          allowed: legalCommands.some((command) => command.type === "playCard" && command.cardId === card.id),
        })),
      },
    },
    selection: null,
    hint: null,
  };
}

// Fixed objective targets for authored scenes, not computed gameplay legality.
const lessonTargets: Partial<Record<TutorialStepId, MatchCommandProjection["legalCommands"]>> = {
  "play-unit": [{ type: "playCard", cardId: TUTORIAL_EMBER_SQUIRE_CARD_ID, target: { type: "hex", coord: TUTORIAL_SUMMON_COORD } }],
  "move-unit": [{ type: "movePiece", pieceId: TUTORIAL_PLAYER_UNIT_ID, to: TUTORIAL_MOVE_COORD }],
  "priority-response": [{ type: "playCard", cardId: TUTORIAL_SPARK_JOLT_CARD_ID, target: { type: "piece", pieceId: TUTORIAL_OPPONENT_UNIT_ID } }],
};

function nextScene(state: TutorialState): TutorialState {
  return sceneForStep(state.stepIndex + 1);
}

// Authored learning snapshots, not another gameplay rules implementation (ADR0015).
function matchForStep(stepIndex: number): MatchState {
  const stepId: TutorialStepId = TUTORIAL_STEPS[stepIndex].id;
  const initial = initialTutorialMatch();
  const summoned = afterSummon(TUTORIAL_SUMMON_COORD, 1);
  const moved = afterSummon(TUTORIAL_MOVE_COORD, 0);
  switch (stepId) {
    case "board-goal":
    case "mana-ap":
    case "play-unit":
      return initial;
    case "resolve-summon":
      return {
        ...summoned,
        prioritySide: "opponent",
        actionStack: [tutorialSummonStackItem()],
        board: initial.board,
        log: ["Ember Squire costs 1 Mana and waits on the stack. Hero AP remains 3."],
      };
    case "move-unit":
      return summoned;
    case "resolve-move":
      return {
        ...afterSummon(TUTORIAL_SUMMON_COORD, 0),
        prioritySide: "opponent",
        actionStack: [tutorialMoveStackItem()],
        log: ["Ember Squire spends its entry AP to queue a move."],
      };
    case "start-attack":
      return { ...moved, log: ["The move resolves. Movement resumes with Ember Squire at 0 AP."] };
    case "finish-attack":
      return { ...moved, phase: "attack", log: ["Attack begins. Movement and proactive Cards are closed."] };
    case "end-turn":
      return { ...moved, phase: "cardPlay", log: ["Final Card Play begins with 2 Mana and 3 Hero AP."] };
    case "priority-response":
      return opponentAttack();
    case "resolve-response": {
      const match = opponentAttack();
      return {
        ...match,
        prioritySide: "opponent",
        actionStack: [pendingTutorialAttack(), sparkJoltStackItem()],
        player: { ...match.player, mana: 1, hand: [], handCount: 0, discardCount: 2 },
        log: ["Spark Jolt costs 1 Mana and is queued above the attack. Hero AP remains 3."],
      };
    }
    case "pass-priority": {
      const response = matchForStep(stepIndex - 1);
      return {
        ...response,
        prioritySide: "player",
        actionStack: [pendingTutorialAttack()],
        board: { ...response.board, units: response.board.units.filter((unit) => unit.id !== TUTORIAL_OPPONENT_UNIT_ID) },
        log: ["Spark Jolt resolves first and defeats Ash Hound. The attack remains pending."],
      };
    }
  }
}

function afterSummon(position: HexCoord, unitAp: number): MatchState {
  const match = initialTutorialMatch();
  return {
    ...match,
    player: { ...match.player, mana: 2, hand: [tutorialSparkJoltCard], handCount: 1, discardCount: 1 },
    board: { ...match.board, units: [tutorialUnit(position, { apRemaining: unitAp }), tutorialOpponentUnit()] },
    log: ["Ember Squire resolves during Movement with 1/2 AP. Hero AP remains 3."],
  };
}

function opponentAttack(): MatchState {
  const match = afterSummon(TUTORIAL_MOVE_COORD, 0);
  return {
    ...match,
    activeSide: "opponent",
    phase: "attack",
    prioritySide: "player",
    actionStack: [pendingTutorialAttack()],
    board: {
      ...match.board,
      units: [tutorialUnit(TUTORIAL_MOVE_COORD, { apRemaining: 0 }), tutorialOpponentUnit({ apRemaining: 2, hasAttacked: true })],
    },
    log: ["The opponent advances through Movement and queues Ash Hound’s attack. You have priority."],
  };
}
