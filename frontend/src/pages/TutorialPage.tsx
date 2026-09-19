import { Activity, Archive, Layers, Play, Zap } from "lucide-react";
import { useEffect, useMemo, useReducer } from "react";
import type { DragEvent as ReactDragEvent } from "react";
import type { AccountPreferenceProps, BoardPiece } from "../appTypes";
import { Board, CardButton, PileDisplay, PlayerBadge, StackDisplay } from "../components/board";
import { AccountActions } from "../components/common";
import { sideLabel } from "../labels";
import {
  cardFanStyle,
  cardTargetForTile,
  handForSide,
  isLegalAttack,
  isLegalMove,
  isPlayableCard,
  opponentSideOf,
  participantBySide,
  pieceAt,
} from "../matchBoardHelpers";
import { createMatchVisualCatalog } from "../matchVisualIdentity";
import {
  TUTORIAL_COMPLETION_STORAGE_KEY,
  currentTutorialHighlights,
  currentTutorialStep,
  tutorialReducer,
  createInitialTutorialState,
} from "../tutorial/tutorialReducer";
import {
  TUTORIAL_EMBER_SQUIRE_CARD_ID,
  TUTORIAL_PLAYER_HERO_ID,
  TUTORIAL_SPARK_JOLT_CARD_ID,
  tutorialCatalogCards,
} from "../tutorial/tutorialFixtures";
import { TutorialIntroOverlay, TutorialObjectivePanel } from "../tutorial/TutorialOverlay";
import type { TutorialHighlight, TutorialTargetId } from "../tutorial/tutorialTypes";
import type { BoardTutorialHighlight } from "../tutorial/tutorialHighlights";
import type { Card, HexTile, MatchState, Side } from "../types";

const viewerSide: Side = "player";

