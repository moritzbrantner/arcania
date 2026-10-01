// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "storybook/test";
import { CardButton } from "./Board";
import { emberSquireCard } from "../board.fixtures";
import type { CardVisualIdentity } from "../../matchVisualIdentity";

const visualIdentity: CardVisualIdentity = {
  name: "Ember Squire",
  templateId: "ember-squire",
  kind: emberSquireCard.kind,
  cost: emberSquireCard.cost,
  text: emberSquireCard.text,
  artPath: null,
  artAlt: "",
  rarity: "basic",
  status: "resolved",
  accentClass: "basic",
};

afterEach(() => cleanup());

describe("CardButton", () => {
  it("keeps a Card readable and playable after artwork fails and loads a replacement choice", async () => {
    const onClick = vi.fn();
    const artwork = { ...visualIdentity, artPath: "/failed.svg", artAlt: "Ember Squire art" };
    const cardBefore = structuredClone(emberSquireCard);
    const props = {
      card: emberSquireCard, visualIdentity: artwork, selected: false, disabled: false, onClick,
    };
    const { rerender } = render(<CardButton {...props} />);
    fireEvent.error(screen.getByRole("img", { name: "Ember Squire art" }));
    expect(screen.queryByRole("img", { name: "Ember Squire art" })).not.toBeInTheDocument();
    const button = screen.getByRole("button", { name: /Ember Squire/i });
    expect(button).toHaveTextContent(emberSquireCard.text);
    expect(button).toBeEnabled();
    await userEvent.click(button);
    expect(onClick).toHaveBeenCalledOnce();
    rerender(<CardButton {...props} visualIdentity={{ ...artwork, artPath: "/replacement.svg" }} />);
    expect(screen.getByRole("img", { name: "Ember Squire art" })).toHaveAttribute("src", "/replacement.svg");
    expect(emberSquireCard).toEqual(cardBefore);
  });

  it("keeps unavailable cards focusable while preventing drag submission", () => {
    const onClick = vi.fn();
    const onFocus = vi.fn();
    const onDragStart = vi.fn();

    render(
      <CardButton
        card={emberSquireCard}
        visualIdentity={visualIdentity}
        selected={false}
        disabled={false}
        unavailable
        availabilityReason="Need 1 mana; you have 0."
        onClick={onClick}
        onFocus={onFocus}
        onDragStart={onDragStart}
      />,
    );

    const button = screen.getByRole("button", { name: /Ember Squire/i });
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button).not.toBeDisabled();

    fireEvent.focus(button);
    fireEvent.click(button);
    fireEvent.dragStart(button);

    expect(onFocus).toHaveBeenCalledOnce();
    expect(onClick).toHaveBeenCalledOnce();
    expect(onDragStart).not.toHaveBeenCalled();
  });
});
