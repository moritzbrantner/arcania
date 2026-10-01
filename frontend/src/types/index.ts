export type Side = "player" | "opponent" | "playerTwo" | "opponentTwo";
export type Team = "player" | "opponent";
export type MatchFormat = "duel" | "twoVTwo";

export type Phase = "movement" | "attack" | "cardPlay" | "matchOver";

export type MatchMode = "solo" | "shared";

export type Rarity = "basic" | "advanced" | "rare";

export type HexCoord = {
  q: number;
  r: number;
};

export type BuffTargetPolicy = "unitsOnly" | "heroesOnly" | "unitsAndHeroes";

export type BuildingEffect =
  | {
      type: "turnStartMana";
      amount: number;
    }
  | {
      type: "auraStatBonus";
      range: number;
      targets: BuffTargetPolicy;
      attack: number;
      armor: number;
      maxAp: number;
    }
  | {
      type: "activatedDamageLine";
      range: number;
      amount: number;
    }
  | {
      type: "activatedHeal";
      range: number;
      amount: number;
      targets: BuffTargetPolicy;
    }
  | {
      type: "activatedStatBonus";
      range: number;
      targets: BuffTargetPolicy;
      attack: number;
      armor: number;
      maxAp: number;
    };

export type SpellEffect =
  | {
      type: "heal";
      amount: number;
    }
  | {
      type: "buff";
      attack: number;
      armor: number;
    }
  | {
      type: "statBuff";
      attack: number;
      armor: number;
      maxAp: number;
      targets: BuffTargetPolicy;
    }
  | {
      type: "damage";
      amount: number;
    }
  | {
      type: "draw";
      amount: number;
    }
  | {
      type: "areaDamage";
      amount: number;
      radius: number;
    }
  | {
      type: "lineDamage";
      amount: number;
    };

export type ItemPassiveEffect = {
  type: "statBonus";
  attack: number;
  armor: number;
  maxAp: number;
};

export type ItemActiveEffect =
  | {
      type: "healCarrier";
      amount: number;
      priority?: number;
    }
  | {
      type: "damageTarget";
      amount: number;
      range: number;
      priority?: number;
    }
  | {
      type: "draw";
      amount: number;
      priority?: number;
    }
  | {
      type: "statMarker";
      attack: number;
      armor: number;
      maxAp: number;
      priority?: number;
    };

export type CardKind =
  | {
      type: "unit";
      attack: number;
      armor: number;
      maxAp: number;
    }
  | {
      type: "spell";
      range: number;
      priority: number;
      effect: SpellEffect;
    }
  | {
      type: "item";
      range: number;
      targets?: BuffTargetPolicy;
      passive: ItemPassiveEffect;
      active?: ItemActiveEffect;
    }
  | {
      type: "building";
      effect: BuildingEffect;
    }
  | {
      type: "manaSource";
    };

export type Card = {
  id: string;
  templateId: string;
  name: string;
  rarity: Rarity;
  cost: number;
  text: string;
  kind: CardKind;
};

export type CatalogCard = {
  id: string;
  templateId: string;
  name: string;
  rarity: Rarity;
  cost: number;
  text: string;
  kind: CardKind;
  copyCount: number;
  artKey: string;
  artPath: string;
};

export type CatalogResponse = {
  cards: CatalogCard[];
};

export type DeckCardCount = {
  templateId: string;
  count: number;
};

export type DeckRules = {
  maxDecksPerAccount: number;
  minCards: number;
  basicCopyLimit: number;
  advancedCopyLimit: number;
  rareCopyLimit: number;
  advancedTotalLimit: number;
  rareTotalLimit: number;
};

export type DeckLegality = {
  legal: boolean;
  totalCards: number;
  basicCards: number;
  advancedCards: number;
  rareCards: number;
  messages: string[];
};

export type DeckRecipeSummary = {
  id: number;
  name: string;
  isDefault: boolean;
  heroType: HeroType;
  runeIds: string[];
  cards: DeckCardCount[];
  legality: DeckLegality;
  createdAt: number;
  updatedAt: number;
};

export type DeckRecipeDetail = DeckRecipeSummary;

export type DeckListResponse = {
  rules: DeckRules;
  decks: DeckRecipeSummary[];
};