export function TutorialPage({
  onNavigate,
  currentUser,
  onSignOut,
  allowSignOut,
  loginNextPath,
  visualPreferences,
}: {
  onNavigate: (to: string) => void;
} & AccountPreferenceProps) {
  const [state, dispatch] = useReducer(tutorialReducer, undefined, createInitialTutorialState);
  const step = currentTutorialStep(state);
  const highlights = currentTutorialHighlights(state);
  const visualCatalog = useMemo(
    () => createMatchVisualCatalog(tutorialCatalogCards),
    [],
  );
  const match = state.match;
  const selectedCardId = state.selection?.type === "card" ? state.selection.cardId : null;
  const selectedCard = selectedCardId
    ? handForSide(match, viewerSide).find((card) => card.id === selectedCardId) ?? null
    : null;
  const selectedPiece = state.selection?.type === "piece"
    ? pieceByTutorialId(match, state.selection.pieceId)
    : null;
  const boardHighlights = highlights.filter(isBoardHighlight);
  const completed = state.phase === "completed";
  const inputPaused = state.phase === "intro" || completed;
  const enemySide = opponentSideOf(viewerSide);
  const hasPendingStack = match.actionStack.length > 0;
  const phaseLabel =
    match.phase === "matchOver"
      ? `${sideLabel(match.winner)} wins`
      : hasPendingStack
        ? `${sideLabel(match.prioritySide)} priority`
        : match.activeSide === viewerSide
          ? phaseLabelFor(match.phase)
          : "Opponent turn";

  useEffect(() => {
    if (!completed) {
      return;
    }

    try {
      window.localStorage.setItem(TUTORIAL_COMPLETION_STORAGE_KEY, "true");
    } catch {
      // Tutorial completion should never block play if local storage is unavailable.
    }
  }, [completed]);

  function tutorialTargetClass(targetId: TutorialTargetId) {
    const highlight = highlights.find(
      (candidate): candidate is Extract<TutorialHighlight, { kind: "ui" }> =>
        candidate.kind === "ui" && candidate.targetId === targetId,
    );
    return highlight ? `tutorial-highlight tutorial-highlight-${highlight.tone}` : "";
  }

  function handleTileClick(tile: HexTile) {
    if (inputPaused) {
      return;
    }

    const piece = pieceAt(match, tile.coord);
    if (selectedCard) {
      const target = cardTargetForTile(match, viewerSide, selectedCard, tile);
      if (target?.type === "piece") {
        dispatch({ type: "interact", interaction: { type: "pieceClick", pieceId: target.pieceId } });
        return;
      }
      dispatch({ type: "interact", interaction: { type: "tileClick", coord: tile.coord } });
      return;
    }

    if (selectedPiece && piece && isLegalAttack(match, viewerSide, selectedPiece, piece)) {
      dispatch({ type: "interact", interaction: { type: "pieceClick", pieceId: piece.id } });
      return;
    }

    if (selectedPiece && !piece && isLegalMove(match, viewerSide, selectedPiece, tile.coord)) {
      dispatch({ type: "interact", interaction: { type: "tileClick", coord: tile.coord } });
      return;
    }

    if (piece) {
      dispatch({ type: "interact", interaction: { type: "pieceClick", pieceId: piece.id } });
      return;
    }

    dispatch({ type: "interact", interaction: { type: "tileClick", coord: tile.coord } });
  }

  function handleCardClick(card: Card) {
    if (inputPaused) {
      return;
    }

    dispatch({ type: "interact", interaction: { type: "cardClick", cardId: card.id } });
  }

  function handleCardDragStart(card: Card, event: ReactDragEvent<HTMLButtonElement>) {
    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", card.id);
    handleCardClick(card);
  }

  function handleCardDrop(tile: HexTile, cardId: string) {
    if (inputPaused) {
      return;
    }

    dispatch({ type: "interact", interaction: { type: "cardDrop", cardId, coord: tile.coord } });
  }

  function handleExit() {
    onNavigate("/play");
  }

  return (
    <main className="app-shell match-app-shell tutorial-shell">
      <section className="table match-table tutorial-table">
        <header className="top-bar match-chrome">
          <div>
            <p className="eyebrow">Arcania</p>
            <h1>Tutorial</h1>
            <p className="match-id">Scripted lesson</p>
          </div>
          <div className="actions">
            <AccountActions
              currentUser={currentUser}
              onNavigate={onNavigate}
              onSignOut={onSignOut}
              allowSignOut={allowSignOut}
              loginNextPath={loginNextPath}
            />
          </div>
        </header>

        <TutorialObjectivePanel
          step={step}
          hint={state.hint}
          completed={completed}
          onRestart={() => dispatch({ type: "restart" })}
          onExit={handleExit}
        />

        <div className="match-action-dock" aria-label="Tutorial actions">
          <button
            className={`primary-button ${tutorialTargetClass("tutorial-end-turn")}`}
            type="button"
            onClick={() => dispatch({ type: "interact", interaction: { type: "endTurn" } })}
            disabled={inputPaused || match.activeSide !== viewerSide || hasPendingStack || match.phase !== "cardPlay"}
            data-tutorial-target="tutorial-end-turn"
          >
            <Play size={18} />
            End Turn
          </button>
          {hasPendingStack ? (
            <button
              className={`primary-button ${tutorialTargetClass("tutorial-pass-priority")}`}
              type="button"
              onClick={() => dispatch({ type: "interact", interaction: { type: "passPriority" } })}
              disabled={inputPaused || match.prioritySide !== viewerSide}
              data-tutorial-target="tutorial-pass-priority"
            >
              <Zap size={18} />
              Pass Priority
            </button>
          ) : null}
        </div>

        <section className={`battlefield ${tutorialTargetClass("tutorial-board")}`}>
          <div className={`battlefield-hud battlefield-hud-player ${tutorialTargetClass("tutorial-player-badge")}`}>
            <PlayerBadge player={participantBySide(match, viewerSide)} />
          </div>
          <div className="battlefield-hud battlefield-hud-opponent">
            <PlayerBadge player={participantBySide(match, enemySide)} />
          </div>
          <div
            className={`battlefield-hud battlefield-hud-phase ${tutorialTargetClass("tutorial-phase")}`}
            data-tutorial-target="tutorial-phase"
          >
            <div className="phase-pill">
              <Activity size={16} />
              {phaseLabel}
            </div>
          </div>
          <Board
            match={match}
            animation={null}
            boardVisualMode={visualPreferences.preferences.boardVisualMode}
            viewerSide="player"
            visualCatalog={visualCatalog}
            selectedCard={selectedCard}
            selectedPiece={selectedPiece}
            focusedCoord={null}
            disabled={inputPaused}
            tutorialHighlights={boardHighlights}
            onTileClick={handleTileClick}
            onTileDrop={handleCardDrop}
          />
          <div className={`hand-overlay ${tutorialTargetClass("tutorial-hand")}`}>
            <div className="hand" aria-label="Hand" data-tutorial-target="tutorial-hand">
              {handForSide(match, viewerSide).map((card, index) => (
                <CardButton
                  key={card.id}
                  card={card}
                  visualIdentity={visualCatalog.card(card)}
                  selected={state.selection?.type === "card" && state.selection.cardId === card.id}
                  style={cardFanStyle(index, handForSide(match, viewerSide).length)}
                  disabled={inputPaused || !isPlayableCard(match, viewerSide, card)}
                  tutorialTargetId={tutorialTargetIdForCard(card.id)}
                  tutorialHighlighted={isCardHighlighted(highlights, card.id)}
                  onClick={() => handleCardClick(card)}
                  onDragStart={(event) => handleCardDragStart(card, event)}
                />
              ))}
            </div>
          </div>
        </section>

        <section className="hand-and-log">
          <div className="player-zone">
            <div className={tutorialTargetClass("tutorial-stack")} data-tutorial-target="tutorial-stack">
              <StackDisplay stack={match.actionStack} prioritySide={match.prioritySide} />
            </div>
            <section className="pile-row" aria-label="Player card piles">
              <PileDisplay icon={<Layers size={19} />} label="Deck" count={match.player.deckCount} status="Remaining" />
              <PileDisplay icon={<Archive size={19} />} label="Discard" count={match.player.discardCount} status={match.player.discardCount === 0 ? "Empty" : "In pile"} />
            </section>
          </div>
          <aside className="log" aria-label="Tutorial log">
            {match.log.map((entry, index) => (
              <p key={`${entry}-${index}`}>{entry}</p>
            ))}
          </aside>
        </section>
      </section>

      {state.phase === "intro" ? (
        <TutorialIntroOverlay
          step={step}
          canGoBack={state.stepIndex > 0}
          onContinue={() => dispatch({ type: "continue" })}
          onBack={() => dispatch({ type: "back" })}
          onRestart={() => dispatch({ type: "restart" })}
          onExit={handleExit}
        />
      ) : null}
    </main>
  );
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

function isBoardHighlight(highlight: TutorialHighlight): highlight is BoardTutorialHighlight {
  return highlight.kind === "coord" || highlight.kind === "piece";
}

function tutorialTargetIdForCard(cardId: string): TutorialTargetId | undefined {
  if (cardId === TUTORIAL_EMBER_SQUIRE_CARD_ID) {
    return "tutorial-card-ember-squire";
  }
  if (cardId === TUTORIAL_SPARK_JOLT_CARD_ID) {
    return "tutorial-card-spark-jolt";
  }
  return undefined;
}

function isCardHighlighted(highlights: TutorialHighlight[], cardId: string) {
  const targetId = tutorialTargetIdForCard(cardId);
  return Boolean(
    targetId &&
      highlights.some((highlight) => highlight.kind === "ui" && highlight.targetId === targetId),
  );
}

function pieceByTutorialId(match: MatchState, pieceId: string): BoardPiece | null {
  if (pieceId === TUTORIAL_PLAYER_HERO_ID) {
    return { ...match.player.hero, pieceType: "hero", name: "Runekeeper" };
  }

  if (pieceId === "tutorial-opponent-hero") {
    return { ...match.opponent.hero, pieceType: "hero", name: "Pyromancer" };
  }

  const unit = match.board.units.find((candidate) => candidate.id === pieceId);
  return unit ? { ...unit, pieceType: "unit" } : null;
}
