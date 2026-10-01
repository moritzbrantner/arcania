// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { loadSharedMatchSummary } from "../api";
import { storySecondSeatSummaryResponse } from "../storybook/fixtures";
import { renderStory } from "../storybook/renderStory";
import meta, { WinningSecondSeat } from "./MatchSummaryPage.stories";

vi.mock("../api", async (importOriginal) => ({
  ...await importOriginal<typeof import("../api")>(),
  loadSharedMatchSummary: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

it("shows the winning second Seat's account and Hero rewards", async () => {
  vi.mocked(loadSharedMatchSummary).mockResolvedValue(storySecondSeatSummaryResponse);
  renderStory(meta, WinningSecondSeat);
  expect(await screen.findByRole("heading", { name: "Victory" })).toBeVisible();
  expect(screen.getByText("Account XP")).toBeVisible();
  expect(screen.getByText("Runekeeper Mastery")).toBeVisible();
  expect(screen.getAllByText("+150")).toHaveLength(2);
  expect(screen.getByText("+50")).toBeVisible();
  expect(loadSharedMatchSummary).toHaveBeenCalledWith("rl-shared", "player-two-seat");
  expect(screen.queryByText("No account progression was awarded for this viewer.")).not.toBeInTheDocument();
});
