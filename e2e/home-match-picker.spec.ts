import { expect, test } from "@playwright/test";

const AUTH_TOKEN_STORAGE_KEY = "rune-lanes-auth-token";
const MATCH_ID = "solo-account-deck";
const SHARED_MATCH_ID = "shared-home-match";

test("mobile navigation contains keyboard focus and restores it on close", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 600 });
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockHomeApi(page, []);
  await page.goto("/");
  const trigger = page.getByRole("button", { name: "Open navigation menu" });
  const dialog = page.getByRole("dialog", { name: "Navigation menu" });
  const close = dialog.getByRole("button", { name: "Close navigation menu" });

  await trigger.focus();
  await page.keyboard.press("Enter");
  await expect(close).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(dialog.getByRole("button").last()).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(close).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();

  await page.keyboard.press("Enter");
  await expect(close).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(trigger).toBeFocused();
  await page.mouse.wheel(0, 1200);
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);

  await trigger.focus();
  await page.keyboard.press("Enter");
  for (const destination of ["Dashboard", "Play", "Decks", "Heroes"]) {
    await page.keyboard.press("Tab");
    await expect(dialog.getByRole("button", { name: destination, exact: true })).toBeFocused();
  }
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/\/heroes$/);
  await expect(page.getByRole("heading", { name: "Heroes", exact: true })).toBeVisible();
  await expect(dialog).toHaveCount(0);
});

test("Dashboard continues the newest active match when the deck library fails", async ({ page }) => {
  const matchRequests = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockHomeApi(page, matchRequests);
  await page.route("**/api/decks", (route) => route.fulfill({ status: 503, json: { message: "Deck library unavailable" } }));
  await page.route("**/api/profile/matches", (route) => route.fulfill({ json: { matches: [
    { matchId: "older-active", mode: "solo", createdAt: 1, updatedAt: 10, round: 1, phase: "movement", winner: null, frameCount: 1 },
    { matchId: MATCH_ID, mode: "solo", createdAt: 1, updatedAt: 20, round: 2, phase: "cardPlay", winner: null, frameCount: 2 },
  ] } }));

  await page.goto("/");
  const loadout = page.getByRole("region", { name: "Current loadout" });
  await expect(loadout.getByRole("heading", { name: "Deck library unavailable" })).toBeVisible();
  await expect(loadout.getByRole("button", { name: "2 active matches" })).toBeVisible();
  await expect(loadout.getByRole("button", { name: "Start match" })).toHaveCount(0);
  await loadout.getByRole("button", { name: "Continue match" }).click();
  await expect(page).toHaveURL(new RegExp(`/match/${MATCH_ID}$`));
  await expect(page.getByRole("region", { name: "Hex board" })).toBeVisible();
  expect(matchRequests).toEqual([]);
});

test("AI deck selection remains visible with system and legal account deck recipes", async ({ page }) => {
  const matchRequests = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockHomeApi(page, matchRequests);

  await page.goto("/play");
  await openAdvancedSetup(page);

  await expect(page.getByLabel("Preconfigured loadouts")).toHaveCount(0);
  await expect(page.getByLabel("Custom deck loadouts")).toHaveCount(0);

  const aiDeckSelector = page.getByLabel("AI Deck");
  await expect(aiDeckSelector).toBeVisible();
  await expect(aiDeckSelector).toContainText("Balanced Starter");
  await expect(aiDeckSelector).toContainText("Tournament Legal");
  await expect(aiDeckSelector).toContainText("Default Legal");
  await expect(aiDeckSelector).not.toContainText("Needs More Basics");

  await aiDeckSelector.selectOption("account:101");
  await page.getByLabel("AI Hero").selectOption("chronomancer");
  await page.getByRole("button", { name: "New Solo Match" }).click();

  await expect(page).toHaveURL(new RegExp(`/match/${MATCH_ID}$`));
  expect(matchRequests).toEqual([
    {
      heroType: "runekeeper",
      playerDeck: { source: "account", deckId: 102 },
      aiOpponent: { source: "account", deckId: 101, heroType: "chronomancer" },
    },
  ]);
});

