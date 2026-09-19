import { House, Search, WandSparkles } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { loadCatalog } from "../api";
import type { CatalogLoadState } from "../appTypes";
import { DetailStat, ShellMessage } from "../components/common";
import { buildingEffectLabel, itemActiveLabel, itemPassiveLabel, kindSummary, spellEffectLabel } from "../labels";
import type { CatalogCard, Rarity } from "../types";

type KindFilter = "all" | CatalogCard["kind"]["type"];
type RarityFilter = "all" | Rarity;

export function CatalogPage({ onNavigate }: { onNavigate: (to: string) => void }) {
  const [loadState, setLoadState] = useState<CatalogLoadState>({ status: "loading" });
  const [query, setQuery] = useState("");
  const [kindFilter, setKindFilter] = useState<KindFilter>("all");
  const [rarityFilter, setRarityFilter] = useState<RarityFilter>("all");
  const [selectedId, setSelectedId] = useState<string | null>(null);

  useEffect(() => {
    loadCatalog()
      .then((response) => setLoadState({ status: "ready", cards: response.cards }))
      .catch((error: unknown) =>
        setLoadState({
          status: "error",
          message: error instanceof Error ? error.message : "Could not load catalog",
        }),
      );
  }, []);

  const filteredCards = useMemo(() => {
    if (loadState.status !== "ready") {
      return [];
    }

    const normalizedQuery = query.trim().toLocaleLowerCase();
    return loadState.cards.filter((card) => {
      const matchesQuery =
        normalizedQuery.length === 0 ||
        card.name.toLocaleLowerCase().includes(normalizedQuery) ||
        card.text.toLocaleLowerCase().includes(normalizedQuery);
      const matchesKind = kindFilter === "all" || card.kind.type === kindFilter;
      const matchesRarity = rarityFilter === "all" || card.rarity === rarityFilter;
      return matchesQuery && matchesKind && matchesRarity;
    });
  }, [kindFilter, loadState, query, rarityFilter]);

  if (loadState.status === "loading") {
    return <ShellMessage title="Card Catalog" message="Loading cards" />;
  }

  if (loadState.status === "error") {
    return (
      <ShellMessage
        title="Card Catalog"
        message={loadState.message}
        actions={
          <button className="primary-button" type="button" onClick={() => onNavigate("/play")}>
            Open play
          </button>
        }
      />
    );
  }

  const selectedCard =
    filteredCards.find((card) => card.id === selectedId) ??
    filteredCards[0] ??
    null;

  return (
    <main className="app-shell catalog-shell">
      <section className="catalog-layout" aria-label="Card catalog">
        <header className="top-bar catalog-header">
          <div>
            <p className="eyebrow">Arcania</p>
            <h1>Card Catalog</h1>
          </div>
          <div className="actions">
            <button
              className="icon-button"
              type="button"
              onClick={() => onNavigate("/")}
              title="Dashboard"
            >
              <House size={18} />
            </button>
          </div>
        </header>

        <section className="catalog-controls" aria-label="Catalog filters">
          <label className="catalog-search" htmlFor="catalog-search">
            <Search size={17} />
            <input
              id="catalog-search"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Search cards"
              autoComplete="off"
            />
          </label>
          <SegmentedFilter
            label="Kind"
            value={kindFilter}
            options={[
              ["all", "All"],
              ["unit", "Units"],
              ["spell", "Spells"],
              ["item", "Items"],
              ["building", "Buildings"],
            ]}
            onChange={(value) => setKindFilter(value as KindFilter)}
          />
          <SegmentedFilter
            label="Rarity"
            value={rarityFilter}
            options={[
              ["all", "All"],
              ["basic", "Basic"],
              ["advanced", "Advanced"],
              ["rare", "Rare"],
            ]}
            onChange={(value) => setRarityFilter(value as RarityFilter)}
          />
        </section>

        <div className="catalog-content">
          <section className="catalog-grid" aria-label="Starter cards">
            {filteredCards.map((card) => (
              <CatalogCardButton
                key={card.id}
                card={card}
                selected={selectedCard?.id === card.id}
                onClick={() => setSelectedId(card.id)}
              />
            ))}
            {filteredCards.length === 0 ? <p className="empty-state">No cards found.</p> : null}
          </section>
          <CatalogDetail card={selectedCard} />
        </div>
      </section>
    </main>
  );
}

export function SegmentedFilter({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: Array<[string, string]>;
  onChange: (value: string) => void;
}) {
  return (
    <fieldset className="segmented-filter">
      <legend>{label}</legend>
      <div>
        {options.map(([optionValue, optionLabel]) => (
          <button
            key={optionValue}
            className={value === optionValue ? "active" : ""}
            type="button"
            onClick={() => onChange(optionValue)}
          >
            {optionLabel}
          </button>
        ))}
      </div>
    </fieldset>
  );
}

export function CatalogCardButton({
  card,
  selected,
  onClick,
}: {
  card: CatalogCard;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className={`catalog-card ${selected ? "selected" : ""} ${card.rarity}`}
      type="button"
      onClick={onClick}
    >
      <img src={card.artPath} alt="" loading="lazy" />
      <span className="card-cost">{card.cost}</span>
      <span className="catalog-card-body">
        <strong>{card.name}</strong>
        <span className="card-stats">
          <WandSparkles size={13} />
          {card.rarity} · {kindSummary(card)}
        </span>
        <span className="card-text">{card.text}</span>
        <span className="copy-count">{card.copyCount} in starter deck</span>
      </span>
    </button>
  );
}

export function CatalogDetail({ card }: { card: CatalogCard | null }) {
  if (!card) {
    return <aside className="catalog-detail empty-state">No card selected.</aside>;
  }

  return (
    <aside className={`catalog-detail ${card.rarity}`} aria-label="Selected card">
      <img src={card.artPath} alt="" />
      <div className="catalog-detail-body">
        <div>
          <p className="eyebrow">{card.rarity} {card.kind.type}</p>
          <h2>{card.name}</h2>
        </div>
        <div className="detail-stat-row">
          <DetailStat label="Cost" value={card.cost} />
          <DetailStat label="Copies" value={card.copyCount} />
          {card.kind.type === "unit" ? (
            <>
              <DetailStat label="Attack" value={card.kind.attack} />
              <DetailStat label="Armor" value={card.kind.armor} />
              <DetailStat label="Unit AP" value={card.kind.maxAp} />
            </>
          ) : card.kind.type === "spell" ? (
            <>
              <DetailStat label="Range" value={card.kind.range} />
              <DetailStat label="Priority" value={card.kind.priority} />
              <DetailStat label="Effect" value={spellEffectLabel(card)} />
            </>
          ) : card.kind.type === "item" ? (
            <>
              <DetailStat label="Range" value={card.kind.range} />
              <DetailStat label="Passive" value={itemPassiveLabel(card.kind.passive)} />
              <DetailStat
                label="Active"
                value={card.kind.active ? itemActiveLabel(card.kind.active) : "None"}
              />
            </>
          ) : card.kind.type === "building" ? (
            <>
              <DetailStat label="Type" value="Building" />
              <DetailStat label="Effect" value={buildingEffectLabel(card.kind.effect)} />
            </>
          ) : (
            <DetailStat label="Source" value="+1 mana when occupied" />
          )}
        </div>
        <p className="detail-rules">{card.text}</p>
      </div>
    </aside>
  );
}
