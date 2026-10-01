// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { emberSquireCard, storyCardProjection, storyMatch } from "../../components/board.fixtures";
import { isLegalCardTarget } from "../../matchBoardHelpers";
import { useMatchBoardController } from "./useMatchBoardController";

afterEach(cleanup);

it("uses the same projected target for highlights, clicks, and drops, then rejects stale selection", () => {
  const submitAction = vi.fn();
  const tile = { coord: { q: 0, r: 0 } };
  const match = storyMatch({ hand: [emberSquireCard] });
  const { result, rerender } = renderHook(
    ({ match }) => useMatchBoardController({ match, viewerSide: "player", submitAction }),
    { initialProps: { match } },
  );
  expect(isLegalCardTarget(match, "player", emberSquireCard, tile.coord, null)).toBe(true);
  act(() => result.current.selectCard(emberSquireCard));
  act(() => result.current.handleTileClick(tile));
  act(() => result.current.handleCardDragStart(emberSquireCard));
  act(() => result.current.handleCardDrop(tile, emberSquireCard.id));
  const command = { type: "playCard", cardId: emberSquireCard.id, target: { type: "hex", coord: tile.coord } };
  expect(submitAction.mock.calls).toEqual([[command], [command]]);

  const rejected = {
    ...match, phase: "attack" as const,
    commandProjection: storyCardProjection([{ card: emberSquireCard, rejection: "wrongPhase" }]),
  };
  rerender({ match: rejected });
  expect(isLegalCardTarget(rejected, "player", emberSquireCard, tile.coord, null)).toBe(false);
  act(() => result.current.handleTileClick(tile));
  act(() => result.current.handleCardDrop(tile, emberSquireCard.id));
  expect(submitAction).toHaveBeenCalledTimes(2);
});
