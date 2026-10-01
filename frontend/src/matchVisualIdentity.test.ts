import { describe, expect, it } from "vitest";
import { createCardArtworkCatalog } from "./cardArtworkCatalog";
import { createMatchVisualCatalog } from "./matchVisualIdentity";
import type {
  Card,
  CardSummary,
  CatalogCard,
  Unit,
  Hero,
  HeroType,
} from "./types";

const emberSquire = {
  id: "ember-squire",
  templateId: "ember-squire",
  name: "Ember Squire",
  rarity: "basic",
  cost: 1,
  text: "1 attack / 2 armor / 2 AP.",
  kind: {
    type: "unit",
    attack: 1,
    armor: 2,
    maxAp: 2,
  },
  copyCount: 5,
  artKey: "ember-squire",
  artPath: "/card-art/ember-squire.svg",
} satisfies CatalogCard;

const sparkJolt = {
  id: "spark-jolt",
  templateId: "spark-jolt",
  name: "Spark Jolt",
  rarity: "basic",
  cost: 1,
  text: "Priority 3. Range 2. Deal 1 damage to an enemy unit or hero.",
  kind: {
    type: "spell",
    range: 2,
    priority: 3,
    effect: {
      type: "damage",
      amount: 1,
    },
  },
  copyCount: 5,
  artKey: "spark-jolt",
  artPath: "/card-art/spark-jolt.svg",
} satisfies CatalogCard;

const catalog = createMatchVisualCatalog([emberSquire, sparkJolt]);

