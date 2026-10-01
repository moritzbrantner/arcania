// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { useState } from "react";
import { userEvent } from "storybook/test";
import { afterEach, expect, it, vi } from "vitest";
import meta, { HeroPickerStory } from "./loadoutControls.stories";
import { renderStory } from "../storybook/renderStory";
import { HeroPicker } from "./loadoutControls";
import { BOARD_VISUAL_MODE_STORAGE_KEY } from "../boardVisualMode";
import type { HeroType } from "../types";

afterEach(() => {
  cleanup();
  localStorage.clear();
  vi.restoreAllMocks();
});

it("selects a Hero through keyboard controls with a DOM portrait fallback", async () => {
  localStorage.setItem(BOARD_VISUAL_MODE_STORAGE_KEY, "2d");
  renderStory(meta, HeroPickerStory);
  expect(screen.getByRole("img", { name: "Runekeeper portrait" })).toBeInTheDocument();
  const pyromancer = screen.getByRole("button", { name: /^Pyromancer/ });
  pyromancer.focus();
  await userEvent.keyboard("{Enter}");
  expect(pyromancer).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByRole("button", { name: /^Runekeeper/ })).toHaveAttribute("aria-pressed", "false");
  expect(screen.getByRole("img", { name: "Pyromancer portrait" })).toBeInTheDocument();
});

it("keeps selection disabled while busy even when WebGL is unavailable", async () => {
  localStorage.setItem(BOARD_VISUAL_MODE_STORAGE_KEY, "3d");
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
  render(<Picker busy />);
  const pyromancer = screen.getByRole("button", { name: /^Pyromancer/ });
  expect(screen.getAllByRole("button")).toHaveLength(8);
  for (const option of screen.getAllByRole("button")) expect(option).toBeDisabled();
  await userEvent.click(pyromancer);
  expect(screen.getByRole("button", { name: /^Runekeeper/ })).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByRole("img", { name: "Runekeeper portrait" })).toBeInTheDocument();
});

function Picker({ busy = false }: { busy?: boolean }) {
  const [hero, setHero] = useState<HeroType>("runekeeper");
  return <HeroPicker selectedHeroType={hero} busy={busy} onSelect={setHero} />;
}
