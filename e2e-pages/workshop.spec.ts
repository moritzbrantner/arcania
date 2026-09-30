import { expect, test } from "@playwright/test";

test.beforeAll(async ({ request }) => {
  const expectedRevision = process.env.ARCANIA_PAGES_REVISION;
  if (expectedRevision) {
    test.setTimeout(120_000);
    // Wait for the CDN to serve this deployment before exercising the public UI.
    await expect.poll(async () => {
      const response = await request.get(`./deployment.json?revision=${expectedRevision}`, { headers: { "Cache-Control": "no-cache" } });
      return response.ok() ? response.json() : null;
    }, { timeout: 90_000 }).toEqual({ revision: expectedRevision });
  }
});

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("rune-lanes-board-visual-mode", "2d");
  });
});

test("Pages can edit rules, create a card, play against a bot, and resume without a server", async ({ page, baseURL }, testInfo) => {
  const apiRequests: string[] = [];
  const failedAssets: string[] = [];
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => { pageErrors.push(error.message); });
  page.on("request", (request) => { if (new URL(request.url()).pathname.includes("/api/")) { apiRequests.push(request.url()); } });
  page.on("response", (response) => {
    // Pages serves 404.html for these client routes. The reload assertions below
    // must still prove that the app loads and restores the saved state.
    const url = new URL(response.url());
    const isPagesFallback = process.env.ARCANIA_PAGES_URL && response.status() === 404
      && response.request().resourceType() === "document"
      && ["workshop", "match/browser-solo"].some((route) => {
        const expected = new URL(route, baseURL);
        return url.origin === expected.origin && url.pathname.replace(/\/$/, "") === expected.pathname;
      });
    if (response.status() >= 400 && !isPagesFallback) { failedAssets.push(response.url()); }
  });
  const entry = await page.goto("./");
  expect(entry?.status()).toBe(200);
  await expect(page.getByRole("button", { name: "Play against bot", exact: true })).toBeVisible();
  await expect(page.getByLabel("Runekeeper 3D preview", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /Pyromancer.*Aggressive/ }).click();
  await expect(page.getByLabel("Pyromancer 3D preview", { exact: true })).toBeVisible();
  await page.getByRole("combobox", { name: "Your deck recipe", exact: true }).selectOption("ember-burn");
  await page.getByRole("button", { name: "Deck recipes", exact: true }).click();
  await expect(page.getByRole("combobox", { name: "Base recipe", exact: true })).toHaveValue("ember-burn");
  await page.getByRole("spinbutton", { name: "Ember Squire copies", exact: true }).fill("3");
  await expect(page.getByText("59 cards", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Save preset", exact: true }).click();
  await page.reload();
  await expect(page.getByRole("spinbutton", { name: "Ember Squire copies", exact: true })).toHaveValue("3");
  await page.getByRole("combobox", { name: "Base recipe", exact: true }).selectOption("balanced-starter");
  await page.screenshot({ path: testInfo.outputPath("workshop-deck-editor.png"), fullPage: true });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.getByRole("button", { name: "Edit match rules", exact: true }).click();
  await page.getByRole("spinbutton", { name: "Base Hero Mana", exact: true }).fill("9");
  await page.getByRole("spinbutton", { name: "Runekeeper HP", exact: true }).fill("35");
  await page.getByRole("button", { name: "Save preset", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("saved");
  await page.reload();
  await expect(page.getByRole("spinbutton", { name: "Base Hero Mana", exact: true })).toHaveValue("9");
  await page.getByRole("button", { name: "Card editor", exact: true }).click();
  await page.getByRole("textbox", { name: "Card name", exact: true }).fill("Moonstone Guardian");
  await page.getByRole("button", { name: "Add card", exact: true }).click();
  await expect(page.getByRole("button", { name: /Moonstone Guardian.*Mana/ })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("card-editor.png"), fullPage: true });
  await page.getByRole("button", { name: "Save & play against bot", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Round 1", exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: /Moonstone Guardian/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Iron Colossus/ })).toBeVisible();
  await page.getByRole("button", { name: "Play Cards", exact: true }).click();
  await page.getByRole("button", { name: /Moonstone Guardian/ }).click();
  // Legal hexes come from the core's command query projection.
  await page.getByRole("button", { name: "q 0, r 2, empty hex", exact: true }).click();
  await expect(page.getByRole("button", { name: "q 0, r 2, occupied by your unit", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "End Turn", exact: true }).click();
  await expect.poll(async () => {
    const pass = page.getByRole("button", { name: "Pass Priority", exact: true });
    if (await pass.count() && await pass.isEnabled()) { await pass.click(); }
    return page.getByRole("heading", { name: "Round 2", exact: true }).isVisible();
  }, { timeout: 30_000 }).toBe(true);
  await page.reload();
  await expect(page.getByRole("heading", { name: "Round 2", exact: true })).toBeVisible();
  await page.screenshot({ path: testInfo.outputPath("match.png"), fullPage: true });
  expect(apiRequests).toEqual([]);
  expect(failedAssets).toEqual([]);
  expect(pageErrors).toEqual([]);
});

test("invalid imported presets are rejected without losing saved rules", async ({ page }) => {
  await page.goto("./workshop?tab=rules");
  await page.getByRole("spinbutton", { name: "Base Hero Mana", exact: true }).fill("7");
  await page.getByRole("button", { name: "Save preset", exact: true }).click();
  await page.getByText("Preset tools & repeatable matches", { exact: true }).click();
  await page.getByLabel("Import preset", { exact: true }).setInputFiles({ name: "broken.json", mimeType: "application/json", buffer: Buffer.from('{"cards":[]}') });
  await expect(page.getByRole("alert")).toContainText("missing field");
  await page.reload();
  await expect(page.getByRole("spinbutton", { name: "Base Hero Mana", exact: true })).toHaveValue("7");
});

test("a custom Spell can win a browser match and return to the workshop", async ({ page }) => {
  await page.goto("./workshop?tab=rules");
  await page.getByRole("spinbutton", { name: "Runekeeper HP", exact: true }).fill("1");
  await page.getByRole("button", { name: "Card editor", exact: true }).click();
  await page.getByRole("textbox", { name: "Card name", exact: true }).fill("Final Spark");
  await page.getByRole("combobox", { name: "Card type", exact: true }).selectOption("spell");
  await page.getByRole("spinbutton", { name: "Spell range", exact: true }).fill("6");
  await page.getByRole("button", { name: "Add card", exact: true }).click();
  await expect(page.getByRole("button", { name: /Final Spark.*Mana/ })).toBeVisible();
  await page.getByRole("button", { name: "Save & play against bot", exact: true }).click();
  await page.getByRole("button", { name: "Play Cards", exact: true }).click();
  await page.getByRole("button", { name: /Final Spark/ }).click();
  await page.getByRole("button", { name: "q 0, r -3, occupied by the opponent's hero", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Victory", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Back to workshop", exact: true }).click();
  await expect(page.getByRole("button", { name: "Play against bot", exact: true })).toBeVisible();
});
