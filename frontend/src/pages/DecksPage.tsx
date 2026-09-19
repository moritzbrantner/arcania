import {
  Copy,
  ExternalLink,
  House,
  Minus,
  Plus,
  Save,
  Search,
  Trash2,
  WandSparkles,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  createDeck,
  deleteDeck,
  duplicateDeck,
  loadCatalog,
  loadDecks,
  loadProgression,
  previewDeckLegality,
  updateDeck,
} from "../api";
import type { AccountProps, CatalogLoadState, DeckLoadState, ProgressionLoadState } from "../appTypes";
import { AccountActions, ShellMessage } from "../components/common";
import { RuneSelector } from "../components/loadoutControls";
import { cardsFromCounts, countsFromCards, rarityLabel } from "../deckHelpers";
import { defaultRuneIdsForHero } from "../labels";
import type {
  AuthUser,
  CatalogCard,
  DeckLegality,
  DeckRecipeSummary,
  ProgressionResponse,
  HeroType,
} from "../types";
import { HERO_OPTIONS } from "../heroes";
import {
  cardCopyState,
  cardKindDetail,
  cardKindLabel,
  catalogFilterOptions,
  deckStats,
  defaultDeckCatalogFilters,
  filterCatalogCards,
  manaCostFilterLabel,
  selectedCatalogCard,
  deckVisualCards,
  type DeckCatalogFilters,
  type ManaCostFilter,
} from "./decks/deckDesignerModel";
import { DeckVisualCardGrid } from "./decks/DeckVisualCardGrid";

