import { expect, test } from "@playwright/test";

test("dashboard and play page expose tutorial mode", async ({ page }) => {
  await mockApi(page);

  await page.goto("/");
  await page.getByRole("button", { name: "Learn & settings" }).click();
  await page.getByRole("menuitem", { name: "Tutorial" }).click();
  await expect(page).toHaveURL(/\/tutorial$/);
  await expect(page.getByRole("heading", { name: "Tutorial" })).toBeVisible();

  await page.goto("/play");
  await expect(page.getByRole("button", { name: "Tutorial" })).toBeVisible();
});

test("runs the tutorial happy path in 2d and stores completion", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("rune-lanes-board-visual-mode", "2d");
  });
  await mockApi(page);

  await page.goto("/tutorial");
  await continueIntro(page);
  await page.getByRole("button", { name: /q 0, r 1, occupied by your hero/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /Ember Squire/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /Ember Squire/i }).click();
  await page.getByRole("button", { name: /q 0, r 0, empty hex/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /q 0, r 0, occupied by your unit/i }).click();
  await page.getByRole("button", { name: /q 1, r 0, empty hex/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /q 1, r 0, occupied by your unit/i }).click();
  await page.getByRole("button", { name: /q 1, r -1, occupied by the opponent's unit/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: "End Turn" }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /Spark Jolt/i }).click();
  await page.getByRole("button", { name: /q 1, r -1, occupied by the opponent's unit/i }).click();

  await expect(page.getByRole("button", { name: "Start Playing" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Pass Priority" })).toHaveCount(0);
  await page.waitForFunction(
    () => window.localStorage.getItem("rune-lanes-tutorial-completed") === "true",
  );
});

test("uses 3d hit target highlights when 3d board mode is selected", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("rune-lanes-board-visual-mode", "3d");
  });
  await mockApi(page);

  await page.goto("/tutorial");

  await expect(page.locator('[data-board-renderer="3d"]').first()).toBeVisible();
  await expect(page.locator(".board-3d-hit-target.tutorial-highlight").first()).toBeVisible();
});

async function continueIntro(page) {
  await page.getByRole("button", { name: "Continue" }).click();
}

async function mockApi(page) {
  await page.route("**://*/api/**", async (route) => {
    const url = new URL(route.request().url());

    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/system-decks") {
      await route.fulfill({
        json: {
          rules: {
            maxDecksPerAccount: 12,
            minCards: 25,
            basicCopyLimit: 12,
            advancedCopyLimit: 3,
            rareCopyLimit: 1,
            advancedTotalLimit: 9,
            rareTotalLimit: 3,
          },
          decks: [
            {
              id: "balanced-starter",
              name: "Balanced Starter",
              heroType: "runekeeper",
              cards: [{ templateId: "ember-squire", count: 12 }],
              legality: {
                legal: true,
                totalCards: 25,
                basicCards: 25,
                advancedCards: 0,
                rareCards: 0,
                messages: [],
              },
            },
          ],
        },
      });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Unexpected request" } });
  });
}
