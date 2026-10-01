// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { renderStory } from "../storybook/renderStory";
import { TUTORIAL_COMPLETION_STORAGE_KEY } from "../tutorial/tutorialReducer";
import meta, { Intro } from "./TutorialPage.stories";

afterEach(() => {
  cleanup();
  window.localStorage.clear();
});

it("plays the Tutorial story through Movement Cards, entry AP and final Card Play", () => {
  renderStory(meta, Intro);
  expectPlayerBudgets(3);
  continueIntro();
  fireEvent.click(tile("q 0, r 1, occupied by your hero"));
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: /Ember Squire/ }));
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: /Ember Squire/ }));
  expect(tile("q 0, r 0, empty hex")).toHaveClass(/\blegal\b/);
  expect(tile("q 0, r 0, empty hex")).toHaveClass("tutorial-highlight");
  fireEvent.click(tile("q 0, r 0, empty hex"));
  expectPlayerBudgets(2);
  expect(document.querySelector('[data-tutorial-target="tutorial-phase"]')).toHaveTextContent("Opponent priority");
  expect(screen.getByRole("button", { name: "End Turn" })).toBeDisabled();
  expect(screen.queryByRole("button", { name: "q 0, r 0, occupied by your unit" })).not.toBeInTheDocument();

  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "Let Opponent Pass" }));
  expect(tile("q 0, r 0, occupied by your unit").querySelector(".piece-token-stat-row > span:nth-of-type(3)")).toHaveTextContent("1");
  expect(screen.getByText("Movement Phase")).toBeVisible();
  continueIntro();
  fireEvent.click(tile("q 0, r 0, occupied by your unit"));
  expect(tile("q 1, r 0, empty hex")).toHaveClass(/\blegal\b/);
  fireEvent.click(tile("q 1, r 0, empty hex"));
  expect(tile("q 0, r 0, occupied by your unit").querySelector(".piece-token-stat-row > span:nth-of-type(3)")).toHaveTextContent("0");
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "Let Opponent Pass" }));
  expect(tile("q 1, r 0, occupied by your unit").querySelector(".piece-token-stat-row > span:nth-of-type(3)")).toHaveTextContent("0");
  expectPlayerBudgets(2);
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "Start Attack" }));
  expect(screen.getByText("Attack Phase")).toBeVisible();
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "Finish Attacks" }));
  expect(screen.getByText("Card Play")).toBeVisible();
  expect(screen.queryByRole("button", { name: "Start Attack" })).not.toBeInTheDocument();
  expectPlayerBudgets(2);
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "End Turn" }));
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: /Spark Jolt/ }));
  fireEvent.click(tile("q 1, r -1, occupied by the opponent's unit"));
  expectPlayerBudgets(1);
  expect(window.localStorage.getItem(TUTORIAL_COMPLETION_STORAGE_KEY)).toBeNull();
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "Let Opponent Pass" }));
  expect(screen.queryByRole("button", { name: "q 1, r -1, occupied by the opponent's unit" })).not.toBeInTheDocument();
  continueIntro();
  fireEvent.click(screen.getByRole("button", { name: "Pass Priority" }));
  expect(screen.getByRole("button", { name: "Start Playing" })).toBeVisible();
  expect(window.localStorage.getItem(TUTORIAL_COMPLETION_STORAGE_KEY)).toBe("true");
});

function continueIntro() {
  fireEvent.click(screen.getByRole("button", { name: "Continue" }));
}

function tile(name: string) {
  return screen.getByRole("button", { name });
}

function expectPlayerBudgets(mana: number) {
  expect(document.querySelector(".player-badge.player > span:nth-of-type(3)")).toHaveTextContent("3/3");
  expect(document.querySelector(".player-badge.player > span:nth-of-type(4)")).toHaveTextContent(`${mana}/3`);
}
