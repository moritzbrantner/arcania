import type { Decorator } from "@storybook/react-vite";
import { useEffect, useRef, type ReactNode } from "react";
import type {
  AccountPreferences,
  AuthSessionResponse,
  AuthUser,
  CatalogResponse,
  CreateSharedMatchResponse,
  DeckListResponse,
  DeckRecipeSummary,
  DeckRules,
  MatchArchiveResponse,
  MatchReplayResponse,
  MatchResponse,
  MatchScenarioListResponse,
  MatchSummaryResponse,
  ProgressionResponse,
  PublicDeckRecipeResponse,
  SharedMatchResponse,
  SystemDeckListResponse,
  SystemDeckRecipe,
} from "../types";
import { DEFAULT_ACCOUNT_PREFERENCES } from "../preferences";
import { catalogCards, storyMatch } from "../components/board.fixtures";

export const storyAccount: AuthUser = {
  id: 1,
  handle: "player-one",
  email: "player@example.com",
  displayName: "Player One",
  avatar: {
    symbol: "sparkles",
    color: "emerald",
  },
  preferredHeroType: "runekeeper",
  boardVisualMode: "2d",
  progressionSummary: {
    totalXp: 420,
    level: 3,
    currentLevelXp: 250,
    nextLevelXp: 600,
    xpIntoLevel: 170,
    xpToNextLevel: 180,
    runeSlots: 2,
  },
};

export const storyPreferences: AccountPreferences = {
  ...DEFAULT_ACCOUNT_PREFERENCES,
  boardVisualMode: "2d",
  motion: "reduced",
  updatedAt: 1_700_000_000,
};

export const storyVisualPreferences = {
  preferences: storyPreferences,
  effectiveMotion: "reduced" as const,
  liveAiDelayMs: 0,
};

export const storyDeckRules: DeckRules = {
  maxDecksPerAccount: 12,
  minCards: 12,
  basicCopyLimit: 3,
  advancedCopyLimit: 2,
  rareCopyLimit: 1,
  advancedTotalLimit: 8,
  rareTotalLimit: 4,
};

export const storyDecks: DeckRecipeSummary[] = [
  {
    id: 1,
    name: "Runekeeper Starter",
    isDefault: true,
    heroType: "runekeeper",
    runeIds: ["ember-rune", "warding-rune"],
    cards: catalogCards.slice(0, 8).map((card) => ({ templateId: card.templateId, count: 1 })),
    legality: {
      legal: true,
      totalCards: 12,
      basicCards: 8,
      advancedCards: 3,
      rareCards: 1,
      messages: [],
    },
    createdAt: 1_700_000_000,
    updatedAt: 1_700_000_500,
  },
  {
    id: 2,
    name: "Pyromancer Draft",
    isDefault: false,
    heroType: "pyromancer",
    runeIds: [],
    cards: catalogCards.slice(8, 14).map((card) => ({ templateId: card.templateId, count: 1 })),
    legality: {
      legal: false,
      totalCards: 6,
      basicCards: 4,
      advancedCards: 2,
      rareCards: 0,
      messages: ["Deck needs at least 12 cards."],
    },
    createdAt: 1_700_000_100,
    updatedAt: 1_700_000_300,
  },
];

export const storySystemDecks: SystemDeckRecipe[] = [
  {
    id: "balanced-starter",
    name: "Balanced Starter",
    heroType: "runekeeper",
    cards: catalogCards.slice(0, 12).map((card) => ({ templateId: card.templateId, count: 1 })),
    legality: {
      legal: true,
      totalCards: 12,
      basicCards: 10,
      advancedCards: 2,
      rareCards: 0,
      messages: [],
    },
  },
  {
    id: "ember-pressure",
    name: "Ember Pressure",
    heroType: "pyromancer",
    cards: catalogCards.slice(4, 16).map((card) => ({ templateId: card.templateId, count: 1 })),
    legality: {
      legal: true,
      totalCards: 12,
      basicCards: 8,
      advancedCards: 3,
      rareCards: 1,
      messages: [],
    },
  },
];