describe("createMatchVisualCatalog", () => {
  it("keeps explicit artwork failures out of legacy name fallback and rejects another Card's binding", () => {
    const revision = { cardId: "custom-squire", revision: 1 };
    const other = { cardId: "other-card", revision: 1 };
    const prepared = createCardArtworkCatalog({
      identities: [
        { id: "valid", cardRevision: revision, artworkAssetId: "art" },
        { id: "other", cardRevision: other, artworkAssetId: "art" },
        { id: "missing", cardRevision: revision, artworkAssetId: "unavailable" },
        { id: "absent", cardRevision: revision, artworkAssetId: null },
      ], assets: [{ id: "art", path: "/card-art/stoneguard.svg" }],
    }, [revision, other]);
    if (!prepared.ok) {
      throw new Error("artwork catalog should prepare");
    }
    const card = { ...emberSquire, id: "player-custom", templateId: "custom-squire", cost: 2 } satisfies Card;
    const visuals = createMatchVisualCatalog([emberSquire], prepared.catalog);
    for (const binding of [
      { cardRevision: revision, visualIdentityId: "unknown" },
      { cardRevision: revision, visualIdentityId: "missing" },
      { cardRevision: revision, visualIdentityId: "absent" },
      { cardRevision: revision, visualIdentityId: null },
      { cardRevision: { ...revision, revision: 2 }, visualIdentityId: "valid" },
      { cardRevision: other, visualIdentityId: "other" },
    ]) {
      expect(visuals.card(card, binding)).toMatchObject({
        status: "unknown", templateId: "custom-squire", name: "Ember Squire", cost: 2, artPath: null,
      });
    }
    expect(createMatchVisualCatalog([emberSquire]).card(card, {
      cardRevision: revision, visualIdentityId: "valid",
    }).artPath).toBeNull();
    expect(visuals.card(card).status).toBe("legacyNameFallback");
  });

  it("swaps exact-revision artwork while preserving the authoritative Card fields and identity", () => {
    const card = {
      ...emberSquire, id: "player-custom-card", templateId: "custom-squire", cost: 2,
      text: "A published custom Unit.", kind: { type: "unit", attack: 3, armor: 4, maxAp: 2 },
    } satisfies Card;
    const revision = { cardId: "custom-squire", revision: 1 };
    const prepared = createCardArtworkCatalog({
      identities: [
        { id: "original", cardRevision: revision, artworkAssetId: "red" },
        { id: "alternate", cardRevision: revision, artworkAssetId: "blue" },
      ],
      assets: [
        { id: "red", path: "/card-art/ember-squire.svg" },
        { id: "blue", path: "/card-art/stoneguard.svg" },
      ],
    }, [revision]);
    if (!prepared.ok) {
      throw new Error("artwork catalog should prepare");
    }
    const before = structuredClone(card);
    const visuals = createMatchVisualCatalog([emberSquire], prepared.catalog);
    const original = visuals.card(card, { cardRevision: revision, visualIdentityId: "original" });
    const alternate = visuals.card(card, { cardRevision: revision, visualIdentityId: "alternate" });
    expect(alternate).toMatchObject({
      status: "resolved", templateId: "custom-squire", name: "Ember Squire", cost: 2,
      text: "A published custom Unit.", kind: { type: "unit", attack: 3, armor: 4, maxAp: 2 },
      artPath: "/card-art/stoneguard.svg",
    });
    expect({ ...alternate, artPath: original.artPath }).toEqual(original);
    expect(card).toEqual(before);
  });

  it("resolves a hand Card by templateId", () => {
    const card = {
      ...emberSquire,
      id: "player-1-ember-squire",
    } satisfies Card;

    expect(catalog.card(card)).toMatchObject({
      status: "resolved",
      templateId: "ember-squire",
      name: "Ember Squire",
      artPath: "/card-art/ember-squire.svg",
      accentClass: "basic",
    });
  });

  it("resolves a CatalogCard directly by templateId", () => {
    expect(catalog.card(emberSquire)).toMatchObject({
      status: "resolved",
      templateId: "ember-squire",
      artPath: "/card-art/ember-squire.svg",
    });
  });

  it("resolves a CardSummary by templateId", () => {
    const summary = {
      templateId: "spark-jolt",
      name: "Spark Jolt",
      rarity: "basic",
      cost: 1,
      kind: sparkJolt.kind,
    } satisfies CardSummary;

    const visual = catalog.card(summary);

    expect(visual).toMatchObject({
      status: "resolved",
      templateId: "spark-jolt",
      artPath: "/card-art/spark-jolt.svg",
      text: sparkJolt.text,
    });
    expect(visual.kind).toEqual(sparkJolt.kind);
  });

  it("resolves a Unit by templateId", () => {
    const unit = makeUnit({ templateId: "ember-squire", name: "Renamed Squire" });

    expect(catalog.unit(unit)).toMatchObject({
      status: "resolved",
      templateId: "ember-squire",
      name: "Ember Squire",
      portraitPath: "/card-art/ember-squire.svg",
      rarity: "basic",
      baseStats: {
        cost: 1,
        attack: 1,
        armor: 2,
        maxAp: 2,
        text: emberSquire.text,
      },
    });
  });

  it("resolves a legacy Unit by exact name when templateId is missing", () => {
    const unit = makeUnit({ templateId: undefined, name: "Ember Squire" });

    expect(catalog.unit(unit)).toMatchObject({
      status: "legacyNameFallback",
      templateId: "ember-squire",
      name: "Ember Squire",
      portraitPath: "/card-art/ember-squire.svg",
    });
  });

  it("returns an unknown Unit visual for missing templateId and unmatched name", () => {
    const unit = makeUnit({ templateId: undefined, name: "Forgotten Guardian" });

    expect(catalog.unit(unit)).toMatchObject({
      status: "unknown",
      templateId: null,
      name: "Forgotten Guardian",
      rarity: "unknown",
      portraitPath: null,
      fallbackLabel: "FOR",
      baseStats: null,
    });
  });

  it("keeps the unknown Unit fallback label stable and short", () => {
    const unit = makeUnit({ templateId: undefined, name: "  Ash Hound  " });

    expect(createMatchVisualCatalog([]).unit(unit).fallbackLabel).toBe("ASH");
  });

  it("resolves every HeroType to a portrait path without catalog cards", () => {
    const emptyCatalog = createMatchVisualCatalog([]);
    const heroTypes = [
      "runekeeper",
      "pyromancer",
      "chronomancer",
      "warden",
      "battlemage",
      "barbarian",
      "archer",
      "builder",
    ] satisfies HeroType[];

    for (const heroType of heroTypes) {
      expect(emptyCatalog.hero(makeHero(heroType))).toMatchObject({
        status: "resolved",
        heroType,
        portraitPath: `/hero-art/${heroType}.svg`,
      });
    }
  });

  it("does not use fuzzy matching for names", () => {
    const unit = makeUnit({ templateId: undefined, name: "ember squire" });

    expect(catalog.unit(unit).status).toBe("unknown");
  });
});

function makeUnit(overrides: { templateId?: string; name: string }): Unit {
  return {
    id: "unit-1",
    side: "player",
    name: overrides.name,
    templateId: overrides.templateId,
    attack: 1,
    attackRange: 1,
    armor: 2,
    maxArmor: 2,
    position: { q: 0, r: 0 },
    apRemaining: 1,
    maxAp: 2,
    hasAttacked: false,
    items: [],
  };
}

function makeHero(heroType: HeroType): Hero {
  return {
    id: "hero-player",
    side: "player",
    heroType,
    hp: 20,
    maxHp: 20,
    attack: 1,
    attackRange: 1,
    position: { q: 0, r: 0 },
    apRemaining: 3,
    maxAp: 3,
    hasAttacked: false,
  };
}
