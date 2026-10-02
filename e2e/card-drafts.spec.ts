import { expect, test, type Page } from "@playwright/test";

const officialDefinition = {
  id: "stoneguard", name: "Stoneguard", rarity: "basic", cost: 2,
  text: "A steady defender.", kind: { type: "unit", attack: 1, armor: 4, maxAp: 2 },
  taxonomy: { faction: "earth-clan", element: "earth", traits: ["armored"], families: ["guards"] },
};
const existingDraft = {
  id: 10, version: 2, catalogId: "custom-existing", sourceRevision: null,
  definition: { ...officialDefinition, id: "custom-local", name: "Existing Guardian", cost: 5 },
  validationErrors: [], createdAt: 1, updatedAt: 2,
};

test("Account Draft browsing duplicates the complete official definition and reloads persisted content", async ({ page }, testInfo) => {
  const state = await mockDraftApi(page);
  await page.goto("/workshop/cards");
  await expect(page.getByRole("heading", { name: "Card Drafts", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Select Existing Guardian" }).click();
  const details = page.getByRole("region", { name: "Draft details" });
  await expect(details.getByRole("heading", { name: "Existing Guardian" })).toBeVisible();
  await expect(details.getByText("5 Mana", { exact: true })).toBeVisible();
  await page.getByLabel("Official Card").selectOption("stoneguard");
  await page.getByRole("button", { name: "Duplicate into Draft" }).click();
  await expect(details.getByRole("heading", { name: "Stoneguard", exact: true })).toBeVisible();
  expect(state.created).toHaveLength(1);
  expect(state.created[0]).toEqual({ ...officialDefinition, id: expect.stringMatching(/^custom-[a-f0-9-]+$/) });
  await page.reload();
  await expect(page.getByRole("button", { name: "Select Stoneguard" })).toBeVisible();
  await page.getByRole("button", { name: "Select Stoneguard" }).click();
  await expect(details.getByText("2 Mana", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Select Existing Guardian" }).click();
  await expect(details.getByRole("heading", { name: "Existing Guardian" })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("card-drafts-desktop.png"), fullPage: true });
});

test("Card Draft browsing fits a narrow viewport and exposes the Account navigation", async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await mockDraftApi(page);
  await page.goto("/workshop/cards");
  await page.getByRole("button", { name: "Select Existing Guardian" }).click();
  await expect(page.getByRole("region", { name: "Draft details" }).getByText("5 Mana", { exact: true })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  await page.getByRole("button", { name: "Open navigation menu" }).click();
  await page.getByRole("dialog", { name: "Navigation menu" }).getByRole("button", { name: "Card Drafts", exact: true }).click();
  await expect(page).toHaveURL(/\/workshop\/cards$/);
  await expect(page.getByRole("dialog", { name: "Navigation menu" })).toHaveCount(0);
  await page.screenshot({ path: testInfo.outputPath("card-drafts-mobile.png"), fullPage: true });
});

test("Draft browsing recovers from list, source and create failures without losing selection", async ({ page }) => {
  const state = await mockDraftApi(page);
  state.failLoad = true;
  await page.goto("/workshop/cards");
  await expect(page.getByRole("alert")).toContainText("Draft list temporarily unavailable");
  state.failLoad = false;
  await page.getByRole("button", { name: "Retry loading" }).click();
  const details = page.getByRole("region", { name: "Draft details" });
  await expect(details.getByRole("heading", { name: "Existing Guardian" })).toBeVisible();
  state.failSource = true;
  await page.getByRole("button", { name: "Duplicate into Draft" }).click();
  await expect(page.getByRole("alert")).toContainText("Official Card temporarily unavailable");
  state.failSource = false;
  expect(state.created).toHaveLength(0);
  await expect(details.getByRole("heading", { name: "Existing Guardian" })).toBeVisible();
  state.failCreate = true;
  await page.getByRole("button", { name: "Duplicate into Draft" }).click();
  await expect(page.getByRole("alert")).toContainText("Draft could not be saved");
  state.failCreate = false;
  expect(state.created).toHaveLength(0);
  await expect(details.getByRole("heading", { name: "Existing Guardian" })).toBeVisible();
  await page.getByRole("button", { name: "Duplicate into Draft" }).click();
  await expect(details.getByRole("heading", { name: "Stoneguard", exact: true })).toBeVisible();
  expect(state.created).toHaveLength(1);
});

test("opening Drafts as another Account clears the previous Account content", async ({ page }) => {
  const state = await mockDraftApi(page);
  await page.goto("/workshop/cards");
  await expect(page.getByRole("button", { name: "Select Existing Guardian" })).toBeVisible();
  state.accountId = 2;
  await page.evaluate(() => localStorage.setItem("rune-lanes-auth-token", "draft-other"));
  await page.reload();
  await expect(page.getByText("No Card Drafts yet. Duplicate an official Card to get started.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Select Existing Guardian" })).toHaveCount(0);
  await expect(page.getByRole("region", { name: "Draft details" })).toContainText("Select a Draft");
});

test("duplicating an official Unit, editing metadata and Unit stats, then saving reloads the exact complete definition", async ({ page }) => {
  const state = await mockDraftApi(page);
  await page.goto("/workshop/cards");
  await page.getByLabel("Official Card").selectOption("stoneguard");
  await page.getByRole("button", { name: "Duplicate into Draft" }).click();
  const details = page.getByRole("region", { name: "Draft details" });
  await expect(details.getByRole("heading", { name: "Stoneguard", exact: true })).toBeVisible();
  await details.getByLabel("Card name").fill("Stoneguard Captain");
  await details.getByLabel("Rarity").selectOption("rare");
  await details.getByLabel("Mana cost").fill("7");
  await details.getByLabel("Unit attack").fill("13");
  await details.getByLabel("Unit armor").fill("1000");
  await details.getByLabel("Unit AP").fill("255");
  await expect(page.getByRole("button", { name: "Select Stoneguard", exact: true })).toContainText("Unsaved changes");
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("status")).toHaveText("Draft saved (version 2).");
  const expected = {
    ...officialDefinition, id: state.created[0].id, name: "Stoneguard Captain", rarity: "rare", cost: 7,
    kind: { type: "unit", attack: 13, armor: 1000, maxAp: 255 },
  };
  expect(state.updates).toEqual([{ draftId: 11, version: 1, definition: expected }]);
  await page.reload();
  await page.getByRole("button", { name: "Select Stoneguard Captain" }).click();
  await expect(details.getByRole("heading", { name: "Stoneguard Captain" })).toBeVisible();
  await expect(details.getByText("7 Mana", { exact: true })).toBeVisible();
  await expect(details.getByLabel("Rarity")).toHaveValue("rare");
  await expect(details.getByLabel("Unit attack")).toHaveValue("13");
  await expect(details.getByLabel("Unit armor")).toHaveValue("1000");
  await expect(details.getByLabel("Unit AP")).toHaveValue("255");
  await expect(details.getByRole("button", { name: "Save Draft" })).toBeDisabled();
});

test("saving shows stored validation diagnostics and keeps edits through request failures and stale versions", async ({ page }) => {
  const state = await mockDraftApi(page);
  await page.goto("/workshop/cards");
  const details = page.getByRole("region", { name: "Draft details" });
  await expect(details.getByRole("heading", { name: "Existing Guardian" })).toBeVisible();

  await details.getByLabel("Unit armor").fill("0");
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("alert")).toContainText("armor must be positive (saved value: 0)");
  await expect(details.getByLabel("Unit armor")).toHaveValue("0");

  await details.getByLabel("Unit armor").fill("");
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("alert").filter({ hasText: "Unit armor must be a whole number" })).toBeVisible();
  expect(state.updates).toHaveLength(1);

  await details.getByLabel("Unit armor").fill("6");
  await details.getByLabel("Card name").fill("Renamed Guardian");
  state.failUpdate = true;
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("alert").filter({ hasText: "Draft storage temporarily unavailable" })).toBeVisible();
  await expect(details.getByLabel("Card name")).toHaveValue("Renamed Guardian");
  await expect(details.getByLabel("Unit armor")).toHaveValue("6");
  state.failUpdate = false;

  // Another tab saved the Draft in the meantime.
  Object.assign(state.drafts[0], { version: 9, definition: { ...state.drafts[0].definition, cost: 9 } });
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("alert").filter({ hasText: "current version is 9" })).toBeVisible();
  await expect(details.getByLabel("Card name")).toHaveValue("Renamed Guardian");
  await details.getByRole("button", { name: "Load latest saved version" }).click();
  await expect(details.getByRole("status")).toContainText("Loaded saved version 9");
  await expect(details.getByText("9 Mana", { exact: true })).toBeVisible();
  await expect(details.getByLabel("Card name")).toHaveValue("Renamed Guardian");
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("status")).toHaveText("Draft saved (version 10).");
  expect(state.updates.at(-1)).toEqual({ draftId: 10, version: 9, definition: { ...existingDraft.definition, name: "Renamed Guardian", kind: { ...existingDraft.definition.kind, armor: 6 } } });
  await expect(details.getByText("Draft validation passed.")).toBeVisible();
  await expect(details.getByText("5 Mana", { exact: true })).toBeVisible();
});

