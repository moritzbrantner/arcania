import { describe, expect, it } from "vitest";
import {
  TUTORIAL_EMBER_SQUIRE_CARD_ID,
  TUTORIAL_MOVE_COORD,
  TUTORIAL_OPPONENT_UNIT_ID,
  TUTORIAL_PLAYER_HERO_ID,
  TUTORIAL_PLAYER_UNIT_ID,
  TUTORIAL_SPARK_JOLT_CARD_ID,
  TUTORIAL_SUMMON_COORD,
} from "./tutorialFixtures";
import {
  createInitialTutorialState,
  currentTutorialHighlights,
  currentTutorialStep,
  tutorialReducer,
} from "./tutorialReducer";
import type { TutorialInteraction, TutorialState } from "./tutorialTypes";

const heroClick: TutorialInteraction = { type: "pieceClick", pieceId: TUTORIAL_PLAYER_HERO_ID };
const emberClick: TutorialInteraction = { type: "cardClick", cardId: TUTORIAL_EMBER_SQUIRE_CARD_ID };
const unitClick: TutorialInteraction = { type: "pieceClick", pieceId: TUTORIAL_PLAYER_UNIT_ID };
const opponentPass: TutorialInteraction = { type: "opponentPass" };

describe("tutorialReducer", () => {
  it("starts at an intro and ignores gameplay until Continue", () => {
    const initial = createInitialTutorialState();
    expect(currentTutorialStep(initial).id).toBe("board-goal");
    expect(initial.phase).toBe("intro");
    expect(tutorialReducer(initial, { type: "interact", interaction: heroClick })).toBe(initial);
    expect(tutorialReducer(initial, { type: "continue" }).phase).toBe("practice");
  });

  it("keeps exploratory interactions on the same step with a hint", () => {
    const state = interact(createInitialTutorialState(), { type: "tileClick", coord: { q: 3, r: 0 } });
    expect(currentTutorialStep(state).id).toBe("board-goal");
    expect(state.hint).toContain("Hero");
  });

  it("resolves a Movement summon and spends partial entry AP before Attack and final Card Play", () => {
    let state = summonPending();
    expect(currentTutorialStep(state).id).toBe("resolve-summon");
    expect(state.match.mode).toBe("shared");
    expectBudgets(state, "movement", 2);
    expect(state.match.actionStack[0].action.type).toBe("playUnit");
    expect(state.match.prioritySide).toBe("opponent");
    expect(playerUnit(state)).toBeUndefined();
    expect(state.match.player.hand?.map((card) => card.id)).toEqual([TUTORIAL_SPARK_JOLT_CARD_ID]);
    expect(state.match.player.discardCount).toBe(1);
    expect(currentTutorialHighlights(state)).toContainEqual({ kind: "ui", targetId: "tutorial-opponent-pass", tone: "primary" });

    // Normal actions cannot skip a pending stack.
    const blocked = interact(state, { type: "startAttackPhase" });
    expect(blocked.match).toEqual(state.match);
    expect(currentTutorialStep(blocked).id).toBe("resolve-summon");

    state = interact(state, opponentPass);
    expect(currentTutorialStep(state).id).toBe("move-unit");
    expectBudgets(state, "movement", 2);
    expect(state.match.actionStack).toHaveLength(0);
    expect(state.match.prioritySide).toBeNull();
    expect(playerUnit(state)).toMatchObject({ position: TUTORIAL_SUMMON_COORD, apRemaining: 1, maxAp: 2 });
    expect(currentTutorialHighlights(state)).toContainEqual({ kind: "coord", coord: TUTORIAL_MOVE_COORD, tone: "primary" });

    state = interact(state, unitClick);
    state = interact(state, { type: "tileClick", coord: TUTORIAL_MOVE_COORD });
    expect(currentTutorialStep(state).id).toBe("resolve-move");
    expectBudgets(state, "movement", 2);
    expect(playerUnit(state)).toMatchObject({ position: TUTORIAL_SUMMON_COORD, apRemaining: 0 });
    expect(state.match.actionStack[0].action).toMatchObject({ type: "movePiece", from: TUTORIAL_SUMMON_COORD, to: TUTORIAL_MOVE_COORD });

    state = interact(state, opponentPass);
    expect(currentTutorialStep(state).id).toBe("start-attack");
    expectBudgets(state, "movement", 2);
    expect(playerUnit(state)).toMatchObject({ position: TUTORIAL_MOVE_COORD, apRemaining: 0 });
    expect(state.match.actionStack).toHaveLength(0);

    state = interact(state, { type: "startAttackPhase" });
    expect(currentTutorialStep(state).id).toBe("finish-attack");
    expectBudgets(state, "attack", 2);
    expect(playerUnit(state)).toMatchObject({ apRemaining: 0, hasAttacked: false });
    state = interact(state, { type: "startCardPlayPhase" });
    expect(currentTutorialStep(state).id).toBe("end-turn");
    expectBudgets(state, "cardPlay", 2);
    const cannotGoBack = interact(state, { type: "startAttackPhase" });
    expect(cannotGoBack.match).toEqual(state.match);
    expect(currentTutorialStep(cannotGoBack).id).toBe("end-turn");
  });

  it("charges only Mana for a response and clears the Spell before the invalidated attack", () => {
    let state = reachFinalCards();
    state = interact(state, { type: "endTurn" });
    expect(currentTutorialStep(state).id).toBe("priority-response");
    expectBudgets(state, "attack", 2);
    expect(state.match.activeSide).toBe("opponent");
    expect(state.match.prioritySide).toBe("player");
    expect(state.match.actionStack).toHaveLength(1);
    expect(state.match.board.units.find((unit) => unit.id === TUTORIAL_OPPONENT_UNIT_ID)).toMatchObject({ armor: 1, maxArmor: 1, apRemaining: 2, maxAp: 3, hasAttacked: true });

    state = interact(state, { type: "cardClick", cardId: TUTORIAL_SPARK_JOLT_CARD_ID });
    state = interact(state, { type: "pieceClick", pieceId: TUTORIAL_OPPONENT_UNIT_ID });
    expect(currentTutorialStep(state).id).toBe("resolve-response");
    expectBudgets(state, "attack", 1);
    expect(state.match.player.hand).toEqual([]);
    expect(state.match.player.discardCount).toBe(2);
    expect(state.match.actionStack.map((item) => item.priority)).toEqual([0, 3]);
    expect(state.match.prioritySide).toBe("opponent");

    state = interact(state, opponentPass);
    expect(currentTutorialStep(state).id).toBe("pass-priority");
    expectBudgets(state, "attack", 1);
    expect(state.match.actionStack).toHaveLength(1);
    expect(state.match.prioritySide).toBe("player");
    expect(state.match.board.units.some((unit) => unit.id === TUTORIAL_OPPONENT_UNIT_ID)).toBe(false);
    state = interact(state, { type: "passPriority" });
    expect(state.phase).toBe("completed");
    expectBudgets(state, "cardPlay", 1);
    expect(state.match.prioritySide).toBeNull();
    expect(state.match.actionStack).toHaveLength(0);
    expect(playerUnit(state)).toMatchObject({ armor: 2, apRemaining: 0 });
    expect(currentTutorialHighlights(state)).toEqual([]);
  });

  it("reconstructs the same pending scene on Back and resets all budgets on Restart", () => {
    const pending = summonPending();
    const resolved = interact(pending, opponentPass);
    const back = tutorialReducer(resolved, { type: "back" });
    expect(back).toEqual(pending);
    const finalCards = reachFinalCards();
    expect(tutorialReducer(finalCards, { type: "back" }).match.phase).toBe("attack");
    expect(tutorialReducer(finalCards, { type: "restart" })).toEqual(createInitialTutorialState());
  });
});