export type PublicDeckRecipeResponse = {
  owner: {
    id: number;
    handle: string;
    displayName: string;
    avatar: GeneratedAvatar;
  };
  deck: DeckRecipeSummary;
};

export type SystemDeckRecipe = {
  id: string;
  name: string;
  heroType: HeroType;
  cards: DeckCardCount[];
  legality: DeckLegality;
};

export type SystemDeckListResponse = {
  rules: DeckRules;
  decks: SystemDeckRecipe[];
};

export type DeckChoice =
  | {
      source: "starter";
    }
  | {
      source: "system";
      systemDeckId: string;
    }
  | {
      source: "account";
      deckId: number;
    };

export type SoloAiOpponentSelection =
  | {
      source: "system";
      systemDeckId: string;
    }
  | {
      source: "account";
      deckId: number;
      heroType: HeroType;
    };

export type AuthUser = {
  id: number;
  handle: string;
  email: string;
  displayName: string;
  avatar: GeneratedAvatar;
  preferredHeroType: HeroType;
  boardVisualMode: BoardVisualMode;
  progressionSummary: ProgressionSummary;
};

export type AuthSessionResponse = {
  token: string;
  user: AuthUser;
};

export type GeneratedAvatar = {
  symbol: string;
  color: string;
};

export type AccountProfile = AuthUser;

export type BoardVisualMode = "2d" | "3d";

export type PreferenceTheme = "system" | "dark" | "light" | "highContrast";

export type MotionPreference = "system" | "reduced" | "full";

export type AnimationSpeed = "slow" | "normal" | "fast";

export type BoardScale = "compact" | "normal" | "large";

export type HotkeyCommandId =
  | "cursorNorthwest"
  | "cursorNortheast"
  | "cursorEast"
  | "cursorWest"
  | "cursorSouthwest"
  | "cursorSoutheast"
  | "confirm"
  | "cancel"
  | "endTurn"
  | "passPriority"
  | "openCardInfo"
  | "openSettings"
  | "openCatalog"
  | "openDecks"
  | "openMatchArchive";

export type HotkeyBinding = {
  commandId: HotkeyCommandId;
  binding: string;
};

export type AccountPreferences = {
  theme: PreferenceTheme;
  motion: MotionPreference;
  animationSpeed: AnimationSpeed;
  boardScale: BoardScale;
  boardVisualMode: BoardVisualMode;
  hotkeys: HotkeyBinding[];
  updatedAt: number | null;
};

export type UpdatePreferencesRequest = Omit<AccountPreferences, "updatedAt">;

export type ProgressionSummary = {
  totalXp: number;
  level: number;
  currentLevelXp: number;
  nextLevelXp: number;
  xpIntoLevel: number;
  xpToNextLevel: number;
  runeSlots: number;
};

export type RuneDefinition = {
  id: string;
  name: string;
  text: string;
  unlockLevel: number;
  unlocked: boolean;
};

export type HeroProgression = {
  heroType: HeroType;
  xp: number;
  level: number;
  currentLevelXp: number;
  nextLevelXp: number;
  xpIntoLevel: number;
  xpToNextLevel: number;
  totalSkillPoints: number;
  spentSkillPoints: number;
  availableSkillPoints: number;
  unlockedSkillIds: string[];
};

export type HeroSkillTree = {
  heroType: HeroType;
  nodes: SkillNodeDefinition[];
};

export type SkillNodeDefinition = {
  id: string;
  name: string;
  text: string;
  root: boolean;
  prerequisiteId: string | null;
};

export type SavedRuneLoadout = {
  heroType: HeroType;
  runeIds: string[];
};

export type ProgressionResponse = {
  account: ProgressionSummary;
  runes: RuneDefinition[];
  heroes: HeroProgression[];
  skillTrees: HeroSkillTree[];
  loadouts: SavedRuneLoadout[];
  heroAppearances: HeroAppearanceProgression[];
};

export type HeroAppearanceProgression = {
  heroType: HeroType;
  selectedAppearanceId: string;
  appearances: HeroAppearanceDefinition[];
};

export type HeroAppearanceDefinition = {
  id: string;
  heroType: HeroType;
  name: string;
  text: string;
  unlockLevel: number | null;
  unlocked: boolean;
};

