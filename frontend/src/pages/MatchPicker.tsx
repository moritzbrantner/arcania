import { BookOpen, ChevronDown, Layers, Plus, Users } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { FormEvent } from "react";
import {
  createMatch,
  createSharedMatch,
  createSharedTwoVTwoMatch,
  loadDecks,
  loadProgression,
  loadSystemDecks,
} from "../api";
import type { AccountProps, DeckLoadState, ProgressionLoadState, SystemDeckLoadState } from "../appTypes";
import { TopNav } from "../components/common";
import { LoadoutCarousel } from "../components/loadoutControls";
import {
  accountDeckToLoadout,
  aiSelectionFromValue,
  type HomeLoadout,
} from "../deckHelpers";
import type { HeroType } from "../types";
import { HERO_OPTIONS } from "../heroes";

export function PlayPage({
  onNavigate,
  currentUser,
  onSignOut,
}: {
  onNavigate: (to: string) => void;
} & AccountProps) {
  const [matchId, setMatchId] = useState("");
  const [deckLoadState, setDeckLoadState] = useState<DeckLoadState | null>(
    currentUser ? { status: "loading" } : null,
  );
  const [systemDeckLoadState, setSystemDeckLoadState] = useState<SystemDeckLoadState>({
    status: "loading",
  });
  const [progressionLoadState, setProgressionLoadState] = useState<ProgressionLoadState | null>(
    currentUser ? { status: "loading" } : null,
  );
  const [selectedLoadoutId, setSelectedLoadoutId] = useState<string | null>(null);
  const [selectedAiDeck, setSelectedAiDeck] = useState<string>("system:balanced-starter");
  const [selectedAiHeroType, setSelectedAiHeroType] = useState<HeroType>("runekeeper");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  useEffect(() => {
    loadSystemDecks()
      .then((response) => {
        setSystemDeckLoadState({ status: "ready", response });
        if (!response.decks.some((deck) => `system:${deck.id}` === selectedAiDeck)) {
          setSelectedAiDeck(`system:${response.decks[0]?.id ?? "balanced-starter"}`);
        }
      })
      .catch((error: unknown) =>
        setSystemDeckLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load AI decks",
        }),
      );
  }, []);

  useEffect(() => {
    if (!currentUser) {
      setDeckLoadState(null);
      setProgressionLoadState(null);
      setSelectedLoadoutId(null);
      return;
    }
    setDeckLoadState({ status: "loading" });
    loadDecks()
      .then((response) => {
        setDeckLoadState({ status: "ready", response });
        setSelectedLoadoutId((current) => {
          const selectedDeck = response.decks.find(
            (deck) => deck.legality.legal && `account:${deck.id}` === current,
          );
          const defaultDeck = response.decks.find((deck) => deck.isDefault && deck.legality.legal);
          const firstLegalDeck = response.decks.find((deck) => deck.legality.legal);
          const fallbackDeck = selectedDeck ?? defaultDeck ?? firstLegalDeck ?? null;
          return fallbackDeck ? `account:${fallbackDeck.id}` : null;
        });
      })
      .catch((error: unknown) =>
        setDeckLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load decks",
        }),
      );
    setProgressionLoadState({ status: "loading" });
    loadProgression()
      .then((progression) => setProgressionLoadState({ status: "ready", progression }))
      .catch((error: unknown) =>
        setProgressionLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load progression",
        }),
      );
  }, [currentUser]);

  const systemDecks =
    systemDeckLoadState.status === "ready" ? systemDeckLoadState.response.decks : [];
  const accountDecks = deckLoadState?.status === "ready" ? deckLoadState.response.decks : [];
  const legalAccountDecks = accountDecks.filter((deck) => deck.legality.legal);
  const legalAccountLoadouts = useMemo(
    () => legalAccountDecks.map(accountDeckToLoadout),
    [legalAccountDecks],
  );
  const selectedLoadout =
    legalAccountLoadouts.find((loadout) => loadout.id === selectedLoadoutId) ??
    null;
  const selectedAiDeckIsAccount = selectedAiDeck.startsWith("account:");
  const canStartMatch = currentUser !== null && deckLoadState?.status === "ready" && selectedLoadout !== null;

  async function handleCreateMatch() {
    if (!selectedLoadout) {
      setNotice("Create a legal configured deck recipe before starting a match.");
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const aiOpponent = aiSelectionFromValue(selectedAiDeck, selectedAiHeroType);
      const created = await createMatch({
        heroType: selectedLoadout.heroType,
        playerDeck: selectedLoadout.deckChoice,
        ...(aiOpponent ? { aiOpponent } : {}),
        ...(selectedLoadout.runeIds.length > 0 ? { runeIds: selectedLoadout.runeIds } : {}),
      });
      onNavigate(`/match/${created.matchId}`);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create match");
    } finally {
      setBusy(false);
    }
  }

  async function handleCreateSharedMatch() {
    if (!selectedLoadout) {
      setNotice("Create a legal configured deck recipe before starting a match.");
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const created = await createSharedMatch(selectedLoadout.heroType);
      sessionStorage.setItem(
        `arcania-invite:${created.matchId}`,
        `${window.location.origin}${created.inviteSeatUrl}`,
      );
      sessionStorage.setItem(`arcania-hero:${created.matchId}`, selectedLoadout.heroType);
      sessionStorage.setItem(`arcania-runes:${created.matchId}`, JSON.stringify(selectedLoadout.runeIds));
      sessionStorage.setItem(
        `arcania-deck-choice:${created.matchId}`,
        JSON.stringify(selectedLoadout.deckChoice),
      );
      onNavigate(created.playerSeatUrl);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create multiplayer match");
    } finally {
      setBusy(false);
    }
  }

  async function handleCreateSharedTwoVTwoMatch() {
    if (!selectedLoadout) {
      setNotice("Create a legal configured deck recipe before starting a match.");
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const created = await createSharedTwoVTwoMatch(selectedLoadout.heroType);
      const seatUrls = created.seatUrls ?? [];
      const playerSeat = seatUrls.find((seat) => seat.side === "player")?.url ?? created.playerSeatUrl;
      sessionStorage.setItem(
        `arcania-invite:${created.matchId}`,
        JSON.stringify(
          seatUrls.map((seat) => ({
            ...seat,
            url: `${window.location.origin}${seat.url}`,
          })),
        ),
      );
      sessionStorage.setItem(`arcania-hero:${created.matchId}`, selectedLoadout.heroType);
      sessionStorage.setItem(`arcania-runes:${created.matchId}`, JSON.stringify(selectedLoadout.runeIds));
      sessionStorage.setItem(
        `arcania-deck-choice:${created.matchId}`,
        JSON.stringify(selectedLoadout.deckChoice),
      );
      onNavigate(playerSeat);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create 2v2 match");
    } finally {
      setBusy(false);
    }
  }

  function handleOpenMatch(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const normalized = matchId.trim();
    if (!normalized) {
      setNotice("Enter a match ID.");
      return;
    }
    onNavigate(`/match/${encodeURIComponent(normalized)}`);
  }

  function handleSelectLoadout(loadout: HomeLoadout) {
    setSelectedLoadoutId(loadout.id);
    setNotice(null);
  }

  return (
    <main className="app-shell home-shell">
      <TopNav currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} activePath="/play" />
      <section className="home-layout play-layout" aria-label="Play setup">
        <header className="home-heading">
          <p className="eyebrow">Arcania</p>
          <h1>Play</h1>
        </header>

        <section className="home-setup-panel play-quickstart-panel" aria-label="Quick start">
          {!currentUser ? (
            <div className="loadout-empty-state">
              <strong>Sign in to play configured deck recipes.</strong>
              <span>Basic Play uses your saved legal deck recipes, heroes, and rune loadouts.</span>
            </div>
          ) : null}
          {currentUser && deckLoadState?.status === "loading" ? (
            <p className="notice">Loading configured deck recipes</p>
          ) : null}
          {currentUser && deckLoadState?.status === "ready" && legalAccountLoadouts.length > 0 ? (
            <LoadoutCarousel
              loadouts={legalAccountLoadouts}
              selectedLoadoutId={selectedLoadoutId}
              progression={progressionLoadState?.status === "ready" ? progressionLoadState.progression : null}
              busy={busy}
              onSelect={handleSelectLoadout}
            />
          ) : null}
          {currentUser && deckLoadState?.status === "ready" && legalAccountLoadouts.length === 0 ? (
            <div className="loadout-empty-state">
              <strong>Create a legal configured deck recipe to play.</strong>
              <span>Draft deck recipes stay in Decks until they satisfy the deck-building rules.</span>
              <button className="secondary-link" type="button" onClick={() => onNavigate("/decks")}>
                <Layers size={18} />
                Manage Decks
              </button>
            </div>
          ) : null}
          {progressionLoadState?.status === "error" ? (
            <p className="notice">{progressionLoadState.message}</p>
          ) : null}
          {deckLoadState?.status === "error" ? <p className="notice">{deckLoadState.message}</p> : null}
          {systemDeckLoadState.status === "error" ? (
            <p className="notice">{systemDeckLoadState.message}</p>
          ) : null}
          <div className="picker-actions home-actions">
            <button
              className="primary-button"
              type="button"
              onClick={() => void handleCreateMatch()}
              disabled={busy || !canStartMatch}
            >
              <Plus size={18} />
              New Solo Match
            </button>
            <button
              className="primary-button"
              type="button"
              onClick={() => void handleCreateSharedMatch()}
              disabled={busy || !canStartMatch}
            >
              <Users size={18} />
              New Multiplayer Match
            </button>
            <button
              className="primary-button"
              type="button"
              onClick={() => void handleCreateSharedTwoVTwoMatch()}
              disabled={busy || !canStartMatch}
            >
              <Users size={18} />
              New 2v2 Match
            </button>
            <button
              className="secondary-link"
              type="button"
              onClick={() => onNavigate("/tutorial")}
              disabled={busy}
            >
              <BookOpen size={18} />
              Tutorial
            </button>
            <button
              className="secondary-link"
              type="button"
              onClick={() => onNavigate("/wiki")}
              disabled={busy}
            >
              <BookOpen size={18} />
              Rules
            </button>
          </div>
          <button
            className="secondary-link advanced-setup-toggle"
            type="button"
            aria-expanded={advancedOpen}
            aria-controls="advanced-play-setup"
            onClick={() => setAdvancedOpen((open) => !open)}
          >
            <ChevronDown size={18} aria-hidden="true" />
            Advanced setup
          </button>
        </section>

        {advancedOpen ? (
          <section id="advanced-play-setup" className="advanced-play-setup" aria-label="Advanced setup">
            <section className="home-setup-panel" aria-label="AI opponent setup">
              <div className="setup-deck-selectors" aria-label="AI opponent selection">
                <label>
                AI Deck
                <select
                  value={selectedAiDeck}
                  onChange={(event) => setSelectedAiDeck(event.target.value)}
                  disabled={busy || systemDeckLoadState.status === "loading"}
                >
                  {systemDecks.map((deck) => (
                    <option key={deck.id} value={`system:${deck.id}`}>
                      {deck.name}
                    </option>
                  ))}
                  {legalAccountDecks.map((deck) => (
                    <option key={deck.id} value={`account:${deck.id}`}>
                      {deck.name}
                    </option>
                  ))}
                </select>
                </label>
                {selectedAiDeckIsAccount ? (
                  <label>
                  AI Hero
                  <select
                    value={selectedAiHeroType}
                    onChange={(event) => setSelectedAiHeroType(event.target.value as HeroType)}
                    disabled={busy}
                  >
                    {HERO_OPTIONS.map((hero) => (
                      <option key={hero.id} value={hero.id}>
                        {hero.name}
                      </option>
                    ))}
                  </select>
                  </label>
                ) : null}
              </div>
            </section>
          </section>
        ) : null}
        <form className="open-match-form" onSubmit={handleOpenMatch}>
          <label htmlFor="match-id">Open Match by ID</label>
          <div>
            <input
              id="match-id"
              value={matchId}
              onChange={(event) => setMatchId(event.target.value)}
              placeholder="arc-lx5n2w"
              autoComplete="off"
            />
            <button className="primary-button" type="submit">
              Open
            </button>
          </div>
        </form>
        {notice ? <p className="notice">{notice}</p> : null}
      </section>
    </main>
  );
}

export const MatchPicker = PlayPage;
