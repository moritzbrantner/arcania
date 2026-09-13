use std::error::Error;
use std::fmt;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

use super::{CardSummary, MatchMode, default_attack_range};

#[derive(Clone, Debug)]
pub struct MatchState {
    pub mode: MatchMode,
    pub round: u32,
    pub phase: Phase,
    pub active_side: Side,
    pub priority_side: Option<Side>,
    pub player: PlayerState,
    pub opponent: PlayerState,
    pub player_two: Option<PlayerState>,
    pub opponent_two: Option<PlayerState>,
    pub board: HexBoard,
    pub action_stack: Vec<StackItem>,
    pub log: Vec<String>,
    pub winner: Option<Side>,
    pub(super) priority_passes: Vec<Side>,
    pub(super) next_stack_item_id: u32,
    pub(super) next_unit_id: u32,
    pub(super) next_item_id: u32,
    pub(super) next_building_id: u32,
}

pub const MAX_CARRIED_ITEMS: usize = 3;

impl Serialize for MatchState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("MatchState", 14)?;
        state.serialize_field("mode", &self.mode)?;
        state.serialize_field("format", &self.format())?;
        state.serialize_field("round", &self.round)?;
        state.serialize_field("phase", &self.phase)?;
        state.serialize_field("activeSide", &self.active_side)?;
        state.serialize_field("prioritySide", &self.priority_side)?;
        state.serialize_field(
            "player",
            &PublicPlayerState {
                player: &self.player,
                expose_hand: true,
            },
        )?;
        state.serialize_field(
            "opponent",
            &PublicPlayerState {
                player: &self.opponent,
                expose_hand: false,
            },
        )?;
        if let Some(player_two) = &self.player_two {
            state.serialize_field(
                "playerTwo",
                &PublicPlayerState {
                    player: player_two,
                    expose_hand: false,
                },
            )?;
        }
        if let Some(opponent_two) = &self.opponent_two {
            state.serialize_field(
                "opponentTwo",
                &PublicPlayerState {
                    player: opponent_two,
                    expose_hand: false,
                },
            )?;
        }
        state.serialize_field(
            "participants",
            &self.public_participants_for_side(Side::Player),
        )?;
        state.serialize_field("board", &self.public_board())?;
        state.serialize_field("actionStack", &self.action_stack)?;
        state.serialize_field("log", &self.log)?;
        state.serialize_field("winner", &self.winner)?;
        state.end()
    }
}

struct PublicPlayerState<'a> {
    player: &'a PlayerState,
    expose_hand: bool,
}

