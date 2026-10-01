import type { BoardPiece } from "./appTypes";
import {
  buildingAt,
  buildingEffectIsActivated,
  commandProjectionForViewer,
  distance,
  handForSide,
  isLegalAttack,
  isLegalMove,
  participantBySide,
  pieceAt,
  pieceById,
  piecesInMatch,
  topStackItem,
} from "./matchBoardHelpers";
import { itemActiveLabel, itemPassiveLabel, kindSummary, sideLabel, stackItemTitle } from "./labels";
import type {
  Building,
  Card,
  CardKind,
  HexCoord,
  MatchState,
  ReplayEvent,
  Side,
} from "./types";

export type ActionAvailabilityReason = {
  code:
    | "matchOver"
    | "waitingForTurn"
    | "waitingForPriority"
    | "stackPending"
    | "notAResponse"
    | "priorityTooLow"
    | "insufficientMana"
    | "wrongPhase"
    | "noLegalTargets"
    | "pieceApEmpty"
    | "alreadyAttacked"
    | "itemExhausted"
    | "buildingExhausted"
    | "connectionBusy";
  message: string;
};

export type CardAvailability = {
  playable: boolean;
  reasons: ActionAvailabilityReason[];
  primaryReason: ActionAvailabilityReason | null;
};

export type ActionPreview = {
  title: string;
  body: string;
  details: string[];
  tone: "neutral" | "attack" | "support" | "blocked";
};

export type ActionTrayEntry = {
  id: string;
  label: string;
  icon: "move" | "attack" | "card" | "item" | "building" | "info";
  enabled: boolean;
  reason?: ActionAvailabilityReason;
  preview?: ActionPreview;
};

export type TurnChecklistItem = {
  id: string;
  label: string;
  value: string;
  status: "available" | "blocked" | "spent" | "waiting";
};

export type ActionRecap = {
  id: string;
  title: string;
  details: string[];
  tone: "neutral" | "attack" | "support" | "turn";
};

export type ActionAvailabilityOptions = {
  canAct?: boolean;
  connectionReady?: boolean;
  busy?: boolean;
};

export type ActionTraySelection =
  | { type: "card"; card: Card }
  | { type: "piece"; piece: BoardPiece }
  | null;

export function cardAvailability(
  match: MatchState,
  viewerSide: Side,
  card: Card,
  options: ActionAvailabilityOptions = {},
): CardAvailability {
  const reasons = cardAvailabilityReasons(match, viewerSide, card, options);
  return {
    playable: reasons.length === 0,
    reasons,
    primaryReason: reasons[0] ?? null,
  };
}

export function cardAvailabilityReason(
  match: MatchState,
  viewerSide: Side,
  card: Card,
  options: ActionAvailabilityOptions = {},
) {
  return cardAvailability(match, viewerSide, card, options).primaryReason;
}

function cardAvailabilityReasons(
  match: MatchState,
  viewerSide: Side,
  card: Card,
  options: ActionAvailabilityOptions,
): ActionAvailabilityReason[] {
  const participant = participantBySide(match, viewerSide);
  const pending = topStackItem(match);
  const reasons: ActionAvailabilityReason[] = [];

  if (options.busy) {
    reasons.push(reason("connectionBusy", "An action is already being submitted."));
  }

  if (options.connectionReady === false) {
    reasons.push(reason("connectionBusy", "The live connection is not ready."));
  }

  if (options.canAct === false) {
    reasons.push(
      reason(
        pending ? "waitingForPriority" : "waitingForTurn",
        pending ? "Waiting for the other side's priority." : "Waiting for your turn.",
      ),
    );
  }

  const projection = commandProjectionForViewer(match, viewerSide);
  const availability = projection?.cards.find((entry) => entry.cardId === card.id);
  if (!availability) {
    reasons.push(reason("connectionBusy", "Card availability is not ready."));
    return prioritizedReasons(reasons);
  }
  if (availability.allowed) {
    return prioritizedReasons(reasons);
  }

  // Context only orders the explanation; the projection owns availability.
  if (match.phase === "matchOver") {
    reasons.push(reason("matchOver", "This match is over."));
  } else if (pending && match.prioritySide !== viewerSide) {
    reasons.push(reason("waitingForPriority", "Waiting for the other side's priority."));
  } else if (!pending && match.activeSide !== viewerSide) {
    reasons.push(reason("waitingForTurn", "Waiting for your turn."));
  }

  switch (availability.rejection) {
    case "matchOver":
      reasons.push(reason("matchOver", "This match is over."));
      break;
    case "notActiveSide":
      reasons.push(reason("waitingForTurn", "Waiting for your turn."));
      break;
    case "notPrioritySide":
      reasons.push(reason("waitingForPriority", "Waiting for the other side's priority."));
      break;
    case "stackPending":
      reasons.push(reason("notAResponse", "Only spells can respond while the stack is pending."));
      break;
    case "priorityTooLow":
      reasons.push(reason("priorityTooLow", `Needs priority higher than ${pending?.priority ?? 0} to respond.`));
      break;
    case "notEnoughMana":
      reasons.push(reason("insufficientMana", `Need ${card.cost} mana; you have ${participant.mana}.`));
      break;
    case "wrongPhase": {
      const windows = projection?.proactiveCardPhases.includes("movement")
        ? "Cards can be played during Movement and Card Play, after attacks."
        : "Cards can be played during Card Play, after attacks.";
      reasons.push(reason("wrongPhase", windows));
      break;
    }
    default:
      reasons.push(reason("noLegalTargets", "No legal targets are available."));
  }
  return prioritizedReasons(reasons);
}

