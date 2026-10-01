// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { userEvent } from "storybook/test";
import { renderStory } from "../../storybook/renderStory";
import * as stories from "./CardButton.stories";

afterEach(() => cleanup());

describe("Card artwork stories", () => {
  it("swaps artwork without changing the Card's displayed mechanics or selected state", async () => {
    renderStory(stories.default, stories.AlternateArtwork);
    const button = screen.getByRole("button", { name: /Ember Squire/i });
    const text = button.textContent;
    await userEvent.click(button);
    expect(button).toHaveClass("selected");
    await userEvent.selectOptions(screen.getByLabelText("Artwork"), "alternate");
    expect(screen.getByRole("img", { name: "Ember Squire art" })).toHaveAttribute("src", "/card-art/stoneguard.svg");
    expect(button.textContent).toBe(text);
    expect(button).toHaveClass("selected");
    fireEvent.error(screen.getByRole("img", { name: "Ember Squire art" }));
    expect(screen.queryByRole("img", { name: "Ember Squire art" })).not.toBeInTheDocument();
    expect(button.textContent).toBe(text);
    await userEvent.selectOptions(screen.getByLabelText("Artwork"), "original");
    expect(screen.getByRole("img", { name: "Ember Squire art" })).toHaveAttribute("src", "/card-art/ember-squire.svg");
  });

  it("keeps missing artwork references readable and selectable", async () => {
    renderStory(stories.default, stories.MissingArtworkReference);
    expect(screen.queryByRole("img", { name: "Ember Squire art" })).not.toBeInTheDocument();
    const button = screen.getByRole("button", { name: /Ember Squire/i });
    expect(button).toBeEnabled();
    expect(button).toHaveTextContent("1 attack / 2 armor / 2 AP.");
    await userEvent.click(button);
    expect(button).toHaveClass("selected");
  });
});
