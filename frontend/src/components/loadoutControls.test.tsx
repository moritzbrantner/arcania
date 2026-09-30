// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HeroPicker, LoadoutCarousel } from "./loadoutControls";
import type { DeckRecipeSummary, HeroType, ProgressionResponse } from "../types";
import type { HomeLoadout } from "../deckHelpers";

vi.mock("../HeroPreview3D", () => ({
  HeroPreview3D: ({ label }: { label: string }) => <div>{label} preview</div>,
}));

afterEach(() => cleanup());

describe("HeroPicker", () => {
  it("shows the selected Hero in the 3D stage and emits roster changes", () => {
    const onSelect = vi.fn();

    render(
      <HeroPicker
        legend="Your Hero"
        selectedHeroType="runekeeper"
        busy={false}
        onSelect={onSelect}
      />,
    );

    expect(screen.getByText("Runekeeper preview")).toBeInTheDocument();
    expect(screen.getByRole("group", { name: "Your Hero" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Pyromancer/ }));

    expect(onSelect).toHaveBeenCalledWith("pyromancer");
  });
});

describe("LoadoutCarousel", () => {
  it("wraps through configured deck recipes and displays saved runes", () => {
    renderCarousel();

    expect(screen.getByRole("button", { name: /Default Legal.*Custom deck recipe/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    fireEvent.click(screen.getByRole("button", { name: "Next configured deck recipe" }));

    expect(screen.getByRole("button", { name: /Tournament Legal.*Custom deck recipe/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByText("Spark Stone")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Next configured deck recipe" }));

    expect(screen.getByRole("button", { name: /Default Legal.*Custom deck recipe/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });

  it("disables carousel movement while busy", () => {
    renderCarousel({ busy: true });

    expect(screen.getByRole("button", { name: "Next configured deck recipe" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Select Tournament Legal" })).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Next configured deck recipe" }));

    expect(screen.getByRole("button", { name: /Default Legal.*Custom deck recipe/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});

function renderCarousel({ busy = false }: { busy?: boolean } = {}) {
  return render(<CarouselHarness busy={busy} />);
}

function CarouselHarness({ busy }: { busy: boolean }) {
  const [selectedLoadoutId, setSelectedLoadoutId] = useState<string | null>("account:102");

  return (
    <LoadoutCarousel
      loadouts={loadouts}
      selectedLoadoutId={selectedLoadoutId}
      progression={progression}
      busy={busy}
      onSelect={(loadout) => setSelectedLoadoutId(loadout.id)}
    />
  );
}

const loadouts: HomeLoadout[] = [
  accountLoadout(101, "Tournament Legal", "pyromancer", ["spark-stone"]),
  accountLoadout(102, "Default Legal", "runekeeper", []),
];

function accountLoadout(
  id: number,
  name: string,
  heroType: HeroType,
  runeIds: string[],
): HomeLoadout {
  const deck: DeckRecipeSummary = {
    id,
    name,
    isDefault: id === 102,
    heroType,
    runeIds,
    cards: [],
    legality: {
      legal: true,
      totalCards: 30,
      basicCards: 30,
      advancedCards: 0,
      rareCards: 0,
      messages: [],
    },
    createdAt: 1,
    updatedAt: 2,
  };

  return {
    kind: "account",
    id: `account:${id}`,
    name,
    heroType,
    cardCount: deck.legality.totalCards,
    legal: deck.legality.legal,
    runeIds,
    deckChoice: { source: "account", deckId: id },
    deck,
  };
}

const progression: ProgressionResponse = {
  account: {
    totalXp: 0,
    level: 2,
    currentLevelXp: 100,
    nextLevelXp: 250,
    xpIntoLevel: 10,
    xpToNextLevel: 140,
    runeSlots: 1,
  },
  runes: [
    {
      id: "spark-stone",
      name: "Spark Stone",
      text: "Start with a brighter spark.",
      unlockLevel: 1,
      unlocked: true,
    },
  ],
  heroes: [],
  skillTrees: [],
  loadouts: [],
  heroAppearances: [],
};
