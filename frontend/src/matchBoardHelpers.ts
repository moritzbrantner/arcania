import type { CSSProperties } from "react";
import type {
  ActionTarget,
  Building,
  BuildingEffect,
  Card,
  HexCoord,
  HexTile,
  MatchParticipantState,
  MatchState,
  ReplayFrame,
  Side,
  Team,
} from "./types";
import type { AnimatedPieceSnapshot, BoardAnimationCue, PieceAnimation } from "./boardAnimations";
import type { BoardPiece } from "./appTypes";
import { viewerSideLabel, heroTypeLabel } from "./labels";
import type { BuffTargetPolicy } from "./types";

export function groupTilesByColumn(tiles: HexTile[]) {
  const columns = new Map<number, HexTile[]>();
  for (const tile of tiles) {
    columns.set(tile.coord.q, [...(columns.get(tile.coord.q) ?? []), tile]);
  }

  return [...columns.entries()]
    .sort(([a], [b]) => a - b)
    .map(([q, columnTiles]) => ({
      q,
      tiles: columnTiles.sort((a, b) => a.coord.r - b.coord.r),
    }));
}

export function piecesForAnimation(pieces: BoardPiece[], animation: BoardAnimationCue | null): BoardPiece[] {
  if (!animation) {
    return pieces;
  }

  const pieceIds = new Set(pieces.map((piece) => piece.id));
  const exitingPieces = animation.exitingPieces
    .filter((piece) => !pieceIds.has(piece.id))
    .map(animatedSnapshotToBoardPiece);

  return [...pieces, ...exitingPieces];
}

export function animatedSnapshotToBoardPiece(piece: AnimatedPieceSnapshot): BoardPiece {
  return piece.pieceType === "hero"
    ? {
        ...piece,
        name: heroTypeLabel(piece.heroType),
      }
    : piece;
}

export function pieceAnimationStyle(animation?: PieceAnimation): CSSProperties | undefined {
  if (!animation?.from || !animation.to) {
    return undefined;
  }

  const deltaQ = animation.from.q - animation.to.q;
  const deltaR = animation.from.r - animation.to.r;

  return {
    "--piece-move-x": `calc(${deltaQ} * var(--hex-width) * 0.75)`,
    "--piece-move-y": `calc(${deltaR} * (var(--hex-height) + var(--hex-vertical-overlap)))`,
  } as CSSProperties;
}

export function pieceAnimationFeedback(animation?: PieceAnimation) {
  if (!animation) {
    return null;
  }

  switch (animation.kind) {
    case "damage":
      return animation.amount ? `-${animation.amount}` : "Hit";
    case "heal":
      return animation.amount ? `+${animation.amount}` : "Heal";
    case "buff": {
      const labels = [
        animation.attackDelta ? `+${animation.attackDelta} ATK` : null,
        animation.armorDelta ? `+${animation.armorDelta} ARM` : null,
      ].filter(Boolean);
      return labels.length > 0 ? labels.join(" ") : "Buff";
    }
    case "attack":
      return "Strike";
    default:
      return null;
  }
}

export function pieceAt(match: MatchState, coord: HexCoord): BoardPiece | null {
  for (const participant of participantsInMatch(match)) {
    if (!participant.knockedOut && sameCoord(participant.hero.position, coord)) {
      return {
        ...participant.hero,
        items: participant.hero.items ?? [],
        statMarkers: participant.hero.statMarkers ?? [],
        pieceType: "hero",
        name: heroTypeLabel(participant.hero.heroType),
      };
    }
  }

  const unit = match.board.units.find((candidate) => sameCoord(candidate.position, coord));
  return unit ? { ...unit, items: unit.items ?? [], statMarkers: unit.statMarkers ?? [], pieceType: "unit" } : null;
}

