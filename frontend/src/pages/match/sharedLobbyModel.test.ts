import { describe, expect, it } from "vitest";
import type { DeckRecipeSummary, SharedMatchResponse, SystemDeckRecipe } from "../../types";
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
} from "./sharedLobbyModel";

describe("shared lobby model", () => {
  it("names per-match session storage keys", () => {
    expect(sharedHeroStorageKey("arc-1")).toBe("arcania-hero:arc-1");
    expect(sharedRuneStorageKey("arc-1")).toBe("arcania-runes:arc-1");
    expect(sharedDeckChoiceStorageKey("arc-1")).toBe("arcania-deck-choice:arc-1");
    expect(sharedInviteStorageKey("arc-1")).toBe("arcania-invite:arc-1");
  });

  it("keeps the existing lobby action labels", () => {
    expect(sharedLobbyActionLabelForHero(shared({ viewerReady: false, viewerSide: "player" }), "runekeeper")).toBe("Ready");
    expect(sharedLobbyActionLabelForHero(shared({ viewerReady: false, viewerSide: "opponent" }), "runekeeper")).toBe("Join Lobby");
    expect(sharedLobbyActionLabelForHero(shared({ viewerReady: true, viewerHeroType: "runekeeper" }), "runekeeper")).toBe("Ready");
    expect(sharedLobbyActionLabelForHero(shared({ viewerReady: true, viewerHeroType: "runekeeper" }), "pyromancer")).toBe("Update Hero");
  });

  it("maps deck selections to join payload values", () => {
    expect(sharedDeckChoiceFromSelectedValue("starter")).toEqual({
      deckRecipeId: undefined,
      deckChoice: { source: "starter" },
    });
    expect(sharedDeckChoiceFromSelectedValue("system:ember")).toEqual({
      deckRecipeId: undefined,
      deckChoice: { source: "system", systemDeckId: "ember" },
    });
    expect(sharedDeckChoiceFromSelectedValue("42")).toEqual({
      deckRecipeId: 42,
      deckChoice: { source: "account", deckId: 42 },
    });
    expect(selectedSharedDeckValue({ source: "account", deckId: 42 })).toBe("42");
  });

  it("filters legal account decks and formats system deck options", () => {
    expect(legalSharedDecks([deck(1, true), deck(2, false)])).toEqual([deck(1, true)]);
    expect(sharedSystemDeckOptions([systemDeck("starter"), systemDeck("ember")])).toEqual([
      { id: "starter", value: "system:starter", label: "starter deck" },
      { id: "ember", value: "system:ember", label: "ember deck" },
    ]);
  });

  it("formats saved seat hero names", () => {
    expect(sharedSeatHeroName("runekeeper")).toBe("Runekeeper");
    expect(sharedSeatHeroName(null)).toBeNull();
  });
});

function shared(overrides: Partial<SharedMatchResponse>): SharedMatchResponse {
  return {
    matchId: "arc-1",
    mode: "shared",
    status: "setup",
    viewerSide: "player",
    viewerHeroType: null,
    opponentHeroType: null,
    viewerReady: false,
    opponentReady: false,
    activeSide: null,
    opponentConnected: false,
    canClaimForfeitAt: null,
    heroAppearances: [],
    matchState: null,
    ...overrides,
  };
}

function deck(id: number, legal: boolean): DeckRecipeSummary {
  return {
    id,
    name: `deck ${id}`,
    isDefault: false,
    heroType: "runekeeper",
    runeIds: [],
    cards: [],
    legality: {
      legal,
      totalCards: legal ? 30 : 3,
      basicCards: legal ? 30 : 3,
      advancedCards: 0,
      rareCards: 0,
      messages: [],
    },
    createdAt: 1,
    updatedAt: 1,
  };
}

function systemDeck(id: string): SystemDeckRecipe {
  return {
    id,
    name: `${id} deck`,
    heroType: "runekeeper",
    cards: [],
    legality: {
      legal: true,
      totalCards: 30,
      basicCards: 30,
      advancedCards: 0,
      rareCards: 0,
      messages: [],
    },
  };
}
