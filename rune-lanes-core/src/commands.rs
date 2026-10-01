use serde::{Deserialize, Serialize};

use crate::{
    ActionTarget, HexCoord, MatchActionRequest, MatchError, MatchState, RecordedReplayFrame, Side,
};

/// A transport-independent request to change authoritative Rune Lanes match state.
///
/// `AdvanceAi` is intentionally absent: choosing when an AI acts is application
/// orchestration, while any action it selects must still enter the domain as one
/// of these commands.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameCommand {
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
        carrier_id: String,
        item_id: String,
        target: Option<ActionTarget>,
    },
    ActivateBuilding {
        building_id: String,
    },
    StartAttackPhase,
    StartCardPlay,
    EndTurn,
    PassPriority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandContext {
    pub side: Side,
    pub action_index: u32,
}

#[derive(Clone, Debug)]
pub struct CommandOutcome {
    pub replay_frames: Vec<RecordedReplayFrame>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandConversionError {
    AdvanceAiIsApplicationOrchestration,
}

impl GameCommand {
    /// Runs the authoritative command rules against a clone and discards the
    /// resulting mutation. Query-side legality checks use this exact path so
    /// presentation code never grows a competing rule implementation.
    pub fn check(&self, state: &MatchState, context: CommandContext) -> Result<(), MatchError> {
        let mut candidate = state.clone();
        self.clone()
            .execute_compatibility(&mut candidate, context)
            .map(|_| ())
    }

    /// Compatibility execution while the aggregate is migrated to
    /// `decide -> domain events -> evolve`.
    ///
    /// New infrastructure should depend on `GameCommand`, not on the legacy
    /// transport-shaped `MatchActionRequest` type. This method deliberately
    /// keeps the current rules implementation and replay semantics unchanged.
    pub fn execute_compatibility(
        self,
        state: &mut MatchState,
        context: CommandContext,
    ) -> Result<CommandOutcome, MatchError> {
        let replay_frames = state.apply_action_recording_for_side(
            context.side,
            self.into_match_action_request(),
            context.action_index,
        )?;
        Ok(CommandOutcome { replay_frames })
    }

    pub fn into_match_action_request(self) -> MatchActionRequest {
        match self {
            Self::PlayCard { card_id, target } => MatchActionRequest::PlayCard { card_id, target },
            Self::MovePiece { piece_id, to } => MatchActionRequest::MovePiece { piece_id, to },
            Self::Attack {
                attacker_id,
                target_id,
            } => MatchActionRequest::Attack {
                attacker_id,
                target_id,
            },
            Self::ActivateItem {
                carrier_id,
                item_id,
                target,
            } => MatchActionRequest::ActivateItem {
                carrier_id,
                item_id,
                target,
            },
            Self::ActivateBuilding { building_id } => {
                MatchActionRequest::ActivateBuilding { building_id }
            }
            Self::StartAttackPhase => MatchActionRequest::StartAttackPhase,
            Self::StartCardPlay => MatchActionRequest::StartCardPlay,
            Self::EndTurn => MatchActionRequest::EndTurn,
            Self::PassPriority => MatchActionRequest::PassPriority,
        }
    }
}

impl TryFrom<MatchActionRequest> for GameCommand {
    type Error = CommandConversionError;

    fn try_from(request: MatchActionRequest) -> Result<Self, Self::Error> {
        Ok(match request {
            MatchActionRequest::PlayCard { card_id, target } => Self::PlayCard { card_id, target },
            MatchActionRequest::MovePiece { piece_id, to } => Self::MovePiece { piece_id, to },
            MatchActionRequest::Attack {
                attacker_id,
                target_id,
            } => Self::Attack {
                attacker_id,
                target_id,
            },
            MatchActionRequest::ActivateItem {
                carrier_id,
                item_id,
                target,
            } => Self::ActivateItem {
                carrier_id,
                item_id,
                target,
            },
            MatchActionRequest::ActivateBuilding { building_id } => {
                Self::ActivateBuilding { building_id }
            }
            MatchActionRequest::StartAttackPhase => Self::StartAttackPhase,
            MatchActionRequest::StartCardPlay => Self::StartCardPlay,
            MatchActionRequest::EndTurn => Self::EndTurn,
            MatchActionRequest::PassPriority => Self::PassPriority,
            MatchActionRequest::AdvanceAi => {
                return Err(CommandConversionError::AdvanceAiIsApplicationOrchestration);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_roundtrip_as_stable_tagged_domain_contract() {
        let command = GameCommand::MovePiece {
            piece_id: "unit-1".to_string(),
            to: HexCoord { q: 1, r: -1 },
        };
        let encoded = serde_json::to_value(&command).unwrap();
        let decoded: GameCommand = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(decoded, command);
        assert_eq!(encoded["type"], "movePiece");
        assert_eq!(encoded["pieceId"], "unit-1");
    }

    #[test]
    fn advance_ai_is_not_a_domain_command() {
        assert_eq!(
            GameCommand::try_from(MatchActionRequest::AdvanceAi),
            Err(CommandConversionError::AdvanceAiIsApplicationOrchestration)
        );
    }

    #[test]
    fn command_check_does_not_mutate_authoritative_state() {
        let state = MatchState::new_with_seed(7);
        let before = state.to_snapshot_json().unwrap();

        GameCommand::StartAttackPhase
            .check(
                &state,
                CommandContext {
                    side: Side::Player,
                    action_index: 0,
                },
            )
            .unwrap();

        assert_eq!(state.to_snapshot_json().unwrap(), before);
    }
}
