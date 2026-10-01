import type { HexCoord, MatchState } from "../types";
import type { BoardTutorialHighlight, TutorialHighlightTone } from "./tutorialHighlights";
import type { Selection } from "../appTypes";

export type TutorialStepId =
  | "board-goal"
  | "mana-ap"
  | "play-unit"
  | "resolve-summon"
  | "move-unit"
  | "resolve-move"
  | "start-attack"
  | "finish-attack"
  | "end-turn"
  | "priority-response"
  | "resolve-response"
  | "pass-priority";

export type TutorialTargetId =
  | "tutorial-board"
  | "tutorial-player-hero"
  | "tutorial-opponent-hero"
  | "tutorial-player-badge"
  | "tutorial-hand"
  | "tutorial-card-ember-squire"
  | "tutorial-card-spark-jolt"
  | "tutorial-end-turn"
  | "tutorial-start-attack"
  | "tutorial-finish-attack"
  | "tutorial-opponent-pass"
  | "tutorial-pass-priority"
  | "tutorial-phase"
  | "tutorial-stack";

export type TutorialUiHighlight = {
  kind: "ui";
  targetId: TutorialTargetId;
  tone: TutorialHighlightTone;
};

export type TutorialHighlight = BoardTutorialHighlight | TutorialUiHighlight;

export type TutorialStep = {
  id: TutorialStepId;
  title: string;
  intro: string;
  objective: string;
  highlights: TutorialHighlight[];
};

export type TutorialPhase = "intro" | "practice" | "completed";

export type TutorialState = {
  stepIndex: number;
  phase: TutorialPhase;
  match: MatchState;
  selection: Selection;
  hint: string | null;
};

export type TutorialInteraction =
  | { type: "pieceClick"; pieceId: string }
  | { type: "cardClick"; cardId: string }
  | { type: "tileClick"; coord: HexCoord }
  | { type: "cardDrop"; cardId: string; coord: HexCoord }
  | { type: "endTurn" }
  | { type: "startAttackPhase" }
  | { type: "startCardPlayPhase" }
  | { type: "opponentPass" }
  | { type: "passPriority" };

export type TutorialAction =
  | { type: "continue" }
  | { type: "back" }
  | { type: "restart" }
  | { type: "interact"; interaction: TutorialInteraction };
