// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { loadDecks, loadProfileMatches, loadProgression } from "../api";
import { DashboardPage } from "./DashboardPage";
import type { AuthUser, DeckRecipeSummary, MatchSummary, ProgressionResponse } from "../types";

vi.mock("../api", () => ({ loadDecks: vi.fn(), loadProfileMatches: vi.fn(), loadProgression: vi.fn() }));
vi.mock("../HeroPreview3D", () => ({ HeroPreview3D: ({ label }: { label: string }) => <div>{label} preview</div> }));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

const user = { id: 1, handle: "player", email: "player@example.com", displayName: "Player", avatar: { symbol: "sparkles", color: "emerald" }, preferredHeroType: "runekeeper", boardVisualMode: "3d", progressionSummary: { totalXp: 10, level: 1, currentLevelXp: 0, nextLevelXp: 100, xpIntoLevel: 10, xpToNextLevel: 90, runeSlots: 1 } } satisfies AuthUser;
const progression = { account: user.progressionSummary, runes: [], heroes: [{ heroType: "runekeeper", xp: 10, level: 1, currentLevelXp: 0, nextLevelXp: 100, xpIntoLevel: 10, xpToNextLevel: 90, totalSkillPoints: 0, spentSkillPoints: 0, availableSkillPoints: 0, unlockedSkillIds: [] }], skillTrees: [], loadouts: [], heroAppearances: [] } satisfies ProgressionResponse;
const deck = { id: 1, name: "Balanced Starter", isDefault: true, heroType: "runekeeper", runeIds: [], cards: [], legality: { legal: true, totalCards: 30, basicCards: 30, advancedCards: 0, rareCards: 0, messages: [] }, createdAt: 1, updatedAt: 1 } satisfies DeckRecipeSummary;
type DeckResponse = Awaited<ReturnType<typeof loadDecks>>;
const deckResponse = { rules: { maxDecksPerAccount: 30, minCards: 30, basicCopyLimit: 5, advancedCopyLimit: 4, rareCopyLimit: 3, advancedTotalLimit: 12, rareTotalLimit: 6 }, decks: [deck] } satisfies DeckResponse;
const unavailableDeckCases = [
  { label: "loading", request: () => new Promise<DeckResponse>(() => {}) },
  { label: "failed", request: () => Promise.reject(new Error("Deck library unavailable")) },
  { label: "no legal deck", request: () => Promise.resolve({ ...deckResponse, decks: [{ ...deck, legality: { ...deck.legality, legal: false } }] }) },
];

function setup(matches: MatchSummary[] = [], decksRequest: Promise<DeckResponse> = Promise.resolve(deckResponse), progressionRequest: Promise<ProgressionResponse> = Promise.resolve(progression)) {
  vi.mocked(loadProgression).mockReturnValue(progressionRequest);
  vi.mocked(loadDecks).mockReturnValue(decksRequest);
  vi.mocked(loadProfileMatches).mockResolvedValue({ matches });
  const onNavigate = vi.fn();
  render(<DashboardPage currentUser={user} onNavigate={onNavigate} onSignOut={vi.fn()} />);
  return onNavigate;
}

describe("DashboardPage", () => {
  it("shows the default configured loadout and starts a match when none is active", async () => {
    const onNavigate = setup();
    expect(await screen.findByRole("region", { name: "Current loadout" })).toHaveTextContent("Balanced Starter");
    fireEvent.click(screen.getByRole("button", { name: "Start match" }));
    expect(onNavigate).toHaveBeenCalledWith("/play");
  });

  it("continues the most recently updated active match and links to all active matches", async () => {
    const onNavigate = setup([
      { matchId: "older", mode: "solo", createdAt: 1, updatedAt: 10, round: 1, phase: "movement", winner: null, frameCount: 1 },
      { matchId: "newer", mode: "solo", createdAt: 1, updatedAt: 20, round: 2, phase: "attack", winner: null, frameCount: 2 },
    ]);
    await screen.findByRole("button", { name: "Continue match" });
    fireEvent.click(screen.getByRole("button", { name: "Continue match" }));
    fireEvent.click(screen.getByRole("button", { name: "2 active matches" }));
    expect(onNavigate).toHaveBeenCalledWith("/match/newer");
    expect(onNavigate).toHaveBeenCalledWith("/matches");
  });

  it("bounds recent match previews to four entries", async () => {
    setup(Array.from({ length: 5 }, (_, index) => ({ matchId: `match-${index}`, mode: "solo" as const, createdAt: index, updatedAt: index, round: 1, phase: "matchOver" as const, winner: "player" as const, frameCount: 1 })));
    await waitFor(() => expect(screen.getByText(/match-4/)).toBeInTheDocument());
    expect(screen.getByText(/match-3/)).toBeInTheDocument();
    expect(screen.queryByText(/match-0/)).not.toBeInTheDocument();
  });

  it.each(unavailableDeckCases)("continues the newest active match with $label deck data", async ({ request }) => {
    const onNavigate = setup([
      { matchId: "older", mode: "solo", createdAt: 1, updatedAt: 10, round: 1, phase: "movement", winner: null, frameCount: 1 },
      { matchId: "newer", mode: "solo", createdAt: 1, updatedAt: 20, round: 2, phase: "attack", winner: null, frameCount: 2 },
    ], request());
    fireEvent.click(await screen.findByRole("button", { name: "Continue match" }));
    fireEvent.click(screen.getByRole("button", { name: "2 active matches" }));
    expect(onNavigate).toHaveBeenCalledWith("/match/newer");
    expect(onNavigate).toHaveBeenCalledWith("/matches");
    expect(screen.queryByRole("button", { name: "Start match" })).not.toBeInTheDocument();
  });

  it.each(unavailableDeckCases)("offers Decks instead of starting a match with $label deck data", async ({ request }) => {
    const onNavigate = setup([], request());
    const loadout = within(screen.getByRole("region", { name: "Current loadout" }));
    fireEvent.click(await loadout.findByRole("button", { name: "Open Decks" }));
    expect(onNavigate).toHaveBeenCalledWith("/decks");
    expect(loadout.queryByRole("button", { name: "Start match" })).not.toBeInTheDocument();
    expect(loadout.queryByRole("button", { name: "Continue match" })).not.toBeInTheDocument();
  });

  it("shows progression failures without loading placeholders and preserves the other sections", async () => {
    const onNavigate = setup([], undefined, Promise.reject(new Error("Progression service unavailable")));
    expect(await screen.findAllByText("Progression service unavailable")).toHaveLength(2);
    expect(screen.queryByText("Loading heroes.")).not.toBeInTheDocument();
    expect(screen.queryByText("Loading progression.")).not.toBeInTheDocument();
    expect(screen.getByText("Balanced Starter")).toBeInTheDocument();
    expect(await screen.findByText("No matches yet.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "View all Heroes" }));
    fireEvent.click(screen.getByRole("button", { name: "View progression" }));
    expect(onNavigate).toHaveBeenCalledWith("/heroes");
    expect(onNavigate).toHaveBeenCalledWith("/progression");
  });

  it("shows deck failures without a loading placeholder and preserves progression and Matches", async () => {
    const onNavigate = setup([], Promise.reject(new Error("Deck service unavailable")));
    expect(await screen.findAllByText("Deck service unavailable")).toHaveLength(2);
    expect(screen.queryByText("Loading decks.")).not.toBeInTheDocument();
    expect(await screen.findByText("Next level in 90 XP")).toBeInTheDocument();
    expect(await screen.findByText("No matches yet.")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Manage decks" }));
    expect(onNavigate).toHaveBeenCalledWith("/decks");
  });
});