export function actionPreviewForCard(
  match: MatchState,
  viewerSide: Side,
  card: Card,
  target?: { coord?: HexCoord; piece?: BoardPiece | null },
  options: ActionAvailabilityOptions = {},
): ActionPreview {
  const availability = cardAvailability(match, viewerSide, card, options);
  const pending = topStackItem(match);
  const details = [
    `${card.cost} mana`,
    cardTargetSummary(card.kind),
    pending && card.kind.type === "spell"
      ? `Priority ${card.kind.priority}; current stack priority ${pending.priority}`
      : null,
    target?.coord ? `Target hex q ${target.coord.q}, r ${target.coord.r}` : null,
    target?.piece ? `Target ${target.piece.side === viewerSide ? "friendly" : "enemy"} ${target.piece.pieceType}` : null,
  ].filter((detail): detail is string => Boolean(detail));

  return {
    title: availability.playable ? card.name : `Cannot play ${card.name}`,
    body: availability.primaryReason?.message ?? `${kindSummary(card)}. ${effectSummary(card.kind)}`,
    details,
    tone: availability.playable ? toneForCard(card) : "blocked",
  };
}

export function actionPreviewForPiece(
  match: MatchState,
  viewerSide: Side,
  piece: BoardPiece,
): ActionPreview {
  const legalMoves = legalMoveCount(match, viewerSide, piece);
  const legalAttacks = legalAttackCount(match, viewerSide, piece);
  const details = [
    `AP ${piece.apRemaining}/${piece.maxAp}`,
    `Attack ${piece.attack}`,
    piece.attackRange > 1 ? `Range ${piece.attackRange}` : "Range 1",
    piece.hasAttacked ? "Already attacked" : "Attack ready",
  ];

  return {
    title: piece.name,
    body:
      piece.side !== viewerSide
        ? "Enemy piece. Inspect its stats and position before committing actions."
        : `${legalMoves} move ${plural(legalMoves, "target")} and ${legalAttacks} attack ${plural(legalAttacks, "target")} available.`,
    details,
    tone: piece.side === viewerSide ? "neutral" : "attack",
  };
}

