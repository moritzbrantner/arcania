use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{
    ActionTarget, Card, CardKind, HexCoord, MatchState, Phase, PlayerState, Rarity, Side,
    StackItem, Team,
};
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

impl MatchState {
    /// Clone for a legality trial: runs the real rules but records no replay frames.
    pub(crate) fn legality_trial_clone(&self) -> Self {
        let mut trial = self.clone();
        trial.skip_replay_frames = true;
        trial
    }

    pub fn initial_replay_frame(&self) -> RecordedReplayFrame {
        RecordedReplayFrame {
            action_index: None,
            event: ReplayEvent::MatchCreated,
            snapshot_json: self
                .to_snapshot_json()
                .expect("match snapshot should serialize for replay"),
        }
    }

    pub fn replay_value(&self, visibility: ReplayVisibility) -> serde_json::Value {
        let mut value = json!({
            "mode": self.mode,
            "format": self.format(),
            "round": self.round,
            "phase": self.phase,
            "activeSide": self.active_side,
            "prioritySide": self.priority_side,
            "player": self.player.replay_value(true),
            "opponent": self.opponent.replay_value(visibility == ReplayVisibility::Revealed),
            "playerTwo": self.player_two.as_ref().map(|player| player.replay_value(visibility == ReplayVisibility::Revealed)),
            "opponentTwo": self.opponent_two.as_ref().map(|player| player.replay_value(visibility == ReplayVisibility::Revealed)),
            "participants": self.replay_participants(visibility),
            "board": self.public_board(),
            "actionStack": self.action_stack,
            "log": self.log,
            "winner": self.winner,
        });
        if self.player_two.is_none() {
            value
                .as_object_mut()
                .expect("match value should be an object")
                .remove("playerTwo");
        }
        if self.opponent_two.is_none() {
            value
                .as_object_mut()
                .expect("match value should be an object")
                .remove("opponentTwo");
        }
        value
    }

    fn replay_participants(&self, visibility: ReplayVisibility) -> Vec<serde_json::Value> {
        self.participants()
            .into_iter()
            .map(|participant| {
                participant.replay_value(
                    visibility == ReplayVisibility::Revealed
                        || participant.side.team() == Team::Player,
                )
            })
            .collect()
    }

    pub(super) fn record_replay_frame(
        &self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
        event: ReplayEvent,
    ) {
        if self.skip_replay_frames {
            return;
        }
        frames.push(RecordedReplayFrame {
            action_index,
            event,
            snapshot_json: self
                .to_snapshot_json()
                .expect("match snapshot should serialize for replay"),
        });
    }
}

impl PlayerState {
    pub(super) fn replay_value(&self, expose_hand: bool) -> serde_json::Value {
        let mut value = json!({
            "side": self.side,
            "team": self.side.team(),
            "knockedOut": self.knocked_out,
            "mana": self.mana,
            "maxMana": self.max_mana,
            "hero": self.hero,
            "progression": self.progression,
            "handCount": self.hand.len(),
            "deckCount": self.deck_count,
            "discardCount": self.discard_count,
        });

        if expose_hand {
            value["hand"] = json!(self.hand);
        }

        value
    }
}
