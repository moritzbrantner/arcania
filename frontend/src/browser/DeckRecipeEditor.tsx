import { Minus, Plus, Swords } from "lucide-react";
import { useState } from "react";
import type { CatalogCard, DeckCardCount } from "../types";
import { DeckSelect } from "./DeckSelect";
import type { WorkshopSystemDeckRecipe } from "./engine";

export type DeckSide = "player" | "opponent";

export function DeckRecipeEditor({
  playerRecipe,
  opponentRecipe,
  catalog,
  systemDecks,
  onChange,
}: {
  playerRecipe: DeckCardCount[];
  opponentRecipe: DeckCardCount[];
  catalog: CatalogCard[];
  systemDecks: WorkshopSystemDeckRecipe[];
  onChange: (side: DeckSide, recipe: DeckCardCount[]) => void;
}) {
  const [side, setSide] = useState<DeckSide>("player");
  const [search, setSearch] = useState("");
  const recipe = side === "player" ? playerRecipe : opponentRecipe;
  const filteredCards = catalog.filter(
    (card) =>
      !card.templateId.startsWith("custom-") &&
      (search.trim() === "" ||
        card.name.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase())),
  );

  function setRecipe(cards: DeckCardCount[]) {
    onChange(side, cards);
  }

  function setCount(card: CatalogCard, count: number) {
    const existing = recipe.find((entry) => entry.templateId === card.templateId);
    if (count === 0) {
      setRecipe(recipe.filter((entry) => entry.templateId !== card.templateId));
      return;
    }
    if (existing) {
      setRecipe(
        recipe.map((entry) =>
          entry.templateId === card.templateId ? { ...entry, count: count } : entry,
        ),
      );
      return;
    }
    setRecipe([...recipe, { templateId: card.templateId, count: count }]);
  }

  return (
    <section className="workshop-deck-editor" aria-label="Deck recipe editor">
      <header className="workshop-deck-heading">
        <div>
          <p className="eyebrow">
            <Swords size={15} />
            Match loadout
          </p>
          <h2>Pick a recipe. Make it yours.</h2>
          <p>Start from any system deck recipe, then tune individual card counts for this browser preset.</p>
        </div>
        <strong>{deckRecipeTotal(recipe)} cards</strong>
      </header>

      <div className="workshop-deck-side" aria-label="Deck recipe side">
        <button
          type="button"
          aria-pressed={side === "player"}
          onClick={() => setSide("player")}
        >
          Your deck recipe
        </button>
        <button
          type="button"
          aria-pressed={side === "opponent"}
          onClick={() => setSide("opponent")}
        >
          Bot deck recipe
        </button>
      </div>

      <div className="workshop-deck-toolbar">
        <DeckSelect label="Base recipe" recipe={recipe} systemDecks={systemDecks} onChange={setRecipe} />
        <label className="workshop-field">
          <span>Find a card</span>
          <input
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search the catalog"
          />
        </label>
      </div>

      <div className="workshop-deck-card-list">
        {filteredCards.map((card) => {
          const count = recipe.find((entry) => entry.templateId === card.templateId)?.count ?? 0;
          return (
            <article key={card.templateId} className={count > 0 ? "selected" : undefined}>
              <img src={card.artPath} alt="" />
              <div>
                <strong>{card.name}</strong>
                <span>
                  {card.rarity} · {card.cost} Mana
                </span>
              </div>
              <div className="workshop-deck-count">
                <button
                  type="button"
                  aria-label={`Remove one ${card.name}`}
                  onClick={() => setCount(card, count - 1)}
                  disabled={count === 0}
                >
                  <Minus size={15} />
                </button>
                <input
                  type="number"
                  min={0}
                  max={30}
                  value={count}
                  aria-label={`${card.name} copies`}
                  onChange={(event) => setCount(card, Number(event.target.value))}
                />
                <button
                  type="button"
                  aria-label={`Add one ${card.name}`}
                  onClick={() => setCount(card, count + 1)}
                  disabled={count >= 30}
                >
                  <Plus size={15} />
                </button>
              </div>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function deckRecipeTotal(recipe: DeckCardCount[]) {
  return recipe.reduce((total, card) => total + card.count, 0);
}

