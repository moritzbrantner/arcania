// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import * as boardStories from "./board.stories";
import { Board } from "./board";
import {
  cinderRingCard,
  pendingAttackStack,
  pendingMoveStack,
  pendingSpellStack,
  storyMatch,
  storyCardProjection,
} from "./board.fixtures";
import { createMatchVisualCatalog } from "../matchVisualIdentity";
import { pieceById } from "../matchBoardHelpers";
import { renderStory } from "../storybook/renderStory";

afterEach(() => cleanup());

describe("board stories", () => {
  it("renders board state stories", () => {
    renderStory(boardStories.default, boardStories.OpeningBoard);
    expect(screen.getByRole("region", { name: "Hex board" })).toBeInTheDocument();
    cleanup();

    renderStory(boardStories.default, boardStories.SelectedCardTargets);
    expect(screen.getByRole("region", { name: "Hex board" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "q 0, r 0, empty hex" })).toHaveClass("legal");
    expect(screen.getByRole("button", { name: /q 1, r 1, occupied/ })).not.toHaveClass("legal");
  });

  it("renders card, stack, and pile stories", () => {
    renderStory(boardStories.default, boardStories.CardButtonStates);
    expect(document.body).toHaveTextContent("Ember Squire");
    cleanup();

    renderStory(boardStories.default, boardStories.StackAndPiles);
    expect(document.body).toHaveTextContent("Stack");
  });

  it("applies 2d tutorial highlights to matching hexes", () => {
    render(
      <Board
        match={storyMatch()}
        boardVisualMode="2d"
        viewerSide="player"
        visualCatalog={createMatchVisualCatalog([])}
        selectedCard={null}
        selectedPiece={null}
        disabled={false}
        tutorialHighlights={[{ kind: "coord", coord: { q: 0, r: 1 }, tone: "primary" }]}
      />,
    );

    expect(screen.getByRole("button", { name: /q 0, r 1/i })).toHaveClass("tutorial-highlight");
  });

  it("renders 2d targeting indicators for selected attacks", async () => {
    const match = storyMatch({ phase: "attack" });

    render(
      <Board
        match={match}
        boardVisualMode="2d"
        viewerSide="player"
        visualCatalog={createMatchVisualCatalog([])}
        selectedCard={null}
        selectedPiece={pieceById(match, "player-hero")}
        disabled={false}
      />,
    );

    await waitFor(() =>
      expect(document.querySelector("[data-targeting-indicator]")).toBeInTheDocument(),
    );
  });

  it("shows selected spell footprints only after target hover", async () => {
    const match = storyMatch({ hand: [cinderRingCard], phase: "cardPlay", commandProjection: storyCardProjection([{ card: cinderRingCard, targets: [{ type: "piece", pieceId: "opponent-hero" }] }]) });

    render(
      <Board
        match={match}
        boardVisualMode="2d"
        viewerSide="player"
        visualCatalog={createMatchVisualCatalog([])}
        selectedCard={cinderRingCard}
        selectedPiece={null}
        disabled={false}
      />,
    );

    await waitFor(() =>
      expect(document.querySelector("[data-targeting-primary]")).toBeInTheDocument(),
    );
    expect(document.querySelector("[data-targeting-footprint]")).not.toBeInTheDocument();

    fireEvent.pointerEnter(screen.getByRole("button", { name: /q 1, r 1, occupied by the opponent's hero/i }));

    await waitFor(() =>
      expect(document.querySelector("[data-targeting-footprint]")).toBeInTheDocument(),
    );
  });

  it("lists queued stack actions and filters board indicators on row hover", async () => {
    render(
      <Board
        match={storyMatch({
          actionStack: [pendingAttackStack, pendingSpellStack, pendingMoveStack],
          prioritySide: "player",
        })}
        boardVisualMode="2d"
        viewerSide="player"
        visualCatalog={createMatchVisualCatalog([])}
        selectedCard={null}
        selectedPiece={null}
        disabled={false}
      />,
    );

    expect(screen.getByRole("region", { name: "Board stack targeting" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Show only Opponent attacks/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Show only You casts Spark Jolt/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Show only You moves player-hero/i })).toBeInTheDocument();

    await waitFor(() => expect(document.querySelectorAll("[data-targeting-indicator]")).toHaveLength(2));

    fireEvent.pointerEnter(screen.getByRole("button", { name: /Show only You casts Spark Jolt/i }));

    await waitFor(() => {
      const indicators = [...document.querySelectorAll("[data-targeting-indicator]")];
      expect(indicators).toHaveLength(1);
      expect(indicators[0]).toHaveAttribute("data-stack-item-id", "story-stack-2");
    });
  });
});
