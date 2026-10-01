import { expect, test } from "@playwright/test";

const EXPERIENCED_EMAIL = "experienced@local.dev";
const EXPERIENCED_PASSWORD = "experienced";
const AUTH_TOKEN = "experienced-e2e-token";

test("selects hero specs and resets them", async ({ page }) => {
  await mockExperiencedAccountApi(page);
  await signInExperiencedAccount(page);

  await page.getByRole("button", { name: "Pyromancer" }).click();

  const heatedFocus = page.getByRole("button", { name: /Heated Focus/ });
  const scorchingScript = page.getByRole("button", { name: /Scorching Script/ });

  await expect(heatedFocus).toHaveAttribute("aria-pressed", "false");
  await expect(heatedFocus).toBeEnabled();
  await expect(scorchingScript).toBeDisabled();

  await heatedFocus.click();

  await expect(heatedFocus).toHaveAttribute("aria-pressed", "true");
  await expect(heatedFocus).toBeDisabled();
  await expect(scorchingScript).toBeEnabled();

  await page.getByRole("button", { name: /Respec/ }).click();

  await expect(heatedFocus).toHaveAttribute("aria-pressed", "false");
  await expect(heatedFocus).toBeEnabled();
  await expect(scorchingScript).toBeDisabled();
});

test("selects and replaces default rune loadout within slot limit", async ({ page }) => {
  await mockExperiencedAccountApi(page);
  await signInExperiencedAccount(page);

  const vitality = page.getByRole("button", { name: /Vitality Rune/ });
  const force = page.getByRole("button", { name: /Force Rune/ });
  const foresight = page.getByRole("button", { name: /Foresight Rune/ });

  await vitality.click();
  await force.click();

  await expect(vitality).toHaveAttribute("aria-pressed", "true");
  await expect(force).toHaveAttribute("aria-pressed", "true");
  await expect(foresight).toBeDisabled();

  await vitality.click();

  await expect(vitality).toHaveAttribute("aria-pressed", "false");
  await expect(foresight).toBeEnabled();

  await foresight.click();

  await expect(foresight).toHaveAttribute("aria-pressed", "true");
  await expect(force).toHaveAttribute("aria-pressed", "true");
});