export function actionTrayEntriesForSelection({
  match,
  viewerSide,
  selection,
  focusedPiece,
  canAct = true,
  availabilityOptions = {},
}: {
  match: MatchState;
  viewerSide: Side;
  selection: ActionTraySelection;
  focusedPiece?: BoardPiece | null;
  canAct?: boolean;
  availabilityOptions?: ActionAvailabilityOptions;
}): ActionTrayEntry[] {
  if (selection?.type === "card") {
    const options = { canAct, ...availabilityOptions };
    const availability = cardAvailability(match, viewerSide, selection.card, options);
    return [
      {
        id: `card-${selection.card.id}`,
        label: availability.playable ? "Choose a target" : "Cannot play",
        icon: "card",
        enabled: availability.playable,
        reason: availability.primaryReason ?? undefined,
        preview: actionPreviewForCard(match, viewerSide, selection.card, undefined, options),
      },
    ];
  }

  const piece = selection?.type === "piece" ? selection.piece : focusedPiece;
  if (!piece) {
    return [];
  }

  const entries: ActionTrayEntry[] = [
    {
      id: `info-${piece.id}`,
      label: "Card info",
      icon: "info",
      enabled: piece.pieceType === "unit",
      preview: actionPreviewForPiece(match, viewerSide, piece),
    },
  ];

  if (piece.side !== viewerSide) {
    return entries;
  }

  const moveCount = legalMoveCount(match, viewerSide, piece);
  const attackCount = legalAttackCount(match, viewerSide, piece);
  const apReason = reason("pieceApEmpty", `${piece.name} has no action points left.`);
  const stackReason = reason("stackPending", "Resolve the stack before moving or attacking.");
  const turnReason = reason("waitingForTurn", "Waiting for your turn.");

  entries.unshift(
    {
      id: `move-${piece.id}`,
      label: moveCount > 0 ? `Move (${moveCount})` : "Move",
      icon: "move",
      enabled: canAct && moveCount > 0,
      reason:
        !canAct
          ? turnReason
          : match.phase !== "movement"
            ? reason("wrongPhase", "Pieces move during the Movement Phase.")
            : piece.apRemaining <= 0
              ? apReason
              : match.actionStack.length > 0
                ? stackReason
                : undefined,
      preview: actionPreviewForPiece(match, viewerSide, piece),
    },
    {
      id: `attack-${piece.id}`,
      label: attackCount > 0 ? `Attack (${attackCount})` : "Attack",
      icon: "attack",
      enabled: canAct && attackCount > 0,
      reason: attackReason(match, viewerSide, piece, canAct),
      preview: actionPreviewForPiece(match, viewerSide, piece),
    },
  );

  for (const item of (piece.items ?? []).filter((candidate) => candidate.active)) {
    const activePriority = item.active?.priority ?? 0;
    const pending = topStackItem(match);
    const priorityReady = pending
      ? match.prioritySide === viewerSide && activePriority > pending.priority
      : match.activeSide === viewerSide;
    entries.push({
      id: `item-${item.id}`,
      label: item.name,
      icon: "item",
      enabled: canAct && piece.apRemaining > 0 && !item.activeUsedThisTurn && priorityReady,
      reason:
        item.activeUsedThisTurn
          ? reason("itemExhausted", `${item.name} has already been activated this turn.`)
          : piece.apRemaining <= 0
            ? apReason
            : pending && match.prioritySide !== viewerSide
              ? reason("waitingForPriority", "Waiting for priority.")
              : pending && activePriority <= pending.priority
                ? reason("priorityTooLow", `Needs priority higher than ${pending.priority} to respond.`)
                : !canAct
                  ? turnReason
                  : undefined,
      preview: {
        title: item.name,
        body: item.active ? itemActiveLabel(item.active) : itemPassiveLabel(item.passive),
        details: ["Costs 1 carrier AP", itemPassiveLabel(item.passive)],
        tone: "support",
      },
    });
  }

  const building = buildingAt(match, piece.position);
  if (building && buildingEffectIsActivated(building.effect)) {
    entries.push({
      id: `building-${building.id}`,
      label: building.name,
      icon: "building",
      enabled: canAct && piece.apRemaining > 0 && !building.activatedThisTurn && match.actionStack.length === 0,
      reason: buildingActivationReason(match, piece, building, canAct),
      preview: {
        title: building.name,
        body: buildingEffectSummary(building.effect),
        details: ["Costs 1 action point", "Requires occupying piece"],
        tone: buildingEffectTone(building.effect),
      },
    });
  }

  return entries;
}