export function tileTitle(
  tile: HexTile,
  piece: BoardPiece | null,
  viewerSide: Side,
  droppedItemCount = 0,
  hasManaSource = false,
) {
  const coordLabel = `q ${tile.coord.q}, r ${tile.coord.r}`;
  const dropLabel =
    droppedItemCount > 0 ? `, ${droppedItemCount} dropped item${droppedItemCount === 1 ? "" : "s"}` : "";
  const sourceLabel = hasManaSource ? ", mana source" : "";
  if (!piece) {
    return `${coordLabel}, empty hex${sourceLabel}${dropLabel}`;
  }

  const owner = viewerSideLabel(piece.side, viewerSide);
  return `${coordLabel}, occupied by ${owner} ${piece.pieceType}${sourceLabel}${dropLabel}`;
}

export function droppedItemsAt(match: MatchState, coord: HexCoord) {
  return (match.board.droppedItems ?? []).filter((item) => sameCoord(item.position, coord));
}

export function isManaSourceAt(match: MatchState, coord: HexCoord) {
  return (
    (match.board.manaSources ?? []).some((source) => sameCoord(source, coord)) ||
    (match.board.buildings ?? []).some(
      (building) => building.effect.type === "turnStartMana" && sameCoord(building.position, coord),
    )
  );
}

export function buildingAt(match: MatchState, coord: HexCoord): Building | null {
  return (match.board.buildings ?? []).find((building) => sameCoord(building.position, coord)) ?? null;
}

export function isBuildingAt(match: MatchState, coord: HexCoord) {
  return !!buildingAt(match, coord) || (match.board.manaSources ?? []).some((source) => sameCoord(source, coord));
}

export function tileAt(match: MatchState, coord: HexCoord) {
  return match.board.tiles.find((tile) => sameCoord(tile.coord, coord)) ?? null;
}

export function pieceStatLabel(piece: BoardPiece) {
  const rangeLabel = piece.attackRange > 1 ? ` R${piece.attackRange}` : "";
  if (piece.pieceType === "hero") {
    return `${piece.attack}/${piece.hp} AP ${piece.apRemaining}${rangeLabel}`;
  }

  return `${piece.attack}/${piece.armor} AP ${piece.apRemaining}${rangeLabel}`;
}

export function pieceById(match: MatchState, pieceId: string): BoardPiece | null {
  for (const participant of participantsInMatch(match)) {
    if (!participant.knockedOut && participant.hero.id === pieceId) {
      return {
        ...participant.hero,
        items: participant.hero.items ?? [],
        statMarkers: participant.hero.statMarkers ?? [],
        pieceType: "hero",
        name: heroTypeLabel(participant.hero.heroType),
      };
    }
  }

  const unit = match.board.units.find((candidate) => candidate.id === pieceId);
  return unit ? { ...unit, items: unit.items ?? [], statMarkers: unit.statMarkers ?? [], pieceType: "unit" } : null;
}

export function participantBySide(match: MatchState, side: Side): MatchParticipantState {
  return participantsInMatch(match).find((participant) => participant.side === side) ?? match.player;
}

export function opponentSideOf(side: Side): Side {
  return teamOfSide(side) === "player" ? "opponent" : "player";
}

export function teamOfSide(side: Side): Team {
  return side === "player" || side === "playerTwo" ? "player" : "opponent";
}

export function sameTeam(a: Side, b: Side) {
  return teamOfSide(a) === teamOfSide(b);
}

export function participantsInMatch(match: MatchState): MatchParticipantState[] {
  if (match.participants?.length) {
    return match.participants;
  }
  return [match.player, match.opponent, match.playerTwo, match.opponentTwo].filter(
    (participant): participant is MatchParticipantState => Boolean(participant),
  );
}

export function handForSide(match: MatchState, side: Side): Card[] {
  return participantBySide(match, side).hand ?? [];
}

export function handCountForSide(match: MatchState, side: Side): number {
  return participantBySide(match, side).handCount;
}

export type CardFanStyle = CSSProperties & {
  "--card-fan-rotation": string;
  "--card-fan-rise": string;
  "--card-fan-shift": string;
};

