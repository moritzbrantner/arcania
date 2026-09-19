import { expect, test } from "@playwright/test";

const MATCH_ID = "e2e-3d-board";
const BOARD_VISUAL_MODE_STORAGE_KEY = "arcania-board-visual-mode";
const MATCH_CHROME_STORAGE_KEY = "arcania-match-chrome-minimized";

test.describe.configure({ timeout: 30_000 });

test("renders Solo matches as a full-screen board with persistent collapsible chrome", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await useStoredBoardVisualMode(page, "3d");
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    hand: [sparkJolt()],
    prioritySide: "player",
    actionStack: [
      {
        id: "pending-spell",
        side: "opponent",
        priority: 0,
        action: {
          type: "castSpell",
          card: { id: "pending-card", templateId: "spark-jolt", name: "Spark Jolt" },
          targetId: "player-hero",
        },
      },
    ],
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expectFullViewportBoard(page, { width: 1280, height: 720 });
  await expect.poll(() => hasReady3dCanvas(page)).toBe(true);
  await expect(page.getByText("You", { exact: true })).toBeVisible();
  await expect(page.getByText("Opponent", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Enemy hand, 0 cards")).toBeVisible();
  await expect(page.getByLabel("Hand").getByRole("button", { name: /Spark Jolt/ })).toBeVisible();
  await expectNoVisibleOverlap(page, matchOverlaySelectors());

  await expect(page.getByRole("button", { name: "Pass Priority" })).toBeVisible();
  await page.getByRole("button", { name: "Minimize match chrome" }).click();
  await expect(page.getByRole("button", { name: "Restore match chrome" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Pass Priority" })).toBeVisible();
  await expectNoVisibleOverlap(page, minimizedMatchOverlaySelectors());
  await expect
    .poll(() => page.evaluate((key) => localStorage.getItem(key), MATCH_CHROME_STORAGE_KEY))
    .toBe("true");

  await page.reload();
  await expect(page.getByRole("button", { name: "Restore match chrome" })).toBeVisible();

  await page.setViewportSize({ width: 390, height: 700 });
  await page.reload();
  await expectFullViewportBoard(page, { width: 390, height: 700 });
  await ensureMatchChromeMinimized(page);
  await expectNoVisibleOverlap(page, minimizedMatchOverlaySelectors());
  await expect(page.getByText("You", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Hand", { exact: true })).toBeVisible();
});

test("renders replays as a full-screen board with persistent collapsible chrome and transport controls", async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await useStoredBoardVisualMode(page, "3d");
  await page.addInitScript((key) => localStorage.removeItem(key), MATCH_CHROME_STORAGE_KEY);
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });
  await mockReplayApi(page, match);

  await page.goto(`/matches/${MATCH_ID}/replay`);
  await expectFullViewportBoard(page, { width: 1280, height: 720 });
  await expect.poll(() => hasReady3dCanvas(page)).toBe(true);
  await expect(page.getByText("You", { exact: true })).toBeVisible();
  await expect(page.getByText("Opponent", { exact: true })).toBeVisible();
  await expect(page.getByLabel("Replay frame")).toBeVisible();
  await expectNoVisibleOverlap(page, replayOverlaySelectors());

  await page.getByRole("button", { name: "Minimize match chrome" }).click();
  await expect(page.getByRole("button", { name: "Restore match chrome" })).toBeVisible();
  await expect(page.getByLabel("Replay frame")).toBeVisible();
  await expect(page.getByRole("button", { name: "Previous frame" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Next frame" })).toBeVisible();
  await expectNoVisibleOverlap(page, minimizedReplayOverlaySelectors());
  await expect
    .poll(() => page.evaluate((key) => localStorage.getItem(key), MATCH_CHROME_STORAGE_KEY))
    .toBe("true");

  await page.reload();
  await expect(page.getByRole("button", { name: "Restore match chrome" })).toBeVisible();
  await expect(page.getByLabel("Replay frame")).toBeVisible();

  await page.setViewportSize({ width: 390, height: 700 });
  await page.reload();
  await expectFullViewportBoard(page, { width: 390, height: 700 });
  await expect(page.getByLabel("Replay timeline")).toBeVisible();
});

test("moves and attacks through the 3D board", async ({ page }) => {
  await useStoredBoardVisualMode(page, "3d");
  let match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });
  const actions = [];

  await mockMatchApi(page, async (action) => {
    actions.push(action);
    if (action.type === "movePiece") {
      match = playableMatch({
        playerHero: action.to,
        opponentHero: { q: 1, r: 1 },
      });
    }

    if (action.type === "startAttackPhase") {
      match = playableMatch({
        phase: "attack",
        playerHero: { q: 0, r: 1 },
        opponentHero: { q: 1, r: 1 },
      });
    }

    if (action.type === "attack") {
      match = playableMatch({
        playerHero: { q: 0, r: 1 },
        opponentHero: { q: 1, r: 1 },
      });
    }

    return matchResponse(match);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();

  await tile(page, "q 0, r 1, occupied by your hero").click();
  await expect(tile(page, "q 0, r 0, empty hex")).toHaveAttribute("data-legal", "true");
  await tile(page, "q 0, r 0, empty hex").click();
  await expect.poll(() => actions).toEqual(
    expect.arrayContaining([
      {
        type: "movePiece",
        pieceId: "player-hero",
        to: { q: 0, r: 0 },
      },
    ]),
  );

  actions.length = 0;
  match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });
  await page.reload();
  await page.getByRole("button", { name: "Start Attack" }).click();
  await expect.poll(() => actions).toContainEqual({ type: "startAttackPhase" });
  await tile(page, "q 0, r 1, occupied by your hero").click();
  await expect(tile(page, "q 1, r 1, occupied by the opponent's hero")).toHaveAttribute(
    "data-legal",
    "true",
  );
  await tile(page, "q 1, r 1, occupied by the opponent's hero").click();

  await expect.poll(() => actions).toEqual(
    expect.arrayContaining([
      {
        type: "attack",
        attackerId: "player-hero",
        targetId: "opponent-hero",
      },
    ]),
  );
});

test("plays unit and spell card targets through the 3D board", async ({ page }) => {
  await useStoredBoardVisualMode(page, "3d");
  let match = playableMatch({
    phase: "cardPlay",
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    hand: [emberSquire(), sparkJolt()],
  });
  const actions = [];

  await mockMatchApi(page, async (action) => {
    actions.push(action);
    if (action.type === "playCard" && action.cardId === "ember-squire-card") {
      match = playableMatch({
        phase: "cardPlay",
        playerHero: { q: 0, r: 1 },
        opponentHero: { q: 1, r: 1 },
        hand: [sparkJolt()],
        units: [ashScout({ q: 0, r: 0 })],
      });
    }

    return matchResponse(match);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await page.getByRole("button", { name: /Ember Squire/ }).click();
  await expect(tile(page, "q 0, r 0, empty hex")).toHaveAttribute("data-legal", "true");
  await tile(page, "q 0, r 0, empty hex").click();
  await expect.poll(() => actions).toEqual(
    expect.arrayContaining([
      {
        type: "playCard",
        cardId: "ember-squire-card",
        target: { type: "hex", coord: { q: 0, r: 0 } },
      },
    ]),
  );

  await page.getByRole("button", { name: /Spark Jolt/ }).click();
  await expect(tile(page, "q 1, r 1, occupied by the opponent's hero")).toHaveAttribute(
    "data-legal",
    "true",
  );
  await tile(page, "q 1, r 1, occupied by the opponent's hero").click();
  await expect.poll(() => actions).toEqual(
    expect.arrayContaining([
      {
        type: "playCard",
        cardId: "spark-jolt-card",
        target: { type: "piece", pieceId: "opponent-hero" },
      },
    ]),
  );
});

test("shows queued stack targeting indicators and filters them from the board overlay", async ({ page }) => {
  await useStoredBoardVisualMode(page, "3d");
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    prioritySide: "player",
    actionStack: [
      {
        id: "pending-attack",
        side: "opponent",
        priority: 0,
        action: {
          type: "attack",
          attackerId: "opponent-hero",
          targetId: "player-hero",
        },
      },
      {
        id: "pending-spell",
        side: "player",
        priority: 1,
        action: {
          type: "castSpell",
          card: {
            templateId: "spark-jolt",
            name: "Spark Jolt",
            rarity: "basic",
            cost: 1,
            kind: { type: "spell", range: 2, priority: 1, effect: { type: "damage", amount: 1 } },
          },
          targetId: "opponent-hero",
        },
      },
      {
        id: "pending-move",
        side: "player",
        priority: 2,
        action: {
          type: "movePiece",
          pieceId: "player-hero",
          from: { q: 0, r: 1 },
          to: { q: 0, r: 0 },
        },
      },
    ],
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();
  await expect.poll(() => hasReady3dCanvas(page)).toBe(true);
  await expect(page.getByRole("region", { name: "Board stack targeting" })).toBeVisible();
  await expect(page.locator("[data-targeting-stack-row='pending-attack']")).toBeVisible();
  await expect(page.locator("[data-targeting-stack-row='pending-spell']")).toBeVisible();
  await expect(page.locator("[data-targeting-stack-row='pending-move']")).toBeVisible();

  await expect(page.locator("[data-targeting-indicator]")).toHaveCount(2);
  await page.locator("[data-targeting-stack-row='pending-spell']").hover();
  await expect(page.locator("[data-targeting-indicator]")).toHaveCount(1);
  await expect(page.locator("[data-targeting-indicator]")).toHaveAttribute("data-stack-item-id", "pending-spell");

  await tile(page, "q 0, r 1, occupied by your hero").click();
  await expect(tile(page, "q 0, r 1, occupied by your hero")).toBeVisible();
});

test("shows selected spell targeting indicators in 2D mode", async ({ page }) => {
  await useStoredBoardVisualMode(page, "2d");
  const match = playableMatch({
    phase: "cardPlay",
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    hand: [sparkJolt()],
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="2d"]')).toBeVisible();
  await page.getByRole("button", { name: /Spark Jolt/ }).click();
  await expect(page.locator("[data-targeting-indicator]")).toHaveCount(1);
  await expect(page.locator("[data-targeting-primary]")).toHaveCount(1);
});

test("keeps projected 3D hit targets usable after viewport resize", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await useStoredBoardVisualMode(page, "3d");
  let match = playableMatch({
    phase: "cardPlay",
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    hand: [emberSquire()],
  });
  const actions = [];

  await mockMatchApi(page, async (action) => {
    actions.push(action);
    if (action.type === "playCard") {
      match = playableMatch({
        phase: "cardPlay",
        playerHero: { q: 0, r: 1 },
        opponentHero: { q: 1, r: 1 },
        hand: [],
        units: [ashScout(action.target.coord)],
      });
    }

    return matchResponse(match);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();
  await expect.poll(() => hasReady3dCanvas(page)).toBe(true);

  const destination = tile(page, "q 0, r 0, empty hex");
  await expect(destination).toBeVisible();
  const beforeResize = await locatorCenter(destination);
  await expectHitTargetInsideCanvas(page, destination);

  await page.setViewportSize({ width: 390, height: 700 });
  await expect(destination).toBeVisible();
  await expect.poll(() => locatorCenter(destination)).not.toEqual(beforeResize);
  await expectHitTargetInsideCanvas(page, destination);

  await page.getByRole("button", { name: /Ember Squire/ }).dragTo(destination);
  await expect.poll(() => actions).toEqual(
    expect.arrayContaining([
      {
        type: "playCard",
        cardId: "ember-squire-card",
        target: { type: "hex", coord: { q: 0, r: 0 } },
      },
    ]),
  );
});

test("zooms and turns the 3D board camera with the mouse", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await useStoredBoardVisualMode(page, "3d");
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();
  await expect.poll(() => hasReady3dCanvas(page)).toBe(true);

  const canvasBox = await page.locator(".board-3d-shell canvas").boundingBox();
  expect(canvasBox).not.toBeNull();
  const center = {
    x: Math.round(canvasBox.x + canvasBox.width / 2),
    y: Math.round(canvasBox.y + canvasBox.height / 2),
  };
  await page.mouse.move(center.x, center.y);

  const initialCamera = await cameraMetrics(page);
  await page.mouse.wheel(0, -700);
  await expect.poll(() => cameraMetrics(page)).toMatchObject({
    distance: expect.any(Number),
  });
  await expect.poll(async () => (await cameraMetrics(page)).distance).toBeLessThan(
    initialCamera.distance - 0.1,
  );

  const zoomedCamera = await cameraMetrics(page);
  await page.mouse.move(center.x, center.y);
  await page.mouse.down();
  await page.mouse.move(center.x + 180, center.y, { steps: 8 });
  await page.mouse.up();
  await expect
    .poll(async () => Math.abs((await cameraMetrics(page)).x - zoomedCamera.x))
    .toBeGreaterThan(0.1);

  await tile(page, "q 0, r 1, occupied by your hero").click();
  await expect(tile(page, "q 0, r 0, empty hex")).toHaveAttribute("data-legal", "true");
});

test("respects disabled state, context menus, visible counts, and replay read-only in 3D mode", async ({
  page,
}) => {
  await useStoredBoardVisualMode(page, "3d");
  let match = playableMatch({
    phase: "matchOver",
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    units: [ashScout({ q: 0, r: 0 })],
  });
  const actions = [];

  await mockMatchApi(page, async (action) => {
    actions.push(action);
    return matchResponse(match);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(tile(page, "q 0, r 1, occupied by your hero")).toBeDisabled();
  await tile(page, "q 0, r 1, occupied by your hero").click({ force: true });
  expect(actions).toEqual([]);

  match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    units: [ashScout({ q: 0, r: 0 })],
  });
  await page.goto(`/match/${MATCH_ID}`);
  await expect(tile(page, "q 0, r 0, occupied by your unit")).toContainText("1/2 AP 2");
  await tile(page, "q 0, r 0, occupied by your unit").click({ button: "right" });
  await expect(page.getByRole("menu", { name: "Ash Scout actions" })).toBeVisible();

  await mockReplayApi(page, match);
  await page.goto(`/matches/${MATCH_ID}/replay`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();
  await expect(tile(page, "q 0, r 1, occupied by your hero")).toHaveAttribute("tabindex", "-1");
  await tile(page, "q 0, r 1, occupied by your hero").click({ force: true });
  expect(actions).toEqual([]);
});

test("persists Board visual mode switches locally", async ({ page }) => {
  await useStoredBoardVisualMode(page, "3d");
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto("/settings");
  await expect(page.getByLabel("Board visual mode")).toHaveValue("3d");
  await page.getByLabel("Board visual mode").selectOption("2d");
  await page.getByRole("button", { name: "Save Settings" }).click();
  await expect(page.getByText("Settings saved.")).toBeVisible();
  await expect
    .poll(() => page.evaluate((key) => localStorage.getItem(key), BOARD_VISUAL_MODE_STORAGE_KEY))
    .toBe("2d");

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="2d"]')).toBeVisible();

  await page.reload();
  await expect(page.locator('section[data-board-renderer="2d"]')).toBeVisible();
});

test("falls back to playable 2D when WebGL cannot start", async ({ page }) => {
  await useStoredBoardVisualMode(page, "3d");
  await page.addInitScript(() => {
    const originalGetContext = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function patchedGetContext(type, ...args) {
      if (String(type).includes("webgl")) {
        return null;
      }

      return originalGetContext.call(this, type, ...args);
    };
  });

  let match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });
  const actions = [];

  await mockMatchApi(page, async (action) => {
    actions.push(action);
    if (action.type === "movePiece") {
      match = playableMatch({
        playerHero: action.to,
        opponentHero: { q: 1, r: 1 },
      });
    }

    return matchResponse(match);
  }, () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="2d"]')).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "3D board unavailable, using 2D." })).toBeVisible();

  await tile(page, "q 0, r 1, occupied by your hero").click();
  await expect(tile(page, "q 0, r 0, empty hex")).toHaveClass(/legal/);
  await tile(page, "q 0, r 0, empty hex").click();
  await expect.poll(() => actions).toEqual(
    expect.arrayContaining([
      {
        type: "movePiece",
        pieceId: "player-hero",
        to: { q: 0, r: 0 },
      },
    ]),
  );
});

test("keeps reduced-motion first-time visitors on the 2D board", async ({ page }) => {
  await page.addInitScript(({ key }) => {
    localStorage.removeItem(key);
    window.matchMedia = (query) => ({
      matches: query === "(prefers-reduced-motion: reduce)",
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    });
  }, { key: BOARD_VISUAL_MODE_STORAGE_KEY });
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="2d"]')).toBeVisible();
  await expect(page.locator(".piece-token[class*='piece-anim-']")).toHaveCount(0);
});

test("uses procedural hero miniatures without an asset failure notice", async ({ page }) => {
  await useStoredBoardVisualMode(page, "3d");
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "procedural miniatures are shown" })).toHaveCount(0);
  await expect(tile(page, "q 0, r 1, occupied by your hero")).toBeVisible();
});

test("uses procedural miniatures without an asset failure notice when no 3D model is configured", async ({
  page,
}) => {
  await useStoredBoardVisualMode(page, "3d");
  const match = playableMatch({
    playerHero: { q: 0, r: 1 },
    opponentHero: { q: 1, r: 1 },
    playerHeroType: "pyromancer",
    opponentHeroType: "warden",
    units: [ashScout({ q: 0, r: 0 })],
  });

  await mockMatchApi(page, async () => matchResponse(match), () => match);

  await page.goto(`/match/${MATCH_ID}`);
  await expect(page.locator('section[data-board-renderer="3d"]')).toBeVisible();
  await expect.poll(() => hasReady3dCanvas(page)).toBe(true);
  await expect(page.getByRole("status").filter({ hasText: "procedural miniatures are shown" })).toHaveCount(0);
  await expect(tile(page, "q 0, r 0, occupied by your unit")).toContainText("1/2 AP 2");
});

async function expectFullViewportBoard(page, viewport) {
  const boardBox = await page.getByRole("region", { name: "Hex board" }).boundingBox();
  expect(boardBox).not.toBeNull();
  expect(boardBox.width).toBeGreaterThanOrEqual(viewport.width - 4);
  expect(boardBox.height).toBeGreaterThanOrEqual(viewport.height - 4);

  const shellBox = await page.locator(".board-3d-shell").boundingBox();
  expect(shellBox).not.toBeNull();
  expect(shellBox.width).toBeGreaterThanOrEqual(viewport.width - 4);
  expect(shellBox.height).toBeGreaterThanOrEqual(viewport.height - 4);
  expect(shellBox.width).toBeLessThanOrEqual(viewport.width + 4);
}

async function expectNoVisibleOverlap(page, selectors) {
  const boxes = await visibleBoxes(page, selectors);
  for (let index = 0; index < boxes.length; index += 1) {
    for (let nextIndex = index + 1; nextIndex < boxes.length; nextIndex += 1) {
      expect(rectanglesOverlap(boxes[index], boxes[nextIndex]), overlapMessage(boxes[index], boxes[nextIndex])).toBe(
        false,
      );
    }
  }
}

async function ensureMatchChromeMinimized(page) {
  const restoreChrome = page.getByRole("button", { name: "Restore match chrome" });
  if ((await page.locator(".match-app-shell.match-chrome-minimized").count()) === 0) {
    await page.getByRole("button", { name: "Minimize match chrome" }).click();
  }

  await expect(page.locator(".match-app-shell.match-chrome-minimized")).toBeVisible();
  await expect(restoreChrome).toBeVisible();
}

async function visibleBoxes(page, selectors) {
  return page.evaluate((items) => {
    return items.flatMap((item) => {
      const element = document.querySelector(item.selector);
      if (!element) {
        return [];
      }

      const style = window.getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      if (
        style.display === "none" ||
        style.visibility === "hidden" ||
        Number(style.opacity) === 0 ||
        rect.width < 2 ||
        rect.height < 2
      ) {
        return [];
      }

      return [
        {
          name: item.name,
          left: rect.left,
          top: rect.top,
          right: rect.right,
          bottom: rect.bottom,
          width: rect.width,
          height: rect.height,
        },
      ];
    });
  }, selectors);
}

function rectanglesOverlap(first, second) {
  const tolerancePx = 2;
  return (
    first.left < second.right - tolerancePx &&
    first.right > second.left + tolerancePx &&
    first.top < second.bottom - tolerancePx &&
    first.bottom > second.top + tolerancePx
  );
}

function overlapMessage(first, second) {
  return `${first.name} overlaps ${second.name}: ${JSON.stringify({ first, second })}`;
}

function matchOverlaySelectors() {
  return [
    { name: "match chrome", selector: ".match-chrome" },
    { name: "match action dock", selector: ".match-action-dock" },
    { name: "turn checklist", selector: ".turn-checklist" },
    { name: "match UX dock", selector: ".match-ux-dock" },
    { name: "opponent badge", selector: ".battlefield-hud-opponent" },
    { name: "opponent hand", selector: ".battlefield-hud-hand" },
    { name: "phase pill", selector: ".battlefield-hud-phase" },
    { name: "player badge", selector: ".battlefield-hud-player" },
    { name: "hand", selector: ".hand-overlay" },
  ];
}

function minimizedMatchOverlaySelectors() {
  return [
    { name: "restore chrome", selector: ".match-chrome-restore" },
    { name: "match action dock", selector: ".match-action-dock" },
    { name: "turn checklist", selector: ".turn-checklist" },
    { name: "match UX dock", selector: ".match-ux-dock" },
    { name: "opponent badge", selector: ".battlefield-hud-opponent" },
    { name: "opponent hand", selector: ".battlefield-hud-hand" },
    { name: "phase pill", selector: ".battlefield-hud-phase" },
    { name: "player badge", selector: ".battlefield-hud-player" },
    { name: "hand", selector: ".hand-overlay" },
  ];
}

function replayOverlaySelectors() {
  return [
    { name: "match chrome", selector: ".match-chrome" },
    { name: "opponent badge", selector: ".battlefield-hud-opponent" },
    { name: "phase pill", selector: ".battlefield-hud-phase" },
    { name: "player badge", selector: ".battlefield-hud-player" },
    { name: "replay inspector", selector: ".replay-inspector" },
  ];
}

function minimizedReplayOverlaySelectors() {
  return [
    { name: "restore chrome", selector: ".match-chrome-restore" },
    { name: "opponent badge", selector: ".battlefield-hud-opponent" },
    { name: "phase pill", selector: ".battlefield-hud-phase" },
    { name: "player badge", selector: ".battlefield-hud-player" },
    { name: "replay inspector", selector: ".replay-inspector" },
  ];
}

function tile(page, name) {
  return page.getByRole("button", { name });
}

async function locatorCenter(locator) {
  const box = await locator.boundingBox();
  expect(box).not.toBeNull();

  return {
    x: Math.round(box.x + box.width / 2),
    y: Math.round(box.y + box.height / 2),
  };
}

async function expectHitTargetInsideCanvas(page, locator) {
  const targetCenter = await locatorCenter(locator);
  const canvasBox = await page.locator(".board-3d-shell canvas").boundingBox();
  expect(canvasBox).not.toBeNull();
  expect(targetCenter.x).toBeGreaterThanOrEqual(Math.floor(canvasBox.x));
  expect(targetCenter.x).toBeLessThanOrEqual(Math.ceil(canvasBox.x + canvasBox.width));
  expect(targetCenter.y).toBeGreaterThanOrEqual(Math.floor(canvasBox.y));
  expect(targetCenter.y).toBeLessThanOrEqual(Math.ceil(canvasBox.y + canvasBox.height));
}

async function cameraMetrics(page) {
  return page.locator(".board-3d-shell canvas").evaluate((canvas) => ({
    distance: Number(canvas.dataset.boardCameraDistance),
    x: Number(canvas.dataset.boardCameraX),
    y: Number(canvas.dataset.boardCameraY),
    z: Number(canvas.dataset.boardCameraZ),
  }));
}

async function useStoredBoardVisualMode(page, mode) {
  await page.addInitScript(
    ({ key, mode }) => {
      if (!localStorage.getItem(key)) {
        localStorage.setItem(key, mode);
      }
    },
    { key: BOARD_VISUAL_MODE_STORAGE_KEY, mode },
  );
}

async function mockMatchApi(page, handleAction, currentMatch) {
  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/auth/me") {
      await route.fulfill({ status: 401, json: { message: "Signed out" } });
      return;
    }

    if (url.pathname === "/api/catalog/cards") {
      await route.fulfill({ json: { cards: [catalogCard(emberSquire()), catalogCard(sparkJolt())] } });
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

async function mockReplayApi(page, match) {
  await page.unroute("**://*/api/**");
  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/auth/me") {
      await route.fulfill({ status: 401, json: { message: "Signed out" } });
      return;
    }

    if (url.pathname === `/api/matches/${MATCH_ID}/replay` && request.method() === "GET") {
      await route.fulfill({
        json: {
          matchId: MATCH_ID,
          visibility: "public",
          summary: {
            matchId: MATCH_ID,
            createdAt: 0,
            updatedAt: 0,
            round: 1,
            phase: "movement",
            winner: null,
            frameCount: 1,
          },
          frames: [{ frameIndex: 0, actionIndex: null, event: { type: "matchCreated" }, matchState: match }],
        },
      });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

function matchResponse(matchState) {
  return {
    matchId: MATCH_ID,
    matchState,
    replayFrames: [],
  };
}

function playableMatch({
  activeSide = "player",
  phase = "movement",
  playerHero,
  opponentHero,
  playerHeroType = "runekeeper",
  opponentHeroType = "pyromancer",
  hand = [],
  units = [],
  prioritySide = null,
  actionStack = [],
}) {
  return {
    mode: "solo",
    round: 1,
    phase,
    activeSide,
    prioritySide,
    player: {
      side: "player",
      mana: 5,
      maxMana: 5,
      hero: {
        id: "player-hero",
        side: "player",
        heroType: playerHeroType,
        hp: 20,
        maxHp: 20,
        attack: 1,
        attackRange: 1,
        position: playerHero,
        apRemaining: activeSide === "player" ? 3 : 0,
        maxAp: 3,
        hasAttacked: false,
      },
      hand,
      handCount: hand.length,
      deckCount: 40,
      discardCount: 0,
      progression: defaultProgression(),
    },
    opponent: {
      side: "opponent",
      mana: 5,
      maxMana: 5,
      hero: {
        id: "opponent-hero",
        side: "opponent",
        heroType: opponentHeroType,
        hp: 20,
        maxHp: 20,
        attack: 1,
        attackRange: 1,
        position: opponentHero,
        apRemaining: activeSide === "opponent" ? 3 : 0,
        maxAp: 3,
        hasAttacked: false,
      },
      handCount: 0,
      deckCount: 40,
      discardCount: 0,
      progression: defaultProgression(),
    },
    board: {
      radius: 3,
      tiles: radiusThreeTiles(),
      units,
      droppedItems: [],
    },
    actionStack,
    log: [],
    winner: phase === "matchOver" ? "opponent" : null,
  };
}

function emberSquire() {
  return {
    id: "ember-squire-card",
    templateId: "ember-squire",
    name: "Ember Squire",
    rarity: "basic",
    cost: 1,
    text: "Summon a unit.",
    kind: { type: "unit", attack: 1, armor: 2, maxAp: 2 },
  };
}

function sparkJolt() {
  return {
    id: "spark-jolt-card",
    templateId: "spark-jolt",
    name: "Spark Jolt",
    rarity: "basic",
    cost: 1,
    text: "Deal 1 damage.",
    kind: { type: "spell", range: 2, priority: 1, effect: { type: "damage", amount: 1 } },
  };
}

function ashScout(position) {
  return {
    id: "ash-scout",
    side: "player",
    name: "Ash Scout",
    templateId: "ember-squire",
    attack: 1,
    attackRange: 1,
    armor: 2,
    maxArmor: 2,
    position,
    apRemaining: 2,
    maxAp: 2,
    hasAttacked: false,
    items: [],
  };
}

async function hasReady3dCanvas(page) {
  return page.locator(".board-3d-shell canvas").evaluate((canvas) => {
    if (!(canvas instanceof HTMLCanvasElement) || canvas.width === 0 || canvas.height === 0) {
      return false;
    }

    const distance = Number(canvas.dataset.boardCameraDistance);
    return Number.isFinite(distance) && distance > 0;
  });
}

function defaultProgression() {
  return {
    runeIds: [],
    skillIds: [],
    effects: {
      maxHpDelta: 0,
      attackDelta: 0,
      maxApDelta: 0,
      manaDelta: 0,
      openingHandDelta: 0,
      summonedUnitArmorDelta: 0,
      firstSummonedUnitArmorDelta: 0,
      spellDamageDelta: 0,
    },
  };
}

function catalogCard(card) {
  return {
    ...card,
    copyCount: 1,
    artKey: card.templateId,
    artPath: "",
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