test("retries a failed preferred-Hero save without losing the account preference", async ({ page }) => {
  await mockExperiencedAccountApi(page);
  let attempts = 0;
  await page.route("**/api/profile/preferred-hero", async (route) => {
    expect(route.request().method()).toBe("PATCH");
    expect(route.request().postDataJSON()).toEqual({ heroType: "pyromancer" });
    attempts += 1;
    if (attempts === 1) {
      await route.fulfill({ status: 503, json: { message: "Could not save your preferred Hero." } });
    } else {
      await route.fulfill({ json: { ...experiencedUser(), preferredHeroType: "pyromancer" } });
    }
  });
  await signInExperiencedAccount(page);
  await page.getByRole("button", { name: "Pyromancer", exact: true }).click();
  await page.getByRole("button", { name: "Make preferred" }).click();
  await expect(page.getByRole("alert")).toHaveText("Could not save your preferred Hero.");
  await expect(page.getByRole("button", { name: "Make preferred" })).toBeEnabled();
  await page.getByRole("button", { name: "Runekeeper", exact: true }).click();
  await expect(page.getByRole("button", { name: "Preferred Hero" })).toBeDisabled();
  await page.getByRole("button", { name: "Pyromancer", exact: true }).click();
  await page.getByRole("button", { name: "Make preferred" }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Preferred Hero" })).toBeDisabled();
  expect(attempts).toBe(2);
});

async function signInExperiencedAccount(page) {
  await page.goto("/heroes");
  await page.getByLabel("Email").fill(EXPERIENCED_EMAIL);
  await page.getByLabel("Password").fill(EXPERIENCED_PASSWORD);
  await page.getByRole("button", { name: /Sign In/ }).click();
  await expect(page.getByRole("heading", { name: "Heroes" })).toBeVisible();
  await expect(page.getByLabel("Hero skill tree")).toBeVisible();
}

async function mockExperiencedAccountApi(page) {
  let progression = experiencedProgression();

  await page.route("**://*/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (!url.pathname.startsWith("/api/")) {
      await route.fallback();
      return;
    }

    if (url.pathname === "/api/auth/login" && request.method() === "POST") {
      const body = JSON.parse(request.postData() ?? "{}");
      if (body.email === EXPERIENCED_EMAIL && body.password === EXPERIENCED_PASSWORD) {
        await route.fulfill({ json: { token: AUTH_TOKEN, user: experiencedUser() } });
        return;
      }
      await route.fulfill({ status: 401, json: { message: "Invalid email or password." } });
      return;
    }

    if (url.pathname === "/api/auth/me") {
      await route.fulfill({ json: experiencedUser() });
      return;
    }

    if (url.pathname === "/api/profile/matches") {
      await route.fulfill({ json: { matches: [] } });
      return;
    }

    if (url.pathname === "/api/progression" && request.method() === "GET") {
      await route.fulfill({ json: progression });
      return;
    }

    const skillMatch = url.pathname.match(/^\/api\/progression\/heroes\/([^/]+)\/skills\/([^/]+)$/);
    if (skillMatch && request.method() === "POST") {
      const [, heroType, nodeId] = skillMatch;
      progression = unlockSkill(progression, heroType, decodeURIComponent(nodeId));
      await route.fulfill({ json: progression });
      return;
    }

    const respecMatch = url.pathname.match(/^\/api\/progression\/heroes\/([^/]+)\/respec$/);
    if (respecMatch && request.method() === "POST") {
      const [, heroType] = respecMatch;
      progression = respecHero(progression, heroType);
      await route.fulfill({ json: progression });
      return;
    }

    const loadoutMatch = url.pathname.match(/^\/api\/progression\/heroes\/([^/]+)\/loadout$/);
    if (loadoutMatch && request.method() === "PATCH") {
      const [, heroType] = loadoutMatch;
      const body = JSON.parse(request.postData() ?? "{}");
      progression = saveLoadout(progression, heroType, body.runeIds ?? []);
      await route.fulfill({ json: progression });
      return;
    }

    await route.fulfill({ status: 404, json: { message: "Not found" } });
  });
}

function experiencedUser() {
  return {
    id: 100,
    handle: "experienced",
    email: EXPERIENCED_EMAIL,
    displayName: "Experienced",
    avatar: { symbol: "sparkles", color: "emerald" },
    preferredHeroType: "runekeeper",
    boardVisualMode: "3d",
    progressionSummary: experiencedProgression().account,
  };
}

function unlockSkill(progression, heroType, nodeId) {
  return {
    ...progression,
    heroes: progression.heroes.map((hero) => {
      if (hero.heroType !== heroType || hero.unlockedSkillIds.includes(nodeId)) {
        return hero;
      }
      const unlockedSkillIds = [...hero.unlockedSkillIds, nodeId];
      return {
        ...hero,
        unlockedSkillIds,
        spentSkillPoints: unlockedSkillIds.length,
        availableSkillPoints: hero.totalSkillPoints - unlockedSkillIds.length,
      };
    }),
  };
}

function respecHero(progression, heroType) {
  return {
    ...progression,
    heroes: progression.heroes.map((hero) =>
      hero.heroType === heroType
        ? {
            ...hero,
            unlockedSkillIds: [],
            spentSkillPoints: 0,
            availableSkillPoints: hero.totalSkillPoints,
          }
        : hero,
    ),
  };
}

function saveLoadout(progression, heroType, runeIds) {
  return {
    ...progression,
    loadouts: progression.loadouts.map((loadout) =>
      loadout.heroType === heroType ? { ...loadout, runeIds } : loadout,
    ),
  };
}