export function cardFanStyle(index: number, total: number): CardFanStyle {
  const centerOffset = index - (total - 1) / 2;
  return {
    "--card-fan-rotation": `${centerOffset * 2.6}deg`,
    "--card-fan-rise": `${Math.abs(centerOffset) * -4}px`,
    "--card-fan-shift": `${centerOffset * 3}px`,
  };
}

export function piecesInMatch(match: MatchState): BoardPiece[] {
  return [
    ...participantsInMatch(match)
      .filter((participant) => !participant.knockedOut)
      .map((participant) => ({
        ...participant.hero,
        items: participant.hero.items ?? [],
        statMarkers: participant.hero.statMarkers ?? [],
        pieceType: "hero" as const,
        name: heroTypeLabel(participant.hero.heroType),
      })),
    ...match.board.units.map((unit) => ({
      ...unit,
      items: unit.items ?? [],
      statMarkers: unit.statMarkers ?? [],
      pieceType: "unit" as const,
    })),
  ];
}

export function hasPlayablePriorityResponse(match: MatchState, viewerSide: Side) {
  if (match.prioritySide !== viewerSide || match.actionStack.length === 0) {
    return false;
  }

  return handForSide(match, viewerSide).some(
    (card) =>
      card.kind.type === "spell" &&
      isPlayableCard(match, viewerSide, card) &&
      piecesInMatch(match).some((piece) =>
        isLegalCardTarget(match, viewerSide, card, piece.position, piece),
      ),
  );
}

export function isPlayableCard(match: MatchState, viewerSide: Side, card: Card) {
  if (viewerSide === "player" && match.legalCommands) {
    return match.legalCommands.some((command) => command.type === "playCard" && command.cardId === card.id);
  }
  const participant = participantBySide(match, viewerSide);
  const pending = topStackItem(match);
  if (pending) {
    return (
      match.phase !== "matchOver" &&
      match.prioritySide === viewerSide &&
      card.kind.type === "spell" &&
      participant.mana >= card.cost &&
      card.kind.priority > pending.priority
    );
  }

  return (
    match.phase === "cardPlay" &&
    match.activeSide === viewerSide &&
    participant.mana >= card.cost
  );
}

export function cardTargetForTile(
  match: MatchState,
  viewerSide: Side,
  card: Card,
  tile: HexTile,
): ActionTarget | null {
  const piece = pieceAt(match, tile.coord);
  if (!isLegalCardTarget(match, viewerSide, card, tile.coord, piece)) {
    return null;
  }

  if (card.kind.type === "unit" || card.kind.type === "manaSource" || card.kind.type === "building") {
    return { type: "hex", coord: tile.coord };
  }

  if (card.kind.type === "item") {
    return piece ? { type: "piece", pieceId: piece.id } : null;
  }

  return piece ? { type: "piece", pieceId: piece.id } : null;
}

export function isLegalCardTarget(
  match: MatchState,
  viewerSide: Side,
  card: Card,
  coord: HexCoord,
  piece: BoardPiece | null,
) {
  if (viewerSide === "player" && match.legalCommands) {
    return match.legalCommands.some((command) => command.type === "playCard" && command.cardId === card.id && (
      command.target.type === "hex" ? sameCoord(command.target.coord, coord) : command.target.pieceId === piece?.id
    ));
  }
  const participant = participantBySide(match, viewerSide);
  if (!isPlayableCard(match, viewerSide, card)) {
    return false;
  }

  if (card.kind.type === "unit") {
    return !piece && distance(participant.hero.position, coord) === 1;
  }

  if (card.kind.type === "manaSource" || card.kind.type === "building") {
    return (
      !piece &&
      !isBuildingAt(match, coord) &&
      distance(participant.hero.position, coord) === 1
    );
  }

  if (card.kind.type === "item") {
    return (
      !!piece &&
      targetPolicyAllows(card.kind.targets ?? "unitsOnly", piece.pieceType === "hero") &&
      sameTeam(piece.side, viewerSide) &&
      distance(participant.hero.position, piece.position) <= card.kind.range
    );
  }

  if (!piece || distance(participant.hero.position, piece.position) > card.kind.range) {
    return false;
  }

  switch (card.kind.effect.type) {
    case "heal":
      return sameTeam(piece.side, viewerSide);
    case "buff":
      return sameTeam(piece.side, viewerSide) && piece.pieceType === "unit";
    case "statBuff":
      return sameTeam(piece.side, viewerSide) && targetPolicyAllows(card.kind.effect.targets, piece.pieceType === "hero");
    case "damage":
    case "areaDamage":
      return !sameTeam(piece.side, viewerSide);
    case "draw":
      return piece.side === viewerSide && piece.pieceType === "hero";
    case "lineDamage":
      return (
        !sameTeam(piece.side, viewerSide) &&
        lineDirection(participant.hero.position, piece.position) !== null
      );
  }
}