export const storyProgression: ProgressionResponse = {
  account: storyAccount.progressionSummary,
  runes: [
    {
      id: "ember-rune",
      name: "Ember Rune",
      text: "Your first spell each round deals +1 damage.",
      unlockLevel: 2,
      unlocked: true,
    },
    {
      id: "warding-rune",
      name: "Warding Rune",
      text: "Your hero starts with +1 shield.",
      unlockLevel: 3,
      unlocked: true,
    },
    {
      id: "chrono-rune",
      name: "Chrono Rune",
      text: "Start round two with +1 AP.",
      unlockLevel: 5,
      unlocked: false,
    },
  ],
  heroes: [
    {
      heroType: "runekeeper",
      xp: 260,
      level: 3,
      currentLevelXp: 250,
      nextLevelXp: 600,
      xpIntoLevel: 10,
      xpToNextLevel: 340,
      totalSkillPoints: 2,
      spentSkillPoints: 1,
      availableSkillPoints: 1,
      unlockedSkillIds: ["root"],
    },
    {
      heroType: "pyromancer",
      xp: 120,
      level: 2,
      currentLevelXp: 100,
      nextLevelXp: 250,
      xpIntoLevel: 20,
      xpToNextLevel: 130,
      totalSkillPoints: 1,
      spentSkillPoints: 0,
      availableSkillPoints: 1,
      unlockedSkillIds: [],
    },
  ],
  skillTrees: [
    {
      heroType: "runekeeper",
      nodes: [
        {
          id: "root",
          name: "Rune Focus",
          text: "Start with one extra mana.",
          root: true,
          prerequisiteId: null,
        },
        {
          id: "sigil-guard",
          name: "Sigil Guard",
          text: "Summoned units gain +1 armor.",
          root: false,
          prerequisiteId: "root",
        },
      ],
    },
    {
      heroType: "pyromancer",
      nodes: [
        {
          id: "root",
          name: "Kindling",
          text: "Damage spells gain +1 damage.",
          root: true,
          prerequisiteId: null,
        },
      ],
    },
  ],
  loadouts: [
    {
      heroType: "runekeeper",
      runeIds: ["ember-rune", "warding-rune"],
    },
  ],
  heroAppearances: [
    {
      heroType: "runekeeper",
      selectedAppearanceId: "base",
      appearances: [
        {
          id: "base",
          heroType: "runekeeper",
          name: "Runekeeper",
          text: "Default board appearance.",
          unlockLevel: null,
          unlocked: true,
        },
        {
          id: "gilded",
          heroType: "runekeeper",
          name: "Gilded Runekeeper",
          text: "A mastery appearance.",
          unlockLevel: 5,
          unlocked: false,
        },
      ],
    },
  ],
};

export const storyMatchSummary = {
  matchId: "arc-story",
  mode: "solo",
  createdAt: 1_700_000_000,
  updatedAt: 1_700_000_600,
  round: 4,
  phase: "matchOver",
  winner: "player",
  frameCount: 18,
} satisfies MatchArchiveResponse["matches"][number];

export const storyMatchResponse: MatchResponse = {
  matchId: "arc-story",
  matchState: storyMatch(),
  heroAppearances: [],
  replayFrames: [],
};

const replaySecondMatch = storyMatch();
replaySecondMatch.log = ["Player turn begins."];

export const storyReplayResponse: MatchReplayResponse = {
  matchId: "arc-story",
  visibility: "public",
  summary: storyMatchSummary,
  frames: [
    {
      frameIndex: 0,
      actionIndex: null,
      event: { type: "matchCreated" },
      matchState: storyMatch(),
    },
    {
      frameIndex: 1,
      actionIndex: 0,
      event: { type: "turnStarted", side: "player", round: 1 },
      matchState: replaySecondMatch,
    },
  ],
};

export const storySummaryResponse: MatchSummaryResponse = {
  matchId: "arc-story",
  summary: storyMatchSummary,
  viewer: {
    side: "player",
    result: "victory",
  },
  reward: {
    side: "player",
    heroType: "runekeeper",
    won: true,
    accountXpGained: 120,
    heroXpGained: 90,
    winBonusXp: 25,
    account: {
      before: { ...storyAccount.progressionSummary, totalXp: 300, level: 2 },
      after: storyAccount.progressionSummary,
    },
    hero: {
      heroType: "runekeeper",
      before: {
        heroType: "runekeeper",
        xp: 170,
        level: 2,
        currentLevelXp: 100,
        nextLevelXp: 250,
        xpIntoLevel: 70,
        xpToNextLevel: 80,
        totalSkillPoints: 1,
        spentSkillPoints: 1,
        availableSkillPoints: 0,
        unlockedSkillIds: ["root"],
      },
      after: storyProgression.heroes[0],
    },
    unlocks: [
      {
        type: "runeUnlocked",
        runeId: "warding-rune",
        name: "Warding Rune",
      },
    ],
  },
};

export const storySharedMatch: SharedMatchResponse = {
  matchId: "arc-shared",
  mode: "shared",
  status: "setup",
  viewerSide: "player",
  viewerTeam: "player",
  viewerHeroType: "runekeeper",
  opponentHeroType: null,
  viewerReady: false,
  opponentReady: false,
  seats: [
    {
      side: "player",
      team: "player",
      label: "Player",
      ready: false,
      connected: true,
      heroType: "runekeeper",
      knockedOut: false,
    },
    {
      side: "opponent",
      team: "opponent",
      label: "Opponent",
      ready: false,
      connected: false,
      heroType: null,
      knockedOut: false,
    },
  ],
  activeSide: null,
  opponentConnected: false,
  canClaimForfeitAt: null,
  heroAppearances: [],
  matchState: null,
};

export const storyScenarios: MatchScenarioListResponse = {
  scenarios: [
    {
      id: "play-unit-card",
      name: "Play Unit Card",
      description: "Start with an affordable unit card and an adjacent empty hex.",
      primaryActions: ["playCard"],
    },
    {
      id: "priority-response",
      name: "Priority Response",
      description: "Start with an opponent stack item and player priority.",
      primaryActions: ["playCard", "passPriority", "advanceAi"],
    },
  ],
};