test("signed-in players choose account deck recipes from the basic carousel for solo match creation", async ({
  page,
}) => {
  const matchRequests = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockHomeApi(page, matchRequests);

  await page.goto("/play");

  const carousel = page.getByLabel("Configured deck recipes");
  await expect(carousel).toBeVisible();
  await expect(carousel.getByRole("button", { name: /Default Legal.*Custom deck recipe/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(carousel.getByText("No runes equipped")).toBeVisible();
  await expect(carousel.getByRole("button", { name: /Balanced Starter/ })).toHaveCount(0);
  await expect(carousel.getByRole("button", { name: /Needs More Basics/ })).toHaveCount(0);

  await page.getByRole("button", { name: "Next configured deck recipe" }).click();
  await expect(carousel.getByRole("button", { name: /Tournament Legal.*Custom deck recipe/ })).toHaveAttribute(
    "aria-pressed",
    "true",
  );
  await expect(carousel.getByText("Spark Stone")).toBeVisible();
  await page.getByRole("button", { name: "New Solo Match" }).click();

  await expect(page).toHaveURL(new RegExp(`/match/${MATCH_ID}$`));
  expect(matchRequests).toEqual([
    {
      heroType: "pyromancer",
      playerDeck: { source: "account", deckId: 101 },
      aiOpponent: { source: "system", systemDeckId: "balanced-starter" },
      runeIds: ["spark-stone"],
    },
  ]);
});

test("signed-in players without legal deck recipes are blocked and sent to Decks", async ({
  page,
}) => {
  const matchRequests = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockHomeApi(page, matchRequests, {
    decks: [deckRecipe(103, "Needs More Basics", false, false)],
  });

  await page.goto("/play");

  await expect(page.getByLabel("Configured deck recipes")).toHaveCount(0);
  await expect(page.getByText("Create a legal configured deck recipe to play.")).toBeVisible();
  await expect(page.getByRole("button", { name: "New Solo Match" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "New Multiplayer Match" })).toBeDisabled();

  await page.getByRole("button", { name: "Manage Decks" }).click();

  await expect(page).toHaveURL(/\/decks$/);
  expect(matchRequests).toEqual([]);
});

test("anonymous players cannot use the account deck carousel or hidden starter fallback", async ({
  page,
}) => {
  const matchRequests = [];
  await mockHomeApi(page, matchRequests, { signedIn: false });

  await page.goto("/play");

  await expect(page.getByText("Sign in to play configured deck recipes.")).toBeVisible();
  await expect(page.getByLabel("Configured deck recipes")).toHaveCount(0);
  await expect(page.getByLabel("Custom deck loadouts")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "New Solo Match" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "New Multiplayer Match" })).toBeDisabled();

  expect(matchRequests).toEqual([]);
});

test("home screen creates a shared match from the multiplayer action", async ({ page }) => {
  const matchRequests = [];
  const sharedMatchRequests = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockHomeApi(page, matchRequests, { sharedMatchRequests });

  await page.goto("/play");

  await page.getByRole("button", { name: "Next configured deck recipe" }).click();
  await page.getByRole("button", { name: "New Multiplayer Match" }).click();

  await expect(page).toHaveURL(new RegExp(`/match/${SHARED_MATCH_ID}/player-seat$`));
  expect(sharedMatchRequests).toEqual([{ heroType: "pyromancer" }]);
  expect(matchRequests).toEqual([]);
  expect(
    await page.evaluate((matchId) => sessionStorage.getItem(`rune-lanes-hero:${matchId}`), SHARED_MATCH_ID),
  ).toBe("pyromancer");
  expect(
    await page.evaluate((matchId) => sessionStorage.getItem(`rune-lanes-runes:${matchId}`), SHARED_MATCH_ID),
  ).toBe(JSON.stringify(["spark-stone"]));
  expect(
    await page.evaluate((matchId) => sessionStorage.getItem(`rune-lanes-deck-choice:${matchId}`), SHARED_MATCH_ID),
  ).toBe(JSON.stringify({ source: "account", deckId: 101 }));
});

async function mockHomeApi(page, matchRequests, options = {}) {
  const signedIn = options.signedIn ?? true;
  const sharedMatchRequests = options.sharedMatchRequests ?? [];
  const decks =
    "decks" in options
      ? options.decks
      : [
          deckRecipe(101, "Tournament Legal", false, true),
          deckRecipe(102, "Default Legal", true, true),
          deckRecipe(103, "Needs More Basics", false, false),
        ];

  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/auth/me") {
      if (signedIn) {
        await route.fulfill({ json: authUser() });
      } else {
        await route.fulfill({ status: 401, json: { message: "Not authenticated" } });
      }
      return;
    }

    if (url.pathname === "/api/preferences") {
      await route.fulfill({ json: preferences() });
      return;
    }

    if (url.pathname === "/api/decks") {
      if (!signedIn) {
        await route.fulfill({ status: 401, json: { message: "Not authenticated" } });
        return;
      }
      await route.fulfill({
        json: {
          rules: deckRules(),
          decks,
        },
      });
      return;
    }

    if (url.pathname === "/api/system-decks") {
      await route.fulfill({
        json: { rules: deckRules(), decks: [systemDeck()] },
      });
      return;
    }

    if (url.pathname === "/api/progression") {
      if (!signedIn) {
        await route.fulfill({ status: 401, json: { message: "Not authenticated" } });
        return;
      }
      await route.fulfill({ json: progression() });
      return;
    }

    if (url.pathname === "/api/matches" && request.method() === "POST") {
      matchRequests.push(JSON.parse(request.postData() ?? "{}"));
      await route.fulfill({ json: matchResponse(playableMatch()) });
      return;
    }

    if (url.pathname === "/api/shared-matches" && request.method() === "POST") {
      sharedMatchRequests.push(JSON.parse(request.postData() ?? "{}"));
      await route.fulfill({
        json: {
          matchId: SHARED_MATCH_ID,
          mode: "shared",
          status: "setup",
          viewerSide: "player",
          playerSeatUrl: `/match/${SHARED_MATCH_ID}/player-seat`,
          inviteSeatUrl: `/match/${SHARED_MATCH_ID}/opponent-seat`,
        },
      });
      return;
    }

    if (url.pathname === `/api/matches/${MATCH_ID}` && request.method() === "GET") {
      await route.fulfill({ json: matchResponse(playableMatch()) });
      return;
    }

    if (url.pathname === "/api/profile/matches" || url.pathname === "/api/matches") {
      await route.fulfill({ json: { matches: [] } });
      return;
    }

    if (url.pathname === "/api/catalog/cards") {
      await route.fulfill({ json: { cards: [] } });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

async function openAdvancedSetup(page) {
  await page.getByRole("button", { name: "Advanced setup" }).click();
}

function authUser() {
  return {
    id: 1,
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

function preferences() {
  return {
    theme: "system",
    motion: "system",
    animationSpeed: "normal",
    boardScale: "normal",
    boardVisualMode: "2d",
    hotkeys: [],
    updatedAt: 10,
  };
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

function deckRecipe(id, name, isDefault, legal) {
  const heroType = id === 101 ? "pyromancer" : "runekeeper";
  const runeIds = id === 101 ? ["spark-stone"] : [];
  return {
    id,
    name,
    heroType,
    runeIds,
    isDefault,
    cards: [],
    legality: {
      legal,
      totalCards: legal ? 30 : 12,
      basicCards: legal ? 30 : 12,
      advancedCards: 0,
      rareCards: 0,
      messages: legal ? [] : ["Add 18 more cards."],
    },
    createdAt: 10,
    updatedAt: 20,
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
      level: 2,
      currentLevelXp: 100,
      nextLevelXp: 250,
      xpIntoLevel: 10,
      xpToNextLevel: 140,
      runeSlots: 1,
    },
    runes: [
      {
        id: "spark-stone",
        name: "Spark Stone",
        text: "Start with a brighter spark.",
        unlockLevel: 1,
        unlocked: true,
      },
    ],
    heroes: [],
    skillTrees: [],
    loadouts: [
      { heroType: "runekeeper", runeIds: [] },
      { heroType: "pyromancer", runeIds: [] },
    ],
  };
}

function matchResponse(matchState) {
  return { matchId: MATCH_ID, matchState, replayFrames: [] };
}

function playableMatch() {
  return {
    mode: "solo",
    round: 1,
    phase: "movement",
    activeSide: "player",
    prioritySide: null,
    player: participant("player"),
    opponent: participant("opponent"),
    board: { radius: 3, tiles: radiusThreeTiles(), units: [], droppedItems: [] },
    actionStack: [],
    log: [],
    winner: null,
  };
}

function participant(side) {
  return {
    side,
    mana: 5,
    maxMana: 5,
    hero: {
      id: `${side}-hero`,
      side,
      heroType: side === "player" ? "pyromancer" : "runekeeper",
      hp: 20,
      maxHp: 20,
      attack: 1,
      attackRange: 1,
      position: side === "player" ? { q: 0, r: 3 } : { q: 0, r: -3 },
      apRemaining: side === "player" ? 3 : 0,
      maxAp: 3,
      hasAttacked: false,
    },
    hand: [],
    handCount: 0,
    deckCount: 40,
    discardCount: 0,
    progression: {
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
