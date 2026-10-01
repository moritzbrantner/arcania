import type { DeckCardCount } from "../types";
import type { WorkshopSystemDeckRecipe } from "./engine";

export function systemDeckForRecipe(
  recipe: readonly DeckCardCount[],
  systemDecks: readonly WorkshopSystemDeckRecipe[],
) {
  return systemDecks.find((deck) => deck.cards.length === recipe.length
    && deck.cards.every((configured) => recipe.some((card) => configured.templateId === card.templateId && configured.count === card.count)));
}

export function DeckSelect({ label, recipe, systemDecks, onChange }: {
  label: string;
  recipe: DeckCardCount[];
  systemDecks: WorkshopSystemDeckRecipe[];
  onChange: (recipe: DeckCardCount[]) => void;
}) {
  const selectedId = systemDeckForRecipe(recipe, systemDecks)?.id ?? "custom";
  return <label className="workshop-field"><span>{label}</span><select value={selectedId} onChange={(event) => {
    const selected = systemDecks.find((deck) => deck.id === event.target.value);
    if (selected) { onChange(selected.cards.map((card) => ({ ...card }))); }
  }}>
    {selectedId === "custom" ? <option value="custom">Custom recipe</option> : null}
    {systemDecks.map((deck) => <option key={deck.id} value={deck.id}>{deck.name}</option>)}
  </select></label>;
}
