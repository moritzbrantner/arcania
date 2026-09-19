import { Archive, Copy, Eye, EyeOff, House, Layers, Play, Sword, Users, Wifi, Zap } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { DragEvent as ReactDragEvent } from "react";
import {
  joinSharedMatch,
  loadCatalog,
  loadDecks,
  loadProgression,
  loadSharedMatch,
  loadSystemDecks,
  sharedMatchWebSocketUrl,
} from "../api";
import type {
  AccountPreferenceProps,
  DeckLoadState,
  ProgressionLoadState,
  SharedLoadState,
  SystemDeckLoadState,
} from "../appTypes";
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
import { LobbySeatStatus, RuneSelector, HeroPicker } from "../components/loadoutControls";
import {
  defaultRuneIdsForHero,
  isHeroType,
  parseDeckChoice,
  parseRuneIds,
  sideLabel,
} from "../labels";
import {
  buildingAt,
  cardFanStyle,
  handCountForSide,
  handForSide,
  opponentSideOf,
  participantBySide,
  participantsInMatch,
  sameTeam,
} from "../matchBoardHelpers";
import { createMatchVisualCatalog } from "../matchVisualIdentity";
import {
  actionPreviewForCard,
  actionPreviewForPiece,
  actionRecapFromSnapshotDiff,
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
  MatchState,
  SharedClientMessage,
  SharedServerMessage,
  SharedSeatUrl,
  Side,
  HeroType,
} from "../types";
import {
  legalSharedDecks,
  selectedSharedDeckValue,
  sharedDeckChoiceFromSelectedValue,
  sharedDeckChoiceStorageKey,
  sharedHeroStorageKey,
  sharedInviteStorageKey,
  sharedLobbyActionLabelForHero,
  sharedRuneStorageKey,
  sharedSeatHeroName,
  sharedSystemDeckOptions,
} from "./match/sharedLobbyModel";
import { MatchEndOverlay } from "./match/MatchEndOverlay";
import { useMatchBoardController } from "./match/useMatchBoardController";

