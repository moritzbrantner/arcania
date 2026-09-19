// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiRequestError, loadReplay, loadSharedReplay } from "../api";
import { storyMatch } from "../components/board.fixtures";
import { DEFAULT_ACCOUNT_PREFERENCES } from "../preferences";
import type { MatchReplayResponse } from "../types";
import { ReplayPage } from "./ReplayPage";

vi.mock("../api", async () => {
  const actual = await vi.importActual<typeof import("../api")>("../api");
  return {
    ...actual,
    loadReplay: vi.fn(),
    loadSharedReplay: vi.fn(),
  };
});

afterEach(() => cleanup());

beforeEach(() => {
  vi.mocked(loadReplay).mockReset();
  vi.mocked(loadSharedReplay).mockReset();
});

describe("ReplayPage", () => {
  it("offers sign-in for signed-out private replay failures", async () => {
    const onNavigate = vi.fn();
    vi.mocked(loadReplay).mockRejectedValue(
      new ApiRequestError("Replay for match arc-private was not found", 404),
    );

    renderReplay({ onNavigate });

    expect(await screen.findByText("Sign in to view this replay")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Sign In" }));

    expect(onNavigate).toHaveBeenCalledWith("/login?next=%2Fmatches%2Farc-private%2Freplay");
  });

  it("renders anonymous ownerless replays after a successful load", async () => {
    vi.mocked(loadReplay).mockResolvedValue(replayResponse());

    renderReplay();

    expect(await screen.findByText("Arcania Replay")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Round 1" })).toBeInTheDocument();
    expect(screen.queryByText("Sign in to view this replay")).not.toBeInTheDocument();
  });

  it("keeps shared seat-link replay failures on the normal error state", async () => {
    vi.mocked(loadSharedReplay).mockRejectedValue(
      new ApiRequestError("Replay for match arc-private was not found", 404),
    );

    renderReplay({ seatToken: "player-seat" });

    expect(await screen.findByText("Replay for match arc-private was not found")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Match Archive" })).toBeInTheDocument();
    expect(screen.queryByText("Sign in to view this replay")).not.toBeInTheDocument();
  });
});

function renderReplay({
  seatToken,
  onNavigate = vi.fn(),
}: {
  seatToken?: string;
  onNavigate?: (to: string) => void;
} = {}) {
  return render(
    <ReplayPage
      matchId="arc-private"
      seatToken={seatToken}
      currentUser={null}
      onNavigate={onNavigate}
      onSignOut={vi.fn()}
      allowSignOut={false}
      loginNextPath="/matches/arc-private/replay"
      visualPreferences={{
        preferences: {
          ...DEFAULT_ACCOUNT_PREFERENCES,
          boardVisualMode: "2d",
        },
        effectiveMotion: "reduced",
        liveAiDelayMs: 0,
      }}
    />,
  );
}

function replayResponse(): MatchReplayResponse {
  const match = storyMatch();
  return {
    matchId: "arc-private",
    visibility: "public",
    summary: {
      matchId: "arc-private",
      mode: "solo",
      createdAt: 1_700_000_000,
      updatedAt: 1_700_000_400,
      round: match.round,
      phase: match.phase,
      winner: match.winner,
      frameCount: 1,
    },
    frames: [
      {
        frameIndex: 0,
        actionIndex: null,
        event: { type: "matchCreated" },
        matchState: match,
      },
    ],
  };
}
