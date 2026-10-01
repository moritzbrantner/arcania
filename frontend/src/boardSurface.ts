import type { BoardAnimationCue } from "./boardAnimations";
import type { BoardPiece } from "./appTypes";
import {
  buildingDecorForTile,
  type BoardSurfaceBuildingDecor,
} from "./buildingVisuals";
import type {
  Building,
  Card,
  DroppedItem,
  HexCoord,
  HexTile,
  MatchState,
  Side,
} from "./types";
import type {
  BoardTutorialHighlight,
  TutorialHighlightTone,
} from "./tutorial/tutorialHighlights";
import { sameTutorialCoord } from "./tutorial/tutorialHighlights";
import {
  buildingAt,
  coordKey,
  droppedItemsAt,
  groupTilesByColumn,
  isLegalAttack,
  isLegalCardTarget,
  isLegalMove,
  isManaSourceAt,
  pieceAt,
  pieceStatLabel,
  piecesForAnimation,
  piecesInMatch,
  sameCoord,
  tileTitle,
} from "./matchBoardHelpers";
import { viewerSideShortLabel } from "./labels";
import {
  selectedTargetingIndicators,
  stackTargetingIndicators,
  type TargetingIndicator,
} from "./targetingIndicators";

export type BoardSurfaceTile = {
  tile: HexTile;
  coord: HexCoord;
  title: string;
  disabled: boolean;
  isLegal: boolean;
  isSelected: boolean;
  isFocused: boolean;
  tutorialHighlightTone?: TutorialHighlightTone;
  hasManaSource: boolean;
  hasBuilding: boolean;
  building: Building | null;
  buildingDecor: BoardSurfaceBuildingDecor | null;
  hasPiece: boolean;
  piece: BoardPiece | null;
  displayPiece: BoardPiece | null;
  pieceSide?: Side;
  pieceType?: BoardPiece["pieceType"];
  pieceLabel?: string;
  pieceStatLabel?: string;
  droppedItems: DroppedItem[];
  droppedItemCount: number;
};

export type BoardSurfaceColumn = {
  q: number;
  tiles: BoardSurfaceTile[];
};

export type BoardSurface = {
  readOnly: boolean;
  disabled: boolean;
  isInteractive: boolean;
  tiles: BoardSurfaceTile[];
  columns: BoardSurfaceColumn[];
  pieces: BoardPiece[];
  targetingIndicators: TargetingIndicator[];
};

export type DeriveBoardSurfaceInput = {
  match: MatchState;
  viewerSide: Side;
  selectedCard: Card | null;
  selectedPiece: BoardPiece | null;
  focusedCoord: HexCoord | null;
  hoveredCoord: HexCoord | null;
  readOnly: boolean;
  disabled: boolean;
  tutorialHighlights: BoardTutorialHighlight[];
  animation: BoardAnimationCue | null;
  activeStackItemId: string | null;
};

export function deriveBoardSurface({
  match,
  viewerSide,
  selectedCard,
  selectedPiece,
  focusedCoord,
  hoveredCoord,
  readOnly,
  disabled,
  tutorialHighlights,
  animation,
  activeStackItemId,
}: DeriveBoardSurfaceInput): BoardSurface {
  const isInteractive = !readOnly && !disabled;
  const pieces = piecesForAnimation(piecesInMatch(match), animation);
  const displayPieceByCoord = new Map(
    pieces.map((piece) => [coordKey(piece.position), piece]),
  );
  const tiles = match.board.tiles.map((tile): BoardSurfaceTile => {
    const piece = pieceAt(match, tile.coord);
    const displayPiece = displayPieceByCoord.get(coordKey(tile.coord)) ?? piece;
    const droppedItems = droppedItemsAt(match, tile.coord);
    const hasManaSource = isManaSourceAt(match, tile.coord);
    const building = buildingAt(match, tile.coord);
    const isLegal =
      isInteractive &&
      Boolean(
        (selectedCard &&
          isLegalCardTarget(match, viewerSide, selectedCard, tile.coord, piece)) ||
          (selectedPiece &&
            ((!piece && isLegalMove(match, viewerSide, selectedPiece, tile.coord)) ||
              (piece && isLegalAttack(match, viewerSide, selectedPiece, piece)))),
      );
    const baseTitle = tileTitle(
      tile,
      piece,
      viewerSide,
      droppedItems.length,
      hasManaSource,
    );

    return {
      tile,
      coord: tile.coord,
      title: building ? `${baseTitle}, building ${building.name}` : baseTitle,
      disabled: !isInteractive,
      isLegal,
      isSelected: Boolean(piece && selectedPiece && piece.id === selectedPiece.id),
      isFocused: focusedCoord ? sameCoord(tile.coord, focusedCoord) : false,
      tutorialHighlightTone: tutorialHighlightForTile(
        tile.coord,
        piece?.id ?? null,
        tutorialHighlights,
      ),
      hasManaSource,
      hasBuilding: Boolean(building),
      building,
      buildingDecor: building ? buildingDecorForTile(building, piece?.side) : null,
      hasPiece: Boolean(piece),
      piece,
      displayPiece,
      pieceSide: piece?.side,
      pieceType: piece?.pieceType,
      pieceLabel: piece ? viewerSideShortLabel(piece.side, viewerSide) : undefined,
      pieceStatLabel: piece ? pieceStatLabel(piece) : undefined,
      droppedItems,
      droppedItemCount: droppedItems.length,
    };
  });
  const tileByKey = new Map(tiles.map((tile) => [coordKey(tile.coord), tile]));
  const columns = groupTilesByColumn(match.board.tiles).map((column) => ({
    q: column.q,
    tiles: column.tiles.map((tile) => tileByKey.get(coordKey(tile.coord))!),
  }));
  const selectedIndicators = selectedTargetingIndicators({
    match,
    viewerSide,
    selectedCard,
    selectedPiece,
    focusedCoord,
    hoveredCoord,
  });
  const stackIndicators = stackTargetingIndicators(match);
  const targetingIndicators = activeStackItemId
    ? stackIndicators.filter(
        (indicator) =>
          indicator.source.type === "stack" &&
          indicator.source.stackItemId === activeStackItemId,
      )
    : [...stackIndicators, ...selectedIndicators];

  return {
    readOnly,
    disabled,
    isInteractive,
    tiles,
    columns,
    pieces,
    targetingIndicators,
  };
}

function tutorialHighlightForTile(
  coord: HexCoord,
  pieceId: string | null,
  highlights: BoardTutorialHighlight[],
): TutorialHighlightTone | undefined {
  return highlights.find((highlight) => {
    if (highlight.kind === "coord") {
      return sameTutorialCoord(highlight.coord, coord);
    }

    return pieceId !== null && highlight.pieceId === pieceId;
  })?.tone;
}
