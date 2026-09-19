import { Activity, Archive, Eye, EyeOff, Layers, Play, Plus, RotateCcw, Sword, Zap } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { DragEvent as ReactDragEvent } from "react";
import {
  activateBuilding,
  activateItem,
  advanceAi,
  attack,
  createMatch,
  endTurn,
  loadCatalog,
  loadMatch,
  movePiece,
  passPriority,
  playCard,
  startAttackPhase,
  startCardPlay,
} from "../api";
import type { AccountPreferenceProps, LoadState } from "../appTypes";
import { createBoardAnimationCue, type BoardAnimationCue } from "../boardAnimations";
import { useHotkeyHandlers, useMatchChromeMinimized } from "../appHooks";
import {
  Board,
  CardButton,
  OpponentHandDisplay,
  PileDisplay,
  PlayerBadge,
  StackDisplay,
  UnitCardModal,
  UnitContextMenuView,
} from "../components/board";
import { AccountActions, ShellMessage } from "../components/common";
import {
  ActionPreviewPanel,
  ActionRecapCallout,
  ActionTray,
  TurnChecklist,
} from "../components/match";
import { sideLabel } from "../labels";
import { liveAiPlaybackFrames, suppressLiveAiFallbackAnimation } from "../livePlayback";
import {
  cardFanStyle,
  buildingAt,
  delay,
  handCountForSide,
  handForSide,
  hasPlayablePriorityResponse,
  latestReplayEvent,
  opponentSideOf,
  participantBySide,
} from "../matchBoardHelpers";
import { createMatchVisualCatalog } from "../matchVisualIdentity";
import {
  actionPreviewForCard,
  actionPreviewForPiece,
  actionRecapFromReplayEvent,
  actionTrayEntriesForSelection,
  cardAvailability,
  turnChecklistForMatch,
  type ActionRecap,
} from "../matchUxModel";
import type { HotkeyHandlers } from "../hotkeyRuntime";
import type {
  Card,
  CatalogCard,
  MatchActionRequest,
  MatchResponse,
  MatchState,
  ReplayEvent,
  Side,
} from "../types";
import { MatchEndOverlay } from "./match/MatchEndOverlay";
import { useMatchBoardController } from "./match/useMatchBoardController";

