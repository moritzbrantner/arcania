import { expect, test } from "@playwright/test";

const AUTH_TOKEN_STORAGE_KEY = "arcania-auth-token";

test.beforeEach(async ({ page }) => {
  await page.addInitScript((authKey) => {
    localStorage.setItem(authKey, "existing-token");
  }, AUTH_TOKEN_STORAGE_KEY);
});

test("signed-in players build a Deck recipe from the catalog sidebar", async ({ page }) => {
  const savedRequests = [];
  await mockDeckDesignerApi(page, savedRequests);

  await page.goto("/decks");

  await expect(page.getByRole("heading", { name: "Decks" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Arcane Draft/ })).toBeVisible();
  await expect(page.getByRole("button", { name: /Default Sparks/ })).toBeVisible();

  const catalog = page.getByLabel("Card catalog");
  const editor = page.getByLabel("Deck editor");
  const details = page.getByLabel("Deck details");

  await expect(editor.getByRole("button", { name: "ID #10" })).toBeVisible();
  await expect(details.getByRole("heading", { name: "Draft deck" })).toBeVisible();
  await expect(details.getByText("Select a Card")).toBeVisible();
  await expect(editor.getByText("Ember Squire")).toBeVisible();
  await expect(editor.locator('img[src="/card-art/ember-squire.svg"]')).toBeVisible();
  await expect(editor.getByText("Starfire Bolt")).toHaveCount(0);

  await catalog.getByPlaceholder("Search cards").fill("damage");
  await catalog.getByLabel("Kind").selectOption("spell");
  await catalog.getByLabel("Rarity").selectOption("advanced");
  await catalog.getByLabel("Mana").selectOption("2");

  await expect(catalog.getByText("Starfire Bolt")).toBeVisible();
  await expect(catalog.getByText("Ember Squire")).toHaveCount(0);

  await catalog.getByRole("button", { name: "Add Starfire Bolt" }).click();
  await expect(editor.getByText("Starfire Bolt")).toBeVisible();
  await expect(details.getByRole("heading", { name: "Starfire Bolt" })).toBeVisible();

  await editor.getByRole("button", { name: "Select Starfire Bolt" }).click();
  await expect(details.getByText("damage 2 rng 3 pri 2")).toBeVisible();

  await details.getByRole("button", { name: "Remove Starfire Bolt" }).click();
  await expect(editor.getByText("Starfire Bolt")).toHaveCount(0);
  await expect(catalog.getByText("Starfire Bolt")).toBeVisible();
  await expect(details.getByText("Select a Card")).toBeVisible();

  await catalog.getByRole("button", { name: "Add Starfire Bolt" }).click();
  await editor.getByRole("button", { name: "Remove Ember Squire" }).click();
  await page.getByRole("button", { name: "Save" }).click();

  await expect.poll(() => savedRequests).toHaveLength(1);
  expect(savedRequests[0]).toMatchObject({
    name: "Arcane Draft",
    cards: [{ templateId: "starfire-bolt", count: 1 }],
    isDefault: false,
    heroType: "runekeeper",
    runeIds: [],
  });

  await editor.getByRole("button", { name: "ID #10" }).click();
  await expect(page).toHaveURL(/\/@rune-player\/decks\/10$/);
  await expect(page.getByRole("heading", { name: "Arcane Draft" })).toBeVisible();
  await expect(page.getByLabel("Deck cards").locator('img[src="/card-art/starfire-bolt.svg"]')).toBeVisible();
});

test("signed-out players can open a public read-only Deck recipe URL", async ({ page }) => {
  await page.addInitScript((authKey) => {
    localStorage.removeItem(authKey);
  }, AUTH_TOKEN_STORAGE_KEY);
  await mockDeckDesignerApi(page, []);

  await page.goto("/@rune-player/decks/10");

  await expect(page.getByRole("heading", { name: "Arcane Draft" })).toBeVisible();
  await expect(page.getByText("Shared by @rune-player")).toBeVisible();
  await expect(page.getByLabel("Deck cards").locator('img[src="/card-art/ember-squire.svg"]')).toBeVisible();
  await expect(page.getByRole("button", { name: "Save" })).toHaveCount(0);
});

