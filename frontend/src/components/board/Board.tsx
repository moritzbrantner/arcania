import {
  Footprints,
  Heart,
  LibraryBig,
  Shield,
  Sparkles,
  Sword,
  WandSparkles,
  X,
  Zap,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type {
  CSSProperties,
  DragEvent as ReactDragEvent,
  MouseEvent as ReactMouseEvent,
  ReactNode,
} from "react";
import { Board3DRenderer } from "../../Board3D";
import type { BoardAnimationCue, PieceAnimation } from "../../boardAnimations";
import { deriveBoardSurface } from "../../boardSurface";
import { useBoardRendererDegradation } from "../../boardRendererDegradation";
import {
  createMatchVisualCatalog,
  type CardVisualIdentity,
  type MatchVisualCatalog,
  type UnitVisualIdentity,
  type HeroVisualIdentity,
} from "../../matchVisualIdentity";
import type { BoardPiece, BoardUnit, UnitContextMenu } from "../../appTypes";
import type {
  Building,
  BoardVisualMode,
  Card,
  HexCoord,
  HexTile,
  HeroAppearanceAssignment,
  MatchParticipantState,
  MatchState,
  Side,
  StackItem,
} from "../../types";
import type { BoardTutorialHighlight } from "../../tutorial/tutorialHighlights";
import { TargetingOverlay } from "../../targetingOverlay";
import { DetailStat } from "../common";
import { useDomBoardProjection } from "./domBoardProjection";
import {
  coordKey,
  pieceAnimationFeedback,
  pieceAnimationStyle,
} from "../../matchBoardHelpers";
import {
  itemActiveLabel,
  itemPassiveLabel,
  kindSummary,
  sideLabel,
  stackItemTitle,
  viewerSideShortLabel,
  heroTypeLabel,
} from "../../labels";

const EMPTY_MATCH_VISUAL_CATALOG = createMatchVisualCatalog([]);

export function UnitContextMenuView({
  menu,
  unit,
  onClose,
  onOpenCardInfo,
  canActivateItems,
  onActivateItem,
  building,
  canActivateBuilding,
  onActivateBuilding,
}: {
  menu: Exclude<UnitContextMenu, null>;
  unit: BoardPiece;
  onClose: () => void;
  onOpenCardInfo: () => void;
  canActivateItems: boolean;
  onActivateItem: (itemId: string) => void;
  building?: Building | null;
  canActivateBuilding?: boolean;
  onActivateBuilding?: (buildingId: string) => void;
}) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    function handlePointerDown() {
      onClose();
    }

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("pointerdown", handlePointerDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [onClose]);

  if (unit.pieceType !== "unit") {
    return null;
  }

  return (
    <div
      className="unit-context-menu"
      role="menu"
      aria-label={`${unit.name} actions`}
      style={{ left: menu.x, top: menu.y }}
      onPointerDown={(event) => event.stopPropagation()}
    >
      <button type="button" role="menuitem" onClick={onOpenCardInfo}>
        <LibraryBig size={15} />
        Card info
      </button>
      {(unit.items ?? [])
        .filter((item) => item.active)
        .map((item) => (
          <button
            key={item.id}
            type="button"
            role="menuitem"
            disabled={!canActivateItems || unit.apRemaining === 0 || item.activeUsedThisTurn}
            onClick={() => onActivateItem(item.id)}
          >
            <Sparkles size={15} />
            {item.name}
          </button>
        ))}
      {building && buildingEffectIsActivated(building.effect) ? (
        <button
          type="button"
          role="menuitem"
          disabled={!canActivateBuilding || unit.apRemaining === 0 || building.activatedThisTurn}
          onClick={() => onActivateBuilding?.(building.id)}
        >
          <Zap size={15} />
          {building.name}
        </button>
      ) : null}
    </div>
  );
}

function buildingEffectIsActivated(effect: Building["effect"]) {
  return (
    effect.type === "activatedDamageLine" ||
    effect.type === "activatedHeal" ||
    effect.type === "activatedStatBonus"
  );
}