type MockResponseMap = Record<string, unknown>;

const baseResponses: MockResponseMap = {
  "/api/auth/me": storyAccount,
  "/api/profile": storyAccount,
  "/api/profile/matches": { matches: [storyMatchSummary] } satisfies MatchArchiveResponse,
  "/api/preferences": storyPreferences,
  "/api/progression": storyProgression,
  "/api/decks": { rules: storyDeckRules, decks: storyDecks } satisfies DeckListResponse,
  "/api/system-decks": { rules: storyDeckRules, decks: storySystemDecks } satisfies SystemDeckListResponse,
  "/api/catalog/cards": { cards: catalogCards } satisfies CatalogResponse,
  "/api/matches": { matches: [storyMatchSummary] } satisfies MatchArchiveResponse,
  "/api/matches/arc-story": storyMatchResponse,
  "/api/matches/arc-story/replay": storyReplayResponse,
  "/api/matches/arc-story/summary": storySummaryResponse,
  "/api/dev/match-scenarios": storyScenarios,
  "/api/users/player-one/decks/1": {
    owner: {
      id: storyAccount.id,
      handle: storyAccount.handle,
      displayName: storyAccount.displayName,
      avatar: storyAccount.avatar,
    },
    deck: storyDecks[0],
  } satisfies PublicDeckRecipeResponse,
  "/api/shared-matches/arc-shared/seats/player-seat": storySharedMatch,
  "/api/shared-matches/arc-shared/seats/player-seat/replay": storyReplayResponse,
  "/api/shared-matches/arc-shared/seats/player-seat/summary": storySummaryResponse,
};

export function withMockApi(overrides: MockResponseMap = {}): Decorator {
  return (Story) => {
    const responses = { ...baseResponses, ...overrides };

    return (
      <MockApiBoundary responses={responses}>
        <Story />
      </MockApiBoundary>
    );
  };
}

function MockApiBoundary({
  responses,
  children,
}: {
  responses: MockResponseMap;
  children: ReactNode;
}) {
  const originalsRef = useRef<{
    fetch: typeof window.fetch;
    WebSocket: typeof window.WebSocket;
  } | null>(null);
  originalsRef.current ??= {
    fetch: window.fetch,
    WebSocket: window.WebSocket,
  };

  window.fetch = async (input, init) => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.pathname : input.url;
    const path = new URL(url, window.location.origin).pathname;
    const method = init?.method ?? "GET";

    if (method !== "GET") {
      const body = postResponseFor(path);
      return jsonResponse(body);
    }

    if (path in responses) {
      return jsonResponse(responses[path]);
    }

    return jsonResponse({ message: `No Storybook mock for ${path}` }, 404);
  };

  window.WebSocket = MockWebSocket as typeof WebSocket;

  useEffect(() => {
    return () => {
      if (originalsRef.current) {
        window.fetch = originalsRef.current.fetch;
        window.WebSocket = originalsRef.current.WebSocket;
      }
    };
  }, []);

  return <>{children}</>;
}

function postResponseFor(path: string) {
  if (path === "/api/auth/login" || path === "/api/auth/register") {
    return { token: "story-token", user: storyAccount } satisfies AuthSessionResponse;
  }

  if (path === "/api/matches") {
    return storyMatchResponse;
  }

  if (path === "/api/shared-matches") {
    return {
      matchId: "arc-shared",
      mode: "shared",
      status: "setup",
      viewerSide: "player",
      playerSeatUrl: "/match/arc-shared/player-seat",
      inviteSeatUrl: "/match/arc-shared/opponent-seat",
    } satisfies CreateSharedMatchResponse;
  }

  if (path.endsWith("/join")) {
    return {
      ...storySharedMatch,
      viewerReady: true,
      opponentReady: true,
    } satisfies SharedMatchResponse;
  }

  if (path.includes("/actions")) {
    return storyMatchResponse;
  }

  return { message: "Story action accepted" };
}

function jsonResponse(body: unknown, status = 200) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

class MockWebSocket extends EventTarget {
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSING = 2;
  static CLOSED = 3;

  readonly CONNECTING = 0;
  readonly OPEN = 1;
  readonly CLOSING = 2;
  readonly CLOSED = 3;
  readonly url: string;
  readyState = MockWebSocket.OPEN;
  binaryType: BinaryType = "blob";
  bufferedAmount = 0;
  extensions = "";
  protocol = "";
  onopen: ((this: WebSocket, event: Event) => unknown) | null = null;
  onmessage: ((this: WebSocket, event: MessageEvent) => unknown) | null = null;
  onerror: ((this: WebSocket, event: Event) => unknown) | null = null;
  onclose: ((this: WebSocket, event: CloseEvent) => unknown) | null = null;

  constructor(url: string | URL) {
    super();
    this.url = String(url);
    queueMicrotask(() => this.dispatchEvent(new Event("open")));
  }

  send() {
    return undefined;
  }

  close() {
    this.readyState = MockWebSocket.CLOSED;
    this.dispatchEvent(new Event("close"));
  }
}
