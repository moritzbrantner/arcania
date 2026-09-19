// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiRequestError, loadMatchSummary, loadSharedMatchSummary } from "../api";
import type { AuthUser, MatchSummaryResponse } from "../types";
import { MatchSummaryPage } from "./MatchSummaryPage";

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    loadMatchSummary: vi.fn(),
    loadSharedMatchSummary: vi.fn(),
  };
});

afterEach(() => cleanup());

beforeEach(() => {
  vi.mocked(loadMatchSummary).mockReset();
  vi.mocked(loadSharedMatchSummary).mockReset();
});

describe("MatchSummaryPage", () => {
  it("offers sign-in for signed-out private summary failures", async () => {
    const onNavigate = vi.fn();
    vi.mocked(loadMatchSummary).mockRejectedValue(
      new ApiRequestError("Match summary for match arc-private was not found", 404),
    );

    renderSummary({ onNavigate });

    expect(await screen.findByText("Sign in to view this match summary")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Sign In" }));

    expect(onNavigate).toHaveBeenCalledWith("/login?next=%2Fmatches%2Farc-private%2Fsummary");
  });

  it("renders anonymous ownerless summaries after a successful load", async () => {
    vi.mocked(loadMatchSummary).mockResolvedValue(summaryResponse());

    renderSummary();

    expect(await screen.findByRole("heading", { name: "Victory" })).toBeInTheDocument();
    expect(screen.queryByText("Sign in to view this match summary")).not.toBeInTheDocument();
    expect(screen.getByText("arc-private")).toBeInTheDocument();
  });

  it("keeps shared seat-link summary failures on the normal error state", async () => {
    vi.mocked(loadSharedMatchSummary).mockRejectedValue(
      new ApiRequestError("Match summary for match arc-private was not found", 404),
    );

    renderSummary({ seatToken: "player-seat" });

    expect(await screen.findByText("Match summary for match arc-private was not found")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open Play" })).toBeInTheDocument();
    expect(screen.queryByText("Sign in to view this match summary")).not.toBeInTheDocument();
  });

  it("keeps signed-in not-found responses on the normal error state", async () => {
    vi.mocked(loadMatchSummary).mockRejectedValue(
      new ApiRequestError("Match summary for match arc-private was not found", 404),
    );

    renderSummary({ currentUser: authUser() });

    expect(await screen.findByText("Match summary for match arc-private was not found")).toBeInTheDocument();
    expect(screen.queryByText("Sign in to view this match summary")).not.toBeInTheDocument();
  });
});

function renderSummary({
  currentUser = null,
  seatToken,
  onNavigate = vi.fn(),
}: {
  currentUser?: AuthUser | null;
  seatToken?: string;
  onNavigate?: (to: string) => void;
} = {}) {
  return render(
    <MatchSummaryPage
      matchId="arc-private"
      seatToken={seatToken}
      currentUser={currentUser}
      onNavigate={onNavigate}
      onSignOut={vi.fn()}
      allowSignOut={false}
      loginNextPath="/matches/arc-private/summary"
    />,
  );
}

function summaryResponse(): MatchSummaryResponse {
  return {
    matchId: "arc-private",
    summary: {
      matchId: "arc-private",
      mode: "solo",
      createdAt: 1_700_000_000,
      updatedAt: 1_700_000_400,
      round: 4,
      phase: "matchOver",
      winner: "player",
      frameCount: 18,
    },
    viewer: {
      side: "player",
      result: "victory",
    },
    reward: null,
  };
}

function authUser(): AuthUser {
  return {
    id: 1,
    handle: "player-one",
    email: "player@example.com",
    displayName: "Player One",
    avatar: {
      symbol: "star",
      color: "#58b7a1",
    },
    preferredHeroType: "runekeeper",
    boardVisualMode: "2d",
    progressionSummary: {
      totalXp: 120,
      level: 2,
      currentLevelXp: 100,
      nextLevelXp: 250,
      xpIntoLevel: 20,
      xpToNextLevel: 130,
      runeSlots: 1,
    },
  };
}