export type MatchProgressionEffects = {
  maxHpDelta: number;
  attackDelta: number;
  maxApDelta: number;
  manaDelta: number;
  openingHandDelta: number;
  summonedUnitArmorDelta: number;
  firstSummonedUnitArmorDelta: number;
  spellDamageDelta: number;
};

export type MatchProgressionLoadout = {
  runeIds: string[];
  skillIds: string[];
  effects: MatchProgressionEffects;
};

export type Hero = {
  id: string;
  side: Side;
  heroType: HeroType;
  hp: number;
  maxHp: number;
  shield?: number;
  attack: number;
  attackRange: number;
  position: HexCoord;
  apRemaining: number;
  maxAp: number;
  hasAttacked: boolean;
  items?: CarriedItem[];
  statMarkers?: StatMarker[];
};

export type HeroType =
  | "runekeeper"
  | "pyromancer"
  | "chronomancer"
  | "warden"
  | "battlemage"
  | "barbarian"
  | "archer"
  | "builder";

export type Unit = {
  id: string;
  side: Side;
  name: string;
  templateId?: string;
  attack: number;
  attackRange: number;
  armor: number;
  maxArmor: number;
  position: HexCoord;
  apRemaining: number;
  maxAp: number;
  hasAttacked: boolean;
  items?: CarriedItem[];
  statMarkers?: StatMarker[];
};

export type CarriedItem = {
  id: string;
  templateId: string;
  name: string;
  passive: ItemPassiveEffect;
  active?: ItemActiveEffect;
  activeUsedThisTurn: boolean;
};

export type StatMarker = {
  id: string;
  sourceItemId: string;
  attack: number;
  armor: number;
  maxAp: number;
};

export type DroppedItem = {
  id: string;
  position: HexCoord;
  item: CarriedItem;
};

export type HexTile = {
  coord: HexCoord;
};

export type HexBoard = {
  radius: number;
  tiles: HexTile[];
  manaSources?: HexCoord[];
  buildings?: Building[];
  units: Unit[];
  droppedItems: DroppedItem[];
};

export type Building = {
  id: string;
  templateId: string;
  name: string;
  position: HexCoord;
  effect: BuildingEffect;
  activatedThisTurn: boolean;
};

export type MatchParticipantState = {
  side: Side;
  team?: Team;
  knockedOut?: boolean;
  mana: number;
  maxMana: number;
  hero: Hero;
  progression: MatchProgressionLoadout;
  hand?: Card[];
  handCount: number;
  deckCount: number;
  discardCount: number;
};

export type MatchPlayerState = MatchParticipantState & {
  hand: Card[];
};

export type MatchState = {
  /** Browser engine query projection for Player; absent on legacy hosted responses. */
  legalCommands?: MatchActionRequest[];
  mode: MatchMode;
  format?: MatchFormat;
  round: number;
  phase: Phase;
  activeSide: Side;
  prioritySide: Side | null;
  player: MatchPlayerState;
  opponent: MatchParticipantState;
  playerTwo?: MatchParticipantState | null;
  opponentTwo?: MatchParticipantState | null;
  participants?: MatchParticipantState[];
  board: HexBoard;
  actionStack: StackItem[];
  log: string[];
  winner: Side | null;
};

export type StackItem = {
  id: string;
  side: Side;
  priority: number;
  action: StackAction;
};

export type StackAction =
  | {
      type: "playUnit";
      card: CardSummary;
      coord: HexCoord;
    }
  | {
      type: "castSpell";
      card: CardSummary;
      targetId: string;
    }
  | {
      type: "movePiece";
      pieceId: string;
      from: HexCoord;
      to: HexCoord;
    }
  | {
      type: "attack";
      attackerId: string;
      targetId: string;
    }
  | {
      type: "equipItem";
      card: CardSummary;
      carrierId: string;
      unitId?: string;
    }
  | {
      type: "buildManaSource";
      card: CardSummary;
      coord: HexCoord;
    }
  | {
      type: "buildBuilding";
      card: CardSummary;
      coord: HexCoord;
    }
  | {
      type: "activateItem";
      carrierId: string;
      unitId?: string;
      itemId: string;
      target?: ActionTarget | null;
    }
  | {
      type: "activateBuilding";
      buildingId: string;
      occupantId: string;
    };

export type MatchResponse = {
  matchId: string;
  matchState: MatchState;
  heroAppearances: HeroAppearanceAssignment[];
  replayFrames?: ReplayFrame[];
};

