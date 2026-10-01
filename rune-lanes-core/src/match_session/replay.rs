use serde::{Deserialize, Serialize};

use super::{ActionTarget, Card, CardKind, HexCoord, Phase, Rarity, Side, StackItem};
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReplayVisibility {
    Public,
    Revealed,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MatchMode {
    Solo,
    Shared,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardSummary {
    pub template_id: String,
    pub name: String,
    pub rarity: Rarity,
    pub cost: u8,
    pub kind: CardKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ReplayEvent {
    MatchCreated,
    TurnStarted {
        side: Side,
        round: u32,
    },
    TurnEnded {
        side: Side,
        round: u32,
    },
    RoundStarted {
        round: u32,
    },
    PhaseChanged {
        side: Side,
        phase: Phase,
    },
    CardDrawn {
        side: Side,
        card: Option<CardSummary>,
        hidden: bool,
    },
    CardPlayed {
        side: Side,
        card: CardSummary,
        target: ActionTarget,
    },
    ActionQueued {
        side: Side,
        item: StackItem,
    },
    UnitSummoned {
        side: Side,
        unit_id: String,
        name: String,
        position: HexCoord,
    },
    PieceMoved {
        side: Side,
        piece_id: String,
        from: HexCoord,
        to: HexCoord,
    },
    PieceAttacked {
        side: Side,
        attacker_id: String,
        target_id: String,
        damage_to_target: i32,
        counter_damage_to_attacker: i32,
    },
    PieceHealed {
        side: Side,
        piece_id: String,
        amount: i32,
    },
    UnitArmorRefreshed {
        side: Side,
        unit_id: String,
        amount: i32,
    },
    PieceBuffed {
        side: Side,
        piece_id: String,
        attack_delta: i32,
        armor_delta: i32,
    },
    PieceDamaged {
        side: Side,
        piece_id: String,
        amount: i32,
    },
    UnitDestroyed {
        side: Side,
        unit_id: String,
        name: String,
    },
    ManaSourceBuilt {
        side: Side,
        coord: HexCoord,
    },
    BuildingBuilt {
        side: Side,
        building_id: String,
        name: String,
        coord: HexCoord,
    },
    BuildingActivated {
        side: Side,
        building_id: String,
        name: String,
        occupant_id: String,
    },
    HeroShielded {
        side: Side,
        hero_id: String,
        amount: i32,
    },
    ManaGained {
        side: Side,
        amount: u8,
        source: ReplayManaSource,
    },
    ItemEquipped {
        side: Side,
        #[serde(alias = "unitId")]
        carrier_id: String,
        item_id: String,
        name: String,
    },
    ItemDropped {
        side: Side,
        #[serde(alias = "unitId")]
        carrier_id: String,
        item_id: String,
        name: String,
        position: HexCoord,
    },
    ItemActivated {
        side: Side,
        #[serde(alias = "unitId")]
        carrier_id: String,
        item_id: String,
        name: String,
    },
    MatchEnded {
        winner: Side,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ReplayManaSource {
    BarbarianKill { hero_id: String, unit_id: String },
}

#[derive(Clone, Debug)]
pub struct RecordedReplayFrame {
    pub action_index: Option<u32>,
    pub event: ReplayEvent,
    pub snapshot_json: String,
}

impl From<&Card> for CardSummary {
    fn from(card: &Card) -> Self {
        Self {
            template_id: card.template_id.clone(),
            name: card.name.clone(),
            rarity: card.rarity,
            cost: card.cost,
            kind: card.kind.clone(),
        }
    }
}

impl ReplayEvent {
    pub fn for_visibility(&self, visibility: ReplayVisibility) -> Self {
        match (visibility, self) {
            (
                ReplayVisibility::Public,
                Self::CardDrawn {
                    side: Side::Opponent,
                    hidden,
                    ..
                },
            ) if *hidden => Self::CardDrawn {
                side: Side::Opponent,
                card: None,
                hidden: true,
            },
            _ => self.clone(),
        }
    }
}
