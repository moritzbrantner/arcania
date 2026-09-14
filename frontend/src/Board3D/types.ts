import type { ProjectedBoardPosition } from "../boardRenderer";
import type { BoardSurfaceTile } from "../boardSurface";
import type { MatchVisualCatalog } from "../matchVisualIdentity";
import type { TargetingIndicator } from "../targetingIndicators";
import type { HexCoord, HexTile, HeroAppearanceAssignment, HeroType, Side, Unit, Hero } from "../types";
import type { BoardAnimationCue } from "../boardAnimations";
import type { BoardPieceVisualManifest } from "../board3dModelManifest";

export type Board3DHero = {
  pieceType: "hero";
  id: string;
  side: Side;
  name: string;
  heroType: HeroType;
  hp: number;
  maxHp: number;
  attack: number;
  attackRange: number;
  position: HexCoord;
  apRemaining: number;
  maxAp: number;
  hasAttacked: boolean;
  items?: Hero["items"];
  statMarkers?: Hero["statMarkers"];
};

export type Board3DUnit = {
  pieceType: "unit";
  id: string;
  side: Side;
  name: string;
  templateId?: string;
  attack: number;
  attackRange: number;
  armor: number;
  maxArmor: number;
  position: HexCoord;
  apRemaining: number;
  maxAp: number;
  hasAttacked: boolean;
  items?: Unit["items"];
  statMarkers?: Unit["statMarkers"];
};

export type Board3DPiece = Board3DHero | Board3DUnit;

export type Board3DTileInteraction = BoardSurfaceTile;

export type BoardProjectedPosition = ProjectedBoardPosition;

export type Board3DRendererProps = {
  tiles: HexTile[];
  pieces: Board3DPiece[];
  visualCatalog: MatchVisualCatalog;
  readOnly: boolean;
  disabled: boolean;
  tileInteractions: Board3DTileInteraction[];
  targetingIndicators: TargetingIndicator[];
  heroAppearances?: HeroAppearanceAssignment[];
  animation?: BoardAnimationCue | null;
  onTileClick?: (tile: HexTile) => void;
  onTileDrop?: (tile: HexTile, cardId: string) => void;
  onTileContextMenu?: (tile: HexTile, event: { clientX: number; clientY: number }) => void;
  onTileHoverChange?: (coord: HexCoord | null) => void;
  onFatalRenderError: () => void;
  onAssetFailure?: (path: string) => void;
  manifest?: BoardPieceVisualManifest;
};
