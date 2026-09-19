import { expect, test } from "@playwright/test";

const AUTH_TOKEN_STORAGE_KEY = "arcania-auth-token";
const BOARD_VISUAL_MODE_STORAGE_KEY = "arcania-board-visual-mode";
const MATCH_ID = "e2e-hotkeys";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(
    ({ authKey, boardKey }) => {
      localStorage.setItem(authKey, "existing-token");
      localStorage.setItem(boardKey, "2d");
    },
    { authKey: AUTH_TOKEN_STORAGE_KEY, boardKey: BOARD_VISUAL_MODE_STORAGE_KEY },
  );
});

test("saved navigation hotkeys use account preferences and ignore editable focus", async ({ page }) => {
  await mockHotkeyApi(page);

  const preferencesLoaded = page.waitForResponse((response) =>
    response.url().endsWith("/api/preferences"),
  );
  await page.goto("/");
  await preferencesLoaded;
  await expect
    .poll(() => page.evaluate(() => document.documentElement.getAttribute("data-animation-speed")))
    .toBe("fast");
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
  await page.keyboard.press("h");
  await expect(page).toHaveURL(/\/settings$/);

  await page.keyboard.press("g");
  await expect(page).toHaveURL(/\/catalog\/?$/);
  await expect(page.getByRole("heading", { name: "Card Catalog" })).toBeVisible();

  await page.getByPlaceholder("Search cards").fill("ember");
  await page.keyboard.press("l");
  await expect(page).toHaveURL(/\/catalog\/?$/);

  await page.getByPlaceholder("Search cards").blur();
  await page.keyboard.press("l");
  await expect(page).toHaveURL(/\/decks$/);

  await page.keyboard.press("v");
  await expect(page).toHaveURL(/\/matches$/);
});

test("saved action hotkeys submit only when the viewer may act", async ({ page }) => {
  let match = playableMatch({ activeSide: "player", phase: "cardPlay" });
  const actions = [];
  await mockHotkeyApi(page, async (action) => {
    actions.push(action);
    if (action.type === "endTurn") {
      match = playableMatch({ activeSide: "opponent" });
      return matchResponse(match);
    }
    if (action.type === "passPriority") {
      match = playableMatch({ activeSide: "player" });
      return matchResponse(match);
    }
    throw new Error(`Unexpected action ${action.type}`);
  }, () => match);

  const preferencesLoaded = page.waitForResponse((response) =>
    response.url().endsWith("/api/preferences"),
  );
  await page.goto(`/match/${MATCH_ID}`);
  await preferencesLoaded;
  await expect(page.getByRole("heading", { name: "Round 1" })).toBeVisible();
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
  await page.keyboard.press("y");
  await expect.poll(() => actions.map((action) => action.type)).toEqual(["endTurn"]);

  await page.keyboard.press("y");
  await page.waitForTimeout(150);
  expect(actions.map((action) => action.type)).toEqual(["endTurn"]);

  match = playableMatch({
    activeSide: "player",
    actionStack: [stackItem()],
    prioritySide: "player",
    hand: [responseSpell()],
  });
  const reloadedMatch = page.waitForResponse((response) =>
    response.url().endsWith(`/api/matches/${MATCH_ID}`),
  );
  await page.reload();
  await reloadedMatch;
  await expect(page.getByText("You priority").first()).toBeVisible();
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });
  await page.keyboard.press("r");
  await expect.poll(() => actions.map((action) => action.type)).toEqual([
    "endTurn",
    "passPriority",
  ]);
});

test("solo board cursor selects and confirms a legal move", async ({ page }) => {
  let match = playableMatch({ activeSide: "player" });
  const actions = [];
  await mockHotkeyApi(page, async (action) => {
    actions.push(action);
    if (action.type === "movePiece") {
      match = playableMatch({
        activeSide: "player",
        playerPosition: action.to,
      });
      return matchResponse(match);
    }

    throw new Error(`Unexpected action ${action.type}`);
  }, () => match);

  const preferencesLoaded = page.waitForResponse((response) =>
    response.url().endsWith("/api/preferences"),
  );
  await page.goto(`/match/${MATCH_ID}`);
  await preferencesLoaded;
  await expect(page.getByRole("heading", { name: "Round 1" })).toBeVisible();
  await page.evaluate(() => {
    if (document.activeElement instanceof HTMLElement) {
      document.activeElement.blur();
    }
  });

  await page.keyboard.press("Enter");
  await page.keyboard.press("q");
  await expect(
    page.locator('.hex-tile.keyboard-focused[aria-label="q 0, r 2, empty hex"]'),
  ).toBeVisible();

  await page.keyboard.press("Enter");
  await expect.poll(() => actions).toEqual([
    { type: "movePiece", pieceId: "player-hero", to: { q: 0, r: 2 } },
  ]);
  await expect(
    page.getByRole("button", { name: "q 0, r 2, occupied by your hero" }),
  ).toBeVisible();
});

