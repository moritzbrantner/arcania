import type { HeroAppearanceAssignment, HeroType, Side } from "./types";

export type HeroAppearanceId = string;

export const HERO_APPEARANCE_STORAGE_PREFIX = "arcania-hero-appearance";

export function baseHeroAppearanceId(heroType: HeroType): HeroAppearanceId {
  return `${heroType}-base`;
}

export function heroAppearanceStorageKey(heroType: HeroType) {
  return `${HERO_APPEARANCE_STORAGE_PREFIX}:${heroType}`;
}

export function readLocalHeroAppearance(
  heroType: HeroType,
  storage: Storage | null | undefined = browserStorage(),
) {
  return storage?.getItem(heroAppearanceStorageKey(heroType)) || null;
}

export function saveLocalHeroAppearance(
  heroType: HeroType,
  appearanceId: HeroAppearanceId,
  storage: Storage | null | undefined = browserStorage(),
) {
  storage?.setItem(heroAppearanceStorageKey(heroType), appearanceId);
}

export function resolveHeroAppearanceId({
  side,
  heroType,
  assignments,
}: {
  side: Side;
  heroType: HeroType;
  assignments?: HeroAppearanceAssignment[];
}) {
  const assigned = assignments?.find((assignment) => assignment.side === side);
  if (assigned?.heroType === heroType) {
    return assigned.appearanceId;
  }

  return readLocalHeroAppearance(heroType) ?? baseHeroAppearanceId(heroType);
}

function browserStorage() {
  if (typeof window === "undefined") {
    return null;
  }

  return window.localStorage;
}
