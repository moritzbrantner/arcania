import { deckChoiceFromValue, deckChoiceValue } from "../../deckHelpers";
import { heroOptionByType } from "../../labels";
import type {
  DeckChoice,
  DeckRecipeSummary,
  HeroType,
  SharedMatchResponse,
  SystemDeckRecipe,
} from "../../types";

export function sharedHeroStorageKey(matchId: string) {
  return `arcania-hero:${matchId}`;
}

export function sharedRuneStorageKey(matchId: string) {
  return `arcania-runes:${matchId}`;
}

export function sharedDeckChoiceStorageKey(matchId: string) {
  return `arcania-deck-choice:${matchId}`;
}

export function sharedInviteStorageKey(matchId: string) {
  return `arcania-invite:${matchId}`;
}

export function legalSharedDecks(decks: DeckRecipeSummary[]) {
  return decks.filter((deck) => deck.legality.legal);
}

export function sharedLobbyActionLabelForHero(shared: SharedMatchResponse, selectedHeroType: HeroType) {
  if (shared.viewerReady) {
    return shared.viewerHeroType !== selectedHeroType ? "Update Hero" : "Ready";
  }
  return shared.viewerSide === "player" ? "Ready" : "Join Lobby";
}

export function sharedSeatHeroName(heroType: HeroType | null) {
  return heroType ? (heroOptionByType(heroType)?.name ?? null) : null;
}

export function sharedDeckChoiceFromSelectedValue(value: string): {
  deckRecipeId?: number;
  deckChoice: DeckChoice;
} {
  const deckChoice = deckChoiceFromValue(value);
  return {
    deckRecipeId: deckChoice.source === "account" ? deckChoice.deckId : undefined,
    deckChoice,
  };
}

export function selectedSharedDeckValue(choice: DeckChoice | null) {
  return deckChoiceValue(choice);
}

export function sharedSystemDeckOptions(decks: SystemDeckRecipe[]) {
  return decks.map((deck) => ({
    id: deck.id,
    value: `system:${deck.id}`,
    label: deck.name,
  }));
}