test("saving metadata of a Spell Draft preserves its untouched mechanics and taxonomy", async ({ page }) => {
  const spellDefinition = {
    id: "custom-spell", name: "Ember Volley", rarity: "advanced", cost: 3, text: "Burns a target.",
    kind: { type: "spell", range: 3, priority: 4, effect: { type: "damage", amount: 2 } },
    taxonomy: { element: "fire", traits: ["ranged"] },
  };
  const state = await mockDraftApi(page);
  state.drafts.push({ ...structuredClone(existingDraft), id: 12, definition: spellDefinition as never });
  await page.goto("/workshop/cards");
  await page.getByRole("button", { name: "Select Ember Volley" }).click();
  const details = page.getByRole("region", { name: "Draft details" });
  await expect(details.getByLabel("Unit attack")).toHaveCount(0);
  await details.getByLabel("Card name").fill("Ember Storm");
  await details.getByLabel("Mana cost").fill("4");
  await details.getByRole("button", { name: "Save Draft" }).click();
  await expect(details.getByRole("status")).toHaveText("Draft saved (version 3).");
  expect(state.updates).toEqual([{ draftId: 12, version: 2, definition: { ...spellDefinition, name: "Ember Storm", cost: 4 } }]);
});

async function mockDraftApi(page: Page) {
  const created: typeof officialDefinition[] = [];
  const updates: { draftId: number; version: number; definition: Record<string, unknown> }[] = [];
  const state = {
    drafts: [structuredClone(existingDraft)] as (typeof existingDraft)[], created, updates,
    accountId: 1, failLoad: false, failSource: false, failCreate: false, failUpdate: false,
  };
  await page.addInitScript(() => {
    if (!localStorage.getItem("rune-lanes-auth-token")) localStorage.setItem("rune-lanes-auth-token", "draft-owner");
  });
  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/auth/me") {
      await route.fulfill({ json: {
        id: state.accountId, handle: "draft-owner", email: "draft-owner@example.com", displayName: "Draft Owner",
        avatar: { symbol: "spark", color: "emerald" }, preferredHeroType: "runekeeper", boardVisualMode: "2d",
        progressionSummary: { totalXp: 0, level: 1, currentLevelXp: 0, nextLevelXp: 100, xpIntoLevel: 0, xpToNextLevel: 100, runeSlots: 1 },
      } });
    } else if (path === "/api/preferences") {
      await route.fulfill({ json: { theme: "system", motion: "system", animationSpeed: "normal", boardScale: "normal", hotkeys: [], updatedAt: null } });
    } else if (path === "/api/catalog/cards") {
      const { taxonomy: _taxonomy, ...catalogDefinition } = officialDefinition;
      await route.fulfill({ json: { cards: [{ ...catalogDefinition, templateId: "stoneguard", copyCount: 4, artKey: "stoneguard", artPath: "/card-art/stoneguard.svg" }] } });
    } else if (path === "/api/card-transfers/stoneguard/1") {
      expect(request.headers().authorization).toBe("Bearer draft-owner");
      if (state.failSource) {
        await route.fulfill({ status: 503, json: { message: "Official Card temporarily unavailable" } });
        return;
      }
      await route.fulfill({ json: { schemaVersion: 1, revision: { id: { cardId: "stoneguard", revision: 1 }, definition: officialDefinition } } });
    } else if (/^\/api\/card-drafts\/\d+$/.test(path)) {
      expect(request.headers().authorization).toBe("Bearer draft-owner");
      const draft = state.drafts.find((entry) => entry.id === Number(path.split("/").pop()));
      if (!draft) {
        await route.fulfill({ status: 404, json: { message: "Card workshop content was not found." } });
      } else if (request.method() === "PATCH") {
        const { version, definition } = request.postDataJSON();
        state.updates.push({ draftId: draft.id, version, definition });
        if (state.failUpdate) {
          await route.fulfill({ status: 503, json: { message: "Draft storage temporarily unavailable" } });
        } else if (version !== draft.version) {
          await route.fulfill({ status: 409, json: { message: `Card draft changed since version ${version}; current version is ${draft.version}.` } });
        } else {
          // Stands in for the core's stored diagnostics; the backend's real validation is covered in Rust.
          const validationErrors = definition.kind.type === "unit" && definition.kind.armor <= 0
            ? [{ code: "nonPositiveValue", field: "kind.armor", value: definition.kind.armor }] : [];
          Object.assign(draft, { version: draft.version + 1, definition, validationErrors, updatedAt: draft.updatedAt + 1 });
          await route.fulfill({ json: draft });
        }
      } else {
        await route.fulfill({ json: draft });
      }
    } else if (path === "/api/card-drafts") {
      expect(request.headers().authorization).toBe(state.accountId === 1 ? "Bearer draft-owner" : "Bearer draft-other");
      if (request.method() === "POST") {
        if (state.failCreate) {
          await route.fulfill({ status: 503, json: { message: "Draft could not be saved" } });
          return;
        }
        const { definition } = request.postDataJSON();
        state.created.push(definition);
        const draft = { ...structuredClone(existingDraft), id: 11, version: 1, catalogId: "custom-duplicated", definition };
        state.drafts.push(draft);
        await route.fulfill({ json: draft });
      } else {
        if (state.failLoad) {
          await route.fulfill({ status: 503, json: { message: "Draft list temporarily unavailable" } });
          return;
        }
        await route.fulfill({ json: { drafts: state.accountId === 1 ? state.drafts : [] } });
      }
    } else {
      await route.fulfill({ status: 404, json: { message: `Unexpected API request: ${path}` } });
    }
  });
  return state;
}