export function UnitCardModal({
  unit,
  unitVisualIdentity,
  onClose,
}: {
  unit: BoardUnit;
  unitVisualIdentity: UnitVisualIdentity;
  onClose: () => void;
}) {
  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div
      className="modal-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <section
        className={`unit-modal ${unitVisualIdentity.rarity === "unknown" ? "basic unknown" : unitVisualIdentity.rarity}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby="unit-modal-title"
      >
        <button className="icon-button modal-close" type="button" onClick={onClose} title="Close">
          <X size={18} />
        </button>
        {unitVisualIdentity.portraitPath ? (
          <img src={unitVisualIdentity.portraitPath} alt={unitVisualIdentity.portraitAlt} />
        ) : (
          <div className="unit-modal-art-placeholder" aria-hidden="true">
            <span>{unitVisualIdentity.fallbackLabel}</span>
          </div>
        )}
        <div className="unit-modal-body">
          <div>
            <p className="eyebrow">
              {unit.side === "player" ? "Your" : "Opponent"} unit
              {unitVisualIdentity.rarity === "unknown" ? " · Card unknown" : ` · ${unitVisualIdentity.rarity}`}
            </p>
            <h2 id="unit-modal-title">{unitVisualIdentity.name}</h2>
          </div>

          <div className="detail-stat-row">
            {unitVisualIdentity.baseStats ? (
              <>
                <DetailStat label="Cost" value={unitVisualIdentity.baseStats.cost} />
                <DetailStat label="Base Attack" value={unitVisualIdentity.baseStats.attack} />
                <DetailStat label="Base Armor" value={unitVisualIdentity.baseStats.armor} />
                <DetailStat label="Base AP" value={unitVisualIdentity.baseStats.maxAp} />
              </>
            ) : (
              <DetailStat label="Card" value="Unknown" />
            )}
          </div>

          <div className="detail-stat-row live-stat-row">
            <DetailStat label="Attack" value={unit.attack} />
            {unit.attackRange > 1 ? <DetailStat label="Range" value={unit.attackRange} /> : null}
            <DetailStat label="Armor" value={`${unit.armor}/${unit.maxArmor}`} />
            <DetailStat label="AP" value={`${unit.apRemaining}/${unit.maxAp}`} />
            <DetailStat label="Attacked" value={unit.hasAttacked ? "Yes" : "No"} />
            <DetailStat label="Items" value={(unit.items ?? []).length} />
          </div>

          {(unit.items ?? []).length > 0 ? (
            <div className="unit-item-list" aria-label="Carried items">
              {(unit.items ?? []).map((item) => (
                <p key={item.id}>
                  <strong>{item.name}</strong>
                  <span>{itemPassiveLabel(item.passive)}</span>
                  {item.active ? <span>{itemActiveLabel(item.active)}</span> : null}
                </p>
              ))}
            </div>
          ) : null}

          <p className="detail-rules">
            {unitVisualIdentity.baseStats?.text ??
              "This unit came from an older match without saved card metadata."}
          </p>
        </div>
      </section>
    </div>
  );
}

export function PileDisplay({
  icon,
  label,
  count,
  status,
}: {
  icon: ReactNode;
  label: string;
  count: number;
  status: string;
}) {
  return (
    <div className="pile-display">
      <span className="pile-icon" aria-hidden="true">
        {icon}
      </span>
      <span className="pile-label">{label}</span>
      <strong>{count}</strong>
      <span className="pile-status">{status}</span>
    </div>
  );
}

export function StackDisplay({
  stack,
  prioritySide,
}: {
  stack: StackItem[];
  prioritySide: Side | null;
}) {
  if (stack.length === 0) {
    return null;
  }

  return (
    <section className="stack-panel" aria-label="Pending stack">
      <div className="stack-panel-header">
        <span>Stack</span>
        <strong>{sideLabel(prioritySide)} priority</strong>
      </div>
      <ol>
        {[...stack].reverse().map((item) => (
          <li key={item.id}>
            <span>{stackItemTitle(item)}</span>
            <strong>Priority {item.priority}</strong>
          </li>
        ))}
      </ol>
    </section>
  );
}

export function TargetingStackOverlay({
  stack,
  prioritySide,
  activeStackItemId,
  onActiveStackItemChange,
}: {
  stack: StackItem[];
  prioritySide: Side | null;
  activeStackItemId: string | null;
  onActiveStackItemChange: (stackItemId: string | null) => void;
}) {
  if (stack.length === 0) {
    return null;
  }

  return (
    <section className="targeting-stack-overlay" aria-label="Board stack targeting">
      <div className="targeting-stack-overlay-header">
        <span>Stack</span>
        <strong>{sideLabel(prioritySide)} priority</strong>
      </div>
      <ol>
        {[...stack].reverse().map((item) => {
          const active = activeStackItemId === item.id;

          return (
            <li key={item.id}>
              <button
                type="button"
                className={active ? "active" : ""}
                aria-label={`Show only ${stackItemTitle(item)} targeting`}
                data-targeting-stack-row={item.id}
                data-active={active ? "true" : "false"}
                onPointerEnter={() => onActiveStackItemChange(item.id)}
                onPointerLeave={() => onActiveStackItemChange(null)}
                onFocus={() => onActiveStackItemChange(item.id)}
                onBlur={() => onActiveStackItemChange(null)}
              >
                <span>{stackItemTitle(item)}</span>
                <strong>Priority {item.priority}</strong>
              </button>
            </li>
          );
        })}
      </ol>
    </section>
  );
}

export function PlayerBadge({ player }: { player: MatchParticipantState }) {
  return (
    <div className={`player-badge ${player.side}`}>
      <strong>{sideLabel(player.side)}</strong>
      <span className="hero-type-pill">
        <WandSparkles size={16} />
        {heroTypeLabel(player.hero.heroType)}
      </span>
      <span>
        <Heart size={16} />
        {player.hero.hp}/{player.hero.maxHp}
      </span>
      {(player.hero.shield ?? 0) > 0 ? (
        <span>
          <Shield size={16} />
          {player.hero.shield ?? 0}
        </span>
      ) : null}
      <span>
        <Zap size={16} />
        {player.hero.apRemaining}/{player.hero.maxAp}
      </span>
      <span>
        <Sparkles size={16} />
        {player.mana}/{player.maxMana}
      </span>
    </div>
  );
}

export function OpponentHandDisplay({ count }: { count: number }) {
  const visibleBacks = Math.min(count, 7);

  return (
    <div className="opponent-hand-display" aria-label={`Enemy hand, ${count} cards`}>
      <span className="opponent-hand-label">Enemy hand</span>
      <div className="opponent-card-backs" aria-hidden="true">
        {Array.from({ length: visibleBacks }, (_, index) => (
          <span className="opponent-card-back" key={index} />
        ))}
      </div>
      <strong>{count}</strong>
    </div>
  );
}

export function Board({
  match,
  animation,
  boardVisualMode,
  viewerSide,
  visualCatalog = EMPTY_MATCH_VISUAL_CATALOG,
  selectedCard,
  selectedPiece,
  focusedCoord,
  disabled,
  readOnly = false,
  onTileClick,
  onTileDrop,
  onUnitContextMenu,
  onFocusedUnitChange,
  tutorialHighlights = [],
}: {
  match: MatchState;
  animation?: BoardAnimationCue | null;
  boardVisualMode: BoardVisualMode;
  viewerSide: Side;
  visualCatalog?: MatchVisualCatalog;
  selectedCard: Card | null;
  selectedPiece: BoardPiece | null;
  focusedCoord?: HexCoord | null;
  heroAppearances?: HeroAppearanceAssignment[];
  disabled: boolean;
  readOnly?: boolean;
  onTileClick?: (tile: HexTile) => void;
  onTileDrop?: (tile: HexTile, cardId: string) => void;
  onUnitContextMenu?: (unit: BoardUnit, position: { x: number; y: number }) => void;
  onFocusedUnitChange?: (pieceId: string | null) => void;
  tutorialHighlights?: BoardTutorialHighlight[];
}) {
  const rendererDegradation = useBoardRendererDegradation({
    requestedMode: boardVisualMode,
    readOnly,
  });
  const [hoveredCoord, setHoveredCoord] = useState<HexCoord | null>(null);
  const [activeStackItemId, setActiveStackItemId] = useState<string | null>(null);
  const { renderer } = rendererDegradation;
  const boardCoords = useMemo(
    () => match.board.tiles.map((tile) => tile.coord),
    [match.board.tiles],
  );
  const domProjection = useDomBoardProjection(boardCoords, renderer === "2d");
  const [visibleAnimation, setVisibleAnimation] = useState<BoardAnimationCue | null>(animation ?? null);
  const animationByPieceId = useMemo(
    () => new Map((visibleAnimation?.pieces ?? []).map((pieceAnimation) => [pieceAnimation.pieceId, pieceAnimation])),
    [visibleAnimation],
  );
  const surface = useMemo(
    () =>
      deriveBoardSurface({
        match,
        viewerSide,
        selectedCard,
        selectedPiece,
        focusedCoord: focusedCoord ?? null,
        hoveredCoord,
        readOnly,
        disabled,
        tutorialHighlights,
        animation: visibleAnimation,
        activeStackItemId,
      }),
    [
      activeStackItemId,
      disabled,
      focusedCoord,
      hoveredCoord,
      match,
      readOnly,
      selectedCard,
      selectedPiece,
      tutorialHighlights,
      viewerSide,
      visibleAnimation,
    ],
  );

  useEffect(() => {
    setVisibleAnimation(animation ?? null);
    if (!animation) {
      return;
    }

    const timeoutId = window.setTimeout(
      () => setVisibleAnimation((current) => (current?.sequence === animation.sequence ? null : current)),
      animation.durationMs,
    );

    return () => window.clearTimeout(timeoutId);
  }, [animation]);

  useEffect(() => {
    if (!activeStackItemId || match.actionStack.some((item) => item.id === activeStackItemId)) {
      return;
    }

    setActiveStackItemId(null);
  }, [activeStackItemId, match.actionStack]);

  useEffect(() => {
    setHoveredCoord(null);
  }, [selectedCard?.id, selectedPiece?.id]);

  if (renderer === "3d") {
    return (
      <section
        className={`board board-visual-mode-${boardVisualMode} ${readOnly ? "read-only" : ""}`}
        data-board-visual-mode={boardVisualMode}
        data-board-renderer="3d"
        data-board-asset-failures={rendererDegradation.assetFailureCount}
        aria-label="Hex board"
      >
        {rendererDegradation.notice ? (
          <div className="board-renderer-notice" role="status">
            {rendererDegradation.notice}
          </div>
        ) : null}
        <TargetingStackOverlay
          stack={match.actionStack}
          prioritySide={match.prioritySide}
          activeStackItemId={activeStackItemId}
          onActiveStackItemChange={setActiveStackItemId}
        />
        <Board3DRenderer
          surface={surface}
          animation={visibleAnimation}
          visualCatalog={visualCatalog}
          onTileClick={onTileClick}
          onTileDrop={onTileDrop}
          onTileContextMenu={(tile, event) => {
            const piece = surface.tiles.find(
              (surfaceTile) => coordKey(surfaceTile.coord) === coordKey(tile.coord),
            )?.piece;
            if (piece?.pieceType !== "unit" || !onUnitContextMenu) {
              return;
            }

            onUnitContextMenu(piece, { x: event.clientX, y: event.clientY });
          }}
          onTileHoverChange={setHoveredCoord}
          onFatalRenderError={rendererDegradation.reportFatalRenderFailure}
          onAssetFailure={rendererDegradation.reportAssetFailure}
        />
      </section>
    );
  }

  return (
    <section
      className={`board board-visual-mode-${boardVisualMode} ${readOnly ? "read-only" : ""}`}
      data-board-visual-mode={boardVisualMode}
      data-board-renderer="2d"
      aria-label="Hex board"
    >
      {rendererDegradation.notice ? (
        <div className="board-renderer-notice" role="status">
          {rendererDegradation.notice}
        </div>
      ) : null}
      <TargetingStackOverlay
        stack={match.actionStack}
        prioritySide={match.prioritySide}
        activeStackItemId={activeStackItemId}
        onActiveStackItemChange={setActiveStackItemId}
      />
      <div className="hex-board-field" ref={domProjection.containerRef}>
        <TargetingOverlay
          indicators={surface.targetingIndicators}
          projection={domProjection.projection}
        />
        <div className="hex-board">
          {surface.columns.map((column) => (
            <div className="hex-column" key={column.q}>
              {column.tiles.map((surfaceTile) => {
                const {
                  tile,
                  piece,
                  displayPiece,
                  droppedItemCount,
                  hasManaSource,
                  hasBuilding,
                  building,
                  isLegal,
                  isSelected,
                  isFocused,
                  tutorialHighlightTone,
                  title,
                } = surfaceTile;
                const tileKey = coordKey(surfaceTile.coord);
                const occupantClass = displayPiece ? `occupied occupied-${displayPiece.side}` : "";

                return (
                  <button
                    key={tileKey}
                    ref={(element) => domProjection.registerElement(surfaceTile.coord, element)}
                    className={`hex-tile ${hasManaSource ? "mana-source" : ""} ${hasBuilding ? "building" : ""} ${building ? `building-${building.effect.type}` : ""} ${occupantClass} ${isLegal ? "legal" : ""} ${isSelected ? "selected-piece" : ""} ${isFocused ? "keyboard-focused" : ""} ${tutorialHighlightTone ? `tutorial-highlight tutorial-highlight-${tutorialHighlightTone}` : ""}`}
                    type="button"
                    disabled={surface.disabled && !surface.readOnly}
                    tabIndex={surface.readOnly ? -1 : undefined}
                    onPointerEnter={() => setHoveredCoord(tile.coord)}
                    onPointerLeave={() => setHoveredCoord(null)}
                    onFocus={() => {
                      setHoveredCoord(tile.coord);
                      onFocusedUnitChange?.(piece?.pieceType === "unit" ? piece.id : null);
                    }}
                    onBlur={() => setHoveredCoord(null)}
                    onClick={() => {
                      if (surface.isInteractive) {
                        onTileClick?.(tile);
                      }
                    }}
                    onDragOver={(event: ReactDragEvent<HTMLButtonElement>) => {
                      if (readOnly || disabled || !selectedCard || !isLegal) {
                        return;
                      }

                      event.preventDefault();
                      event.dataTransfer.dropEffect = "move";
                    }}
                    onDrop={(event: ReactDragEvent<HTMLButtonElement>) => {
                      if (readOnly || disabled || !onTileDrop) {
                        return;
                      }

                      event.preventDefault();
                      const cardId = event.dataTransfer.getData("text/plain");
                      if (cardId) {
                        onTileDrop(tile, cardId);
                      }
                    }}
                    onContextMenu={(event: ReactMouseEvent<HTMLButtonElement>) => {
                      if (readOnly || piece?.pieceType !== "unit" || !onUnitContextMenu) {
                        return;
                      }

                      event.preventDefault();
                      onUnitContextMenu(piece, { x: event.clientX, y: event.clientY });
                    }}
                    title={title}
                    aria-label={title}
                  >
                    {hasManaSource ? (
                      <span className="mana-source-marker" aria-hidden="true">
                        M
                      </span>
                    ) : null}
                    {building && !hasManaSource ? (
                      <span className="building-marker" aria-hidden="true">
                        B
                      </span>
                    ) : null}
                    {displayPiece ? (
                      <>
                        <span
                          className={`hex-occupant-marker ${displayPiece.side}`}
                          aria-hidden="true"
                        >
                          {viewerSideShortLabel(displayPiece.side, viewerSide)}
                        </span>
                        <PieceToken
                          key={`${displayPiece.id}-${visibleAnimation?.sequence ?? 0}`}
                          piece={displayPiece}
                          animation={animationByPieceId.get(displayPiece.id)}
                          viewerSide={viewerSide}
                          visualCatalog={visualCatalog}
                        />
                      </>
                    ) : null}
                    {droppedItemCount > 0 ? (
                      <span className="dropped-item-count" aria-hidden="true">
                        {droppedItemCount}
                      </span>
                    ) : null}
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}

export function PieceToken({
  piece,
  animation,
  viewerSide,
  visualCatalog,
}: {
  piece: BoardPiece;
  animation?: PieceAnimation;
  viewerSide: Side;
  visualCatalog: MatchVisualCatalog;
}) {
  let visualIdentity: UnitVisualIdentity | HeroVisualIdentity;
  let accentClass: string;
  if (piece.pieceType === "hero") {
    const heroVisualIdentity = visualCatalog.hero(piece);
    visualIdentity = heroVisualIdentity;
    accentClass = heroVisualIdentity.accentClass;
  } else {
    const unitVisualIdentity = visualCatalog.unit(piece);
    visualIdentity = unitVisualIdentity;
    accentClass = unitVisualIdentity.rarity;
  }
  const unknownClass = visualIdentity.status === "unknown" ? "unknown" : "";
  const animationClass = animation ? `piece-anim-${animation.kind}` : "";
  const feedbackLabel = pieceAnimationFeedback(animation);

  return (
    <span
      className={`piece-token portrait ${piece.side} ${piece.pieceType} ${accentClass} ${unknownClass} ${animationClass}`}
      style={pieceAnimationStyle(animation)}
      title={visualIdentity.name}
    >
      <span className={`piece-side-badge ${piece.side}`}>
        {viewerSideShortLabel(piece.side, viewerSide)}
      </span>
      <span className="piece-token-portrait" aria-hidden="true">
        {visualIdentity.portraitPath ? (
          <img src={visualIdentity.portraitPath} alt="" />
        ) : (
          <span>{visualIdentity.fallbackLabel}</span>
        )}
      </span>
      <strong>{visualIdentity.fallbackLabel}</strong>
      <span className="piece-token-stat-row">
        <span>
          <Sword size={11} />
          {piece.attack}
        </span>
        {piece.pieceType === "hero" ? (
          <span>
            <Heart size={11} />
            {piece.hp}
          </span>
        ) : (
          <span>
            <Shield size={11} />
            {piece.armor}
          </span>
        )}
        <span>
          <Footprints size={11} />
          {piece.apRemaining}
        </span>
        {piece.pieceType === "unit" && (piece.items ?? []).length > 0 ? (
          <span>
            <Sparkles size={11} />
            {(piece.items ?? []).length}
          </span>
        ) : null}
      </span>
      {feedbackLabel ? <span className="piece-feedback">{feedbackLabel}</span> : null}
    </span>
  );
}

export function CardButton({
  card,
  visualIdentity,
  selected,
  dragging = false,
  played = false,
  style,
  disabled,
  unavailable = false,
  availabilityReason,
  onClick,
  onFocus,
  onDragStart,
  onDragEnd,
  tutorialTargetId,
  tutorialHighlighted = false,
}: {
  card: Card;
  visualIdentity: CardVisualIdentity;
  selected: boolean;
  dragging?: boolean;
  played?: boolean;
  style?: CSSProperties;
  disabled: boolean;
  unavailable?: boolean;
  availabilityReason?: string | null;
  onClick: () => void;
  onFocus?: () => void;
  onDragStart?: (event: ReactDragEvent<HTMLButtonElement>) => void;
  onDragEnd?: () => void;
  tutorialTargetId?: string;
  tutorialHighlighted?: boolean;
}) {
  return (
    <button
      className={`card-button ${selected ? "selected" : ""} ${dragging ? "dragging" : ""} ${played ? "played" : ""} ${unavailable ? "unavailable" : ""} ${tutorialHighlighted ? "tutorial-highlight tutorial-highlight-primary" : ""} ${card.rarity}`}
      type="button"
      disabled={disabled}
      aria-disabled={disabled || unavailable}
      draggable={!disabled && !unavailable}
      style={style}
      title={availabilityReason ?? undefined}
      data-tutorial-target={tutorialTargetId}
      onClick={onClick}
      onFocus={onFocus}
      onDragStart={(event) => {
        if (unavailable) {
          event.preventDefault();
          return;
        }
        onDragStart?.(event);
      }}
      onDragEnd={onDragEnd}
    >
      {visualIdentity.artPath ? (
        <img className="card-button-art" src={visualIdentity.artPath} alt={visualIdentity.artAlt} />
      ) : null}
      <span className="card-cost">{card.cost}</span>
      <strong>{card.name}</strong>
      <span className="card-stats">
        <WandSparkles size={13} />
        {card.rarity} · {kindSummary(card)}
      </span>
      <span className="card-text">{card.text}</span>
    </button>
  );
}