export function DecksPage({ currentUser, onNavigate, onSignOut }: AccountProps & { currentUser: AuthUser }) {
  const [deckLoadState, setDeckLoadState] = useState<DeckLoadState>({ status: "loading" });
  const [catalogLoadState, setCatalogLoadState] = useState<CatalogLoadState>({ status: "loading" });
  const [selectedDeckId, setSelectedDeckId] = useState<number | null>(null);
  const [deckName, setDeckName] = useState("");
  const [cardCounts, setCardCounts] = useState<Record<string, number>>({});
  const [previewLegality, setPreviewLegality] = useState<DeckLegality | null>(null);
  const [defaultDeck, setDefaultDeck] = useState(false);
  const [selectedHeroType, setSelectedHeroType] = useState<HeroType>(
    currentUser.preferredHeroType,
  );
  const [selectedRuneIds, setSelectedRuneIds] = useState<string[]>([]);
  const [progressionLoadState, setProgressionLoadState] = useState<ProgressionLoadState>({
    status: "loading",
  });
  const [catalogFilters, setCatalogFilters] = useState<DeckCatalogFilters>(defaultDeckCatalogFilters);
  const [selectedCardTemplateId, setSelectedCardTemplateId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const legalityPreviewRequestRef = useRef(0);

  useEffect(() => {
    void reloadDecks();
    loadCatalog()
      .then((response) => setCatalogLoadState({ status: "ready", cards: response.cards }))
      .catch((error: unknown) =>
        setCatalogLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load catalog",
        }),
      );
    loadProgression()
      .then((progression) => setProgressionLoadState({ status: "ready", progression }))
      .catch((error: unknown) =>
        setProgressionLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load progression",
        }),
      );
  }, []);

  async function reloadDecks(nextSelectedId?: number) {
    setDeckLoadState({ status: "loading" });
    try {
      const response = await loadDecks();
      setDeckLoadState({ status: "ready", response });
      const nextDeck =
        response.decks.find((deck) => deck.id === nextSelectedId) ??
        response.decks.find((deck) => deck.id === selectedDeckId) ??
        response.decks[0] ??
        null;
      selectDeck(nextDeck);
    } catch (error) {
      setDeckLoadState({
        status: "error",
        message: error instanceof Error ? error.message : "Could not load decks",
      });
    }
  }

  function selectDeck(deck: DeckRecipeSummary | null) {
    setSelectedDeckId(deck?.id ?? null);
    setDeckName(deck?.name ?? "New Deck");
    setDefaultDeck(deck?.isDefault ?? false);
    setCardCounts(countsFromCards(deck?.cards ?? []));
    setPreviewLegality(deck?.legality ?? null);
    setSelectedHeroType(deck?.heroType ?? currentUser.preferredHeroType);
    setSelectedRuneIds(deck?.runeIds ?? []);
    setSelectedCardTemplateId(null);
    setNotice(null);
  }

  const decks = deckLoadState.status === "ready" ? deckLoadState.response.decks : [];
  const rules = deckLoadState.status === "ready" ? deckLoadState.response.rules : null;
  const catalogCards = catalogLoadState.status === "ready" ? catalogLoadState.cards : [];
  const progression = progressionLoadState.status === "ready" ? progressionLoadState.progression : null;
  const localCards = useMemo(() => cardsFromCounts(cardCounts), [cardCounts]);
  const catalogOptions = useMemo(() => catalogFilterOptions(catalogCards), [catalogCards]);
  const filteredCards = useMemo(
    () => filterCatalogCards(catalogCards, catalogFilters),
    [catalogCards, catalogFilters],
  );
  const deckCardRows = useMemo(
    () => deckVisualCards(catalogCards, localCards, rules),
    [catalogCards, localCards, rules],
  );
  const selectedCard = useMemo(
    () => selectedCatalogCard(catalogCards, selectedCardTemplateId),
    [catalogCards, selectedCardTemplateId],
  );
  const stats = useMemo(
    () => deckStats(previewLegality, rules, selectedHeroType, selectedRuneIds, progression),
    [previewLegality, rules, selectedHeroType, selectedRuneIds, progression],
  );

  useEffect(() => {
    if (!selectedDeckId) {
      return;
    }

    const requestId = ++legalityPreviewRequestRef.current;
    const timeout = window.setTimeout(() => {
      previewDeckLegality(localCards)
        .then((legality) => {
          if (legalityPreviewRequestRef.current === requestId) {
            setPreviewLegality(legality);
          }
        })
        .catch((error: unknown) => {
          if (legalityPreviewRequestRef.current === requestId) {
            setNotice(error instanceof Error ? error.message : "Could not preview deck legality");
          }
        });
    }, 250);

    return () => window.clearTimeout(timeout);
  }, [localCards, selectedDeckId]);

  function updateCatalogFilters(patch: Partial<DeckCatalogFilters>) {
    setCatalogFilters((current) => ({ ...current, ...patch }));
  }

  function adjustCount(card: CatalogCard, delta: number) {
    const currentCount = cardCounts[card.templateId] ?? 0;
    const nextCount =
      delta > 0
        ? Math.min(currentCount + delta, cardCopyState(card, cardCounts, rules).copyLimit)
        : Math.max(0, currentCount + delta);

    setCardCounts((current) => {
      const next = { ...current };
      if (nextCount === 0) {
        delete next[card.templateId];
      } else {
        next[card.templateId] = nextCount;
      }
      return next;
    });

    if (nextCount === 0 && selectedCardTemplateId === card.templateId) {
      setSelectedCardTemplateId(null);
    }
  }

  function addCard(card: CatalogCard) {
    adjustCount(card, 1);
    setSelectedCardTemplateId(card.templateId);
  }

  async function handleSave() {
    if (!selectedDeckId) {
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const saved = await updateDeck(selectedDeckId, deckName, localCards, defaultDeck, {
        heroType: selectedHeroType,
        runeIds: selectedRuneIds,
      });
      await reloadDecks(saved.id);
      setNotice("Deck saved.");
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not save deck");
    } finally {
      setBusy(false);
    }
  }

  async function handleCreate() {
    if (decks.length >= (rules?.maxDecksPerAccount ?? 30)) {
      setNotice("Deck library limit reached.");
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const created = await createDeck("New Deck", [], false);
      await reloadDecks(created.id);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not create deck");
    } finally {
      setBusy(false);
    }
  }

  async function handleDuplicate() {
    if (!selectedDeckId) {
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      const created = await duplicateDeck(selectedDeckId);
      await reloadDecks(created.id);
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not duplicate deck");
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (!selectedDeckId || decks.length <= 1) {
      return;
    }
    setBusy(true);
    setNotice(null);
    try {
      await deleteDeck(selectedDeckId);
      await reloadDecks();
    } catch (error) {
      setNotice(error instanceof Error ? error.message : "Could not delete deck");
    } finally {
      setBusy(false);
    }
  }

  if (
    deckLoadState.status === "loading" ||
    catalogLoadState.status === "loading" ||
    progressionLoadState.status === "loading"
  ) {
    return <ShellMessage title="Decks" message="Loading deck library" />;
  }

  if (deckLoadState.status === "error") {
    return <ShellMessage title="Decks" message={deckLoadState.message} />;
  }

  if (catalogLoadState.status === "error") {
    return <ShellMessage title="Decks" message={catalogLoadState.message} />;
  }

  if (progressionLoadState.status === "error") {
    return <ShellMessage title="Decks" message={progressionLoadState.message} />;
  }

  return (
    <main className="app-shell deck-shell">
      <section className="deck-layout" aria-label="Deck library">
        <header className="top-bar catalog-header deck-page-header">
          <div>
            <p className="eyebrow">Arcania</p>
            <h1>Decks</h1>
          </div>
          <div className="actions">
            <AccountActions currentUser={currentUser} onNavigate={onNavigate} onSignOut={onSignOut} />
            <button className="icon-button" type="button" onClick={() => onNavigate("/")} title="Dashboard">
              <House size={18} />
            </button>
          </div>
        </header>

        <section className="deck-recipe-strip" aria-label="Deck recipes">
          <button className="new-deck-card" type="button" onClick={() => void handleCreate()} disabled={busy}>
            <Plus size={18} />
            New Deck
          </button>
          <div className="deck-recipe-scroll">
            {decks.map((deck) => (
              <button
                key={deck.id}
                className={`deck-recipe-card ${selectedDeckId === deck.id ? "selected" : ""}`}
                type="button"
                aria-pressed={selectedDeckId === deck.id}
                onClick={() => selectDeck(deck)}
              >
                <strong>{deck.name}</strong>
                <span>{deck.legality.legal ? "Legal" : "Draft"} · {deck.legality.totalCards} cards</span>
                {deck.isDefault ? <small>Default</small> : null}
              </button>
            ))}
          </div>
        </section>

        <aside className="deck-catalog-sidebar" aria-label="Card catalog">
          <div className="deck-panel-heading">
            <div>
              <p className="eyebrow">Catalog</p>
              <h2>Available Cards</h2>
            </div>
            <span>{filteredCards.length}</span>
          </div>

          <label className="catalog-search deck-search" htmlFor="deck-card-search">
            <Search size={17} />
            <input
              id="deck-card-search"
              value={catalogFilters.query}
              onChange={(event) => updateCatalogFilters({ query: event.target.value })}
              placeholder="Search cards"
              autoComplete="off"
            />
          </label>

          <div className="deck-filter-grid" aria-label="Catalog filters">
            <label>
              Kind
              <select
                value={catalogFilters.kind}
                onChange={(event) =>
                  updateCatalogFilters({ kind: event.target.value as DeckCatalogFilters["kind"] })
                }
              >
                <option value="all">All kinds</option>
                <option value="unit">Units</option>
                <option value="spell">Spells</option>
                <option value="item">Items</option>
                <option value="building">Buildings</option>
              </select>
            </label>
            <label>
              Rarity
              <select
                value={catalogFilters.rarity}
                onChange={(event) =>
                  updateCatalogFilters({ rarity: event.target.value as DeckCatalogFilters["rarity"] })
                }
              >
                <option value="all">All rarities</option>
                <option value="basic">Basic</option>
                <option value="advanced">Advanced</option>
                <option value="rare">Rare</option>
              </select>
            </label>
            <label>
              Mana
              <select
                value={String(catalogFilters.manaCost)}
                onChange={(event) =>
                  updateCatalogFilters({ manaCost: parseManaCostFilter(event.target.value) })
                }
              >
                {catalogOptions.manaCosts.map((manaCost) => (
                  <option key={String(manaCost)} value={String(manaCost)}>
                    {manaCostFilterLabel(manaCost)}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="deck-catalog-list">
            {filteredCards.map((card) => {
              const copyState = cardCopyState(card, cardCounts, rules);

              return (
                <div key={card.id} className={`deck-catalog-row ${card.rarity}`}>
                  <button
                    type="button"
                    className="deck-row-select"
                    onClick={() => setSelectedCardTemplateId(card.templateId)}
                    aria-label={`Select ${card.name}`}
                  >
                    <span>
                      <strong>{card.name}</strong>
                      <small>{rarityLabel(card.rarity)} · {card.cost} mana · {cardKindLabel(card)}</small>
                    </span>
                    {copyState.count > 0 ? <em>{copyState.count}</em> : null}
                  </button>
                  <button
                    className="icon-button deck-add-button"
                    type="button"
                    onClick={() => addCard(card)}
                    disabled={busy || copyState.copyLimitReached}
                    title={
                      copyState.copyLimitReached
                        ? `${rarityLabel(card.rarity)} copy limit reached`
                        : `Add ${card.name}`
                    }
                    aria-label={`Add ${card.name}`}
                  >
                    <Plus size={17} />
                  </button>
                </div>
              );
            })}
          </div>
        </aside>

        <section className="deck-editor" aria-label="Deck editor">
          <div className="deck-editor-header">
            <label>
              Name
              <input value={deckName} onChange={(event) => setDeckName(event.target.value)} />
            </label>
            <label className="deck-default-toggle">
              <input
                type="checkbox"
                checked={defaultDeck}
                onChange={(event) => setDefaultDeck(event.target.checked)}
              />
              Default
            </label>
            <button className="primary-button" type="button" onClick={() => void handleSave()} disabled={busy || !selectedDeckId}>
              <Save size={18} />
              Save
            </button>
            {selectedDeckId ? (
              <button
                className="secondary-link deck-public-link"
                type="button"
                onClick={() => onNavigate(`/@${currentUser.handle}/decks/${selectedDeckId}`)}
              >
                <ExternalLink size={18} />
                ID #{selectedDeckId}
              </button>
            ) : null}
            <button className="secondary-link" type="button" onClick={() => void handleDuplicate()} disabled={busy || !selectedDeckId}>
              <Copy size={18} />
              Duplicate
            </button>
            <button className="secondary-link" type="button" onClick={() => void handleDelete()} disabled={busy || !selectedDeckId || decks.length <= 1}>
              <Trash2 size={18} />
              Delete
            </button>
          </div>

          {notice ? <p className="notice">{notice}</p> : null}

          <div className="deck-panel-heading">
            <div>
              <p className="eyebrow">Deck recipe</p>
              <h2>Picked Cards</h2>
            </div>
            <span>{stats.totalCards}/{stats.minCards}</span>
          </div>

          <DeckVisualCardGrid
            cards={deckCardRows}
            selectedTemplateId={selectedCardTemplateId}
            busy={busy}
            onSelect={setSelectedCardTemplateId}
            onAdjust={adjustCount}
          />
        </section>

        <aside className="deck-details-panel" aria-label="Deck details">
          <DeckStatsPanel
            stats={stats}
            selectedHeroType={selectedHeroType}
            selectedRuneIds={selectedRuneIds}
            progression={progressionLoadState.progression}
            onHeroChange={(heroType) => {
              setSelectedHeroType(heroType);
              setSelectedRuneIds(defaultRuneIdsForHero(progressionLoadState.progression, heroType));
            }}
            onRuneChange={setSelectedRuneIds}
          />

          <section className="selected-card-panel" aria-label="Selected card details">
            <div className="deck-panel-heading">
              <div>
                <p className="eyebrow">Card</p>
                <h2>{selectedCard ? selectedCard.name : "Select a Card"}</h2>
              </div>
              {selectedCard ? <span>{rarityLabel(selectedCard.rarity)}</span> : null}
            </div>

            {selectedCard ? (
              <SelectedCardDetails
                card={selectedCard}
                count={cardCopyState(selectedCard, cardCounts, rules).count}
                copyLimit={cardCopyState(selectedCard, cardCounts, rules).copyLimit}
                copyLimitReached={cardCopyState(selectedCard, cardCounts, rules).copyLimitReached}
                busy={busy}
                onAdjust={adjustCount}
              />
            ) : (
              <div className="deck-empty-state compact">
                <WandSparkles size={20} />
                <span>Pick a Card from the catalog or Deck recipe list to inspect its details.</span>
              </div>
            )}
          </section>
        </aside>
      </section>
    </main>
  );
}

function DeckStatsPanel({
  stats,
  selectedHeroType,
  selectedRuneIds,
  progression,
  onHeroChange,
  onRuneChange,
}: {
  stats: ReturnType<typeof deckStats>;
  selectedHeroType: HeroType;
  selectedRuneIds: string[];
  progression: ProgressionResponse;
  onHeroChange: (heroType: HeroType) => void;
  onRuneChange: (runeIds: string[]) => void;
}) {
  return (
    <section className={`deck-stats-panel ${stats.legal ? "legal" : "draft"}`} aria-label="Deck stats">
      <div className="deck-panel-heading">
        <div>
          <p className="eyebrow">Stats</p>
          <h2>{stats.statusLabel}</h2>
        </div>
        <span>{stats.totalCards}/{stats.minCards}</span>
      </div>

      <div className="deck-stat-grid">
        <DetailStat label="Total" value={`${stats.totalCards}/${stats.minCards}`} />
        <DetailStat label="Basic" value={String(stats.basicCards)} />
        <DetailStat
          label="Advanced"
          value={
            stats.advancedTotalLimit === null
              ? String(stats.advancedCards)
              : `${stats.advancedCards}/${stats.advancedTotalLimit}`
          }
        />
        <DetailStat
          label="Rare"
          value={
            stats.rareTotalLimit === null
              ? String(stats.rareCards)
              : `${stats.rareCards}/${stats.rareTotalLimit}`
          }
        />
      </div>

      {stats.messages.length > 0 ? (
        <ul className="deck-message-list">
          {stats.messages.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}

      <div className="deck-config-panel" aria-label="Hero configuration">
        <div className="preferred-hero-control">
          <span>Hero</span>
          <select
            value={selectedHeroType}
            onChange={(event) => onHeroChange(event.target.value as HeroType)}
          >
            {HERO_OPTIONS.map((hero) => (
              <option key={hero.id} value={hero.id}>
                {hero.name}
              </option>
            ))}
          </select>
        </div>
        <div className="deck-config-skills">
          <span>Active Skills</span>
          <strong>{stats.skillNames.join(", ") || "Base skill only"}</strong>
        </div>
        <RuneSelector
          progression={progression}
          heroType={selectedHeroType}
          selectedRuneIds={selectedRuneIds}
          onChange={onRuneChange}
        />
      </div>
    </section>
  );
}

function SelectedCardDetails({
  card,
  count,
  copyLimit,
  copyLimitReached,
  busy,
  onAdjust,
}: {
  card: CatalogCard;
  count: number;
  copyLimit: number;
  copyLimitReached: boolean;
  busy: boolean;
  onAdjust: (card: CatalogCard, delta: number) => void;
}) {
  return (
    <div className={`selected-card-details ${card.rarity}`}>
      <div className="selected-card-meta">
        <DetailStat label="Mana" value={String(card.cost)} />
        <DetailStat label="Kind" value={cardKindLabel(card)} />
        <DetailStat label="Count" value={`${count}/${copyLimit}`} />
      </div>
      <p>{card.text}</p>
      <p className="selected-card-kind">{cardKindDetail(card)}</p>
      <DeckCountControls
        card={card}
        count={count}
        copyLimitReached={copyLimitReached}
        busy={busy}
        onAdjust={onAdjust}
      />
    </div>
  );
}

function DeckCountControls({
  card,
  count,
  copyLimitReached,
  busy,
  onAdjust,
}: {
  card: CatalogCard;
  count: number;
  copyLimitReached: boolean;
  busy: boolean;
  onAdjust: (card: CatalogCard, delta: number) => void;
}) {
  return (
    <div className="deck-count-controls" aria-label={`${card.name} count`}>
      <button
        type="button"
        onClick={() => onAdjust(card, -1)}
        disabled={busy || count <= 0}
        aria-label={`Remove ${card.name}`}
      >
        <Minus size={15} />
      </button>
      <strong>{count}</strong>
      <button
        type="button"
        onClick={() => onAdjust(card, 1)}
        disabled={busy || copyLimitReached}
        title={copyLimitReached ? `${rarityLabel(card.rarity)} copy limit reached` : undefined}
        aria-label={`Add ${card.name}`}
      >
        <Plus size={15} />
      </button>
    </div>
  );
}

function DetailStat({ label, value }: { label: string; value: string }) {
  return (
    <span className="detail-stat">
      <small>{label}</small>
      <strong>{value}</strong>
    </span>
  );
}

function parseManaCostFilter(value: string): ManaCostFilter {
  if (value === "all" || value === "5plus") {
    return value;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : "all";
}

export function DeckLegalityPanel({ legality }: { legality: DeckLegality }) {
  return (
    <section className={`deck-legality ${legality.legal ? "legal" : "draft"}`} aria-label="Deck legality">
      <div>
        <strong>{legality.legal ? "Legal deck" : "Draft deck"}</strong>
        <span>
          {legality.totalCards} cards · {legality.basicCards} Basic · {legality.advancedCards} Advanced · {legality.rareCards} Rare
        </span>
      </div>
      {legality.messages.length > 0 ? (
        <ul>
          {legality.messages.map((message) => (
            <li key={message}>{message}</li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}
