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

  const matchRequests: string[] = [];
  page.on("request", (request) => {
    if (new URL(request.url()).pathname.startsWith("/api/matches")) matchRequests.push(request.url());
  });
  await page.goto("/tutorial");
  await expectPlayerBudgets(page, 3);
  await expect(page.locator('[data-tutorial-target="tutorial-phase"]')).toHaveText("Movement Phase");
  await continueIntro(page);
  await page.getByRole("button", { name: /q 0, r 1, occupied by your hero/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /Ember Squire/i }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /Ember Squire/i }).click();
  const summonHex = page.getByRole("button", { name: /q 0, r 0, empty hex/i });
  await expect(summonHex).toHaveClass(/\blegal\b/);
  await expect(summonHex).toHaveClass(/tutorial-highlight/);
  await page.getByRole("button", { name: /Ember Squire/i }).dragTo(summonHex);
  await expectPlayerBudgets(page, 2);
  await expect(page.locator('[data-tutorial-target="tutorial-phase"]')).toHaveText("Opponent priority");
  await expect(page.getByRole("button", { name: /q 0, r 0, occupied by your unit/i })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "End Turn" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Start Attack" })).toHaveCount(0);

  await continueIntro(page);
  await page.getByRole("button", { name: "Let Opponent Pass" }).click();
  await expect(page.locator('[data-tutorial-target="tutorial-phase"]')).toHaveText("Movement Phase");
  await expect(page.getByRole("button", { name: /q 0, r 0, occupied by your unit/i }).locator(".piece-token-stat-row > span").nth(2)).toHaveText("1");

  await continueIntro(page);
  await page.getByRole("button", { name: /q 0, r 0, occupied by your unit/i }).click();
  const moveHex = page.getByRole("button", { name: /q 1, r 0, empty hex/i });
  await expect(moveHex).toHaveClass(/\blegal\b/);
  await expect(moveHex).toHaveClass(/tutorial-highlight/);
  await moveHex.click();
  await expect(page.getByRole("button", { name: /q 0, r 0, occupied by your unit/i }).locator(".piece-token-stat-row > span").nth(2)).toHaveText("0");
  await continueIntro(page);
  await page.getByRole("button", { name: "Let Opponent Pass" }).click();
  await expect(page.getByRole("button", { name: /q 1, r 0, occupied by your unit/i }).locator(".piece-token-stat-row > span").nth(2)).toHaveText("0");
  await expectPlayerBudgets(page, 2);
  await continueIntro(page);
  await page.getByRole("button", { name: "Start Attack" }).click();
  await expect(page.locator('[data-tutorial-target="tutorial-phase"]')).toHaveText("Attack Phase");
  await continueIntro(page);
  await page.getByRole("button", { name: "Finish Attacks" }).click();
  await expect(page.locator('[data-tutorial-target="tutorial-phase"]')).toHaveText("Card Play");
  await expectPlayerBudgets(page, 2);
  await expect(page.getByRole("button", { name: "Start Attack" })).toHaveCount(0);

  await continueIntro(page);
  await page.getByRole("button", { name: "End Turn" }).click();

  await continueIntro(page);
  await page.getByRole("button", { name: /Spark Jolt/i }).click();
  const responseTarget = page.getByRole("button", { name: /q 1, r -1, occupied by the opponent's unit/i });
  await expect(responseTarget).toHaveClass(/\blegal\b/);
  await responseTarget.click();
  await expectPlayerBudgets(page, 1);
  await expect(page.getByRole("button", { name: "Start Playing" })).toHaveCount(0);
  await continueIntro(page);
  await page.getByRole("button", { name: "Let Opponent Pass" }).click();
  await expect(page.getByRole("button", { name: /q 1, r -1, occupied by the opponent's unit/i })).toHaveCount(0);
  await continueIntro(page);
  await page.getByRole("button", { name: "Pass Priority" }).click();

  await expect(page.getByRole("button", { name: "Start Playing" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Pass Priority" })).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => window.localStorage.getItem("rune-lanes-tutorial-completed"))).toBe("true");
  expect(matchRequests).toEqual([]);
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

async function expectPlayerBudgets(page, mana: number) {
  const stats = page.locator(".player-badge.player > span");
  await expect(stats.nth(2)).toHaveText("3/3");
  await expect(stats.nth(3)).toHaveText(`${mana}/3`);
}

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
