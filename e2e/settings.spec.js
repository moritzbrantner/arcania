import { expect, test } from "@playwright/test";

const AUTH_TOKEN_STORAGE_KEY = "arcania-auth-token";
const BOARD_VISUAL_MODE_STORAGE_KEY = "arcania-board-visual-mode";

test("signed-out players save local board visual mode from settings", async ({ page }) => {
  await mockSettingsApi(page);
  await page.addInitScript(
    ({ authKey, boardKey }) => {
      localStorage.removeItem(authKey);
      if (localStorage.getItem(boardKey) === null) {
        localStorage.setItem(boardKey, "3d");
      }
    },
    { authKey: AUTH_TOKEN_STORAGE_KEY, boardKey: BOARD_VISUAL_MODE_STORAGE_KEY },
  );

  await page.goto("/settings");

  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByText("Local preferences")).toBeVisible();
  await expect(page.getByLabel("Theme")).toHaveCount(0);
  await expect(page.getByLabel("Board visual mode")).toHaveValue("3d");
  await expect(page.getByRole("button", { name: "Sign out" })).toHaveCount(0);

  await page.getByLabel("Board visual mode").selectOption("2d");
  await page.getByRole("button", { name: "Save Settings" }).click();

  await expect(page.getByText("Settings saved.")).toBeVisible();
  await expect
    .poll(() => page.evaluate((key) => localStorage.getItem(key), BOARD_VISUAL_MODE_STORAGE_KEY))
    .toBe("2d");

  await page.reload();
  await expect(page.getByLabel("Board visual mode")).toHaveValue("2d");
});

test("settings load, save, apply, and persist visual preferences", async ({ page }) => {
  let preferences = defaultPreferences();
  const savedPayloads = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockSettingsApi(page, {
    loadPreferences: () => preferences,
    savePreferences: (payload) => {
      savedPayloads.push(payload);
      preferences = { ...payload, updatedAt: 99 };
      return preferences;
    },
  });

  const preferencesLoaded = waitForPreferencesLoad(page);
  await page.goto("/settings");
  await preferencesLoaded;

  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByText("Loading settings.")).toBeHidden();
  await expect(page.getByLabel("Theme")).toBeEnabled();
  await expect(page.getByLabel("Theme")).toHaveValue("system");
  await expect(page.getByLabel("Board visual mode")).toHaveValue("2d");

  await page.getByLabel("Theme").selectOption("highContrast");
  await page.getByLabel("Motion").selectOption("reduced");
  await page.getByLabel("Animation speed").selectOption("fast");
  await page.getByLabel("Board scale").selectOption("large");
  await page.getByLabel("Board visual mode").selectOption("3d");
  await page.getByRole("button", { name: "Save Settings" }).click();

  await expect(page.getByText("Settings saved.")).toBeVisible();
  expect(savedPayloads).toHaveLength(1);
  expect(savedPayloads[0]).toMatchObject({
    theme: "highContrast",
    motion: "reduced",
    animationSpeed: "fast",
    boardScale: "large",
    boardVisualMode: "3d",
  });
  expect(savedPayloads[0].hotkeys).toHaveLength(15);
  await expect
    .poll(() => page.evaluate(() => document.documentElement.getAttribute("data-theme")))
    .toBe("highContrast");
  await expect
    .poll(() => page.evaluate(() => document.documentElement.getAttribute("data-motion")))
    .toBe("reduced");
  await expect
    .poll(() => page.evaluate(() => document.documentElement.getAttribute("data-animation-speed")))
    .toBe("fast");
  await expect
    .poll(() => page.evaluate(() => document.documentElement.getAttribute("data-board-scale")))
    .toBe("large");

  await page.reload();

  await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  await expect(page.getByLabel("Theme")).toHaveValue("highContrast");
  await expect(page.getByLabel("Motion")).toHaveValue("reduced");
  await expect(page.getByLabel("Animation speed")).toHaveValue("fast");
  await expect(page.getByLabel("Board scale")).toHaveValue("large");
  await expect(page.getByLabel("Board visual mode")).toHaveValue("3d");
});