export type CreateSharedMatchResponse = {
  matchId: string;
  mode: "shared";
  format?: MatchFormat;
  status: SharedMatchStatus;
  viewerSide: Side;
  playerSeatUrl: string;
  inviteSeatUrl: string;
  seatUrls?: SharedSeatUrl[];
};

export type SharedSeatUrl = {
  side: Side;
  team: Team;
  label: string;
  url: string;
};

export type SharedMatchStatus = "setup" | "active" | "completed" | "forfeited";

export type SharedMatchResponse = {
  matchId: string;
  mode: "shared";
  format?: MatchFormat;
  status: SharedMatchStatus;
  viewerSide: Side;
  viewerTeam?: Team;
  viewerHeroType: HeroType | null;
  opponentHeroType: HeroType | null;
  viewerReady: boolean;
  opponentReady: boolean;
  seats?: SharedSeat[];
  activeSide: Side | null;
  opponentConnected: boolean;
  canClaimForfeitAt: number | null;
  heroAppearances: HeroAppearanceAssignment[];
  matchState: MatchState | null;
};

export type HeroAppearanceAssignment = {
  side: Side;
  heroType: HeroType;
  appearanceId: string;
  source: "ownerSelection" | "viewerSelection" | "base";
};

export type SharedSeat = {
  side: Side;
  team: Team;
  label: string;
  ready: boolean;
  connected: boolean;
  heroType: HeroType | null;
  knockedOut: boolean;
};

export type ReplayVisibility = "public" | "revealed";

export type MatchSummary = {
  matchId: string;
  mode: MatchMode;
  createdAt: number;
  updatedAt: number;
  round: number;
  phase: Phase;
  winner: Side | null;
  frameCount: number;
  viewerTeam?: Team;
  viewerHeroTypes?: HeroType[];
  opposingHeroTypes?: HeroType[];
  viewerDeckName?: string | null;
  viewerResult?: ViewerResult | null;
};

export type MatchArchiveResponse = {
  matches: MatchSummary[];
};

export type ViewerResult = "victory" | "defeat" | "spectator";

export type MatchSummaryResponse = {
  matchId: string;
  summary: MatchSummary;
  viewer: {
    side: Side | null;
    result: ViewerResult;
  };
  reward: MatchRewardSummary | null;
};

export type MatchRewardSummary = {
  side: Side;
  heroType: HeroType;
  won: boolean;
  accountXpGained: number;
  heroXpGained: number;
  winBonusXp: number;
  account: ProgressionDelta;
  hero: HeroProgressionDelta;
  unlocks: MatchUnlockCallout[];
};

export type ProgressionDelta = {
  before: ProgressionSummary;
  after: ProgressionSummary;
};

export type HeroProgressionDelta = {
  heroType: HeroType;
  before: HeroProgression;
  after: HeroProgression;
};

export type MatchUnlockCallout =
  | {
      type: "accountLevel";
      level: number;
    }
  | {
      type: "runeUnlocked";
      runeId: string;
      name: string;
    }
  | {
      type: "runeSlotUnlocked";
      runeSlots: number;
    }
  | {
      type: "heroMasteryLevel";
      heroType: HeroType;
      level: number;
    }
  | {
      type: "skillPointUnlocked";
      heroType: HeroType;
      skillPoints: number;
    }
  | {
      type: "heroAppearanceUnlocked";
      heroType: HeroType;
      appearanceId: string;
      name: string;
    };

export type MatchScenarioSummary = {
  id: string;
  name: string;
  description: string;
  primaryActions: string[];
};

export type MatchScenarioListResponse = {
  scenarios: MatchScenarioSummary[];
};

export type ActionTarget =
  | {
      type: "hex";
      coord: HexCoord;
    }
  | {
      type: "piece";
      pieceId: string;
    };

export type MatchActionRequest =
  | {
      type: "playCard";
      cardId: string;
      target: ActionTarget;
    }
  | {
      type: "movePiece";
      pieceId: string;
      to: HexCoord;
    }
  | {
      type: "attack";
      attackerId: string;
      targetId: string;
    }
  | {
      type: "activateItem";
      carrierId: string;
      unitId?: string;
      itemId: string;
      target?: ActionTarget | null;
    }
  | {
      type: "activateBuilding";
      buildingId: string;
    }
  | {
      type: "startAttackPhase";
    }
  | {
      type: "startCardPlay";
    }
  | {
      type: "endTurn";
    }
  | {
      type: "passPriority";
    }
  | {
      type: "advanceAi";
    };

