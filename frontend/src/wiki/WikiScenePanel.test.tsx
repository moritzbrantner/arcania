// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { WikiScenePanel } from "./WikiScenePanel";
import { wikiSceneById } from "./wikiScenes";

afterEach(() => cleanup());

describe("WikiScenePanel", () => {
  it("renders a board scene title, board, first step, and callouts", () => {
    const { container } = render(<WikiScenePanel scene={scene("mana-source")} />);

    expect(screen.getByRole("heading", { name: "Mana Source" })).toBeInTheDocument();
    expect(screen.getByRole("region", { name: "Hex board" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Base Mana" })).toBeInTheDocument();
    const callouts = container.querySelector(".wiki-scene-callouts") as HTMLElement;
    expect(within(callouts).getByText("Base Mana")).toBeInTheDocument();
    expect(within(callouts).getByText("3")).toBeInTheDocument();
  });

  it("advances, returns, and resets board scene steps", () => {
    render(<WikiScenePanel scene={scene("mana-source")} />);

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Occupy a Mana Well" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Previous" }));
    expect(screen.getByRole("heading", { name: "Base Mana" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(screen.getByRole("heading", { name: "Base Mana" })).toBeInTheDocument();
  });

  it("applies board highlights to at least one Hex or piece", () => {
    const { container } = render(<WikiScenePanel scene={scene("mana-source")} />);

    expect(container.querySelector(".hex-tile.tutorial-highlight")).toBeInTheDocument();
  });

  it("shows independent budgets through Movement, Attack, and final Card Play", () => {
    render(<WikiScenePanel scene={scene("action-points-budget")} />);
    const next = screen.getByRole("button", { name: "Next" });
    expectCallout("Hero AP", "3/3");
    expectCallout("Mana", "3/3");
    fireEvent.click(next);
    expect(screen.getByRole("heading", { name: "Summon during Movement" })).toBeVisible();
    expectCallout("Hero AP", "3/3");
    expectCallout("Mana", "2/3");
    expectCallout("Unit AP", "1/2");
    fireEvent.click(next);
    expectCallout("Unit AP", "0/2");
    expect(screen.getByRole("button", { name: "q 0, r 1, occupied by your unit" })).toBeVisible();
    fireEvent.click(next);
    expectCallout("Phase", "Attack");
    expectCallout("Proactive Cards", "Unavailable");
    fireEvent.click(next);
    expectCallout("Phase", "Card Play");
    fireEvent.click(next);
    expectCallout("Mana", "1/3");
    expectCallout("Hero AP", "3/3");
    expect(next).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: "Previous" }));
    expectCallout("Mana", "2/3");
    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expectCallout("Phase", "Movement");
    expectCallout("Mana", "3/3");
    expect(screen.getByRole("button", { name: "Previous" })).toBeDisabled();
  });

  it("renders the Deck recipe widget with initial Draft messages", () => {
    render(<WikiScenePanel scene={scene("deck-recipe-legality")} />);

    expect(screen.getByText("Draft deck recipe")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: /Deck Recipe Legality interactive scene/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Increase Basic cards" })).toBeInTheDocument();
    expect(screen.getByText("Deck recipe needs at least 60 cards.")).toBeInTheDocument();
  });

  it("updates Deck recipe counts to legal and resets", () => {
    render(<WikiScenePanel scene={scene("deck-recipe-legality")} />);

    fireEvent.click(screen.getByRole("button", { name: "Increase Basic cards" }));
    expect(screen.getByText("Legal deck recipe")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Reset" }));
    expect(screen.getByText("Draft deck recipe")).toBeInTheDocument();
  });
});

function expectCallout(label: string, value: string) {
  const term = within(screen.getByLabelText("Scene callouts")).getByText(label, { exact: true });
  expect(term.nextElementSibling).toHaveTextContent(value);
}

function scene(id: string) {
  const resolved = wikiSceneById(id);
  if (!resolved) {
    throw new Error(`Missing scene ${id}`);
  }
  return resolved;
}