export function turnChecklistForMatch(
  match: MatchState,
  viewerSide: Side,
  canAct = true,
): TurnChecklistItem[] {
  const participant = participantBySide(match, viewerSide);
  const pieces = piecesInMatch(match).filter((piece) => piece.side === viewerSide);
  const playableCards = handForSide(match, viewerSide).filter(
    (card) => cardAvailability(match, viewerSide, card, { canAct }).playable,
  ).length;
  const unitsAbleToAct = pieces.filter(
    (piece) => piece.pieceType === "unit" && piece.apRemaining > 0 && match.actionStack.length === 0 && canAct,
  ).length;
  const readyActivations = pieces.filter(
    (piece) =>
      canAct &&
      match.actionStack.length === 0 &&
      piece.apRemaining > 0 &&
      ((piece.items ?? []).some((item) => item.active && !item.activeUsedThisTurn) || Boolean(activatedReadyBuilding(match, piece))),
  ).length;

  if (match.phase === "matchOver") {
    return [
      {
        id: "match-over",
        label: "Match",
        value: `${sideLabel(match.winner)} wins`,
        status: "spent",
      },
    ];
  }

  const waitingItem: TurnChecklistItem | null = canAct
    ? null
    : {
        id: "waiting",
        label: "Waiting",
        value: match.actionStack.length > 0 ? `${sideLabel(match.prioritySide)} priority` : `${sideLabel(match.activeSide)} turn`,
        status: "waiting",
      };

  return [
    waitingItem,
    {
      id: "hero-ap",
      label: "Hero AP",
      value: `${participant.hero.apRemaining}/${participant.hero.maxAp}`,
      status: participant.hero.apRemaining > 0 && canAct ? "available" : "spent",
    },
    {
      id: "playable-cards",
      label: "Playable cards",
      value: String(playableCards),
      status: playableCards > 0 ? "available" : "blocked",
    },
    {
      id: "units-able",
      label: "Units able",
      value: String(unitsAbleToAct),
      status: unitsAbleToAct > 0 ? "available" : "spent",
    },
    {
      id: "activations",
      label: "Activations",
      value: String(readyActivations),
      status: readyActivations > 0 ? "available" : "spent",
    },
  ].filter((item): item is TurnChecklistItem => item !== null);
}

export function actionRecapFromReplayEvent(
  event: ReplayEvent,
  viewerSide: Side,
): ActionRecap | null {
  switch (event.type) {
    case "turnStarted":
      return recap(`Turn started`, [`${sideLabel(event.side)} begins round ${event.round}.`], "turn", event);
    case "turnEnded":
      return recap(`Turn ended`, [`${sideLabel(event.side)} ended round ${event.round}.`], "turn", event);
    case "actionQueued":
      return recap(
        `${sideLabel(event.side)} queued an action`,
        [stackItemTitle(event.item)],
        "neutral",
        event,
      );
    case "unitSummoned":
      return recap(`${sideLabel(event.side)} summoned ${event.name}`, [coordDetail(event.position)], "neutral", event);
    case "pieceMoved":
      return recap(`${sideLabel(event.side)} moved`, [`${coordDetail(event.from)} to ${coordDetail(event.to)}`], "neutral", event);
    case "pieceAttacked":
      return recap(
        `${sideLabel(event.side)} attacked`,
        [
          `${pieceLabel(event.attackerId)} hit ${pieceLabel(event.targetId)} for ${event.damageToTarget}.`,
          event.counterDamageToAttacker > 0
            ? `${pieceLabel(event.attackerId)} took ${event.counterDamageToAttacker} counter damage.`
            : null,
        ].filter((detail): detail is string => Boolean(detail)),
        "attack",
        event,
      );
    case "pieceHealed":
      return recap(`${sideLabel(event.side)} healed`, [`${pieceLabel(event.pieceId)} healed ${event.amount}.`], "support", event);
    case "unitArmorRefreshed":
      return recap(`${sideLabel(event.side)} refreshed armor`, [`${pieceLabel(event.unitId)} restored ${event.amount} armor.`], "support", event);
    case "pieceBuffed":
      return recap(
        `${sideLabel(event.side)} buffed`,
        [`${pieceLabel(event.pieceId)} gained ${event.attackDelta} attack and ${event.armorDelta} armor.`],
        "support",
        event,
      );
    case "pieceDamaged":
      return recap(`${sideLabel(event.side)} dealt damage`, [`${pieceLabel(event.pieceId)} took ${event.amount}.`], "attack", event);
    case "unitDestroyed":
      return recap(`${event.name} was destroyed`, [`${sideLabel(event.side)} lost a Unit.`], "attack", event);
    case "manaGained":
      return recap(`${sideLabel(event.side)} gained mana`, [`Gained ${event.amount} mana.`], "support", event);
    case "buildingBuilt":
      return recap(`${sideLabel(event.side)} built ${event.name}`, [coordDetail(event.coord)], "neutral", event);
    case "buildingActivated":
      return recap(`${sideLabel(event.side)} activated ${event.name}`, [`Occupant ${event.occupantId}.`], "support", event);
    case "itemEquipped":
      return recap(`${sideLabel(event.side)} equipped ${event.name}`, [`Equipped to ${pieceLabel(event.carrierId ?? event.unitId)}.`], "support", event);
    case "itemActivated":
      return recap(`${sideLabel(event.side)} activated ${event.name}`, [`Carrier ${pieceLabel(event.carrierId ?? event.unitId)}.`], "support", event);
    case "matchEnded":
      return recap(`${sideLabel(event.winner)} wins`, ["The match is over."], event.winner === viewerSide ? "support" : "attack", event);
    default:
      return null;
  }
}

