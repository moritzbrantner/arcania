// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { loadProgression, updatePreferredHero } from "../api";
import { storyAccount, storyProgression } from "../storybook/fixtures";
import { HeroesPage } from "./HeroesPage";

vi.mock("../api", async (importOriginal) => ({
  ...await importOriginal<typeof import("../api")>(),
  loadProgression: vi.fn(),
  updatePreferredHero: vi.fn(),
}));
vi.mock("../HeroPreview3D", () => ({ HeroPreview3D: () => <div>Hero preview</div> }));

afterEach(() => {
  cleanup();
  vi.resetAllMocks();
});

it("keeps the preferred Hero after a failed save and permits a successful retry", async () => {
  vi.mocked(loadProgression).mockResolvedValue(storyProgression);
  vi.mocked(updatePreferredHero).mockRejectedValueOnce(new Error("Could not save your preferred Hero."));
  const onProfileUpdated = vi.fn();
  const props = { currentUser: storyAccount, onNavigate: vi.fn(), onSignOut: vi.fn(), onProfileUpdated };
  const { rerender } = render(<HeroesPage {...props} />);

  fireEvent.click(await screen.findByRole("button", { name: "Pyromancer" }));
  fireEvent.click(screen.getByRole("button", { name: "Make preferred" }));
  expect(await screen.findByRole("alert")).toHaveTextContent("Could not save your preferred Hero.");
  expect(screen.getByRole("button", { name: "Make preferred" })).toBeEnabled();
  expect(onProfileUpdated).not.toHaveBeenCalled();

  fireEvent.click(screen.getByRole("button", { name: "Runekeeper" }));
  expect(screen.getByRole("button", { name: "Preferred Hero" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Pyromancer" }));

  const updated = { ...storyAccount, preferredHeroType: "pyromancer" as const };
  vi.mocked(updatePreferredHero).mockResolvedValueOnce(updated);
  fireEvent.click(screen.getByRole("button", { name: "Make preferred" }));
  await waitFor(() => expect(onProfileUpdated).toHaveBeenCalledExactlyOnceWith(updated));
  rerender(<HeroesPage {...props} currentUser={updated} />);
  expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Preferred Hero" })).toBeDisabled();
  expect(updatePreferredHero).toHaveBeenNthCalledWith(1, "pyromancer");
  expect(updatePreferredHero).toHaveBeenNthCalledWith(2, "pyromancer");
});
