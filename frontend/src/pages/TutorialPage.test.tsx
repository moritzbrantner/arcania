// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEFAULT_ACCOUNT_PREFERENCES } from "../preferences";
import { TUTORIAL_COMPLETION_STORAGE_KEY } from "../tutorial/tutorialReducer";
import type { BoardTutorialHighlight } from "../tutorial/tutorialHighlights";
import type { Card, HexTile, MatchParticipantState, MatchState } from "../types";
import { TutorialPage } from "./TutorialPage";

vi.mock("../components/board", () => ({
  Board: ({
    match,
    disabled,
    tutorialHighlights = [],
    onTileClick,
  }: {
    match: MatchState;
    disabled: boolean;
    tutorialHighlights?: BoardTutorialHighlight[];
    onTileClick?: (tile: HexTile) => void;
  }) => (
    <section aria-label="Hex board">
      {match.board.tiles
        .filter((tile) => isHighlightedTutorialTile(match, tile, tutorialHighlights))
        .map((tile) => (
          <button
            key={`${tile.coord.q}:${tile.coord.r}`}
            aria-label={`Tutorial Hex q ${tile.coord.q}, r ${tile.coord.r}`}
            className="tutorial-highlight"
            type="button"
            disabled={disabled}
            onClick={() => onTileClick?.(tile)}
          />
        ))}
    </section>
  ),
  CardButton: ({
    card,
    disabled,
    tutorialTargetId,
    tutorialHighlighted,
    onClick,
  }: {
    card: Card;
    disabled: boolean;
    tutorialTargetId?: string;
    tutorialHighlighted?: boolean;
    onClick: () => void;
  }) => (
    <button
      className={tutorialHighlighted ? "tutorial-highlight" : undefined}
      type="button"
      disabled={disabled}
      data-tutorial-target={tutorialTargetId}
      onClick={onClick}
    >
      {card.name}
    </button>
  ),
  PileDisplay: ({ label }: { label: string }) => <div>{label}</div>,
  PlayerBadge: ({ player }: { player: MatchParticipantState }) => <div>{player.side}</div>,
  StackDisplay: () => <div>Stack</div>,
}));

afterEach(() => cleanup());

beforeEach(() => {
  window.localStorage.clear();
});

describe("TutorialPage", () => {
  it("renders intro overlay and highlighted targets", () => {
    renderTutorial();

    expect(screen.getByRole("dialog", { name: "Board and Goal" })).toBeInTheDocument();
    expect(screen.getByLabelText("Tutorial Hex q 0, r 1")).toHaveClass("tutorial-highlight");
  });

  it("runs the happy path and stores local completion", () => {
    renderTutorial();

    continueIntro();
    clickTile(0, 1);

    continueIntro();
    clickTutorialTarget("tutorial-card-ember-squire");

    continueIntro();
    clickTutorialTarget("tutorial-card-ember-squire");
    clickTile(0, 0);

    continueIntro();
    clickTile(0, 0);
    clickTile(1, 0);

    continueIntro();
    clickTile(1, 0);
    clickTile(1, -1);

    continueIntro();
    fireEvent.click(screen.getByText("End Turn", { selector: "button" }));

    continueIntro();
    clickTutorialTarget("tutorial-card-spark-jolt");
    clickTile(1, -1);

    expect(window.localStorage.getItem(TUTORIAL_COMPLETION_STORAGE_KEY)).toBe("true");
    expect(screen.getByText("Start Playing", { selector: "button" })).toBeInTheDocument();
  });
});

function renderTutorial() {
  return render(
    <TutorialPage
      currentUser={null}
      onNavigate={vi.fn()}
      onSignOut={vi.fn()}
      allowSignOut={false}
      loginNextPath="/tutorial"
      visualPreferences={{
        preferences: {
          ...DEFAULT_ACCOUNT_PREFERENCES,
          boardVisualMode: "2d",
        },
        effectiveMotion: "full",
        liveAiDelayMs: 0,
      }}
    />,
  );
}

function continueIntro() {
  fireEvent.click(screen.getByText("Continue", { selector: "button" }));
}

function clickTile(q: number, r: number) {
  fireEvent.click(screen.getByLabelText(`Tutorial Hex q ${q}, r ${r}`));
}

function clickTutorialTarget(targetId: string) {
  const target = document.querySelector<HTMLElement>(`[data-tutorial-target="${targetId}"]`);
  if (!target) {
    throw new Error(`Expected tutorial target ${targetId} to render`);
  }
  fireEvent.click(target);
}

function tutorialPieceIdAt(match: MatchState, tile: HexTile) {
  if (sameCoord(match.player.hero.position, tile.coord)) {
    return match.player.hero.id;
  }
  if (sameCoord(match.opponent.hero.position, tile.coord)) {
    return match.opponent.hero.id;
  }
  return match.board.units.find((unit) => sameCoord(unit.position, tile.coord))?.id ?? null;
}

function isHighlightedTutorialTile(
  match: MatchState,
  tile: HexTile,
  highlights: BoardTutorialHighlight[],
) {
  const pieceId = tutorialPieceIdAt(match, tile);
  return highlights.some((highlight) =>
    highlight.kind === "coord"
      ? sameCoord(highlight.coord, tile.coord)
      : highlight.pieceId === pieceId,
  );
}

function sameCoord(left: HexTile["coord"], right: HexTile["coord"]) {
  return left.q === right.q && left.r === right.r;
}