export function actionRecapFromSnapshotDiff(
  previous: MatchState | null,
  next: MatchState,
  viewerSide: Side,
): ActionRecap | null {
  if (!previous) {
    return null;
  }

  const previousPieces = new Map(piecesInMatch(previous).map((piece) => [piece.id, piece]));
  const nextPieces = new Map(piecesInMatch(next).map((piece) => [piece.id, piece]));

  for (const [pieceId, nextPiece] of nextPieces) {
    const previousPiece = previousPieces.get(pieceId);
    if (!previousPiece) {
      return {
        id: `diff-summon-${pieceId}`,
        title: `${sideLabel(nextPiece.side)} added ${nextPiece.name}`,
        details: [coordDetail(nextPiece.position)],
        tone: "neutral",
      };
    }
    if (!sameCoord(previousPiece.position, nextPiece.position)) {
      return {
        id: `diff-move-${pieceId}-${coordKey(nextPiece.position)}`,
        title: `${sideLabel(nextPiece.side)} moved ${nextPiece.name}`,
        details: [`${coordDetail(previousPiece.position)} to ${coordDetail(nextPiece.position)}`],
        tone: "neutral",
      };
    }
    const statRecap = statChangeRecap(previousPiece, nextPiece);
    if (statRecap) {
      return statRecap;
    }
  }

  for (const [pieceId, previousPiece] of previousPieces) {
    if (!nextPieces.has(pieceId)) {
      return {
        id: `diff-destroy-${pieceId}`,
        title: `${previousPiece.name} left the board`,
        details: [`${sideLabel(previousPiece.side)} lost a piece.`],
        tone: "attack",
      };
    }
  }

  if (previous.activeSide !== next.activeSide || previous.prioritySide !== next.prioritySide) {
    return {
      id: `diff-turn-${next.round}-${next.activeSide}-${next.prioritySide ?? "none"}`,
      title: next.actionStack.length > 0 ? `${sideLabel(next.prioritySide)} priority` : `${sideLabel(next.activeSide)} turn`,
      details: [`Round ${next.round}.`],
      tone: "turn",
    };
  }

  const newLog = next.log.find((entry) => !previous.log.includes(entry));
  return newLog
    ? {
        id: `diff-log-${newLog}`,
        title: "Latest action",
        details: [newLog],
        tone: "neutral",
      }
    : null;
}

