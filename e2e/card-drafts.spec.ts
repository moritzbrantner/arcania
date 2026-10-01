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

async function mockDraftApi(page: Page) {
  const created: typeof officialDefinition[] = [];
  const state = { drafts: [existingDraft], created, accountId: 1, failLoad: false, failSource: false, failCreate: false };
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
    } else if (path === "/api/card-drafts") {
      expect(request.headers().authorization).toBe(state.accountId === 1 ? "Bearer draft-owner" : "Bearer draft-other");
      if (request.method() === "POST") {
        if (state.failCreate) {
          await route.fulfill({ status: 503, json: { message: "Draft could not be saved" } });
          return;
        }
        const { definition } = request.postDataJSON();
        state.created.push(definition);
        const draft = { ...existingDraft, id: 11, version: 1, catalogId: "custom-duplicated", definition };
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