async function mockDeckDesignerApi(page, savedRequests) {
  let accountDecks = [
    {
      id: 10,
      name: "Arcane Draft",
      isDefault: false,
      heroType: "runekeeper",
      runeIds: [],
      cards: [{ templateId: "ember-squire", count: 1 }],
      legality: legalityFor([{ templateId: "ember-squire", count: 1 }]),
      createdAt: 1,
      updatedAt: 1,
    },
    {
      id: 11,
      name: "Default Sparks",
      isDefault: true,
      heroType: "pyromancer",
      runeIds: [],
      cards: [{ templateId: "starfire-bolt", count: 2 }],
      legality: legalityFor([{ templateId: "starfire-bolt", count: 2 }]),
      createdAt: 2,
      updatedAt: 2,
    },
  ];

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
      await route.fulfill({ json: accountPreferences() });
      return;
    }

    if (url.pathname === "/api/catalog/cards") {
      await route.fulfill({ json: { cards: catalogCards } });
      return;
    }

    if (url.pathname === "/api/progression") {
      await route.fulfill({ json: progression() });
      return;
    }

    if (url.pathname === "/api/decks" && request.method() === "GET") {
      await route.fulfill({ json: { rules: deckRules(), decks: accountDecks } });
      return;
    }

    const publicDeckMatch = url.pathname.match(/^\/api\/users\/([^/]+)\/decks\/(\d+)$/);
    if (publicDeckMatch && request.method() === "GET") {
      const [, handle, deckId] = publicDeckMatch;
      const deck = accountDecks.find((candidate) => candidate.id === Number(deckId));
      if (handle !== "rune-player" || !deck) {
        await route.fulfill({ status: 404, json: { message: "Deck recipe was not found." } });
        return;
      }
      await route.fulfill({
        json: {
          owner: {
            id: 1,
            handle: "rune-player",
            displayName: "Rune Player",
            avatar: { symbol: "spark", color: "emerald" },
          },
          deck,
        },
      });
      return;
    }

    if (url.pathname === "/api/decks/legality-preview" && request.method() === "POST") {
      const body = JSON.parse(request.postData() ?? "{}");
      await route.fulfill({ json: legalityFor(body.cards ?? []) });
      return;
    }

    const deckMatch = url.pathname.match(/^\/api\/decks\/(\d+)$/);
    if (deckMatch && request.method() === "PATCH") {
      const deckId = Number(deckMatch[1]);
      const body = JSON.parse(request.postData() ?? "{}");
      savedRequests.push(body);
      const updatedDeck = {
        id: deckId,
        name: body.name,
        isDefault: body.isDefault,
        heroType: body.heroType,
        runeIds: body.runeIds,
        cards: body.cards,
        legality: legalityFor(body.cards),
        createdAt: 1,
        updatedAt: 3,
      };
      accountDecks = accountDecks.map((deck) => (deck.id === deckId ? updatedDeck : deck));
      await route.fulfill({ json: updatedDeck });
      return;
    }

    if (url.pathname === "/api/decks" && request.method() === "POST") {
      const created = {
        id: 12,
        name: "New Deck",
        isDefault: false,
        heroType: "runekeeper",
        runeIds: [],
        cards: [],
        legality: legalityFor([]),
        createdAt: 4,
        updatedAt: 4,
      };
      accountDecks = [created, ...accountDecks];
      await route.fulfill({ json: created });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

function legalityFor(cards) {
  const totals = cards.reduce(
    (current, card) => {
      const catalogCard = catalogCards.find((candidate) => candidate.templateId === card.templateId);
      if (!catalogCard) {
        return current;
      }

      return {
        totalCards: current.totalCards + card.count,
        basicCards: current.basicCards + (catalogCard.rarity === "basic" ? card.count : 0),
        advancedCards: current.advancedCards + (catalogCard.rarity === "advanced" ? card.count : 0),
        rareCards: current.rareCards + (catalogCard.rarity === "rare" ? card.count : 0),
      };
    },
    { totalCards: 0, basicCards: 0, advancedCards: 0, rareCards: 0 },
  );

  return {
    legal: totals.totalCards >= deckRules().minCards,
    ...totals,
    messages: totals.totalCards >= deckRules().minCards ? [] : ["Deck recipe needs at least 30 cards."],
  };
}

function authUser() {
  return {
    id: 1,
    handle: "rune-player",
    email: "player@local.dev",
    displayName: "Rune Player",
    avatar: { symbol: "spark", color: "emerald" },
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

function accountPreferences() {
  return {
    theme: "system",
    motion: "system",
    animationSpeed: "normal",
    boardScale: "normal",
    hotkeys: [],
    updatedAt: null,
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

function deckRules() {
  return {
    maxDecksPerAccount: 12,
    minCards: 30,
    basicCopyLimit: 4,
    advancedCopyLimit: 3,
    rareCopyLimit: 1,
    advancedTotalLimit: 12,
    rareTotalLimit: 6,
  };
}

const catalogCards = [
  {
    id: "ember-squire-catalog",
    templateId: "ember-squire",
    name: "Ember Squire",
    rarity: "basic",
    cost: 1,
    text: "Summon a steady unit.",
    kind: { type: "unit", attack: 1, armor: 2, maxAp: 2 },
    copyCount: 4,
    artKey: "ember-squire",
    artPath: "/card-art/ember-squire.svg",
  },
  {
    id: "starfire-bolt-catalog",
    templateId: "starfire-bolt",
    name: "Starfire Bolt",
    rarity: "advanced",
    cost: 2,
    text: "Deal damage to a unit.",
    kind: { type: "spell", range: 3, priority: 2, effect: { type: "damage", amount: 2 } },
    copyCount: 3,
    artKey: "starfire-bolt",
    artPath: "/card-art/starfire-bolt.svg",
  },
  {
    id: "meteor-bloom-catalog",
    templateId: "meteor-bloom",
    name: "Meteor Bloom",
    rarity: "rare",
    cost: 5,
    text: "Area damage around a Hex.",
    kind: {
      type: "spell",
      range: 4,
      priority: 1,
      effect: { type: "areaDamage", amount: 3, radius: 1 },
    },
    copyCount: 1,
    artKey: "meteor-bloom",
    artPath: "/card-art/meteor-bloom.svg",
  },
];