export function MatchPage({
  matchId,
  onNavigate,
  currentUser,
  onSignOut,
  allowSignOut,
  loginNextPath,
  visualPreferences,
}: {
  matchId: string;
  onNavigate: (to: string) => void;
} & AccountPreferenceProps) {
  const viewerSide: Side = "player";
  const boardVisualMode = visualPreferences.preferences.boardVisualMode;
  const [loadState, setLoadState] = useState<LoadState>({ status: "loading" });
  const [catalogCards, setCatalogCards] = useState<CatalogCard[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [boardAnimation, setBoardAnimation] = useState<BoardAnimationCue | null>(null);
  const [actionRecap, setActionRecap] = useState<ActionRecap | null>(null);
  const [matchChromeMinimized, setMatchChromeMinimized] = useMatchChromeMinimized();
  const animationSequenceRef = useRef(0);
  const reducedMotion = visualPreferences.effectiveMotion === "reduced";

  useEffect(() => {
    setLoadState({ status: "loading" });
    setNotice(null);
    setBoardAnimation(null);
    setActionRecap(null);
    loadMatch(matchId)
      .then((response) =>
        setLoadState({
          status: "ready",
          match: response.matchState,
          heroAppearances: response.heroAppearances,
        }),
      )
      .catch((error: unknown) =>
        setLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load match",
        }),
      );
  }, [matchId]);

  useEffect(() => {
    loadCatalog()
      .then((response) => setCatalogCards(response.cards))
      .catch(() => setCatalogCards([]));
  }, []);

  const visualCatalog = useMemo(
    () => createMatchVisualCatalog(catalogCards),
    [catalogCards],
  );
  const readyMatch = loadState.status === "ready" ? loadState.match : null;
  const heroAppearances = loadState.status === "ready" ? loadState.heroAppearances ?? [] : [];
  const playerHasPriorityResponse = useMemo(
    () => (readyMatch ? hasPlayablePriorityResponse(readyMatch, viewerSide) : false),
    [readyMatch, viewerSide],
  );

  const boardController = useMatchBoardController({
    match: readyMatch,
    viewerSide,
    submitAction: submitSoloAction,
    readOnly: busy,
    onNotice: setNotice,
  });

  const {
    boardCursor,
    contextMenuUnit,
    cursorHotkeyHandlers,
    draggedCardId,
    focusedUnit,
    handleActivateUnitBuilding,
    handleActivateUnitItem,
    handleCardDragEnd,
    handleCardDragStart,
    handleCardDrop,
    handleTileClick,
    modalUnit,
    openUnitContextMenu,
    playedCardId,
    selectCard,
    selectedCard,
    selectedPiece,
    selection,
    setFocusedUnitPieceId,
    setPlayedCardId,
    setUnitContextMenu,
    setUnitModalPieceId,
    unitContextMenu,
  } = boardController;

  const modalUnitVisualIdentity = useMemo(
    () => (modalUnit ? visualCatalog.unit(modalUnit) : null),
    [modalUnit, visualCatalog],
  );

  async function runAction(action: () => Promise<MatchResponse>) {
    setBusy(true);
    setNotice(null);

    try {
      boardController.clearTransientState();
      const response = await action();
      await playLiveActionResponse(response);
    } catch (error) {
      setPlayedCardId(null);
      setNotice(error instanceof Error ? error.message : "Action failed");
    } finally {
      setBusy(false);
    }
  }

  function submitSoloAction(action: MatchActionRequest) {
    void runAction(() => soloActionRequest(matchId, action));
  }

  async function playLiveActionResponse(response: MatchResponse) {
    const playbackFrames = liveAiPlaybackFrames(response.replayFrames ?? []);
    let previousMatch = loadState.status === "ready" ? loadState.match : null;

    function acceptMatch(
      nextMatch: MatchState,
      event?: ReplayEvent | null,
      options: { suppressAnimation?: boolean } = {},
    ) {
      setLoadState({
        status: "ready",
        match: nextMatch,
        heroAppearances: response.heroAppearances,
      });
      setBoardAnimation(options.suppressAnimation
        ? null
        : createBoardAnimationCue({
            previous: previousMatch,
            next: nextMatch,
            event,
            sequence: ++animationSequenceRef.current,
            reducedMotion,
          }));
      previousMatch = nextMatch;
    }

    if (playbackFrames.length === 0) {
      const event = latestReplayEvent(response.replayFrames);
      const suppressAnimation = suppressLiveAiFallbackAnimation(event);
      acceptMatch(response.matchState, event, { suppressAnimation });
      if (!suppressAnimation) {
        updateActionRecap(event);
      }
      return;
    }

    for (const frame of playbackFrames) {
      await delay(visualPreferences.liveAiDelayMs);
      acceptMatch(frame.matchState, frame.event);
      updateActionRecap(frame.event);
    }

    setLoadState({
      status: "ready",
      match: response.matchState,
      heroAppearances: response.heroAppearances,
    });
  }

  function updateActionRecap(event: ReplayEvent | null | undefined) {
    if (!event || ("side" in event && event.side === viewerSide)) {
      return;
    }

    const recap = actionRecapFromReplayEvent(event, viewerSide);
    if (recap) {
      setActionRecap(recap);
    }
  }

  useEffect(() => {
    if (!readyMatch || busy || readyMatch.phase === "matchOver") {
      return;
    }

    let action: (() => Promise<MatchResponse>) | null = null;
    if (readyMatch.actionStack.length > 0) {
      if (readyMatch.prioritySide === "player" && !playerHasPriorityResponse) {
        action = () => passPriority(matchId);
      } else if (readyMatch.prioritySide === "opponent") {
        action = () => advanceAi(matchId);
      }
    } else if (readyMatch.activeSide === "opponent") {
      action = () => advanceAi(matchId);
    }

    if (!action) {
      return;
    }

    const scheduledAction = action;
    const timeoutId = window.setTimeout(() => {
      void runAction(scheduledAction);
    }, visualPreferences.liveAiDelayMs);

    return () => window.clearTimeout(timeoutId);
  }, [readyMatch, busy, matchId, playerHasPriorityResponse, visualPreferences.liveAiDelayMs]);

  const matchHotkeyHandlers = useMemo<HotkeyHandlers>(
    () => ({
      ...cursorHotkeyHandlers,
      endTurn: () => {
        if (
          !readyMatch ||
          busy ||
          readyMatch.phase === "matchOver" ||
          readyMatch.phase !== "cardPlay" ||
          readyMatch.activeSide !== viewerSide ||
          readyMatch.actionStack.length > 0
        ) {
          return false;
        }

        void runAction(() => endTurn(matchId));
        return true;
      },
      passPriority: () => {
        if (
          !readyMatch ||
          busy ||
          readyMatch.phase === "matchOver" ||
          readyMatch.actionStack.length === 0 ||
          readyMatch.prioritySide !== viewerSide
        ) {
          return false;
        }

        void runAction(() => passPriority(matchId));
        return true;
      },
    }),
    [busy, cursorHotkeyHandlers, matchId, readyMatch],
  );
  useHotkeyHandlers(visualPreferences.preferences.hotkeys, matchHotkeyHandlers);

  async function handleCreateSeparateMatch() {
    setBusy(true);
    setNotice(null);
    try {
      const created = await createMatch();
      onNavigate(`/match/${created.matchId}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create match");
    } finally {
      setBusy(false);
    }
  }

  if (loadState.status === "loading") {
    return <ShellMessage title={`Match ${matchId}`} message="Loading board" />;
  }

  if (loadState.status === "error") {
    return (
      <ShellMessage
        title={`Match ${matchId}`}
        message={loadState.message}
        actions={
          <>
            <button className="primary-button" type="button" onClick={() => onNavigate("/")}>
              Open another
            </button>
            <button
              className="icon-button"
              type="button"
              onClick={() => void handleCreateSeparateMatch()}
              title="New match"
            >
              <Plus size={18} />
            </button>
          </>
        }
      />
    );
  }

  const { match } = loadState;
  const hasPendingStack = match.actionStack.length > 0;
  const isPlayerPriority = hasPendingStack && match.prioritySide === viewerSide;
  const canAct =
    match.phase !== "matchOver" && (hasPendingStack ? isPlayerPriority : match.activeSide === viewerSide);
  const canPassSoloPriority = isPlayerPriority && playerHasPriorityResponse;
  const phaseLabel =
    match.phase === "matchOver"
      ? `${sideLabel(match.winner)} wins`
      : hasPendingStack
        ? `${sideLabel(match.prioritySide)} priority`
        : match.activeSide === viewerSide
          ? phaseLabelFor(match.phase)
          : "AI thinking";
  const enemySide = opponentSideOf(viewerSide);
  const viewerHand = handForSide(match, viewerSide);
  const selectedCardAvailability = selectedCard
    ? cardAvailability(match, viewerSide, selectedCard, { canAct, busy })
    : null;
  const selectedPreview = selectedCard
    ? actionPreviewForCard(match, viewerSide, selectedCard)
    : selectedPiece
      ? actionPreviewForPiece(match, viewerSide, selectedPiece)
      : focusedUnit
        ? actionPreviewForPiece(match, viewerSide, focusedUnit)
        : null;
  const actionTrayEntries = actionTrayEntriesForSelection({
    match,
    viewerSide,
    selection: selectedCard
      ? { type: "card", card: selectedCard }
      : selectedPiece
        ? { type: "piece", piece: selectedPiece }
        : null,
    focusedPiece: focusedUnit,
    canAct: canAct && !busy,
  });
  const turnChecklistItems = turnChecklistForMatch(match, viewerSide, canAct && !busy);

  function startCardDrag(card: Card, event: ReactDragEvent<HTMLButtonElement>) {
    if (!cardAvailability(match, viewerSide, card, { canAct, busy }).playable) {
      event.preventDefault();
      return;
    }

    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", card.id);
    handleCardDragStart(card);
  }

  return (
    <main className={`app-shell match-app-shell ${matchChromeMinimized ? "match-chrome-minimized" : ""}`}>
      <section className="table match-table">
        <header className="top-bar match-chrome">
          <div>
            <p className="eyebrow">Arcania</p>
            <h1>Round {match.round}</h1>
            <p className="match-id">Match {matchId}</p>
          </div>
          <div className="actions">
            <AccountActions
              currentUser={currentUser}
              onNavigate={onNavigate}
              onSignOut={onSignOut}
              allowSignOut={allowSignOut}
              loginNextPath={loginNextPath}
            />
            <button
              className="icon-button"
              type="button"
              onClick={() => void handleCreateSeparateMatch()}
              disabled={busy}
              title="New match"
            >
              <RotateCcw size={18} />
            </button>
            <button
              className="icon-button"
              type="button"
              onClick={() => setMatchChromeMinimized(true)}
              title="Minimize match chrome"
              aria-label="Minimize match chrome"
            >
              <EyeOff size={18} />
            </button>
          </div>
        </header>
        <button
          className="icon-button match-chrome-restore"
          type="button"
          onClick={() => setMatchChromeMinimized(false)}
          title="Restore match chrome"
          aria-label="Restore match chrome"
        >
          <Eye size={18} />
        </button>
        <div className="match-action-dock" aria-label="Match actions">
          {!hasPendingStack && match.phase === "movement" ? (
            <>
              <button
                className="primary-button"
                type="button"
                onClick={() => void runAction(() => startAttackPhase(matchId))}
                disabled={busy || match.activeSide !== viewerSide}
              >
                <Sword size={18} />
                Start Attack
              </button>
              <button
                className="primary-button"
                type="button"
                onClick={() => void runAction(() => startCardPlay(matchId))}
                disabled={busy || match.activeSide !== viewerSide}
              >
                <Layers size={18} />
                Play Cards
              </button>
            </>
          ) : null}
          {!hasPendingStack && match.phase === "attack" ? (
            <button
              className="primary-button"
              type="button"
              onClick={() => void runAction(() => startCardPlay(matchId))}
              disabled={busy || match.activeSide !== viewerSide}
            >
              <Play size={18} />
              Finish Attacks
            </button>
          ) : null}
          {!hasPendingStack && match.phase === "cardPlay" ? (
            <button
              className="primary-button"
              type="button"
              onClick={() => void runAction(() => endTurn(matchId))}
              disabled={busy || match.activeSide !== viewerSide}
            >
              <Play size={18} />
              End Turn
            </button>
          ) : null}
          {hasPendingStack ? (
            <button
              className="primary-button"
              type="button"
              onClick={() => void runAction(() => passPriority(matchId))}
              disabled={busy || match.phase === "matchOver" || !isPlayerPriority}
            >
              <Zap size={18} />
              Pass Priority
            </button>
          ) : null}
        </div>
        <TurnChecklist items={turnChecklistItems} />
        <div className="match-ux-dock">
          <ActionRecapCallout recap={actionRecap} />
          <ActionPreviewPanel
            preview={selectedPreview}
            reason={selectedCardAvailability?.primaryReason ?? null}
          />
          <ActionTray
            entries={actionTrayEntries}
            onBlockedEntry={(entry) => setNotice(entry.reason?.message ?? null)}
          />
        </div>

        <section className="battlefield">
          <div className="battlefield-hud battlefield-hud-player">
            <PlayerBadge player={participantBySide(match, viewerSide)} />
          </div>
          <div className="battlefield-hud battlefield-hud-opponent">
            <PlayerBadge player={participantBySide(match, enemySide)} />
          </div>
          <div className="battlefield-hud battlefield-hud-hand">
            <OpponentHandDisplay count={handCountForSide(match, enemySide)} />
          </div>
          <div className="battlefield-hud battlefield-hud-phase">
            <div className="phase-pill">
              <Activity size={16} />
              {phaseLabel}
            </div>
          </div>
          <Board
            match={match}
            animation={boardAnimation}
            boardVisualMode={boardVisualMode}
            viewerSide="player"
            visualCatalog={visualCatalog}
            selectedCard={selectedCard}
            selectedPiece={selectedPiece}
            focusedCoord={boardCursor.visible ? boardCursor.coord : null}
            heroAppearances={heroAppearances}
            disabled={busy || match.phase === "matchOver"}
            onTileClick={handleTileClick}
            onTileDrop={handleCardDrop}
            onUnitContextMenu={(unit, position) => openUnitContextMenu(unit, position.x, position.y)}
            onFocusedUnitChange={setFocusedUnitPieceId}
          />
          <div className="hand-overlay">
            <div className="hand" aria-label="Hand">
              {viewerHand.map((card, index) => {
                const availability = cardAvailability(match, viewerSide, card, { canAct, busy });

                return (
                  <CardButton
                    key={card.id}
                    card={card}
                    visualIdentity={visualCatalog.card(card)}
                    selected={selection?.type === "card" && card.id === selection.cardId}
                    dragging={draggedCardId === card.id}
                    played={playedCardId === card.id}
                    style={cardFanStyle(index, viewerHand.length)}
                    disabled={busy}
                    unavailable={!availability.playable}
                    availabilityReason={availability.primaryReason?.message ?? null}
                    onClick={() => selectCard(card)}
                    onDragStart={(event) => startCardDrag(card, event)}
                    onDragEnd={handleCardDragEnd}
                  />
                );
              })}
            </div>
          </div>
        </section>

        <section className="hand-and-log">
          <div className="player-zone">
            <StackDisplay stack={match.actionStack} prioritySide={match.prioritySide} />
            <section className="pile-row" aria-label="Player card piles">
              <PileDisplay
                icon={<Layers size={19} />}
                label="Deck"
                count={match.player.deckCount}
                status="Remaining"
              />
              <PileDisplay
                icon={<Archive size={19} />}
                label="Discard"
                count={match.player.discardCount}
                status={match.player.discardCount === 0 ? "Empty" : "In pile"}
              />
            </section>
          </div>
          <aside className="log" aria-label="Match log">
            {notice ? <p className="notice">{notice}</p> : null}
            {canPassSoloPriority ? <p>Play a response or pass priority.</p> : null}
            {match.log.map((entry, index) => (
              <p key={`${entry}-${index}`}>{entry}</p>
            ))}
          </aside>
        </section>
      </section>
      {modalUnit ? (
        <UnitCardModal
          unit={modalUnit}
          unitVisualIdentity={modalUnitVisualIdentity ?? visualCatalog.unit(modalUnit)}
          onClose={() => setUnitModalPieceId(null)}
        />
      ) : null}
      {unitContextMenu && contextMenuUnit ? (
        <UnitContextMenuView
          menu={unitContextMenu}
          unit={contextMenuUnit}
          onClose={() => setUnitContextMenu(null)}
          onOpenCardInfo={() => {
            setUnitModalPieceId(contextMenuUnit.id);
            setUnitContextMenu(null);
          }}
          canActivateItems={
            ((match.actionStack.length === 0 && match.activeSide === viewerSide) ||
              (match.actionStack.length > 0 && match.prioritySide === viewerSide)) &&
            contextMenuUnit.side === viewerSide
          }
          onActivateItem={(itemId) => handleActivateUnitItem(contextMenuUnit, itemId)}
          building={buildingAt(match, contextMenuUnit.position)}
          canActivateBuilding={
            match.actionStack.length === 0 &&
            match.activeSide === viewerSide &&
            contextMenuUnit.side === viewerSide
          }
          onActivateBuilding={handleActivateUnitBuilding}
        />
      ) : null}
      {match.phase === "matchOver" ? (
        <MatchEndOverlay
          winner={match.winner}
          viewerSide={viewerSide}
          onOpenSummary={() => onNavigate(`/matches/${matchId}/summary`)}
        />
      ) : null}
    </main>
  );
}

function soloActionRequest(matchId: string, action: MatchActionRequest) {
  switch (action.type) {
    case "playCard":
      return playCard(matchId, action.cardId, action.target);
    case "movePiece":
      return movePiece(matchId, action.pieceId, action.to);
    case "attack":
      return attack(matchId, action.attackerId, action.targetId);
    case "activateItem":
      return activateItem(matchId, action.carrierId ?? action.unitId, action.itemId, action.target ?? null);
    case "activateBuilding":
      return activateBuilding(matchId, action.buildingId);
    case "startAttackPhase":
      return startAttackPhase(matchId);
    case "startCardPlay":
      return startCardPlay(matchId);
    case "endTurn":
      return endTurn(matchId);
    case "passPriority":
      return passPriority(matchId);
    case "advanceAi":
      return advanceAi(matchId);
  }
}

function phaseLabelFor(phase: MatchState["phase"]) {
  switch (phase) {
    case "movement":
      return "Movement Phase";
    case "attack":
      return "Attack Phase";
    case "cardPlay":
      return "Card Play";
    case "matchOver":
      return "Match over";
  }
}