impl Serialize for PublicPlayerState<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let field_count = if self.expose_hand { 11 } else { 10 };
        let mut state = serializer.serialize_struct("PlayerState", field_count)?;
        state.serialize_field("side", &self.player.side)?;
        state.serialize_field("team", &self.player.side.team())?;
        state.serialize_field("knockedOut", &self.player.knocked_out)?;
        state.serialize_field("mana", &self.player.mana)?;
        state.serialize_field("maxMana", &self.player.max_mana)?;
        state.serialize_field("hero", &self.player.hero)?;
        state.serialize_field("progression", &self.player.progression)?;
        if self.expose_hand {
            state.serialize_field("hand", &self.player.hand)?;
        }
        state.serialize_field("handCount", &self.player.hand.len())?;
        state.serialize_field("deckCount", &self.player.deck_count)?;
        state.serialize_field("discardCount", &self.player.discard_count)?;
        state.end()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[serde(alias = "planning")]
    Movement,
    Attack,
    CardPlay,
    MatchOver,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Player,
    Opponent,
    PlayerTwo,
    OpponentTwo,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Team {
    Player,
    Opponent,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub side: Side,
    #[serde(default)]
    pub knocked_out: bool,
    pub mana: u8,
    pub max_mana: u8,
    pub hero: Hero,
    #[serde(default)]
    pub progression: MatchProgressionLoadout,
    pub hand: Vec<Card>,
    pub deck_count: usize,
    pub discard_count: usize,
    pub(crate) deck: Vec<Card>,
    pub(crate) discard: Vec<Card>,
    pub(super) rng_seed: u64,
    pub(super) has_started_first_turn: bool,
    #[serde(default)]
    pub(super) summoned_unit_count: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchProgressionLoadout {
    #[serde(default)]
    pub rune_ids: Vec<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub effects: MatchProgressionEffects,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchProgressionEffects {
    #[serde(default)]
    pub max_hp_delta: i32,
    #[serde(default)]
    pub attack_delta: i32,
    #[serde(default)]
    pub max_ap_delta: i8,
    #[serde(default)]
    pub mana_delta: i8,
    #[serde(default)]
    pub opening_hand_delta: u8,
    #[serde(default)]
    pub summoned_unit_armor_delta: i32,
    #[serde(default)]
    pub first_summoned_unit_armor_delta: i32,
    #[serde(default)]
    pub spell_damage_delta: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hero {
    pub id: String,
    pub side: Side,
    #[serde(default)]
    pub knocked_out: bool,
    #[serde(default)]
    pub hero_type: HeroType,
    pub hp: i32,
    pub max_hp: i32,
    #[serde(default)]
    pub shield: i32,
    pub attack: i32,
    #[serde(default = "default_attack_range")]
    pub attack_range: u8,
    pub position: HexCoord,
    pub ap_remaining: u8,
    pub max_ap: u8,
    pub has_attacked: bool,
    #[serde(default)]
    pub items: Vec<CarriedItem>,
    #[serde(default)]
    pub stat_markers: Vec<StatMarker>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HeroType {
    #[default]
    Runekeeper,
    Pyromancer,
    Chronomancer,
    Warden,
    Battlemage,
    Barbarian,
    Archer,
    Builder,
}

pub(super) struct HeroProfile {
    pub(super) max_hp: i32,
    pub(crate) attack: i32,
    pub(crate) max_ap: u8,
    pub(crate) attack_range: u8,
}

impl HeroType {
    pub(super) fn profile(self) -> HeroProfile {
        match self {
            Self::Runekeeper => HeroProfile {
                max_hp: 20,
                attack: 1,
                max_ap: 3,
                attack_range: default_attack_range(),
            },
            Self::Pyromancer => HeroProfile {
                max_hp: 18,
                attack: 2,
                max_ap: 3,
                attack_range: default_attack_range(),
            },
            Self::Chronomancer => HeroProfile {
                max_hp: 16,
                attack: 1,
                max_ap: 4,
                attack_range: default_attack_range(),
            },
            Self::Warden => HeroProfile {
                max_hp: 24,
                attack: 1,
                max_ap: 2,
                attack_range: default_attack_range(),
            },
            Self::Battlemage => HeroProfile {
                max_hp: 20,
                attack: 2,
                max_ap: 2,
                attack_range: default_attack_range(),
            },
            Self::Barbarian => HeroProfile {
                max_hp: 22,
                attack: 3,
                max_ap: 2,
                attack_range: default_attack_range(),
            },
            Self::Archer => HeroProfile {
                max_hp: 16,
                attack: 2,
                max_ap: 4,
                attack_range: 2,
            },
            Self::Builder => HeroProfile {
                max_hp: 24,
                attack: 1,
                max_ap: 2,
                attack_range: default_attack_range(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HexBoard {
    pub radius: i32,
    pub tiles: Vec<HexTile>,
    #[serde(default)]
    pub mana_sources: Vec<HexCoord>,
    #[serde(default)]
    pub buildings: Vec<Building>,
    pub units: Vec<Unit>,
    #[serde(default)]
    pub dropped_items: Vec<DroppedItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HexTile {
    pub coord: HexCoord,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
    pub id: String,
    pub side: Side,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    pub attack: i32,
    #[serde(default = "default_attack_range")]
    pub attack_range: u8,
    pub armor: i32,
    pub max_armor: i32,
    pub position: HexCoord,
    pub ap_remaining: u8,
    pub max_ap: u8,
    pub has_attacked: bool,
    #[serde(default)]
    pub items: Vec<CarriedItem>,
    #[serde(default)]
    pub stat_markers: Vec<StatMarker>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarriedItem {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub passive: ItemPassiveEffect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<ItemActiveEffect>,
    #[serde(default)]
    pub active_used_this_turn: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatMarker {
    pub id: String,
    pub source_item_id: String,
    pub attack: i32,
    pub armor: i32,
    pub max_ap: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DroppedItem {
    pub id: String,
    pub position: HexCoord,
    pub item: CarriedItem,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Building {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub position: HexCoord,
    pub effect: BuildingEffect,
    #[serde(default)]
    pub activated_this_turn: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub rarity: Rarity,
    pub cost: u8,
    pub text: String,
    pub kind: CardKind,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Rarity {
    Basic,
    Advanced,
    Rare,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CardKind {
    Unit {
        attack: i32,
        armor: i32,
        max_ap: u8,
    },
    Spell {
        range: u8,
        #[serde(default)]
        priority: u8,
        effect: SpellEffect,
    },
    Item {
        range: u8,
        #[serde(default = "default_item_targets")]
        targets: BuffTargetPolicy,
        passive: ItemPassiveEffect,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        active: Option<ItemActiveEffect>,
    },
    Building {
        effect: BuildingEffect,
    },
    ManaSource,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BuffTargetPolicy {
    UnitsOnly,
    HeroesOnly,
    UnitsAndHeroes,
}

fn default_item_targets() -> BuffTargetPolicy {
    BuffTargetPolicy::UnitsOnly
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum BuildingEffect {
    TurnStartMana {
        amount: u8,
    },
    AuraStatBonus {
        range: u8,
        targets: BuffTargetPolicy,
        attack: i32,
        armor: i32,
        max_ap: i8,
    },
    ActivatedDamageLine {
        range: u8,
        amount: i32,
    },
    ActivatedHeal {
        range: u8,
        amount: i32,
        targets: BuffTargetPolicy,
    },
    ActivatedStatBonus {
        range: u8,
        targets: BuffTargetPolicy,
        attack: i32,
        armor: i32,
        max_ap: i8,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SpellEffect {
    Heal {
        amount: i32,
    },
    Buff {
        attack: i32,
        armor: i32,
    },
    StatBuff {
        attack: i32,
        armor: i32,
        max_ap: i8,
        targets: BuffTargetPolicy,
    },
    Damage {
        amount: i32,
    },
    Draw {
        amount: u8,
    },
    AreaDamage {
        amount: i32,
        radius: u8,
    },
    LineDamage {
        amount: i32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ItemPassiveEffect {
    StatBonus { attack: i32, armor: i32, max_ap: i8 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ItemActiveEffect {
    HealCarrier {
        amount: i32,
        #[serde(default)]
        priority: u8,
    },
    DamageTarget {
        amount: i32,
        range: u8,
        #[serde(default)]
        priority: u8,
    },
    Draw {
        amount: u8,
        #[serde(default)]
        priority: u8,
    },
    StatMarker {
        attack: i32,
        armor: i32,
        max_ap: i8,
        #[serde(default)]
        priority: u8,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MatchActionRequest {
    PlayCard {
        card_id: String,
        target: ActionTarget,
    },
    MovePiece {
        piece_id: String,
        to: HexCoord,
    },
    Attack {
        attacker_id: String,
        target_id: String,
    },
    ActivateItem {
        #[serde(alias = "unitId")]
        carrier_id: String,
        item_id: String,
        #[serde(default)]
        target: Option<ActionTarget>,
    },
    ActivateBuilding {
        building_id: String,
    },
    StartAttackPhase,
    StartCardPlay,
    EndTurn,
    PassPriority,
    AdvanceAi,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AiAdvanceOutcome {
    ActionApplied,
    PriorityPassed,
    FinishedTurn,
    IllegalIntent { reason: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActionTarget {
    Hex { coord: HexCoord },
    Piece { piece_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchError {
    MatchOver,
    NotActiveSide,
    NotPrioritySide,
    CardNotFound,
    NotEnoughMana,
    NoActionPoints,
    InvalidHex,
    OccupiedHex,
    InvalidTarget,
    PieceNotFound,
    NotYourPiece,
    NotAdjacent,
    AlreadyAttacked,
    ItemNotFound,
    ItemExhausted,
    BuildingNotFound,
    BuildingExhausted,
    StackPending,
    EmptyStack,
    PriorityTooLow,
    WrongPhase,
    AiUnavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackItem {
    pub id: String,
    pub side: Side,
    pub priority: u8,
    pub action: StackAction,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StackAction {
    PlayUnit {
        card: CardSummary,
        coord: HexCoord,
    },
    CastSpell {
        card: CardSummary,
        target_id: String,
    },
    MovePiece {
        piece_id: String,
        from: HexCoord,
        to: HexCoord,
    },
    Attack {
        attacker_id: String,
        target_id: String,
    },
    EquipItem {
        card: CardSummary,
        #[serde(alias = "unitId")]
        carrier_id: String,
    },
    BuildManaSource {
        card: CardSummary,
        coord: HexCoord,
    },
    BuildBuilding {
        card: CardSummary,
        coord: HexCoord,
    },
    ActivateItem {
        #[serde(alias = "unitId")]
        carrier_id: String,
        item_id: String,
        #[serde(default)]
        target: Option<ActionTarget>,
    },
    ActivateBuilding {
        building_id: String,
        occupant_id: String,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct PieceView {
    pub(crate) id: String,
    pub(crate) side: Side,
    pub(crate) position: HexCoord,
    pub(crate) attack: i32,
    pub(crate) attack_range: u8,
    pub(crate) ap_remaining: u8,
    pub(crate) has_attacked: bool,
    pub(crate) is_hero: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct DestroyedUnit {
    pub(crate) side: Side,
    pub(crate) unit_id: String,
}

#[derive(Clone, Copy)]
pub(crate) struct StatBonus {
    pub(crate) attack: i32,
    pub(crate) armor: i32,
    pub(crate) max_ap: i8,
    pub(crate) targets: BuffTargetPolicy,
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MatchOver => "the match is over",
            Self::NotActiveSide => "it is not your turn",
            Self::NotPrioritySide => "you do not have priority",
            Self::CardNotFound => "card is no longer in hand",
            Self::NotEnoughMana => "not enough mana",
            Self::NoActionPoints => "not enough action points",
            Self::InvalidHex => "that hex is not on the board",
            Self::OccupiedHex => "that hex is occupied",
            Self::InvalidTarget => "that target is not legal",
            Self::PieceNotFound => "piece not found",
            Self::NotYourPiece => "that piece is not yours",
            Self::NotAdjacent => "target is not adjacent",
            Self::AlreadyAttacked => "that piece has already attacked this turn",
            Self::ItemNotFound => "item not found",
            Self::ItemExhausted => "that item has already been activated this turn",
            Self::BuildingNotFound => "building not found",
            Self::BuildingExhausted => "that building has already been activated this turn",
            Self::StackPending => "resolve the stack before taking that action",
            Self::EmptyStack => "there are no pending actions to resolve",
            Self::PriorityTooLow => "spell priority must be greater than the pending action",
            Self::WrongPhase => "that action is not available in this phase",
            Self::AiUnavailable => "the AI is not ready to act",
        };

        f.write_str(message)
    }
}

impl Error for MatchError {}
