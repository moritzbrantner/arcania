import { expect, test } from "@playwright/test";

const MATCH_ID = "e2e-ai";
const BOARD_VISUAL_MODE_STORAGE_KEY = "arcania-board-visual-mode";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "2d"),
    { key: BOARD_VISUAL_MODE_STORAGE_KEY },
  );
});

test("plays replay frames from one AI action one at a time", async ({ page }) => {
  let match = matchAt({ q: 0, r: -3 }, "player", "cardPlay");

  await mockMatchApi(page, async (action) => {
    if (action.type === "endTurn") {
      match = matchAt({ q: 0, r: -3 }, "opponent");
      return matchResponse(match);
    }

    if (action.type === "advanceAi") {
      const firstMove = matchAt({ q: 0, r: -2 }, "opponent");
      const secondMove = matchAt({ q: 0, r: -1 }, "player");
      match = secondMove;

      return matchResponse(secondMove, [
        replayFrame(0, firstMove, {
          type: "pieceMoved",
          side: "opponent",
          pieceId: "opponent-hero",
          from: { q: 0, r: -3 },
          to: { q: 0, r: -2 },
        }),
        replayFrame(1, secondMove, {
          type: "pieceMoved",
          side: "opponent",
          pieceId: "opponent-hero",
          from: { q: 0, r: -2 },
          to: { q: 0, r: -1 },
        }),
      ]);
    }

    throw new Error(`Unexpected action ${action.type}`);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(opponentHeroAt(page, { q: 0, r: -3 })).toBeVisible();

  const advanceResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/matches/${MATCH_ID}/actions`) &&
      response.request().postData()?.includes("advanceAi") === true,
  );
  await page.getByRole("button", { name: /End Turn/ }).click();
  await advanceResponse;

  await page.waitForTimeout(100);
  await expect(opponentHeroAt(page, { q: 0, r: -3 })).toBeVisible();
  await expect(opponentHeroAt(page, { q: 0, r: -2 })).toBeHidden();

  await page.waitForTimeout(1_000);
  await expect(opponentHeroAt(page, { q: 0, r: -2 })).toBeVisible();
  await expect(opponentHeroAt(page, { q: 0, r: -1 })).toBeHidden();

  await page.waitForTimeout(1_000);
  await expect(opponentHeroAt(page, { q: 0, r: -1 })).toBeVisible();
});

test("does not start the next AI action until current playback finishes", async ({ page }) => {
  let match = matchAt({ q: 0, r: -3 }, "player", "cardPlay");
  let advanceCount = 0;
  let secondAdvanceRequested = false;

  await mockMatchApi(page, async (action) => {
    if (action.type === "endTurn") {
      match = matchAt({ q: 0, r: -3 }, "opponent");
      return matchResponse(match);
    }

    if (action.type !== "advanceAi") {
      throw new Error(`Unexpected action ${action.type}`);
    }

    advanceCount += 1;

    if (advanceCount === 1) {
      const firstMove = matchAt({ q: 0, r: -2 }, "opponent");
      const secondMove = matchAt({ q: 0, r: -1 }, "opponent");
      match = secondMove;

      return matchResponse(secondMove, [
        replayFrame(0, firstMove, {
          type: "pieceMoved",
          side: "opponent",
          pieceId: "opponent-hero",
          from: { q: 0, r: -3 },
          to: { q: 0, r: -2 },
        }),
        replayFrame(1, secondMove, {
          type: "pieceMoved",
          side: "opponent",
          pieceId: "opponent-hero",
          from: { q: 0, r: -2 },
          to: { q: 0, r: -1 },
        }),
      ]);
    }

    secondAdvanceRequested = true;
    match = matchAt({ q: 0, r: -1 }, "player");
    return matchResponse(match);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  const firstAdvanceResponse = page.waitForResponse(
    (response) =>
      response.url().endsWith(`/api/matches/${MATCH_ID}/actions`) &&
      response.request().postData()?.includes("advanceAi") === true,
  );

  await page.getByRole("button", { name: /End Turn/ }).click();
  await firstAdvanceResponse;

  await page.waitForTimeout(1_500);
  expect(secondAdvanceRequested).toBe(false);
  await expect(opponentHeroAt(page, { q: 0, r: -2 })).toBeVisible();

  await expect
    .poll(() => secondAdvanceRequested, { timeout: 2_500 })
    .toBe(true);
});

async function mockMatchApi(
  page,
  handleAction,
  currentMatch,
) {
  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/catalog/cards") {
      await route.fulfill({ json: { cards: [] } });
      return;
    }

    if (url.pathname === `/api/matches/${MATCH_ID}` && request.method() === "GET") {
      await route.fulfill({ json: matchResponse(currentMatch()) });
      return;
    }

    if (url.pathname === `/api/matches/${MATCH_ID}/actions` && request.method() === "POST") {
      const action = JSON.parse(request.postData() ?? "{}");
      await route.fulfill({ json: await handleAction(action) });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

function matchResponse(matchState, replayFrames = []) {
  return {
    matchId: MATCH_ID,
    matchState,
    replayFrames,
  };
}

function replayFrame(
  frameIndex,
  matchState,
  event,
) {
  return {
    frameIndex,
    actionIndex: 1,
    event,
    matchState,
  };
}

function opponentHeroAt(page, coord) {
  return page.getByRole("button", {
    name: `q ${coord.q}, r ${coord.r}, occupied by the opponent's hero`,
  });
}

function matchAt(opponentPosition, activeSide, phase = "movement") {
  return {
    mode: "solo",
    round: 1,
    phase,
    activeSide,
    prioritySide: null,
    player: {
      side: "player",
      mana: 2,
      maxMana: 2,
      hero: {
        id: "player-hero",
        side: "player",
        heroType: "runekeeper",
        hp: 20,
        maxHp: 20,
        attack: 1,
        attackRange: 1,
        position: { q: 0, r: 3 },
        apRemaining: activeSide === "player" ? 3 : 0,
        maxAp: 3,
        hasAttacked: false,
      },
      hand: [],
      deckCount: 40,
      discardCount: 0,
    },
    opponent: {
      side: "opponent",
      mana: 2,
      maxMana: 2,
      hero: {
        id: "opponent-hero",
        side: "opponent",
        heroType: "pyromancer",
        hp: 20,
        maxHp: 20,
        attack: 1,
        attackRange: 1,
        position: opponentPosition,
        apRemaining: activeSide === "opponent" ? 3 : 0,
        maxAp: 3,
        hasAttacked: false,
      },
      deckCount: 40,
      discardCount: 0,
    },
    board: {
      radius: 3,
      tiles: radiusThreeTiles(),
      units: [],
      droppedItems: [],
    },
    actionStack: [],
    log: [],
    winner: null,
  };
}

function radiusThreeTiles() {
  const tiles = [];
  for (let q = -3; q <= 3; q += 1) {
    const minR = Math.max(-3, -q - 3);
    const maxR = Math.min(3, -q + 3);
    for (let r = minR; r <= maxR; r += 1) {
      tiles.push({ coord: { q, r } });
    }
  }
  return tiles;
}