function experiencedProgression() {
  const heroTypes = [
    "runekeeper",
    "pyromancer",
    "chronomancer",
    "warden",
    "battlemage",
    "barbarian",
    "archer",
    "builder",
  ];
  return {
    account: {
      totalXp: 20_000,
      level: 20,
      currentLevelXp: 19_000,
      nextLevelXp: 21_000,
      xpIntoLevel: 1_000,
      xpToNextLevel: 1_000,
      runeSlots: 2,
    },
    runes: [
      {
        id: "vitality",
        name: "Vitality Rune",
        text: "Hero starts with +2 max HP.",
        unlockLevel: 2,
        unlocked: true,
      },
      {
        id: "force",
        name: "Force Rune",
        text: "Hero starts with +1 attack.",
        unlockLevel: 4,
        unlocked: true,
      },
      {
        id: "foresight",
        name: "Foresight Rune",
        text: "Draw +1 opening hand card.",
        unlockLevel: 6,
        unlocked: true,
      },
      {
        id: "wellspring",
        name: "Wellspring Rune",
        text: "Gain +1 natural mana.",
        unlockLevel: 8,
        unlocked: true,
      },
      {
        id: "bulwark",
        name: "Bulwark Rune",
        text: "Summoned units enter with +1 armor.",
        unlockLevel: 12,
        unlocked: true,
      },
    ],
    heroes: heroTypes.map((heroType) => ({
      heroType,
      xp: 20_000,
      level: 20,
      currentLevelXp: 19_000,
      nextLevelXp: 21_000,
      xpIntoLevel: 1_000,
      xpToNextLevel: 1_000,
      totalSkillPoints: 19,
      spentSkillPoints: 0,
      availableSkillPoints: 19,
      unlockedSkillIds: [],
    })),
    skillTrees: [
      skillTree("runekeeper", [
        ["runekeeper-runic-balance", "Runic Balance", "The root of Runekeeper mastery.", true, null],
        ["runekeeper-steady-glyph", "Steady Glyph", "Hero starts with +1 max HP.", false, "runekeeper-runic-balance"],
        ["runekeeper-channel-stone", "Channel Stone", "Gain +1 natural mana.", false, "runekeeper-runic-balance"],
        [
          "runekeeper-warding-script",
          "Warding Script",
          "First summoned unit each match enters with +1 armor.",
          false,
          "runekeeper-steady-glyph",
        ],
        ["runekeeper-archive-spark", "Archive Spark", "Draw +1 opening hand card.", false, "runekeeper-channel-stone"],
      ]),
      skillTree("pyromancer", [
        ["pyromancer-ember-path", "Ember Path", "The root of Pyromancer mastery.", true, null],
        ["pyromancer-heated-focus", "Heated Focus", "Hero starts with +1 attack.", false, "pyromancer-ember-path"],
        ["pyromancer-kindling-reserve", "Kindling Reserve", "Gain +1 natural mana.", false, "pyromancer-ember-path"],
        ["pyromancer-scorching-script", "Scorching Script", "Damaging spells deal +1 damage.", false, "pyromancer-heated-focus"],
        [
          "pyromancer-glass-flame",
          "Glass Flame",
          "Draw +1 opening hand card and start with -1 max HP.",
          false,
          "pyromancer-kindling-reserve",
        ],
      ]),
      skillTree("chronomancer", [
        ["chronomancer-time-thread", "Time Thread", "The root of Chronomancer mastery.", true, null],
        ["chronomancer-quick-step", "Quick Step", "Hero starts with +1 max AP.", false, "chronomancer-time-thread"],
        ["chronomancer-stored-moment", "Stored Moment", "Gain +1 natural mana.", false, "chronomancer-time-thread"],
        ["chronomancer-early-loop", "Early Loop", "Draw +1 opening hand card.", false, "chronomancer-quick-step"],
        ["chronomancer-temporal-guard", "Temporal Guard", "Hero starts with +1 max HP.", false, "chronomancer-stored-moment"],
      ]),
      skillTree("warden", [
        ["warden-stone-oath", "Stone Oath", "The root of Warden mastery.", true, null],
        ["warden-stone-skin", "Stone Skin", "Hero starts with +2 max HP.", false, "warden-stone-oath"],
        ["warden-guard-drill", "Guard Drill", "Summoned units enter with +1 armor.", false, "warden-stone-oath"],
        [
          "warden-anchored-stance",
          "Anchored Stance",
          "Hero starts with +1 max HP and gains +1 natural mana.",
          false,
          "warden-stone-skin",
        ],
        ["warden-shield-line", "Shield Line", "First summoned unit each match enters with +1 armor.", false, "warden-guard-drill"],
      ]),
      skillTree("battlemage", [
        ["battlemage-duelist-oath", "Duelist Oath", "The root of Battlemage mastery.", true, null],
        ["battlemage-weapon-drill", "Weapon Drill", "Hero starts with +1 attack.", false, "battlemage-duelist-oath"],
        ["battlemage-iron-focus", "Iron Focus", "Hero starts with +1 max HP.", false, "battlemage-duelist-oath"],
        ["battlemage-battle-rhythm", "Battle Rhythm", "Gain +1 natural mana.", false, "battlemage-weapon-drill"],
        [
          "battlemage-frontline-command",
          "Frontline Command",
          "Summoned units enter with +1 armor.",
          false,
          "battlemage-iron-focus",
        ],
      ]),
      skillTree("barbarian", [
        ["barbarian-fury-path", "Fury Path", "The root of Barbarian mastery.", true, null],
        ["barbarian-brutal-stamina", "Brutal Stamina", "Hero starts with +2 max HP.", false, "barbarian-fury-path"],
        ["barbarian-weapon-practice", "Weapon Practice", "Hero starts with +1 attack.", false, "barbarian-fury-path"],
        ["barbarian-battle-hunger", "Battle Hunger", "Gain +1 natural mana.", false, "barbarian-weapon-practice"],
        ["barbarian-warband-hide", "Warband Hide", "Summoned units enter with +1 armor.", false, "barbarian-brutal-stamina"],
        ["barbarian-opening-rage", "Opening Rage", "Draw +1 opening hand card.", false, "barbarian-brutal-stamina"],
        ["barbarian-deep-cuts", "Deep Cuts", "Damaging spells deal +1 damage.", false, "barbarian-weapon-practice"],
      ]),
      skillTree("archer", [
        ["archer-long-watch", "Long Watch", "The root of Archer mastery.", true, null],
        ["archer-fleet-footing", "Fleet Footing", "Hero starts with +1 max AP.", false, "archer-long-watch"],
        ["archer-keen-shot", "Keen Shot", "Damaging spells deal +1 damage.", false, "archer-long-watch"],
        ["archer-scout-cache", "Scout Cache", "Draw +1 opening hand card.", false, "archer-fleet-footing"],
        ["archer-trail-rations", "Trail Rations", "Gain +1 natural mana.", false, "archer-keen-shot"],
        ["archer-screening-line", "Screening Line", "First summoned unit each match enters with +1 armor.", false, "archer-fleet-footing"],
        ["archer-light-armor", "Light Armor", "Hero starts with +1 max HP.", false, "archer-keen-shot"],
      ]),
      skillTree("builder", [
        ["builder-foundation-plan", "Foundation Plan", "The root of Builder mastery.", true, null],
        ["builder-reinforced-frame", "Reinforced Frame", "Hero starts with +2 max HP.", false, "builder-foundation-plan"],
        ["builder-supply-cache", "Supply Cache", "Gain +1 natural mana.", false, "builder-foundation-plan"],
        ["builder-work-crew-drill", "Work Crew Drill", "Summoned units enter with +1 armor.", false, "builder-reinforced-frame"],
        ["builder-first-wall", "First Wall", "First summoned unit each match enters with +1 armor.", false, "builder-reinforced-frame"],
        ["builder-field-manual", "Field Manual", "Draw +1 opening hand card.", false, "builder-supply-cache"],
        ["builder-tool-ready", "Tool Ready", "Hero starts with +1 max AP.", false, "builder-supply-cache"],
      ]),
    ],
    loadouts: heroTypes.map((heroType) => ({ heroType, runeIds: [] })),
  };
}

function skillTree(heroType, nodes) {
  return {
    heroType,
    nodes: nodes.map(([id, name, text, root, prerequisiteId]) => ({
      id,
      name,
      text,
      root,
      prerequisiteId,
    })),
  };
}
