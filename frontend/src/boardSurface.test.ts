import { describe, expect, it } from "vitest";
import {
  cinderRingCard,
  pendingAttackStack,
  pendingSpellStack,
  storyMatch,
  storyUnit,
} from "./components/board.fixtures";
import { pieceById } from "./matchBoardHelpers";
import { deriveBoardSurface } from "./boardSurface";

describe("board surface", () => {
  it("exposes selected Card targets and their targeting indicators", () => {
    const match = storyMatch({ hand: [cinderRingCard], phase: "cardPlay" });

    const surface = deriveBoardSurface({
      match,
      viewerSide: "player",
      selectedCard: cinderRingCard,
      selectedPiece: null,
      focusedCoord: { q: 1, r: 1 },
      hoveredCoord: null,
      readOnly: false,
      disabled: false,
      tutorialHighlights: [],
      animation: null,
      activeStackItemId: null,
    });

    expect(tileAt(surface, 1, 1)).toMatchObject({
      isLegal: true,
      isFocused: true,
      piece: { id: "opponent-hero" },
    });
    expect(tileAt(surface, 0, 0)?.isLegal).toBe(false);
    expect(surface.targetingIndicators).toEqual([
      expect.objectContaining({
        actionType: "spell",
        primaryTargetPieceId: "opponent-hero",
        secondaryFootprintCoords: expect.not.arrayContaining([{ q: 1, r: 1 }]),
      }),
    ]);
    expect(surface.targetingIndicators[0]?.secondaryFootprintCoords.length).toBeGreaterThan(0);
  });

  it("exposes selected Piece targets for the active phase", () => {
    const movementMatch = storyMatch({
      units: [storyUnit({ q: 0, r: 0 }, { id: "friendly-unit" })],
      opponentHero: { q: 1, r: 0 },
      phase: "movement",
    });
    const selectedPiece = pieceById(movementMatch, "friendly-unit");

    const movementSurface = deriveBoardSurface({
      match: movementMatch,
      viewerSide: "player",
      selectedCard: null,
      selectedPiece,
      focusedCoord: null,
      hoveredCoord: null,
      readOnly: false,
      disabled: false,
      tutorialHighlights: [],
      animation: null,
      activeStackItemId: null,
    });

    expect(tileAt(movementSurface, 0, 0)).toMatchObject({ isSelected: true, isLegal: false });
    expect(tileAt(movementSurface, -1, 0)).toMatchObject({ isLegal: true, piece: null });
    expect(tileAt(movementSurface, 1, 0)).toMatchObject({
      isLegal: false,
      piece: { id: "opponent-hero" },
    });

    const attackMatch = { ...movementMatch, phase: "attack" as const };
    const attackSurface = deriveBoardSurface({
      match: attackMatch,
      viewerSide: "player",
      selectedCard: null,
      selectedPiece,
      focusedCoord: null,
      hoveredCoord: null,
      readOnly: false,
      disabled: false,
      tutorialHighlights: [],
      animation: null,
      activeStackItemId: null,
    });

    expect(tileAt(attackSurface, -1, 0)).toMatchObject({ isLegal: false, piece: null });
    expect(tileAt(attackSurface, 1, 0)).toMatchObject({
      isLegal: true,
      piece: { id: "opponent-hero" },
    });
    expect(attackSurface.targetingIndicators).toEqual([
      expect.objectContaining({
        actionType: "attack",
        sourcePieceId: "friendly-unit",
        primaryTargetPieceId: "opponent-hero",
      }),
    ]);
  });

  it("exposes stack targeting and filters it through the same surface", () => {
    const match = storyMatch({
      actionStack: [pendingAttackStack, pendingSpellStack],
      prioritySide: "player",
    });

    const allIndicators = deriveBoardSurface({
      match,
      viewerSide: "player",
      selectedCard: null,
      selectedPiece: null,
      focusedCoord: null,
      hoveredCoord: null,
      readOnly: false,
      disabled: false,
      tutorialHighlights: [],
      animation: null,
      activeStackItemId: null,
    }).targetingIndicators;
    const filteredIndicators = deriveBoardSurface({
      match,
      viewerSide: "player",
      selectedCard: null,
      selectedPiece: null,
      focusedCoord: null,
      hoveredCoord: null,
      readOnly: false,
      disabled: false,
      tutorialHighlights: [],
      animation: null,
      activeStackItemId: pendingSpellStack.id,
    }).targetingIndicators;

    expect(allIndicators.map((indicator) => indicator.source)).toEqual([
      expect.objectContaining({ stackItemId: pendingAttackStack.id }),
      expect.objectContaining({ stackItemId: pendingSpellStack.id }),
    ]);
    expect(filteredIndicators).toEqual([
      expect.objectContaining({
        source: expect.objectContaining({ stackItemId: pendingSpellStack.id }),
      }),
    ]);
  });

  it("shares tutorial, feature, label, and disabled records across 2D and 3D adapters", () => {
    const match = storyMatch({ units: [storyUnit({ q: 0, r: 0 })] });
    match.board.buildings = [
      {
        id: "mana-well",
        templateId: "mana-well",
        name: "Mana Well",
        position: { q: 0, r: 0 },
        effect: { type: "turnStartMana", amount: 1 },
        activatedThisTurn: false,
      },
    ];
    match.board.droppedItems = [
      {
        id: "dropped-flask",
        position: { q: 0, r: 0 },
        item: {
          id: "flask",
          templateId: "ember-flask",
          name: "Ember Flask",
          passive: { type: "statBonus", attack: 1, armor: 0, maxAp: 0 },
          activeUsedThisTurn: false,
        },
      },
    ];

    const surface = deriveBoardSurface({
      match,
      viewerSide: "player",
      selectedCard: null,
      selectedPiece: null,
      focusedCoord: null,
      hoveredCoord: null,
      readOnly: true,
      disabled: false,
      tutorialHighlights: [
        { kind: "piece", pieceId: "story-player-unit", tone: "primary" },
        { kind: "coord", coord: { q: -1, r: 0 }, tone: "danger" },
      ],
      animation: null,
      activeStackItemId: null,
    });

    expect(surface.isInteractive).toBe(false);
    expect(tileAt(surface, 0, 0)).toMatchObject({
      disabled: true,
      tutorialHighlightTone: "primary",
      hasManaSource: true,
      hasBuilding: true,
      droppedItemCount: 1,
      pieceLabel: "YOU",
    });
    expect(tileAt(surface, 0, 0)?.title).toContain("1 dropped item");
    expect(tileAt(surface, 0, 0)?.title).toContain("building Mana Well");
    expect(tileAt(surface, -1, 0)?.tutorialHighlightTone).toBe("danger");
    const twoDimensionalTiles = surface.columns.flatMap((column) => column.tiles);
    expect(twoDimensionalTiles).toHaveLength(surface.tiles.length);
    for (const tile of twoDimensionalTiles) {
      expect(surface.tiles).toContain(tile);
    }
  });
});

function tileAt(surface: ReturnType<typeof deriveBoardSurface>, q: number, r: number) {
  return surface.tiles.find((tile) => tile.coord.q === q && tile.coord.r === r);
}
