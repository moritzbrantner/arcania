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
  await page.screenshot({ path: testInfo.outputPath("workshop.png"), fullPage: true });
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
  await page.getByRole("button", { name: "Start Attack", exact: true }).click();
  await page.getByRole("button", { name: "Finish Attacks", exact: true }).click();
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

test("imported independent deck recipes drive the match and its saved command journal", async ({ page }) => {
  const apiRequests: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).pathname.includes("/api/")) { apiRequests.push(request.url()); }
  });
  await page.goto("./workshop?tab=rules");
  await page.getByRole("button", { name: "Save preset", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("saved");
  const stored: unknown = await page.evaluate(() => JSON.parse(localStorage.getItem("rune-lanes.workshop.v1") ?? "null"));
  if (typeof stored !== "object" || stored === null || Array.isArray(stored)) {
    throw new Error("The Workshop must save a preset before recipe import.");
  }
  const playerDeckRecipe = [{ templateId: "ember-squire", count: 20 }];
  const opponentDeckRecipe = [{ templateId: "spark-jolt", count: 20 }];
  await page.getByText("Preset tools & repeatable matches", { exact: true }).click();
  await page.getByLabel("Import preset", { exact: true }).setInputFiles({
    name: "independent-decks.json", mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify({ ...stored, playerDeckRecipe, opponentDeckRecipe })),
  });
  await expect(page.getByRole("status")).toContainText("imported");
  await page.getByRole("button", { name: "Save & play against bot", exact: true }).click();
  const hand = page.getByLabel("Hand", { exact: true });
  await expect(hand.getByRole("button", { name: /Ember Squire/ })).toHaveCount(7);
  await expect(hand.getByRole("button", { name: /Spark Jolt/ })).toHaveCount(0);
  await page.getByRole("button", { name: "Start Attack", exact: true }).click();
  const journal: unknown = await page.evaluate(() => JSON.parse(localStorage.getItem("rune-lanes.browser-match.v1") ?? "null"));
  expect(journal).toMatchObject({ setup: { playerDeckRecipe, opponentDeckRecipe }, commands: [{ command: { type: "startAttackPhase" } }] });
  await page.reload();
  await expect(page.getByRole("button", { name: "Finish Attacks", exact: true })).toBeVisible();
  await expect(hand.getByRole("button", { name: /Ember Squire/ })).toHaveCount(7);
  expect(apiRequests).toEqual([]);
});

test("summons Ember Squire in Movement, moves it, and advances through Attack and Card Play", async ({ page }) => {
  await page.goto("./workshop");
  await page.getByText("Preset tools & repeatable matches", { exact: true }).click();
  await page.getByRole("spinbutton", { name: "Match seed", exact: true }).fill("1");
  await page.getByRole("button", { name: "Play against bot", exact: true }).click();
  await expect(page.getByRole("button", { name: "Play Cards", exact: true })).toHaveCount(0);
  const card = page.getByLabel("Hand", { exact: true }).getByRole("button", { name: /Ember Squire/ });
  await expect(card).toBeEnabled();
  await card.click();
  const summon = page.getByRole("button", { name: "q 0, r 2, empty hex", exact: true });
  await expect(summon).toHaveClass(/\blegal\b/);
  await summon.click();
  await expect(page.getByRole("region", { name: "Turn checklist" })).toContainText("3/3");
  await page.getByRole("button", { name: "q 0, r 2, occupied by your unit", exact: true }).click();
  const move = page.getByRole("button", { name: "q 0, r 1, empty hex", exact: true });
  await expect(move).toHaveClass(/\blegal\b/);
  await move.click();
  await expect(page.getByRole("button", { name: "q 0, r 1, occupied by your unit", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Start Attack", exact: true }).click();
  await expect(page.getByRole("button", { name: "Start Attack", exact: true })).toHaveCount(0);
  await expect(page.getByLabel("Hand", { exact: true }).getByRole("button", { name: /Prism Initiate/ })).toBeDisabled();
  await page.getByRole("button", { name: "Finish Attacks", exact: true }).click();
  await expect(page.getByRole("button", { name: "End Turn", exact: true })).toBeVisible();
  await expect(page.getByLabel("Hand", { exact: true }).getByRole("button", { name: /Prism Initiate/ })).toBeEnabled();
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
  await page.getByRole("button", { name: "Start Attack", exact: true }).click();
  await page.getByRole("button", { name: "Finish Attacks", exact: true }).click();
  await page.getByRole("button", { name: /Final Spark/ }).click();
  await page.getByRole("button", { name: "q 0, r -3, occupied by the opponent's hero", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Victory", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Back to workshop", exact: true }).click();
  await expect(page.getByRole("button", { name: "Play against bot", exact: true })).toBeVisible();
});
