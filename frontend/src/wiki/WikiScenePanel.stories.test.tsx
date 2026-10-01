// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, screen, within } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { renderStory } from "../storybook/renderStory";
import meta, { MovementCards, MovementResponses } from "./WikiScenePanel.stories";

afterEach(() => cleanup());

it("renders the Movement Cards story with partial-entry Unit AP and highlights", () => {
  renderStory(meta, MovementCards);
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  expect(screen.getByRole("heading", { name: "Summon during Movement" })).toBeVisible();
  expectCallout("Hero AP", "3/3");
  expectCallout("Unit AP", "1/2");
  expect(screen.getByRole("button", { name: "q 0, r 0, occupied by your unit" })).toHaveClass("tutorial-highlight");
});

it("renders the response story with a pending summon that resumes Movement", () => {
  renderStory(meta, MovementResponses);
  expectCallout("Stack items", "1");
  expectCallout("Priority", "Opponent");
  expect(screen.queryByRole("button", { name: "q 0, r 0, occupied by your unit" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  expectCallout("Top of stack", "Spark Jolt");
  expectCallout("Priority", "Player");
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  expectCallout("Hero HP", "19/20");
  expectCallout("Stack items", "1");
  fireEvent.click(screen.getByRole("button", { name: "Next" }));
  expect(screen.getByRole("heading", { name: "Movement resumes" })).toBeVisible();
  expectCallout("Phase", "Movement");
  expectCallout("Stack items", "0");
  expectCallout("Hero AP", "3/3");
  expectCallout("Unit AP", "1/2");
  expect(screen.getByRole("button", { name: "q 0, r 0, occupied by your unit" })).toBeVisible();
});

function expectCallout(label: string, value: string) {
  const term = within(screen.getByLabelText("Scene callouts")).getByText(label, { exact: true });
  expect(term.nextElementSibling).toHaveTextContent(value);
}