export type SharedClientMessage =
  | {
      type: "action";
      requestId: string;
      action: MatchActionRequest;
    }
  | {
      type: "claimForfeit";
      requestId: string;
    }
  | {
      type: "heartbeat";
    };

export type SharedServerMessage =
  | {
      type: "snapshot";
      payload: SharedMatchResponse;
    }
  | {
      type: "actionAccepted";
      requestId: string;
      payload: SharedMatchResponse;
    }
  | {
      type: "actionRejected";
      requestId: string;
      message: string;
    }
  | {
      type: "presenceChanged";
      payload: SharedMatchResponse;
    }
  | {
      type: "error";
      message: string;
    };

export type CardSummary = {
  templateId: string;
  name: string;
  rarity: Rarity;
  cost: number;
  kind: CardKind;
};

export type ReplayEvent =
  | {
      type: "matchCreated";
    }
  | {
      type: "turnStarted";
      side: Side;
      round: number;
    }
  | {
      type: "turnEnded";
      side: Side;
      round: number;
    }
  | {
      type: "roundStarted";
      round: number;
    }
  | {
      type: "phaseChanged";
      side: Side;
      phase: Phase;
    }
  | {
      type: "cardDrawn";
      side: Side;
      card: CardSummary | null;
      hidden: boolean;
    }
  | {
      type: "cardPlayed";
      side: Side;
      card: CardSummary;
      target: ActionTarget;
    }
  | {
      type: "actionQueued";
      side: Side;
      item: StackItem;
    }
  | {
      type: "unitSummoned";
      side: Side;
      unitId: string;
      name: string;
      position: HexCoord;
    }
  | {
      type: "pieceMoved";
      side: Side;
      pieceId: string;
      from: HexCoord;
      to: HexCoord;
    }
  | {
      type: "pieceAttacked";
      side: Side;
      attackerId: string;
      targetId: string;
      damageToTarget: number;
      counterDamageToAttacker: number;
    }
  | {
      type: "pieceHealed";
      side: Side;
      pieceId: string;
      amount: number;
    }
  | {
      type: "unitArmorRefreshed";
      side: Side;
      unitId: string;
      amount: number;
    }
  | {
      type: "pieceBuffed";
      side: Side;
      pieceId: string;
      attackDelta: number;
      armorDelta: number;
    }
  | {
      type: "pieceDamaged";
      side: Side;
      pieceId: string;
      amount: number;
    }
  | {
      type: "unitDestroyed";
      side: Side;
      unitId: string;
      name: string;
    }
  | {
      type: "manaSourceBuilt";
      side: Side;
      coord: HexCoord;
    }
  | {
      type: "buildingBuilt";
      side: Side;
      buildingId: string;
      name: string;
      coord: HexCoord;
    }
  | {
      type: "buildingActivated";
      side: Side;
      buildingId: string;
      name: string;
      occupantId: string;
    }
  | {
      type: "heroShielded";
      side: Side;
      heroId: string;
      amount: number;
    }
  | {
      type: "manaGained";
      side: Side;
      amount: number;
      source: {
        type: "barbarianKill";
        heroId: string;
        unitId: string;
      };
    }
  | {
      type: "itemEquipped";
      side: Side;
      carrierId: string;
      unitId?: string;
      itemId: string;
      name: string;
    }
  | {
      type: "itemDropped";
      side: Side;
      carrierId: string;
      unitId?: string;
      itemId: string;
      name: string;
      position: HexCoord;
    }
  | {
      type: "itemActivated";
      side: Side;
      carrierId: string;
      unitId?: string;
      itemId: string;
      name: string;
    }
  | {
      type: "matchEnded";
      winner: Side;
    };

export type ReplayFrame = {
  frameIndex: number;
  actionIndex: number | null;
  event: ReplayEvent;
  matchState: MatchState;
};

export type MatchReplayResponse = {
  matchId: string;
  visibility: ReplayVisibility;
  summary: MatchSummary;
  frames: ReplayFrame[];
};