function statChangeRecap(previousPiece: BoardPiece, nextPiece: BoardPiece): ActionRecap | null {
  if (previousPiece.attack !== nextPiece.attack) {
    return {
      id: `diff-buff-${nextPiece.id}-${nextPiece.attack}`,
      title: `${nextPiece.name} changed attack`,
      details: [`Attack ${previousPiece.attack} to ${nextPiece.attack}.`],
      tone: "support",
    };
  }

  if (previousPiece.pieceType === "hero" && nextPiece.pieceType === "hero") {
    if (nextPiece.hp < previousPiece.hp) {
      return {
        id: `diff-damage-${nextPiece.id}-${nextPiece.hp}`,
        title: `${nextPiece.name} took damage`,
        details: [`HP ${previousPiece.hp} to ${nextPiece.hp}.`],
        tone: "attack",
      };
    }
    if (nextPiece.hp > previousPiece.hp) {
      return {
        id: `diff-heal-${nextPiece.id}-${nextPiece.hp}`,
        title: `${nextPiece.name} healed`,
        details: [`HP ${previousPiece.hp} to ${nextPiece.hp}.`],
        tone: "support",
      };
    }
  }

  if (previousPiece.pieceType === "unit" && nextPiece.pieceType === "unit") {
    if (nextPiece.armor < previousPiece.armor) {
      return {
        id: `diff-damage-${nextPiece.id}-${nextPiece.armor}`,
        title: `${nextPiece.name} took damage`,
        details: [`Armor ${previousPiece.armor} to ${nextPiece.armor}.`],
        tone: "attack",
      };
    }
    if (nextPiece.armor > previousPiece.armor) {
      return {
        id: `diff-heal-${nextPiece.id}-${nextPiece.armor}`,
        title: `${nextPiece.name} gained armor`,
        details: [`Armor ${previousPiece.armor} to ${nextPiece.armor}.`],
        tone: "support",
      };
    }
  }

  return null;
}

function legalMoveCount(match: MatchState, viewerSide: Side, piece: BoardPiece) {
  return match.board.tiles.filter((tile) => isLegalMove(match, viewerSide, piece, tile.coord)).length;
}

function legalAttackCount(match: MatchState, viewerSide: Side, piece: BoardPiece) {
  return piecesInMatch(match).filter((target) => isLegalAttack(match, viewerSide, piece, target)).length;
}

function attackReason(match: MatchState, viewerSide: Side, piece: BoardPiece, canAct: boolean) {
  if (!canAct) {
    return reason("waitingForTurn", "Waiting for your turn.");
  }
  if (match.actionStack.length > 0) {
    return reason("stackPending", "Resolve the stack before attacking.");
  }
  if (match.phase !== "attack") {
    return reason("wrongPhase", "Attacks happen during the Attack Phase.");
  }
  if (piece.apRemaining <= 0) {
    return reason("pieceApEmpty", `${piece.name} has no action points left.`);
  }
  if (piece.hasAttacked) {
    return reason("alreadyAttacked", `${piece.name} has already attacked this turn.`);
  }
  return undefined;
}

function buildingActivationReason(
  match: MatchState,
  piece: BoardPiece,
  building: Building,
  canAct: boolean,
) {
  if (!canAct) {
    return reason("waitingForTurn", "Waiting for your turn.");
  }
  if (building.activatedThisTurn) {
    return reason("buildingExhausted", `${building.name} has already been activated this turn.`);
  }
  if (piece.apRemaining <= 0) {
    return reason("pieceApEmpty", `${piece.name} has no action points left.`);
  }
  if (match.actionStack.length > 0) {
    return reason("stackPending", "Resolve the stack before activating this Building.");
  }
  return undefined;
}

function activatedReadyBuilding(match: MatchState, piece: BoardPiece) {
  const building = buildingAt(match, piece.position);
  return building &&
    buildingEffectIsActivated(building.effect) &&
    !building.activatedThisTurn
    ? building
    : null;
}

function cardTargetSummary(kind: CardKind) {
  switch (kind.type) {
    case "unit":
      return "Adjacent empty hex";
    case "manaSource":
      return "Adjacent empty hex without a Building";
    case "building":
      return "Adjacent empty hex without a Building";
    case "item":
      return `${itemTargetLabel(kind.targets ?? "unitsOnly")} within range ${kind.range}`;
    case "spell":
      return `Piece within range ${kind.range}`;
  }
}

function itemTargetLabel(targets: Extract<CardKind, { type: "item" }>["targets"]) {
  switch (targets) {
    case "heroesOnly":
      return "Friendly Hero";
    case "unitsAndHeroes":
      return "Friendly Unit or Hero";
    case "unitsOnly":
    default:
      return "Friendly Unit";
  }
}

function effectSummary(kind: CardKind) {
  switch (kind.type) {
    case "unit":
      return `Summons a ${kind.attack}/${kind.armor} Unit with ${kind.maxAp} AP.`;
    case "manaSource":
      return "Builds a Mana source for future turn starts.";
    case "building":
      return buildingEffectSummary(kind.effect);
    case "item":
      return `${itemPassiveLabel(kind.passive)}${kind.active ? ` ${itemActiveLabel(kind.active)}` : ""}`;
    case "spell":
      return spellEffectSummary(kind.effect);
  }
}