test("settings record, validate, reset, and persist hotkeys", async ({ page }) => {
  let preferences = defaultPreferences();
  const savedPayloads = [];
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockSettingsApi(page, {
    loadPreferences: () => preferences,
    savePreferences: (payload) => {
      savedPayloads.push(payload);
      preferences = { ...payload, updatedAt: 101 };
      return preferences;
    },
  });

  await page.goto("/settings");

  await expect(page.getByRole("button", { name: "End Turn hotkey" })).toContainText("T");

  await page.getByRole("button", { name: "End Turn hotkey" }).click();
  await page.keyboard.press("Y");
  await expect(page.getByRole("button", { name: "End Turn hotkey" })).toContainText("Y");

  await page.getByRole("button", { name: "Pass Priority hotkey" }).click();
  await page.keyboard.press("Y");
  await expect(page.getByText("Already used by End Turn.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Save Settings" })).toBeDisabled();

  await page.getByRole("button", { name: "Pass Priority hotkey" }).click();
  await page.keyboard.press("R");
  await expect(page.getByText("Already used by End Turn.")).toBeHidden();

  await page.getByRole("button", { name: "Open Catalog hotkey" }).click();
  await page.keyboard.press("Escape");
  await expect(page.getByText("Escape is reserved for Cancel.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Save Settings" })).toBeDisabled();

  await page.getByRole("button", { name: "Open Catalog hotkey" }).click();
  await page.keyboard.press("G");
  await page.getByRole("button", { name: "Save Settings" }).click();

  await expect(page.getByText("Settings saved.")).toBeVisible();
  expect(savedPayloads).toHaveLength(1);
  expect(savedPayloads[0].hotkeys).toEqual(
    expect.arrayContaining([
      { commandId: "endTurn", binding: "Y" },
      { commandId: "passPriority", binding: "R" },
      { commandId: "openCatalog", binding: "G" },
    ]),
  );

  await page.reload();

  await expect(page.getByRole("button", { name: "End Turn hotkey" })).toContainText("Y");
  await expect(page.getByRole("button", { name: "Pass Priority hotkey" })).toContainText("R");
  await expect(page.getByRole("button", { name: "Open Catalog hotkey" })).toContainText("G");

  await page.getByRole("button", { name: "Reset Defaults" }).click();
  await expect(page.getByRole("button", { name: "End Turn hotkey" })).toContainText("T");
  await page.getByRole("button", { name: "Save Settings" }).click();

  await expect(page.getByText("Settings saved.")).toBeVisible();
  expect(savedPayloads.at(-1).hotkeys).toEqual(defaultPreferences().hotkeys);
});

test("settings normalize malformed stored hotkeys through defaults", async ({ page }) => {
  await page.addInitScript(
    ({ key }) => localStorage.setItem(key, "existing-token"),
    { key: AUTH_TOKEN_STORAGE_KEY },
  );
  await mockSettingsApi(page, {
    loadPreferences: () => ({
      ...defaultPreferences(),
      hotkeys: [
        { commandId: "endTurn", binding: "Y" },
        { commandId: "passPriority", binding: "Y" },
        { commandId: "openCatalog", binding: "Escape" },
        { commandId: "openDecks", binding: "Shift+K" },
        { commandId: "openMatchArchive", binding: "v" },
      ],
    }),
  });

  await page.goto("/settings");

  await expect(page.getByRole("button", { name: "End Turn hotkey" })).toContainText("Y");
  await expect(page.getByRole("button", { name: "Pass Priority hotkey" })).toContainText("P");
  await expect(page.getByRole("button", { name: "Open Catalog hotkey" })).toContainText("C");
  await expect(page.getByRole("button", { name: "Open Decks hotkey" })).toContainText("K");
  await expect(page.getByRole("button", { name: "Open Match Archive hotkey" })).toContainText("V");
});

async function mockSettingsApi(page, handlers = {}) {
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

    if (url.pathname === "/api/preferences" && request.method() === "GET") {
      await route.fulfill({ json: handlers.loadPreferences?.() ?? defaultPreferences() });
      return;
    }

    if (url.pathname === "/api/preferences" && request.method() === "PATCH") {
      const payload = request.postDataJSON();
      await route.fulfill({
        json: handlers.savePreferences?.(payload) ?? { ...payload, updatedAt: 99 },
      });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

function waitForPreferencesLoad(page) {
  return page.waitForResponse((response) => {
    const request = response.request();
    return new URL(response.url()).pathname === "/api/preferences" && request.method() === "GET";
  });
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

function defaultPreferences() {
  return {
    theme: "system",
    motion: "system",
    animationSpeed: "normal",
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
      { commandId: "endTurn", binding: "T" },
      { commandId: "passPriority", binding: "P" },
      { commandId: "openCardInfo", binding: "I" },
      { commandId: "openSettings", binding: "," },
      { commandId: "openCatalog", binding: "C" },
      { commandId: "openDecks", binding: "K" },
      { commandId: "openMatchArchive", binding: "M" },
    ],
    updatedAt: null,
  };
}
