// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { loadProgression } from "../api";
import { storyAccount, storyProgression } from "../storybook/fixtures";
import { ProgressionPage } from "./ProgressionPage";

vi.mock("../api", async (importOriginal) => ({
  ...await importOriginal<typeof import("../api")>(),
  loadProgression: vi.fn(),
}));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

function renderProgression(runes = storyProgression.runes) {
  vi.mocked(loadProgression).mockResolvedValue({ ...storyProgression, runes });
  render(<ProgressionPage currentUser={storyAccount} onNavigate={vi.fn()} onSignOut={vi.fn()} />);
}

it("shows each rune once in its unlock group while retaining account progress", async () => {
  renderProgression();
  const unlocked = within(await screen.findByRole("region", { name: "Unlocked runes" }));
  const locked = within(screen.getByRole("region", { name: "Locked runes" }));
  for (const rune of storyProgression.runes) {
    const group = rune.unlocked ? unlocked : locked;
    expect(group.getByText(rune.name, { exact: true })).toBeVisible();
    expect(screen.getAllByText(rune.name, { exact: true })).toHaveLength(1);
    expect(group.getByText(rune.unlocked ? rune.text : `Unlocks at level ${rune.unlockLevel}`)).toBeVisible();
  }
  expect(screen.getByText("Next level in 180 XP")).toBeVisible();
  expect(screen.getByText("Next rune: Chrono Rune at level 5")).toBeVisible();
  expect(unlocked.queryByRole("button")).not.toBeInTheDocument();
  expect(locked.queryByRole("button")).not.toBeInTheDocument();
});

it("explains an empty locked group when every rune is unlocked", async () => {
  renderProgression(storyProgression.runes.map((rune) => ({ ...rune, unlocked: true })));
  const unlocked = await screen.findByRole("region", { name: "Unlocked runes" });
  expect(within(unlocked).getAllByRole("article")).toHaveLength(storyProgression.runes.length);
  expect(within(screen.getByRole("region", { name: "Locked runes" })).getByText("No locked runes.")).toBeVisible();
  expect(screen.getByText("All runes unlocked.")).toBeVisible();
});

it("explains an empty unlocked group before the first rune unlock", async () => {
  renderProgression(storyProgression.runes.map((rune) => ({ ...rune, unlocked: false })));
  const unlocked = await screen.findByRole("region", { name: "Unlocked runes" });
  expect(within(unlocked).getByText("No runes unlocked yet.")).toBeVisible();
  expect(within(screen.getByRole("region", { name: "Locked runes" })).getAllByRole("article")).toHaveLength(storyProgression.runes.length);
  expect(screen.getByText("Next rune: Ember Rune at level 2")).toBeVisible();
});

it("explains an empty catalog without claiming all runes are unlocked", async () => {
  renderProgression([]);
  expect(await screen.findByText("No rune unlocks available.")).toBeVisible();
  expect(screen.queryByText("All runes unlocked.")).not.toBeInTheDocument();
  expect(screen.queryAllByRole("article")).toHaveLength(0);
  expect(screen.getByText("Next level in 180 XP")).toBeVisible();
});
