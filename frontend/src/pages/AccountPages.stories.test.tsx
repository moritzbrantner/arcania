// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, screen, within } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { loadProgression } from "../api";
import { storyProgression } from "../storybook/fixtures";
import { renderStory } from "../storybook/renderStory";
import heroesMeta, { Ready as HeroesReady } from "./HeroesPage.stories";
import progressionMeta, { Ready as ProgressionReady } from "./ProgressionPage.stories";

vi.mock("../api", async (importOriginal) => ({
  ...await importOriginal<typeof import("../api")>(),
  loadProgression: vi.fn(),
}));
vi.mock("../HeroPreview3D", () => ({ HeroPreview3D: () => <div>Hero preview</div> }));

beforeEach(() => vi.mocked(loadProgression).mockResolvedValue(storyProgression));
afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

it("renders the Heroes story and changes the selected Hero from its roster", async () => {
  renderStory(heroesMeta, HeroesReady);
  const roster = within(await screen.findByRole("list", { name: "Hero roster" }));
  expect(roster.getAllByRole("listitem")).toHaveLength(8);
  expect(screen.getByRole("region", { name: "Runekeeper mastery" })).toHaveTextContent("Rune Focus");
  fireEvent.click(roster.getByText("Pyromancer", { exact: true }));
  expect(screen.getByRole("region", { name: "Pyromancer mastery" })).toHaveTextContent("Kindling");
  expect(screen.queryByRole("region", { name: "Runekeeper mastery" })).not.toBeInTheDocument();
});

it("renders account progress and both rune groups from the Progression story", async () => {
  renderStory(progressionMeta, ProgressionReady);
  const unlocked = within(await screen.findByRole("region", { name: "Unlocked runes" }));
  const locked = within(screen.getByRole("region", { name: "Locked runes" }));
  expect(unlocked.getByText("Ember Rune", { exact: true })).toBeVisible();
  expect(unlocked.getByText("Warding Rune", { exact: true })).toBeVisible();
  expect(locked.getByText("Chrono Rune", { exact: true })).toBeVisible();
  expect(locked.getByText("Unlocks at level 5")).toBeVisible();
  expect(screen.getByText("Next level in 180 XP")).toBeVisible();
});
