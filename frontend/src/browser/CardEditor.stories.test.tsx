// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, cleanup } from "@testing-library/react";
import { afterEach, expect, it } from "vitest";
import { composeStories } from "@storybook/react-vite";
import * as stories from "./CardEditor.stories";

const { Empty } = composeStories(stories);
afterEach(cleanup);
it("creates, edits and removes a custom card through the editor", async () => {
  render(<Empty />);
  fireEvent.change(screen.getByLabelText("Card name"), { target: { value: "Test Guardian" } });
  fireEvent.click(screen.getByRole("button", { name: "Add card" }));
  expect(await screen.findByRole("button", { name: /Test Guardian.*Mana/ })).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Unit armor"), { target: { value: "6" } });
  fireEvent.click(screen.getByRole("button", { name: "Update card" }));
  fireEvent.click(screen.getByRole("button", { name: /Test Guardian.*Mana/ }));
  expect(screen.getByLabelText("Unit armor")).toHaveValue(6);
  fireEvent.click(screen.getByRole("button", { name: "Remove Test Guardian" }));
  expect(screen.getByText(/No custom cards yet/)).toBeInTheDocument();
});

it("keeps a validated advanced effect when returning to the form fields", async () => {
  render(<Empty />);
  fireEvent.click(screen.getByText("Advanced effect editor"));
  const json = await screen.findByLabelText("Card effect JSON");
  fireEvent.change(json, { target: { value: JSON.stringify({ type: "unit", attack: 7, armor: 9, maxAp: 3 }) } });
  fireEvent.click(screen.getByRole("button", { name: "Add card" }));
  await screen.findByRole("button", { name: "Update card" });
  fireEvent.click(screen.getByText("Advanced effect editor"));
  expect(screen.getByLabelText("Unit attack")).toHaveValue(7);
  expect(screen.getByLabelText("Unit armor")).toHaveValue(9);
});