function targetPolicyAllows(policy: BuffTargetPolicy, isHero: boolean) {
  switch (policy) {
    case "unitsOnly":
      return !isHero;
    case "heroesOnly":
      return isHero;
    case "unitsAndHeroes":
      return true;
  }
}

export function isLegalMove(match: MatchState, viewerSide: Side, piece: BoardPiece, coord: HexCoord) {
  if (viewerSide === "player" && match.legalCommands) {
    return match.legalCommands.some((command) => command.type === "movePiece" && command.pieceId === piece.id && sameCoord(command.to, coord));
  }
  return (
    match.actionStack.length === 0 &&
    match.phase === "movement" &&
    match.activeSide === viewerSide &&
    piece.side === viewerSide &&
    piece.apRemaining > 0 &&
    distance(piece.position, coord) === 1 &&
    !pieceAt(match, coord)
  );
}

export function isLegalAttack(
  match: MatchState,
  viewerSide: Side,
  attacker: BoardPiece,
  target: BoardPiece,
) {
  if (viewerSide === "player" && match.legalCommands) {
    return match.legalCommands.some((command) => command.type === "attack" && command.attackerId === attacker.id && command.targetId === target.id);
  }
  return (
    match.actionStack.length === 0 &&
    match.phase === "attack" &&
    match.activeSide === viewerSide &&
    attacker.side === viewerSide &&
    !sameTeam(target.side, viewerSide) &&
    attacker.apRemaining > 0 &&
    !attacker.hasAttacked &&
    distance(attacker.position, target.position) >= 1 &&
    distance(attacker.position, target.position) <= attacker.attackRange
  );
}

export function buildingEffectIsActivated(effect: BuildingEffect) {
  return (
    effect.type === "activatedDamageLine" ||
    effect.type === "activatedHeal" ||
    effect.type === "activatedStatBonus"
  );
}

export function topStackItem(match: MatchState) {
  return match.actionStack[match.actionStack.length - 1] ?? null;
}

export function distance(a: HexCoord, b: HexCoord) {
  const dq = a.q - b.q;
  const dr = a.r - b.r;
  const ds = -a.q - a.r - (-b.q - b.r);
  return Math.max(Math.abs(dq), Math.abs(dr), Math.abs(ds));
}

export function lineDirection(a: HexCoord, b: HexCoord): HexCoord | null {
  const hexDistance = distance(a, b);
  if (hexDistance === 0) {
    return null;
  }

  const directions = [
    { q: 1, r: 0 },
    { q: 1, r: -1 },
    { q: 0, r: -1 },
    { q: -1, r: 0 },
    { q: -1, r: 1 },
    { q: 0, r: 1 },
  ];

  return (
    directions.find(
      (direction) =>
        a.q + direction.q * hexDistance === b.q &&
        a.r + direction.r * hexDistance === b.r,
    ) ?? null
  );
}

export function sameCoord(a: HexCoord, b: HexCoord) {
  return a.q === b.q && a.r === b.r;
}

export function delay(milliseconds: number) {
  return new Promise((resolve) => window.setTimeout(resolve, milliseconds));
}

export function latestReplayEvent(frames: ReplayFrame[] | undefined) {
  return frames?.at(-1)?.event ?? null;
}

export function coordKey(coord: HexCoord) {
  return `${coord.q}:${coord.r}`;
}
