import type { CardKind, CatalogResponse, HeroType, MatchActionRequest, MatchResponse, Rarity } from "../types";

export type CustomCard = { id: string; name: string; rarity: Rarity; cost: number; text: string; kind: CardKind };
export type HeroRule = { maxHp: number; attack: number; maxAp: number; attackRange: number };
export type WorkshopSetup = {
  seed: number;
  playerHero: HeroType;
  opponentHero: HeroType;
  ruleset: {
    schemaVersion: number;
    arena: { duelRadius: number; twoVTwoRadius: number };
    turn: { baseHeroMana: number; openingHandSize: number; maxCarriedItems: number; defaultAttackRange: number };
    heroes: Record<HeroType, HeroRule>;
  };
  cards: CustomCard[];
};

type BrowserMatch = {
  view(): string;
  journal(): string;
  catalog(): string;
  act(action: string): string;
  free(): void;
};
type Engine = {
  default(): Promise<unknown>;
  BrowserMatch: { new(setup: string): BrowserMatch; restore(journal: string): BrowserMatch };
  workshop_defaults(): string;
  validate_setup(setup: string): string;
  catalog(): string;
};

const SETUP_KEY = "rune-lanes.workshop.v1";
const MATCH_KEY = "rune-lanes.browser-match.v1";
export const BROWSER_MATCH_ID = "browser-solo";
export const pagesMode = import.meta.env.VITE_BROWSER_ONLY === "1";
let enginePromise: Promise<Engine> | undefined;
let match: BrowserMatch | undefined;

async function engine() {
  if (!enginePromise) {
    const url = `${import.meta.env.BASE_URL}engine/rune_lanes_browser.js`;
    enginePromise = import(/* @vite-ignore */ url).then(async (module: Engine) => {
      await module.default();
      return module;
    }).catch((error: unknown) => {
      enginePromise = undefined;
      throw new Error(`Could not load the browser engine. Reload to try again. ${String(error)}`);
    });
  }
  return enginePromise;
}

export async function validateSetup(value: unknown): Promise<WorkshopSetup> {
  const wasm = await engine();
  // Rust deserialization and domain validation are the authority for imported/saved data.
  return JSON.parse(wasm.validate_setup(JSON.stringify(value)));
}

export async function defaultSetup(): Promise<WorkshopSetup> {
  return JSON.parse((await engine()).workshop_defaults());
}

export async function loadSetup() {
  const stored = localStorage.getItem(SETUP_KEY);
  return stored === null ? defaultSetup() : validateSetup(JSON.parse(stored));
}

export async function saveSetup(setup: WorkshopSetup) {
  const valid = await validateSetup(setup);
  localStorage.setItem(SETUP_KEY, JSON.stringify(valid));
  return valid;
}

export function hasBrowserMatch() {
  return localStorage.getItem(MATCH_KEY) !== null;
}

export async function startBrowserMatch(setup: WorkshopSetup): Promise<MatchResponse> {
  const wasm = await engine();
  const candidate = new wasm.BrowserMatch(JSON.stringify(setup));
  try {
    localStorage.setItem(MATCH_KEY, candidate.journal());
  } catch (error) {
    candidate.free();
    throw error;
  }
  match?.free();
  match = candidate;
  return response(match.view());
}

async function currentMatch() {
  if (!match) {
    const journal = localStorage.getItem(MATCH_KEY);
    if (!journal) { throw new Error("No saved browser match. Start one in the workshop."); }
    const wasm = await engine();
    if (!match) { match = wasm.BrowserMatch.restore(journal); }
  }
  return match;
}

function response(value: string): MatchResponse {
  return { ...JSON.parse(value), matchId: BROWSER_MATCH_ID };
}

export async function loadBrowserMatch() {
  return response((await currentMatch()).view());
}

export async function browserAction(action: MatchActionRequest) {
  const wasm = await engine();
  const current = await currentMatch();
  const before = current.journal();
  const result = current.act(JSON.stringify(action));
  try {
    localStorage.setItem(MATCH_KEY, current.journal());
  } catch (error) {
    current.free();
    match = wasm.BrowserMatch.restore(before);
    throw new Error(`Could not save the move; it was rolled back. ${String(error)}`);
  }
  return response(result);
}

export async function browserCatalog(forMatch = false): Promise<CatalogResponse> {
  const catalog: CatalogResponse = JSON.parse(forMatch ? (await currentMatch()).catalog() : (await engine()).catalog());
  return { cards: catalog.cards.map((card) => ({ ...card, artPath: `${import.meta.env.BASE_URL}${card.artPath}` })) };
}