async function mockHotkeyApi(page, handleAction = null, currentMatch = null) {
  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/auth/me") {
      await route.fulfill({ json: authUser() });
      return;
    }

    if (url.pathname === "/api/preferences") {
      await route.fulfill({ json: preferences() });
      return;
    }

    if (url.pathname === "/api/catalog/cards") {
      await route.fulfill({ json: { cards: [catalogCard(emberSquire())] } });
      return;
    }

    if (url.pathname === "/api/decks") {
      await route.fulfill({ json: { rules: deckRules(), decks: [] } });
      return;
    }

    if (url.pathname === "/api/system-decks") {
      await route.fulfill({ json: { rules: deckRules(), decks: [systemDeck()] } });
      return;
    }

    if (url.pathname === "/api/progression") {
      await route.fulfill({ json: progression() });
      return;
    }

    if (url.pathname === "/api/matches" || url.pathname === "/api/profile/matches") {
      await route.fulfill({ json: { matches: [] } });
      return;
    }

    if (url.pathname === `/api/matches/${MATCH_ID}` && request.method() === "GET") {
      await route.fulfill({ json: matchResponse(currentMatch?.() ?? playableMatch({ activeSide: "player" })) });
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

function preferences() {
  return {
    theme: "system",
    motion: "system",
    animationSpeed: "fast",
    boardScale: "normal",
    boardVisualMode: "2d",
    hotkeys: [
      { commandId: "cursorNorthwest", binding: "Q" },
      { commandId: "cursorNortheast", binding: "W" },
      { commandId: "cursorEast", binding: "E" },
      { commandId: "cursorWest", binding: "A" },
      { commandId: "cursorSouthwest", binding: "S" },
      { commandId: "cursorSoutheast", binding: "D" },
      { commandId: "confirm", binding: "Enter" },
      { commandId: "cancel", binding: "Escape" },
      { commandId: "endTurn", binding: "Y" },
      { commandId: "passPriority", binding: "R" },
      { commandId: "openCardInfo", binding: "I" },
      { commandId: "openSettings", binding: "H" },
      { commandId: "openCatalog", binding: "G" },
      { commandId: "openDecks", binding: "L" },
      { commandId: "openMatchArchive", binding: "V" },
    ],
    updatedAt: 10,
  };
}

function authUser() {
  return {
    id: 1,
    handle: "rune-player",
    email: "player@local.dev",
    displayName: "Rune Player",
    avatar: { symbol: "sparkles", color: "emerald" },
    preferredHeroType: "runekeeper",
    boardVisualMode: "2d",
    progressionSummary: {
      totalXp: 0,
      level: 1,
      currentLevelXp: 0,
      nextLevelXp: 100,
      xpIntoLevel: 0,
      xpToNextLevel: 100,
      runeSlots: 1,
    },
  };
}

function matchResponse(matchState) {
  return { matchId: MATCH_ID, matchState, replayFrames: [] };
}

function playableMatch({
  activeSide,
  phase = "movement",
  actionStack = [],
  prioritySide = null,
  hand = [],
  playerPosition = { q: 0, r: 3 },
  opponentPosition = { q: 0, r: -3 },
}) {
  return {
    mode: "solo",
    round: 1,
    phase,
    activeSide,
    prioritySide,
    player: participant("player", activeSide, playerPosition, hand),
    opponent: participant("opponent", activeSide, opponentPosition, undefined),
    board: { radius: 3, tiles: radiusThreeTiles(), units: [], droppedItems: [] },
    actionStack,
    log: [],
    winner: null,
  };
}

function participant(side, activeSide, position, hand) {
  return {
    side,
    mana: 5,
    maxMana: 5,
    hero: {
      id: `${side}-hero`,
      side,
      heroType: side === "player" ? "runekeeper" : "pyromancer",
      hp: 20,
      maxHp: 20,
      attack: 1,
      attackRange: 1,
      position,
      apRemaining: activeSide === side ? 3 : 0,
      maxAp: 3,
      hasAttacked: false,
    },
    hand,
    handCount: hand?.length ?? 0,
    deckCount: 40,
    discardCount: 0,
    progression: defaultProgression(),
  };
}

function stackItem() {
  return {
    id: "stack-1",
    side: "opponent",
    priority: 1,
    action: { type: "castSpell", card: { id: "spark", name: "Spark", rarity: "basic" }, targetId: "player-hero" },
  };
}

function responseSpell() {
  return {
    id: "response-spell",
    templateId: "response-spell",
    name: "Response Spell",
    rarity: "basic",
    cost: 1,
    text: "Respond.",
    kind: { type: "spell", range: 3, priority: 2, effect: { type: "heal", amount: 1 } },
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

function catalogCard(card) {
  return { ...card, copyCount: 1, artKey: card.templateId, artPath: "" };
}

function deckRules() {
  return {
    maxDecksPerAccount: 30,
    minCards: 30,
    basicCopyLimit: 99,
    advancedCopyLimit: 3,
    rareCopyLimit: 1,
    advancedTotalLimit: 10,
    rareTotalLimit: 4,
  };
}

function systemDeck() {
  return {
    id: "balanced-starter",
    name: "Balanced Starter",
    heroType: "runekeeper",
    cards: [],
    legality: {
      legal: true,
      totalCards: 30,
      basicCards: 30,
      advancedCards: 0,
      rareCards: 0,
      messages: [],
    },
  };
}

function progression() {
  return {
    account: {
      totalXp: 0,
      level: 1,
      currentLevelXp: 0,
      nextLevelXp: 100,
      xpIntoLevel: 0,
      xpToNextLevel: 100,
      runeSlots: 1,
    },
    runes: [],
    heroes: [],
    skillTrees: [],
    loadouts: [],
  };
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
