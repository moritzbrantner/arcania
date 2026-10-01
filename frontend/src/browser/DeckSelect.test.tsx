// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, expect, it } from "vitest";
import { DeckSelect } from "./DeckSelect";
import type { WorkshopSystemDeckRecipe } from "./engine";

afterEach(cleanup);

const decks: WorkshopSystemDeckRecipe[] = [
  { id: "balanced-starter", name: "Balanced Starter", heroType: "runekeeper", cards: [{ templateId: "ember-squire", count: 4 }, { templateId: "spark-jolt", count: 4 }] },
  { id: "ember-burn", name: "Ember Burn", heroType: "pyromancer", cards: [{ templateId: "ember-squire", count: 5 }] },
];

function Selectors({ busy = false }: { busy?: boolean }) {
  const [player, setPlayer] = useState(decks[0].cards);
  const [opponent, setOpponent] = useState(decks[0].cards);
  return <fieldset disabled={busy}>
    <DeckSelect label="Your deck" recipe={player} systemDecks={decks} onChange={setPlayer} />
    <DeckSelect label="Bot deck" recipe={opponent} systemDecks={decks} onChange={setOpponent} />
  </fieldset>;
}

it("changes one side while preserving the other selection", () => {
  render(<Selectors />);
  fireEvent.change(screen.getByRole("combobox", { name: "Your deck" }), { target: { value: "ember-burn" } });
  expect(screen.getByRole("combobox", { name: "Your deck" })).toHaveValue("ember-burn");
  expect(screen.getByRole("combobox", { name: "Bot deck" })).toHaveValue("balanced-starter");
  fireEvent.change(screen.getByRole("combobox", { name: "Bot deck" }), { target: { value: "ember-burn" } });
  expect(screen.getByRole("combobox", { name: "Bot deck" })).toHaveValue("ember-burn");
});

it("recognizes reordered counts and presents unmatched counts as a custom recipe", () => {
  const { rerender } = render(<DeckSelect label="Your deck" recipe={[...decks[0].cards].reverse()} systemDecks={decks} onChange={() => {}} />);
  expect(screen.getByRole("combobox", { name: "Your deck" })).toHaveValue("balanced-starter");
  rerender(<DeckSelect label="Your deck" recipe={[{ templateId: "spark-jolt", count: 20 }]} systemDecks={decks} onChange={() => {}} />);
  expect(screen.getByRole("combobox", { name: "Your deck" })).toHaveValue("custom");
  expect(screen.getByRole("option", { name: "Custom recipe" })).toBeVisible();
});

it("uses the surrounding busy state to disable both selectors", () => {
  render(<Selectors busy />);
  expect(screen.getByRole("combobox", { name: "Your deck" })).toBeDisabled();
  expect(screen.getByRole("combobox", { name: "Bot deck" })).toBeDisabled();
});
