// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { act, cleanup, screen, within } from "@testing-library/react";
import { userEvent } from "storybook/test";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { catalogCards } from "../components/board.fixtures";
import { renderStory } from "../storybook/renderStory";
import meta, { Ready } from "./CardDraftsPage.stories";
import { storyCardDrafts } from "./CardDraftsPage.fixtures";

beforeEach(() => {
  vi.stubGlobal("fetch", vi.fn(async (input: RequestInfo | URL) => {
    if (input === "/api/card-drafts") return Response.json({ drafts: storyCardDrafts });
    if (input === "/api/catalog/cards") return Response.json({ cards: catalogCards });
    return Response.json({ message: "Unexpected request" }, { status: 404 });
  }));
});
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

it("shows saved Draft details and the core's validation diagnostics in the Ready story", async () => {
  const user = userEvent.setup();
  renderStory(meta, Ready);
  await user.click(await screen.findByRole("button", { name: "Select Granite Draft" }));
  const details = within(screen.getByRole("region", { name: "Draft details" }));
  expect(details.getByRole("heading", { name: "Granite Draft" })).toBeVisible();
  expect(details.getByText("7 Mana", { exact: true })).toBeVisible();
  expect(details.getByRole("alert")).toHaveTextContent("armor cannot be negative (saved value: -1)");
});

it("leaving the Draft browser before source loading completes prevents a later create request", async () => {
  const user = userEvent.setup();
  let resolveSource: (response: Response) => void = () => { throw new Error("Source request was not started"); };
  const source = new Promise<Response>((resolve) => { resolveSource = resolve; });
  const fetch = vi.mocked(window.fetch);
  fetch.mockImplementationOnce(async () => Response.json({ drafts: storyCardDrafts }));
  fetch.mockImplementationOnce(async () => Response.json({ cards: catalogCards }));
  const view = renderStory(meta, Ready);
  const duplicate = await screen.findByRole("button", { name: "Duplicate into Draft" });
  expect(duplicate).toBeEnabled();
  fetch.mockImplementationOnce(() => source);
  fetch.mockImplementationOnce(async () => Response.json(storyCardDrafts[0]));
  await user.click(duplicate);
  expect(fetch.mock.calls.some(([input]) => typeof input === "string" && input.startsWith("/api/card-transfers/"))).toBe(true);
  view.unmount();
  await act(async () => {
    resolveSource(Response.json({ schemaVersion: 1, revision: { id: { cardId: "ember-squire", revision: 1 }, definition: storyCardDrafts[0].definition } }));
  });
  expect(fetch.mock.calls.filter(([input, init]) => input === "/api/card-drafts" && init?.method === "POST")).toHaveLength(0);
});

it("keeps unsaved edits per Draft and blocks saving values the Draft API cannot represent", async () => {
  const user = userEvent.setup();
  renderStory(meta, Ready);
  const details = within(await screen.findByRole("region", { name: "Draft details" }));
  const cost = await details.findByLabelText("Mana cost");
  await user.clear(cost);
  await user.type(cost, "256");
  expect(screen.getByRole("button", { name: "Select Ember Draft" })).toHaveTextContent("Unsaved changes");
  await user.click(details.getByRole("button", { name: "Save Draft" }));
  expect(details.getByRole("alert")).toHaveTextContent("Mana cost must be a whole number from 0 to 255.");
  await user.click(screen.getByRole("button", { name: "Select Granite Draft" }));
  await user.click(screen.getByRole("button", { name: "Select Ember Draft" }));
  expect(within(screen.getByRole("region", { name: "Draft details" })).getByLabelText("Mana cost")).toHaveValue(256);
  expect(vi.mocked(window.fetch).mock.calls.some(([, init]) => init?.method === "PATCH")).toBe(false);
});
