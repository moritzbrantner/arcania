import type { HeroType } from "./types";

export type HeroOption = {
  id: HeroType;
  name: string;
  role: string;
  hp: number;
  attack: number;
  ap: number;
  text: string;
  token: string;
};

export const HERO_OPTIONS = [
  {
    id: "runekeeper",
    name: "Runekeeper",
    role: "Balanced",
    hp: 20,
    attack: 1,
    ap: 3,
    text: "Steady stats for flexible card play.",
    token: "Run",
  },
  {
    id: "pyromancer",
    name: "Pyromancer",
    role: "Aggressive",
    hp: 18,
    attack: 2,
    ap: 3,
    text: "Higher melee damage with a smaller health pool.",
    token: "Pyr",
  },
  {
    id: "chronomancer",
    name: "Chronomancer",
    role: "Mobile",
    hp: 16,
    attack: 1,
    ap: 4,
    text: "Extra action point for repositioning, attacks, and activations. Summons spend Mana only.",
    token: "Chr",
  },
  {
    id: "warden",
    name: "Warden",
    role: "Defensive",
    hp: 24,
    attack: 1,
    ap: 2,
    text: "Durable but slower across the board.",
    token: "War",
  },
  {
    id: "battlemage",
    name: "Battlemage",
    role: "Bruiser",
    hp: 20,
    attack: 2,
    ap: 2,
    text: "Tougher frontline duelist with fewer actions.",
    token: "Bat",
  },
  {
    id: "barbarian",
    name: "Barbarian",
    role: "Brawler",
    hp: 22,
    attack: 3,
    ap: 2,
    text: "Heavy melee pressure that gains mana when the hero destroys units.",
    token: "Bar",
  },
  {
    id: "archer",
    name: "Archer",
    role: "Skirmisher",
    hp: 16,
    attack: 2,
    ap: 4,
    text: "Fast positioning with range-2 hero attacks.",
    token: "Arc",
  },
  {
    id: "builder",
    name: "Builder",
    role: "Support",
    hp: 24,
    attack: 1,
    ap: 2,
    text: "Durable support for armor-heavy boards.",
    token: "Bld",
  },
] as const satisfies readonly HeroOption[];
