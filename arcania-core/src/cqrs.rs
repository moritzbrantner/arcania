use serde::{Deserialize, Serialize};

use crate::{
    ActionTarget, HexCoord, MatchActionRequest, MatchError, MatchState, Phase, RecordedReplayFrame,
    Side,
};

/// A transport-independent request to change authoritative Arcania match state.
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

/// Small typed query surface used to establish the read side without giving it
/// mutation access. Richer projections can be added without changing command
/// handling or the aggregate's write contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameQuery {
    Round,
    Phase,
    ActiveSide,
    Winner,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameQueryResult {
    Round(u32),
    Phase(Phase),
    ActiveSide(Side),
    Winner(Option<Side>),
}

impl GameQuery {
    pub fn execute(self, state: &MatchState) -> GameQueryResult {
        match self {
            Self::Round => GameQueryResult::Round(state.round),
            Self::Phase => GameQueryResult::Phase(state.phase.clone()),
            Self::ActiveSide => GameQueryResult::ActiveSide(state.active_side),
            Self::Winner => GameQueryResult::Winner(state.winner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_ai_is_not_a_domain_command() {
        assert_eq!(
            GameCommand::try_from(MatchActionRequest::AdvanceAi),
            Err(CommandConversionError::AdvanceAiIsApplicationOrchestration)
        );
    }

    #[test]
    fn query_does_not_require_mutable_match_state() {
        let state = MatchState::new_with_seed(7);
        assert_eq!(
            GameQuery::Round.execute(&state),
            GameQueryResult::Round(state.round)
        );
        assert_eq!(
            GameQuery::ActiveSide.execute(&state),
            GameQueryResult::ActiveSide(state.active_side)
        );
    }
}
