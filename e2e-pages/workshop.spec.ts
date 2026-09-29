import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem("rune-lanes-board-visual-mode", "2d");
  });
});

test("Pages can edit rules, create a card, play against a bot, and resume without a server", async ({ page }, testInfo) => {
  const apiRequests: string[] = [];
  const failedAssets: string[] = [];
  page.on("request", (request) => { if (new URL(request.url()).pathname.includes("/api/")) { apiRequests.push(request.url()); } });
  page.on("response", (response) => { if (response.status() >= 400) { failedAssets.push(response.url()); } });
  await page.goto("./");
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
