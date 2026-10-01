// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { composeStories } from "@storybook/react-vite";
import { afterEach, expect, it } from "vitest";
import * as stories from "./DeckRecipeEditor.stories";

const { SystemRecipe, Empty } = composeStories(stories);
afterEach(cleanup);

it("edits exact counts independently for each side", () => {
  render(<SystemRecipe />);
  fireEvent.change(screen.getByRole("spinbutton", { name: "Ember Squire copies" }), { target: { value: "5" } });
  expect(screen.getByText("9 cards", { exact: true })).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Bot deck recipe" }));
  expect(screen.getByRole("spinbutton", { name: "Ember Squire copies" })).toHaveValue(4);
  fireEvent.change(screen.getByRole("spinbutton", { name: "Spark Jolt copies" }), { target: { value: "3" } });
  expect(screen.getByText("7 cards", { exact: true })).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Your deck recipe" }));
  expect(screen.getByRole("spinbutton", { name: "Ember Squire copies" })).toHaveValue(5);
  expect(screen.getByRole("spinbutton", { name: "Spark Jolt copies" })).toHaveValue(4);
});

it("removes and re-adds a Card while recognizing the restored system counts", () => {
  render(<SystemRecipe />);
  fireEvent.change(screen.getByRole("spinbutton", { name: "Ember Squire copies" }), { target: { value: "0" } });
  expect(screen.getByRole("combobox", { name: "Base recipe" })).toHaveValue("custom");
  expect(screen.getByRole("button", { name: "Remove one Ember Squire" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Add one Ember Squire" }));
  expect(screen.getByRole("spinbutton", { name: "Ember Squire copies" })).toHaveValue(1);
  fireEvent.change(screen.getByRole("spinbutton", { name: "Ember Squire copies" }), { target: { value: "4" } });
  expect(screen.getByRole("combobox", { name: "Base recipe" })).toHaveValue("starter-sample");
  expect(screen.getByText("8 cards", { exact: true })).toBeVisible();
});

it("searches Cards and keeps an out-of-range draft count for validation", () => {
  render(<Empty />);
  fireEvent.change(screen.getByRole("searchbox", { name: "Find a card" }), { target: { value: "ember" } });
  expect(screen.queryByRole("spinbutton", { name: "Spark Jolt copies" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByRole("spinbutton", { name: "Ember Squire copies" }), { target: { value: "31" } });
  expect(screen.getByRole("spinbutton", { name: "Ember Squire copies" })).toHaveValue(31);
  expect(screen.getByText("31 cards", { exact: true })).toBeVisible();
});