function spellEffectSummary(effect: Extract<CardKind, { type: "spell" }>["effect"]) {
  switch (effect.type) {
    case "heal":
      return `Heals ${effect.amount}.`;
    case "buff":
      return `Buffs a Unit for +${effect.attack} attack and +${effect.armor} armor.`;
    case "statBuff":
      return `Grants +${effect.attack} attack, +${effect.armor} armor, and +${effect.maxAp} max AP.`;
    case "damage":
      return `Deals ${effect.amount} damage.`;
    case "draw":
      return `Draws ${effect.amount} ${plural(effect.amount, "card")}.`;
    case "areaDamage":
      return `Deals ${effect.amount} area damage within radius ${effect.radius}.`;
    case "lineDamage":
      return `Deals ${effect.amount} line damage.`;
  }
}

function buildingEffectSummary(effect: Building["effect"]) {
  switch (effect.type) {
    case "turnStartMana":
      return `Grants ${effect.amount} Mana at turn start to the occupying side.`;
    case "auraStatBonus":
      return `Aura range ${effect.range}: +${effect.attack} attack, +${effect.armor} armor, +${effect.maxAp} max AP.`;
    case "activatedDamageLine":
      return `Activated line damage: range ${effect.range}, ${effect.amount} damage.`;
    case "activatedHeal":
      return `Activated heal: range ${effect.range}, heals ${effect.amount}.`;
    case "activatedStatBonus":
      return `Activated bonus: range ${effect.range}, +${effect.attack} attack, +${effect.armor} armor, +${effect.maxAp} max AP.`;
  }
}

function buildingEffectTone(effect: Building["effect"]): ActionPreview["tone"] {
  return effect.type === "activatedDamageLine" ? "attack" : "support";
}

function toneForCard(card: Card): ActionPreview["tone"] {
  if (card.kind.type === "spell") {
    switch (card.kind.effect.type) {
      case "damage":
      case "areaDamage":
      case "lineDamage":
        return "attack";
      case "heal":
      case "buff":
      case "statBuff":
      case "draw":
        return "support";
    }
  }
  if (card.kind.type === "item") {
    return "support";
  }
  return "neutral";
}

function prioritizedReasons(reasons: ActionAvailabilityReason[]) {
  const priority: ActionAvailabilityReason["code"][] = [
    "matchOver",
    "connectionBusy",
    "waitingForTurn",
    "waitingForPriority",
    "stackPending",
    "notAResponse",
    "priorityTooLow",
    "insufficientMana",
    "wrongPhase",
    "noLegalTargets",
    "pieceApEmpty",
    "alreadyAttacked",
    "itemExhausted",
    "buildingExhausted",
  ];
  const priorityIndex = new Map(priority.map((code, index) => [code, index]));
  return [...dedupeReasons(reasons)].sort(
    (left, right) => (priorityIndex.get(left.code) ?? 99) - (priorityIndex.get(right.code) ?? 99),
  );
}

function dedupeReasons(reasons: ActionAvailabilityReason[]) {
  const byCode = new Map<ActionAvailabilityReason["code"], ActionAvailabilityReason>();
  for (const current of reasons) {
    byCode.set(current.code, current);
  }
  return byCode.values();
}

function reason(code: ActionAvailabilityReason["code"], message: string): ActionAvailabilityReason {
  return { code, message };
}

function recap(
  title: string,
  details: string[],
  tone: ActionRecap["tone"],
  event: { type: string },
): ActionRecap {
  return {
    id: `${event.type}-${title}-${details.join("|")}`,
    title,
    details,
    tone,
  };
}

function pieceLabel(pieceId: string) {
  return pieceId.replace(/-/g, " ");
}

function coordDetail(coord: HexCoord) {
  return `q ${coord.q}, r ${coord.r}`;
}

function coordKey(coord: HexCoord) {
  return `${coord.q}:${coord.r}`;
}

function sameCoord(a: HexCoord, b: HexCoord) {
  return a.q === b.q && a.r === b.r;
}

function plural(count: number, singular: string) {
  return count === 1 ? singular : `${singular}s`;
}
