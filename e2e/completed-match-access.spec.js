import { expect, test } from "@playwright/test";

const AUTH_TOKEN_STORAGE_KEY = "arcania-auth-token";

test("private completed summary prompts signed-out viewers to sign in and preserves next path", async ({
  page,
}) => {
  await mockCompletedMatchAccessApi(page);

  await page.goto("/matches/arc-private/summary");

  await expect(page.getByText("Sign in to view this match summary")).toBeVisible();
  await page.getByRole("button", { name: "Sign In" }).click();

  await expect(page).toHaveURL(/\/login\?next=%2Fmatches%2Farc-private%2Fsummary$/);
});

test("private completed replay prompts signed-out viewers to sign in and preserves next path", async ({
  page,
}) => {
  await mockCompletedMatchAccessApi(page);

  await page.goto("/matches/arc-private/replay");

  await expect(page.getByText("Sign in to view this replay")).toBeVisible();
  await page.getByRole("button", { name: "Sign In" }).click();

  await expect(page).toHaveURL(/\/login\?next=%2Fmatches%2Farc-private%2Freplay$/);
});

test("shared seat-link summary still loads without account login", async ({ page }) => {
  await mockCompletedMatchAccessApi(page);

  await page.goto("/match/arc-shared/player-seat/summary");

  await expect(page.getByRole("heading", { name: "Victory" })).toBeVisible();
  await expect(page.getByText("Sign in to view this match summary")).toHaveCount(0);
});

async function mockCompletedMatchAccessApi(page) {
  await page.addInitScript(
    ({ key }) => localStorage.removeItem(key),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );

  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/auth/me") {
      await route.fulfill({ status: 401, json: { message: "Sign in to continue." } });
      return;
    }

    if (url.pathname === "/api/preferences") {
      await route.fulfill({ status: 401, json: { message: "Sign in to continue." } });
      return;
    }

    if (url.pathname === "/api/matches/arc-private/summary") {
      await route.fulfill({
        status: 404,
        json: { message: "Match summary for match arc-private was not found" },
      });
      return;
    }

    if (url.pathname === "/api/matches/arc-private/replay") {
      await route.fulfill({
        status: 404,
        json: { message: "Replay for match arc-private was not found" },
      });
      return;
    }

    if (url.pathname === "/api/shared-matches/arc-shared/seats/player-seat/summary") {
      await route.fulfill({ json: summaryResponse("arc-shared", "shared") });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

function summaryResponse(matchId, mode) {
  return {
    matchId,
    summary: {
      matchId,
      mode,
      createdAt: 1_700_000_000,
      updatedAt: 1_700_000_400,
      round: 4,
      phase: "matchOver",
      winner: "player",
      frameCount: 18,
    },
    viewer: {
      side: "player",
      result: "victory",
    },
    reward: null,
  };
}