function interact(state: TutorialState, interaction: TutorialInteraction): TutorialState {
  return tutorialReducer(tutorialReducer(state, { type: "continue" }), { type: "interact", interaction });
}

function summonPending(): TutorialState {
  let state = interact(createInitialTutorialState(), heroClick);
  state = interact(state, emberClick);
  return interact(state, { type: "cardDrop", cardId: TUTORIAL_EMBER_SQUIRE_CARD_ID, coord: TUTORIAL_SUMMON_COORD });
}

function reachFinalCards(): TutorialState {
  let state = interact(summonPending(), opponentPass);
  state = interact(state, unitClick);
  state = interact(state, { type: "tileClick", coord: TUTORIAL_MOVE_COORD });
  state = interact(state, opponentPass);
  state = interact(state, { type: "startAttackPhase" });
  return interact(state, { type: "startCardPlayPhase" });
}

function playerUnit(state: TutorialState) {
  return state.match.board.units.find((unit) => unit.id === TUTORIAL_PLAYER_UNIT_ID);
}

function expectBudgets(state: TutorialState, phase: TutorialState["match"]["phase"], mana: number) {
  expect(state.match.phase).toBe(phase);
  expect(state.match.player.mana).toBe(mana);
  expect(state.match.player.hero.apRemaining).toBe(3);
}
