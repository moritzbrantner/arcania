import { browserRoutePath } from "./routes";
import type {
  Card,
  CardKind,
  CardSummary,
  CatalogCard,
  Rarity,
  Unit,
  Hero,
  HeroType,
} from "./types";
import { HERO_OPTIONS } from "./heroes";

export type VisualIdentityStatus = "resolved" | "legacyNameFallback" | "unknown";

export type CardVisualIdentity = {
  status: VisualIdentityStatus;
  templateId: string | null;
  name: string;
  rarity: Rarity;
  kind: CardKind;
  cost: number;
  text: string;
  artPath: string | null;
  artAlt: string;
  accentClass: Rarity;
};

export type UnitVisualIdentity = {
  status: VisualIdentityStatus;
  templateId: string | null;
  name: string;
  rarity: Rarity | "unknown";
  portraitPath: string | null;
  portraitAlt: string;
  fallbackLabel: string;
  baseStats:
    | {
        cost: number;
        attack: number;
        armor: number;
        maxAp: number;
        text: string;
      }
    | null;
};

export type HeroVisualIdentity = {
  status: "resolved";
  heroType: HeroType;
  name: string;
  portraitPath: string;
  portraitAlt: string;
  accentClass: HeroType;
  fallbackLabel: string;
};

export type MatchVisualCatalog = {
  card(card: Card | CatalogCard | CardSummary): CardVisualIdentity;
  unit(unit: Unit): UnitVisualIdentity;
  hero(hero: Hero): HeroVisualIdentity;
};

export function createMatchVisualCatalog(cards: CatalogCard[]): MatchVisualCatalog {
  const byTemplateId = new Map(cards.map((card) => [card.templateId, card]));
  const unitByName = new Map(
    cards
      .filter((card) => card.kind.type === "unit")
      .map((card) => [card.name, card]),
  );
  const cardByName = new Map(cards.map((card) => [card.name, card]));

  return {
    card(card) {
      const templateMatch = byTemplateId.get(card.templateId);
      if (templateMatch) {
        return cardVisualFromCatalog(templateMatch, "resolved");
      }

      const nameMatch = cardByName.get(card.name);
      if (nameMatch) {
        return cardVisualFromCatalog(nameMatch, "legacyNameFallback");
      }

      return {
        status: "unknown",
        templateId: card.templateId || null,
        name: card.name,
        rarity: card.rarity,
        kind: card.kind,
        cost: card.cost,
        text: textFromSource(card),
        artPath: null,
        artAlt: `${card.name} art unavailable`,
        accentClass: card.rarity,
      };
    },
    unit(unit) {
      const templateMatch = unit.templateId ? unitByTemplateId(byTemplateId, unit.templateId) : null;
      if (templateMatch) {
        return unitVisualFromCatalog(templateMatch, "resolved");
      }

      const nameMatch = unitByName.get(unit.name);
      if (nameMatch) {
        return unitVisualFromCatalog(nameMatch, "legacyNameFallback");
      }

      return {
        status: "unknown",
        templateId: null,
        name: unit.name,
        rarity: "unknown",
        portraitPath: null,
        portraitAlt: `${unit.name} portrait unavailable`,
        fallbackLabel: fallbackLabel(unit.name),
        baseStats: null,
      };
    },
    hero(hero) {
      const option =
        HERO_OPTIONS.find((candidate) => candidate.id === hero.heroType) ??
        HERO_OPTIONS[0];
      return {
        status: "resolved",
        heroType: hero.heroType,
        name: option.name,
        portraitPath: browserRoutePath(`/hero-art/${hero.heroType}.svg`),
        portraitAlt: `${option.name} portrait`,
        accentClass: hero.heroType,
        fallbackLabel: option.token,
      };
    },
  };
}

function cardVisualFromCatalog(
  card: CatalogCard,
  status: Exclude<VisualIdentityStatus, "unknown">,
): CardVisualIdentity {
  return {
    status,
    templateId: card.templateId,
    name: card.name,
    rarity: card.rarity,
    kind: card.kind,
    cost: card.cost,
    text: card.text,
    artPath: browserRoutePath(card.artPath),
    artAlt: `${card.name} art`,
    accentClass: card.rarity,
  };
}

function unitVisualFromCatalog(
  card: CatalogCard,
  status: Exclude<VisualIdentityStatus, "unknown">,
): UnitVisualIdentity {
  if (card.kind.type !== "unit") {
    return {
      status: "unknown",
      templateId: null,
      name: card.name,
      rarity: "unknown",
      portraitPath: null,
      portraitAlt: `${card.name} portrait unavailable`,
      fallbackLabel: fallbackLabel(card.name),
      baseStats: null,
    };
  }

  return {
    status,
    templateId: card.templateId,
    name: card.name,
    rarity: card.rarity,
    portraitPath: browserRoutePath(card.artPath),
    portraitAlt: `${card.name} portrait`,
    fallbackLabel: fallbackLabel(card.name),
    baseStats: {
      cost: card.cost,
      attack: card.kind.attack,
      armor: card.kind.armor,
      maxAp: card.kind.maxAp,
      text: card.text,
    },
  };
}

function unitByTemplateId(
  byTemplateId: Map<string, CatalogCard>,
  templateId: string,
): CatalogCard | null {
  const card = byTemplateId.get(templateId);
  return card?.kind.type === "unit" ? card : null;
}

function textFromSource(card: Card | CatalogCard | CardSummary) {
  return "text" in card ? card.text : "";
}

function fallbackLabel(name: string) {
  const compactName = Array.from(name.trim().replace(/\s+/g, ""));
  return (compactName.slice(0, 3).join("") || "?").toUpperCase();
}