export function SharedMatchPage({
  matchId,
  seatToken,
  onNavigate,
  currentUser,
  onSignOut,
  allowSignOut,
  loginNextPath,
  visualPreferences,
}: {
  matchId: string;
  seatToken: string;
  onNavigate: (to: string) => void;
} & AccountPreferenceProps) {
  const boardVisualMode = visualPreferences.preferences.boardVisualMode;
  const [loadState, setLoadState] = useState<SharedLoadState>({ status: "loading" });
  const [catalogCards, setCatalogCards] = useState<CatalogCard[]>([]);
  const [selectedHeroType, setSelectedHeroType] = useState<HeroType>(() => {
    const stored = sessionStorage.getItem(sharedHeroStorageKey(matchId));
    return isHeroType(stored) ? stored : (currentUser?.preferredHeroType ?? "runekeeper");
  });
  const [deckLoadState, setDeckLoadState] = useState<DeckLoadState | null>(
    currentUser ? { status: "loading" } : null,
  );
  const [systemDeckLoadState, setSystemDeckLoadState] = useState<SystemDeckLoadState>({
    status: "loading",
  });
  const [progressionLoadState, setProgressionLoadState] = useState<ProgressionLoadState | null>(
    currentUser ? { status: "loading" } : null,
  );
  const [selectedDeckId, setSelectedDeckId] = useState<string>(() => {
    const stored = sessionStorage.getItem(sharedDeckChoiceStorageKey(matchId));
    return selectedSharedDeckValue(parseDeckChoice(stored));
  });
  const [selectedRuneIds, setSelectedRuneIds] = useState<string[]>(() => {
    const stored = sessionStorage.getItem(sharedRuneStorageKey(matchId));
    return stored ? parseRuneIds(stored) : [];
  });
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));
  const [boardAnimation, setBoardAnimation] = useState<BoardAnimationCue | null>(null);
  const [actionRecap, setActionRecap] = useState<ActionRecap | null>(null);
  const [matchChromeMinimized, setMatchChromeMinimized] = useMatchChromeMinimized();
  const socketRef = useRef<WebSocket | null>(null);
  const latestSharedMatchRef = useRef<MatchState | null>(null);
  const pendingLocalRequestIdsRef = useRef(new Set<string>());
  const animationSequenceRef = useRef(0);
  const reducedMotion = visualPreferences.effectiveMotion === "reduced";

  useEffect(() => {
    setLoadState({ status: "loading" });
    setNotice(null);
    setBoardAnimation(null);
    setActionRecap(null);
    latestSharedMatchRef.current = null;
    pendingLocalRequestIdsRef.current.clear();
    loadSharedMatch(matchId, seatToken)
      .then((shared) => {
        latestSharedMatchRef.current = shared.matchState;
        setLoadState({ status: "ready", shared });
      })
      .catch((error: unknown) =>
        setLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load shared match",
        }),
      );
  }, [matchId, seatToken]);

  useEffect(() => {
    loadCatalog()
      .then((response) => setCatalogCards(response.cards))
      .catch(() => setCatalogCards([]));
  }, []);

  useEffect(() => {
    loadSystemDecks()
      .then((response) => setSystemDeckLoadState({ status: "ready", response }))
      .catch((error: unknown) =>
        setSystemDeckLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load system decks",
        }),
      );
  }, []);

  const visualCatalog = useMemo(
    () => createMatchVisualCatalog(catalogCards),
    [catalogCards],
  );

  useEffect(() => {
    if (!currentUser) {
      setDeckLoadState(null);
      setProgressionLoadState(null);
      setSelectedDeckId("starter");
      setSelectedRuneIds([]);
      return;
    }
    setDeckLoadState({ status: "loading" });
    loadDecks()
      .then((response) => {
        setDeckLoadState({ status: "ready", response });
        const defaultDeck = response.decks.find((deck) => deck.isDefault && deck.legality.legal);
        if (defaultDeck) {
          setSelectedDeckId(String(defaultDeck.id));
        }
      })
      .catch((error: unknown) =>
        setDeckLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load decks",
        }),
      );
    setProgressionLoadState({ status: "loading" });
    loadProgression()
      .then((progression) => {
        setProgressionLoadState({ status: "ready", progression });
        setSelectedRuneIds((current) =>
          current.length > 0 ? current : defaultRuneIdsForHero(progression, selectedHeroType),
        );
      })
      .catch((error: unknown) =>
        setProgressionLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load progression",
        }),
      );
  }, [currentUser]);

  useEffect(() => {
    if (progressionLoadState?.status === "ready") {
      const stored = sessionStorage.getItem(sharedRuneStorageKey(matchId));
      setSelectedRuneIds(
        stored ? parseRuneIds(stored) : defaultRuneIdsForHero(progressionLoadState.progression, selectedHeroType),
      );
    }
  }, [selectedHeroType, progressionLoadState?.status, matchId]);

  useEffect(() => {
    const interval = window.setInterval(() => setNow(Math.floor(Date.now() / 1000)), 1000);
    return () => window.clearInterval(interval);
  }, []);

  useEffect(() => {
    const socket = new WebSocket(sharedMatchWebSocketUrl(matchId, seatToken));
    socketRef.current = socket;

    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data as string) as SharedServerMessage;
      if (
        message.type === "snapshot" ||
        message.type === "presenceChanged" ||
        message.type === "actionAccepted"
      ) {
        const previousMatch = latestSharedMatchRef.current;
        const isLocalAcceptedAction =
          message.type === "actionAccepted" && pendingLocalRequestIdsRef.current.delete(message.requestId);
        if (message.type === "actionAccepted" && message.payload.matchState) {
          setBoardAnimation(
            createBoardAnimationCue({
              previous: previousMatch,
              next: message.payload.matchState,
              sequence: ++animationSequenceRef.current,
              reducedMotion,
            }),
          );
          if (!isLocalAcceptedAction) {
            setActionRecap(actionRecapFromSnapshotDiff(previousMatch, message.payload.matchState, message.payload.viewerSide));
          }
        }
        latestSharedMatchRef.current = message.payload.matchState;
        setLoadState({ status: "ready", shared: message.payload });
        setBusy(false);
        if (message.type === "actionAccepted") {
          boardController.clearTransientState();
        }
      } else if (message.type === "actionRejected") {
        pendingLocalRequestIdsRef.current.delete(message.requestId);
        boardController.setPlayedCardId(null);
        setNotice(message.message);
        setBusy(false);
      } else if (message.type === "error") {
        boardController.setPlayedCardId(null);
        setNotice(message.message);
        setBusy(false);
      }
    });

    socket.addEventListener("close", () => {
      if (socketRef.current === socket) {
        setNotice("Live connection closed. Reload the page to reconnect.");
      }
    });

    const heartbeat = window.setInterval(() => {
      if (socket.readyState === WebSocket.OPEN) {
        const message: SharedClientMessage = { type: "heartbeat" };
        socket.send(JSON.stringify(message));
      }
    }, 15_000);

    return () => {
      window.clearInterval(heartbeat);
      socketRef.current = null;
      socket.close();
    };
  }, [matchId, seatToken, reducedMotion]);

  const shared = loadState.status === "ready" ? loadState.shared : null;
  const match = shared?.matchState ?? null;
  const viewerSide = shared?.viewerSide ?? "player";
  const isActiveViewer = Boolean(match && shared?.activeSide === viewerSide);
  const hasPendingStack = Boolean(match && match.actionStack.length > 0);
  const isPriorityViewer = Boolean(match && match.prioritySide === viewerSide);
  const canAct = Boolean(match && (hasPendingStack ? isPriorityViewer : isActiveViewer));

  useEffect(() => {
    if (shared?.status === "setup" && shared.viewerHeroType) {
      setSelectedHeroType(shared.viewerHeroType);
    }
  }, [shared?.status, shared?.viewerHeroType]);

  const boardController = useMatchBoardController({
    match,
    viewerSide,
    submitAction: sendSharedAction,
    readOnly: busy || !canAct,
    cursorDisabled: busy,
    contextMenuDisabled: busy,
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

  function sendSharedAction(action: MatchActionRequest) {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setNotice("Live connection is not ready.");
      return;
    }

    setBusy(true);
    setNotice(null);
    const requestId = crypto.randomUUID();
    pendingLocalRequestIdsRef.current.add(requestId);
    const message: SharedClientMessage = {
      type: "action",
      requestId,
      action,
    };
    socket.send(JSON.stringify(message));
  }

  const sharedHotkeyHandlers = useMemo<HotkeyHandlers>(
    () => ({
      ...cursorHotkeyHandlers,
      endTurn: () => {
        if (!match || busy || match.phase !== "cardPlay" || !isActiveViewer || hasPendingStack) {
          return false;
        }

        sendSharedAction({ type: "endTurn" });
        return true;
      },
      passPriority: () => {
        if (!match || busy || match.phase === "matchOver" || !hasPendingStack || !isPriorityViewer) {
          return false;
        }

        sendSharedAction({ type: "passPriority" });
        return true;
      },
    }),
    [
      busy,
      cursorHotkeyHandlers,
      hasPendingStack,
      isActiveViewer,
      isPriorityViewer,
      match,
    ],
  );
  useHotkeyHandlers(visualPreferences.preferences.hotkeys, sharedHotkeyHandlers);

  function claimForfeit() {
    const socket = socketRef.current;
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      setNotice("Live connection is not ready.");
      return;
    }

    setBusy(true);
    setNotice(null);
    const requestId = crypto.randomUUID();
    pendingLocalRequestIdsRef.current.add(requestId);
    const message: SharedClientMessage = {
      type: "claimForfeit",
      requestId,
    };
    socket.send(JSON.stringify(message));
  }

  async function handleJoinSharedMatch() {
    setBusy(true);
    setNotice(null);
    try {
      const joined = await joinSharedMatch(
        matchId,
        seatToken,
        selectedHeroType,
        sharedDeckChoiceFromSelectedValue(selectedDeckId).deckRecipeId,
        selectedRuneIds,
        sharedDeckChoiceFromSelectedValue(selectedDeckId).deckChoice,
      );
      sessionStorage.setItem(sharedHeroStorageKey(matchId), selectedHeroType);
      sessionStorage.setItem(sharedRuneStorageKey(matchId), JSON.stringify(selectedRuneIds));
      sessionStorage.setItem(
        sharedDeckChoiceStorageKey(matchId),
        JSON.stringify(sharedDeckChoiceFromSelectedValue(selectedDeckId).deckChoice),
      );
      setLoadState({ status: "ready", shared: joined });
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not join match");
    } finally {
      setBusy(false);
    }
  }

  function handleSelectLobbyHero(heroType: HeroType) {
    setSelectedHeroType(heroType);
    sessionStorage.setItem(sharedHeroStorageKey(matchId), heroType);
    sessionStorage.removeItem(sharedRuneStorageKey(matchId));
  }

  function startCardDrag(card: Card, event: ReactDragEvent<HTMLButtonElement>) {
    if (
      !match ||
      !cardAvailability(match, viewerSide, card, {
        canAct,
        busy,
        connectionReady: socketRef.current?.readyState === WebSocket.OPEN,
      }).playable
    ) {
      event.preventDefault();
      return;
    }

    event.dataTransfer.effectAllowed = "move";
    event.dataTransfer.setData("text/plain", card.id);
    handleCardDragStart(card);
  }

  if (loadState.status === "loading") {
    return <ShellMessage title={`Match ${matchId}`} message="Loading multiplayer match" />;
  }

  if (loadState.status === "error") {
    return (
      <ShellMessage
        title={`Match ${matchId}`}
        message={loadState.message}
        actions={
          <button className="primary-button" type="button" onClick={() => onNavigate("/play")}>
            Open play
          </button>
        }
      />
    );
  }

  if (!shared) {
    return <ShellMessage title={`Match ${matchId}`} message="Shared match unavailable" />;
  }

  if (!match) {
    const storedInvite = sessionStorage.getItem(sharedInviteStorageKey(matchId));
    const inviteLinks = parseStoredInviteLinks(storedInvite);
    const inviteUrl = inviteLinks.length === 0 ? "Invite link unavailable after reload." : inviteLinks[0].url;
    const savedHeroName = sharedSeatHeroName(shared.viewerHeroType);
    const opponentHeroName = sharedSeatHeroName(shared.opponentHeroType);
    const legalDecks =
      deckLoadState?.status === "ready"
        ? legalSharedDecks(deckLoadState.response.decks)
        : [];
    const systemDecks =
      systemDeckLoadState.status === "ready" ? sharedSystemDeckOptions(systemDeckLoadState.response.decks) : [];
    const lobbyActionLabel = sharedLobbyActionLabelForHero(shared, selectedHeroType);
    return (
      <main className="app-shell picker-shell">
        <section className="match-picker" aria-label="Shared match setup">
          <header className="picker-header">
            <div>
              <p className="eyebrow">Arcania Multiplayer</p>
              <h1>Lobby</h1>
              <p className="match-id">Match {matchId}</p>
            </div>
            <AccountActions
              currentUser={currentUser}
              onNavigate={onNavigate}
              onSignOut={onSignOut}
              allowSignOut={allowSignOut}
              loginNextPath={loginNextPath}
            />
          </header>
          {viewerSide === "player" ? (
            <div className="share-panel">
              <span>{inviteLinks.length > 1 ? "Invite Links" : "Invite Link"}</span>
              {inviteLinks.length > 1 ? (
                <div className="invite-link-list">
                  {inviteLinks
                    .filter((link) => link.side !== "player")
                    .map((link) => (
                      <p key={link.side}>
                        <span>{link.label}</span>
                        <strong>{link.url}</strong>
                        <button
                          className="icon-button"
                          type="button"
                          onClick={() => void navigator.clipboard?.writeText(link.url)}
                          title={`Copy ${link.label} link`}
                          aria-label={`Copy ${link.label} link`}
                        >
                          <Copy size={16} />
                        </button>
                      </p>
                    ))}
                </div>
              ) : (
                <strong>{inviteUrl}</strong>
              )}
              <button
                className="primary-button"
                type="button"
                onClick={() => void navigator.clipboard?.writeText(inviteUrl)}
              >
                <Copy size={18} />
                Copy
              </button>
            </div>
          ) : null}
          <HeroPicker
            selectedHeroType={selectedHeroType}
            busy={busy}
            onSelect={handleSelectLobbyHero}
          />
          <section className="setup-deck-selectors" aria-label="Shared deck selection">
            <label>
              Your Deck
              <select
                value={selectedDeckId}
                onChange={(event) => setSelectedDeckId(event.target.value)}
                disabled={busy || systemDeckLoadState.status === "loading"}
              >
                <option value="starter">Starter</option>
                {systemDecks.map((deck) => (
                  <option key={deck.id} value={deck.value}>
                    {deck.label}
                  </option>
                ))}
                {legalDecks.map((deck) => (
                  <option key={deck.id} value={deck.id}>
                    {deck.name}
                  </option>
                ))}
              </select>
            </label>
            {deckLoadState?.status === "error" ? <p className="notice">{deckLoadState.message}</p> : null}
            {systemDeckLoadState.status === "error" ? (
              <p className="notice">{systemDeckLoadState.message}</p>
            ) : null}
          </section>
          {progressionLoadState?.status === "ready" ? (
            <RuneSelector
              progression={progressionLoadState.progression}
              heroType={selectedHeroType}
              selectedRuneIds={selectedRuneIds}
              onChange={setSelectedRuneIds}
            />
          ) : null}
          {progressionLoadState?.status === "error" ? (
            <p className="notice">{progressionLoadState.message}</p>
          ) : null}
          <div className="lobby-status-grid" aria-label="Lobby status">
            {(shared.seats?.length ? shared.seats : null)?.map((seat) => (
              <LobbySeatStatus
                key={seat.side}
                label={seat.side === viewerSide ? "You" : seat.label}
                ready={seat.ready}
                heroName={sharedSeatHeroName(seat.heroType)}
              />
            )) ?? (
              <>
                <LobbySeatStatus
                  label="You"
                  ready={shared.viewerReady}
                  heroName={savedHeroName}
                />
                <LobbySeatStatus
                  label="Opponent"
                  ready={shared.opponentReady}
                  heroName={opponentHeroName}
                />
              </>
            )}
          </div>
          <button
            className="primary-button"
            type="button"
            onClick={() => void handleJoinSharedMatch()}
            disabled={busy}
          >
            <Users size={18} />
            {lobbyActionLabel}
          </button>
          <p className="notice">
            {shared.viewerReady
              ? "Waiting for both players to be ready."
              : "Choose a hero to enter the lobby."}
          </p>
          {notice ? <p className="notice">{notice}</p> : null}
        </section>
      </main>
    );
  }

  const canClaimForfeit =
    shared.canClaimForfeitAt !== null &&
    now >= shared.canClaimForfeitAt &&
    match.phase !== "matchOver";
  const enemySide = opponentSideOf(viewerSide);
  const viewerHand = handForSide(match, viewerSide);
  const teammate = participantsInMatch(match).find(
    (participant) => participant.side !== viewerSide && sameTeam(participant.side, viewerSide),
  );
  const teammateHand = teammate ? handForSide(match, teammate.side) : [];
  const liveConnectionReady = socketRef.current?.readyState === WebSocket.OPEN;
  const selectedCardAvailability = selectedCard
    ? cardAvailability(match, viewerSide, selectedCard, {
        canAct,
        busy,
        connectionReady: liveConnectionReady,
      })
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
    canAct: canAct && !busy && liveConnectionReady,
  });
  const turnChecklistItems = turnChecklistForMatch(match, viewerSide, canAct && !busy);
  const phaseLabel =
    match.phase === "matchOver"
      ? `${sideLabel(match.winner)} wins`
      : hasPendingStack
        ? `${sideLabel(match.prioritySide)} priority`
        : isActiveViewer
          ? phaseLabelFor(match.phase)
          : "Waiting";

  return (
    <main className={`app-shell match-app-shell ${matchChromeMinimized ? "match-chrome-minimized" : ""}`}>
      <section className="table match-table">
        <header className="top-bar match-chrome">
          <div>
            <p className="eyebrow">Arcania Multiplayer</p>
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
              onClick={() => onNavigate("/")}
              title="Dashboard"
            >
              <House size={18} />
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
          {canClaimForfeit ? (
            <button className="primary-button" type="button" onClick={claimForfeit} disabled={busy}>
              <Sword size={18} />
              Claim Forfeit
            </button>
          ) : null}
          {!hasPendingStack && match.phase === "movement" ? (
            <>
              <button
                className="primary-button"
                type="button"
                onClick={() => sendSharedAction({ type: "startAttackPhase" })}
                disabled={busy || !isActiveViewer}
              >
                <Sword size={18} />
                Start Attack
              </button>
              <button
                className="primary-button"
                type="button"
                onClick={() => sendSharedAction({ type: "startCardPlay" })}
                disabled={busy || !isActiveViewer}
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
              onClick={() => sendSharedAction({ type: "startCardPlay" })}
              disabled={busy || !isActiveViewer}
            >
              <Play size={18} />
              Finish Attacks
            </button>
          ) : null}
          {!hasPendingStack && match.phase === "cardPlay" ? (
            <button
              className="primary-button"
              type="button"
              onClick={() => sendSharedAction({ type: "endTurn" })}
              disabled={busy || !isActiveViewer}
            >
              <Play size={18} />
              End Turn
            </button>
          ) : null}
          {hasPendingStack ? (
            <button
              className="primary-button"
              type="button"
              onClick={() => sendSharedAction({ type: "passPriority" })}
              disabled={busy || match.phase === "matchOver" || !isPriorityViewer}
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
              <Wifi size={16} />
              {phaseLabel}
            </div>
          </div>
          <Board
            match={match}
            animation={boardAnimation}
            boardVisualMode={boardVisualMode}
            viewerSide={viewerSide}
            visualCatalog={visualCatalog}
            selectedCard={selectedCard}
            selectedPiece={selectedPiece}
            focusedCoord={boardCursor.visible ? boardCursor.coord : null}
            heroAppearances={shared.heroAppearances}
            disabled={busy || match.phase === "matchOver" || !canAct}
            onTileClick={handleTileClick}
            onTileDrop={handleCardDrop}
            onUnitContextMenu={(unit, position) => openUnitContextMenu(unit, position.x, position.y)}
            onFocusedUnitChange={setFocusedUnitPieceId}
          />
          <div className="hand-overlay">
            <div className="hand" aria-label="Hand">
              {viewerHand.map((card, index) => {
                const availability = cardAvailability(match, viewerSide, card, {
                  canAct,
                  busy,
                  connectionReady: liveConnectionReady,
                });

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
            {teammate && teammateHand.length > 0 ? (
              <section className="teammate-hand-panel" aria-label={`${teammate.side} hand`}>
                <span>{teammate.side === "playerTwo" ? "Player 2" : "Teammate"} Hand</span>
                <div className="teammate-hand">
                  {teammateHand.map((card) => (
                    <CardButton
                      key={card.id}
                      card={card}
                      visualIdentity={visualCatalog.card(card)}
                      selected={false}
                      disabled
                      unavailable
                      availabilityReason="Teammate card"
                      onClick={() => undefined}
                    />
                  ))}
                </div>
              </section>
            ) : null}
            <section className="pile-row" aria-label="Player card piles">
              <PileDisplay
                icon={<Layers size={19} />}
                label="Deck"
                count={participantBySide(match, viewerSide).deckCount}
                status="Remaining"
              />
              <PileDisplay
                icon={<Archive size={19} />}
                label="Discard"
                count={participantBySide(match, viewerSide).discardCount}
                status={
                  participantBySide(match, viewerSide).discardCount === 0 ? "Empty" : "In pile"
                }
              />
            </section>
          </div>
          <aside className="log" aria-label="Match log">
            {notice ? <p className="notice">{notice}</p> : null}
            {!shared.opponentConnected && match.phase !== "matchOver" ? (
              <p>Opponent disconnected.</p>
            ) : null}
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
            canAct &&
            ((match.actionStack.length === 0 && match.activeSide === viewerSide) ||
              (match.actionStack.length > 0 && match.prioritySide === viewerSide)) &&
            contextMenuUnit.side === viewerSide
          }
          onActivateItem={(itemId) => handleActivateUnitItem(contextMenuUnit, itemId)}
          building={buildingAt(match, contextMenuUnit.position)}
          canActivateBuilding={
            canAct &&
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
          onOpenSummary={() => onNavigate(`/match/${matchId}/${seatToken}/summary`)}
        />
      ) : null}
    </main>
  );
}

function parseStoredInviteLinks(stored: string | null): SharedSeatUrl[] {
  if (!stored) {
    return [];
  }
  try {
    const parsed = JSON.parse(stored) as SharedSeatUrl[];
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [
      {
        side: "opponent",
        team: "opponent",
        label: "Opponent",
        url: stored,
      },
    ];
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
