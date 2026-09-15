// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { createBoardProjection } from "../boardProjection";
import type { TargetingIndicator } from "../targetingIndicators";
import { TargetingIndicatorLayer } from "./TargetingIndicatorLayer";

afterEach(() => cleanup());

describe("TargetingIndicatorLayer", () => {
  it("renders visible targeting lines and footprint markers", () => {
    const { container } = render(
      <TargetingIndicatorLayer
        indicators={[
          indicator({
            id: "stack-spell",
            source: { type: "stack", stackItemId: "stack-1", priority: 0 },
            actionType: "spell",
            tone: "damageSpell",
            secondaryFootprintCoords: [{ q: 1, r: 0 }],
          }),
        ]}
        projection={createBoardProjection([
          { coord: { q: 0, r: 0 }, position: { x: 10, y: 20, visible: true } },
          { coord: { q: 1, r: 1 }, position: { x: 80, y: 90, visible: true } },
          { coord: { q: 1, r: 0 }, position: { x: 60, y: 70, visible: true } },
        ])}
      />,
    );

    const line = container.querySelector("[data-targeting-indicator='stack-spell']");
    expect(line).toBeInTheDocument();
    expect(line).toHaveAttribute("data-stack-item-id", "stack-1");
    expect(container.querySelector("[data-targeting-primary='stack-spell']")).toBeInTheDocument();
    expect(container.querySelector("[data-targeting-footprint='stack-spell']")).toBeInTheDocument();
  });

  it("skips indicators whose endpoints are not visible", () => {
    const { container } = render(
      <TargetingIndicatorLayer
        indicators={[indicator({ id: "hidden" })]}
        projection={createBoardProjection([
          { coord: { q: 0, r: 0 }, position: { x: 10, y: 20, visible: true } },
          { coord: { q: 1, r: 1 }, position: { x: 80, y: 90, visible: false } },
        ])}
      />,
    );

    expect(container.querySelector("[data-targeting-indicator]")).not.toBeInTheDocument();
  });
});

function indicator(overrides: Partial<TargetingIndicator>): TargetingIndicator {
  return {
    id: "targeting",
    source: { type: "selection" },
    actionType: "attack",
    tone: "attack",
    label: "Attack",
    sourcePieceId: "player-hero",
    sourceCoord: { q: 0, r: 0 },
    primaryTargetPieceId: "opponent-hero",
    primaryTargetCoord: { q: 1, r: 1 },
    secondaryFootprintCoords: [],
    ...overrides,
  };
}
