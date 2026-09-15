import { useFrame, useThree } from "@react-three/fiber";
import { useRef, type DragEvent } from "react";
import { createBoardProjection, type BoardProjection } from "../boardProjection";
import { projectBoardPositionToViewport } from "../boardRenderer";
import type { HexCoord } from "../types";
import { coordKey } from "./geometry";
import type { Board3DTileInteraction, BoardProjectedPosition } from "./types";

export function ProjectedHitTargetSync({
  tileInteractions,
  onProjectionChange,
}: {
  tileInteractions: Board3DTileInteraction[];
  onProjectionChange: (projection: BoardProjection) => void;
}) {
  const { camera, size } = useThree();
  const previousProjectionRef = useRef<BoardProjection | undefined>(undefined);

  useFrame(() => {
    camera.updateMatrixWorld();
    const nextProjection = createBoardProjection(
      tileInteractions.map((interaction) => ({
        coord: interaction.coord,
        position: projectBoardPositionToViewport(interaction.coord, camera, size),
      })),
      previousProjectionRef.current,
    );

    if (nextProjection !== previousProjectionRef.current) {
      previousProjectionRef.current = nextProjection;
      onProjectionChange(nextProjection);
    }
  });

  return null;
}

export function Board3DHitTarget({
  interaction,
  projectedPosition,
  readOnly,
  disabled,
  onClick,
  onDrop,
  onContextMenu,
  onHoverChange,
}: {
  interaction: Board3DTileInteraction;
  projectedPosition?: BoardProjectedPosition;
  readOnly: boolean;
  disabled: boolean;
  onClick: (coord: HexCoord) => void;
  onDrop: (coord: HexCoord, cardId: string) => void;
  onContextMenu: (coord: HexCoord, event: { clientX: number; clientY: number }) => void;
  onHoverChange?: (coord: HexCoord | null) => void;
}) {
  const className = [
    "board-3d-hit-target",
    interaction.hasManaSource ? "mana-source" : "",
    interaction.hasBuilding ? "building" : "",
    interaction.buildingDecor?.className ?? "",
    interaction.buildingDecor ? `building-accent-${interaction.buildingDecor.accent}` : "",
    interaction.buildingDecor?.activatedThisTurn ? "building-exhausted" : "",
    interaction.hasPiece ? "occupied" : "",
    interaction.pieceSide ? `occupied-${interaction.pieceSide}` : "",
    interaction.isLegal ? "legal" : "",
    interaction.isSelected ? "selected-piece" : "",
    interaction.isFocused ? "keyboard-focused" : "",
    interaction.tutorialHighlightTone ? "tutorial-highlight" : "",
    interaction.tutorialHighlightTone ? `tutorial-highlight-${interaction.tutorialHighlightTone}` : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      type="button"
      className={className}
      style={{
        left: projectedPosition ? `${projectedPosition.x}px` : "50%",
        top: projectedPosition ? `${projectedPosition.y}px` : "50%",
        visibility: projectedPosition?.visible ? "visible" : "hidden",
      }}
      disabled={disabled && !readOnly}
      tabIndex={readOnly ? -1 : undefined}
      aria-label={interaction.title}
      title={interaction.title}
      data-board-3d-tile={coordKey(interaction.coord)}
      data-legal={interaction.isLegal ? "true" : "false"}
      onClick={() => onClick(interaction.coord)}
      onPointerEnter={() => onHoverChange?.(interaction.coord)}
      onPointerLeave={() => onHoverChange?.(null)}
      onFocus={() => onHoverChange?.(interaction.coord)}
      onBlur={() => onHoverChange?.(null)}
      onDragOver={(event: DragEvent<HTMLButtonElement>) => {
        if (readOnly || disabled || !interaction.isLegal) {
          return;
        }

        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
      }}
      onDrop={(event: DragEvent<HTMLButtonElement>) => {
        if (readOnly || disabled) {
          return;
        }

        event.preventDefault();
        const cardId = event.dataTransfer.getData("text/plain");
        if (cardId) {
          onDrop(interaction.coord, cardId);
        }
      }}
      onContextMenu={(event) => {
        if (readOnly || disabled) {
          return;
        }

        event.preventDefault();
        onContextMenu(interaction.coord, {
          clientX: event.clientX,
          clientY: event.clientY,
        });
      }}
    >
      {interaction.buildingDecor ? (
        <span aria-hidden="true">{interaction.buildingDecor.glyph}</span>
      ) : null}
      {interaction.pieceLabel ? <span>{interaction.pieceLabel}</span> : null}
      {interaction.pieceStatLabel ? <strong>{interaction.pieceStatLabel}</strong> : null}
      {interaction.droppedItemCount ? (
        <em aria-label={`${interaction.droppedItemCount} dropped items`}>
          {interaction.droppedItemCount}
        </em>
      ) : null}
    </button>
  );
}
