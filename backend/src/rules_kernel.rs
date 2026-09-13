use std::error::Error;
use std::fmt;

use crate::match_session::{
    ActionTarget, HexCoord, MatchActionRequest, MatchError, MatchState, RecordedReplayFrame, Side,
};

/// Player intent expressed in tabletop language.
///
/// Application orchestration such as advancing the Solo AI intentionally does not
/// belong here: AI must choose and submit the same commands a player can submit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum GameCommand {
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
    FinishMovement,
    FinishAttacks,
    EndTurn,
    PassPriority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NonGameCommand {
    AdvanceAi,
}

impl TryFrom<MatchActionRequest> for GameCommand {
    type Error = NonGameCommand;

    fn try_from(request: MatchActionRequest) -> Result<Self, Self::Error> {
        match request {
            MatchActionRequest::PlayCard { card_id, target } => {
                Ok(Self::PlayCard { card_id, target })
            }
            MatchActionRequest::MovePiece { piece_id, to } => Ok(Self::MovePiece { piece_id, to }),
            MatchActionRequest::Attack {
                attacker_id,
                target_id,
            } => Ok(Self::Attack {
                attacker_id,
                target_id,
            }),
            MatchActionRequest::ActivateItem {
                carrier_id,
                item_id,
                target,
            } => Ok(Self::ActivateItem {
                carrier_id,
                item_id,
                target,
            }),
            MatchActionRequest::ActivateBuilding { building_id } => {
                Ok(Self::ActivateBuilding { building_id })
            }
            MatchActionRequest::StartAttackPhase => Ok(Self::FinishMovement),
            MatchActionRequest::StartCardPlay => Ok(Self::FinishAttacks),
            MatchActionRequest::EndTurn => Ok(Self::EndTurn),
            MatchActionRequest::PassPriority => Ok(Self::PassPriority),
            MatchActionRequest::AdvanceAi => Err(NonGameCommand::AdvanceAi),
        }
    }
}

impl From<GameCommand> for MatchActionRequest {
    fn from(command: GameCommand) -> Self {
        match command {
            GameCommand::PlayCard { card_id, target } => Self::PlayCard { card_id, target },
            GameCommand::MovePiece { piece_id, to } => Self::MovePiece { piece_id, to },
            GameCommand::Attack {
                attacker_id,
                target_id,
            } => Self::Attack {
                attacker_id,
                target_id,
            },
            GameCommand::ActivateItem {
                carrier_id,
                item_id,
                target,
            } => Self::ActivateItem {
                carrier_id,
                item_id,
                target,
            },
            GameCommand::ActivateBuilding { building_id } => Self::ActivateBuilding { building_id },
            GameCommand::FinishMovement => Self::StartAttackPhase,
            GameCommand::FinishAttacks => Self::StartCardPlay,
            GameCommand::EndTurn => Self::EndTurn,
            GameCommand::PassPriority => Self::PassPriority,
        }
    }
}

/// Stable rule identifiers are the bridge between the rulebook, engine tests,
/// UI explanations, and eventual printable component references.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuleId {
    MatchActive,
    ActiveParticipant,
    PriorityParticipant,
    CardInHand,
    ManaAvailable,
    ActionPointsAvailable,
    HexOnBoard,
    HexUnoccupied,
    TargetLegal,
    PieceExists,
    PieceOwnedByActor,
    MovementDestinationOnBoard,
    MovementDestinationEmpty,
    MovementDestinationAdjacent,
    AttackEnemyTarget,
    AttackTargetInRange,
    AttackUnused,
    ItemExists,
    ItemReady,
    BuildingExists,
    BuildingReady,
    StackResolved,
    StackPendingAction,
    PriorityHigherThanPending,
    PhaseAllowsCommand,
    AiAvailable,
}

impl RuleId {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::MatchActive => "match.active",
            Self::ActiveParticipant => "turn.active-participant",
            Self::PriorityParticipant => "priority.active-participant",
            Self::CardInHand => "card.in-hand",
            Self::ManaAvailable => "card.mana-available",
            Self::ActionPointsAvailable => "piece.action-points-available",
            Self::HexOnBoard => "board.hex-on-board",
            Self::HexUnoccupied => "board.hex-unoccupied",
            Self::TargetLegal => "action.target-legal",
            Self::PieceExists => "piece.exists",
            Self::PieceOwnedByActor => "piece.owned-by-actor",
            Self::MovementDestinationOnBoard => "movement.destination-on-board",
            Self::MovementDestinationEmpty => "movement.destination-empty",
            Self::MovementDestinationAdjacent => "movement.destination-adjacent",
            Self::AttackEnemyTarget => "combat.enemy-target",
            Self::AttackTargetInRange => "combat.target-in-range",
            Self::AttackUnused => "combat.attack-unused",
            Self::ItemExists => "item.exists",
            Self::ItemReady => "item.ready",
            Self::BuildingExists => "building.exists",
            Self::BuildingReady => "building.ready",
            Self::StackResolved => "stack.resolved",
            Self::StackPendingAction => "stack.pending-action",
            Self::PriorityHigherThanPending => "stack.priority-higher-than-pending",
            Self::PhaseAllowsCommand => "turn.phase-allows-command",
            Self::AiAvailable => "application.ai-available",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuleViolation {
    rule_id: RuleId,
    source: MatchError,
}

impl RuleViolation {
    pub(crate) fn for_command(command: &GameCommand, source: MatchError) -> Self {
        let rule_id = match (command, &source) {
            (GameCommand::MovePiece { .. }, MatchError::InvalidHex) => {
                RuleId::MovementDestinationOnBoard
            }
            (GameCommand::MovePiece { .. }, MatchError::OccupiedHex) => {
                RuleId::MovementDestinationEmpty
            }
            (GameCommand::MovePiece { .. }, MatchError::NotAdjacent) => {
                RuleId::MovementDestinationAdjacent
            }
            (GameCommand::Attack { .. }, MatchError::InvalidTarget) => RuleId::AttackEnemyTarget,
            (GameCommand::Attack { .. }, MatchError::NotAdjacent) => RuleId::AttackTargetInRange,
            _ => generic_rule_id(&source),
        };

        Self { rule_id, source }
    }

    pub(crate) fn rule_id(&self) -> RuleId {
        self.rule_id
    }

    pub(crate) fn into_match_error(self) -> MatchError {
        self.source
    }
}

fn generic_rule_id(source: &MatchError) -> RuleId {
    match source {
        MatchError::MatchOver => RuleId::MatchActive,
        MatchError::NotActiveSide => RuleId::ActiveParticipant,
        MatchError::NotPrioritySide => RuleId::PriorityParticipant,
        MatchError::CardNotFound => RuleId::CardInHand,
        MatchError::NotEnoughMana => RuleId::ManaAvailable,
        MatchError::NoActionPoints => RuleId::ActionPointsAvailable,
        MatchError::InvalidHex => RuleId::HexOnBoard,
        MatchError::OccupiedHex => RuleId::HexUnoccupied,
        MatchError::InvalidTarget => RuleId::TargetLegal,
        MatchError::PieceNotFound => RuleId::PieceExists,
        MatchError::NotYourPiece => RuleId::PieceOwnedByActor,
        MatchError::NotAdjacent => RuleId::TargetLegal,
        MatchError::AlreadyAttacked => RuleId::AttackUnused,
        MatchError::ItemNotFound => RuleId::ItemExists,
        MatchError::ItemExhausted => RuleId::ItemReady,
        MatchError::BuildingNotFound => RuleId::BuildingExists,
        MatchError::BuildingExhausted => RuleId::BuildingReady,
        MatchError::StackPending => RuleId::StackResolved,
        MatchError::EmptyStack => RuleId::StackPendingAction,
        MatchError::PriorityTooLow => RuleId::PriorityHigherThanPending,
        MatchError::WrongPhase => RuleId::PhaseAllowsCommand,
        MatchError::AiUnavailable => RuleId::AiAvailable,
    }
}

impl fmt::Display for RuleViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.source, self.rule_id().as_str())
    }
}

impl Error for RuleViolation {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// A decision is prepared against a clone of authoritative state. It can be
/// inspected or rejected without mutating the match; `evolve` is the only
/// operation in this seam that commits the decided next state.
#[derive(Clone, Debug)]
pub(crate) struct GameDecision {
    next_state: MatchState,
    replay_frames: Vec<RecordedReplayFrame>,
}

pub(crate) mod actions {
    use super::*;

    pub(crate) fn decide(
        state: &MatchState,
        side: Side,
        command: GameCommand,
        action_index: u32,
    ) -> Result<GameDecision, RuleViolation> {
        let mut next_state = state.clone();
        let attempted_command = command.clone();
        let replay_frames = next_state
            .apply_game_command_recording_for_side(side, command, action_index)
            .map_err(|error| RuleViolation::for_command(&attempted_command, error))?;

        Ok(GameDecision {
            next_state,
            replay_frames,
        })
    }

    pub(crate) fn evolve(
        state: &mut MatchState,
        decision: GameDecision,
    ) -> Vec<RecordedReplayFrame> {
        *state = decision.next_state;
        decision.replay_frames
    }

    pub(crate) fn apply_recording(
        state: &mut MatchState,
        side: Side,
        command: GameCommand,
        action_index: u32,
    ) -> Result<Vec<RecordedReplayFrame>, RuleViolation> {
        let decision = decide(state, side, command, action_index)?;
        Ok(evolve(state, decision))
    }
}

pub(crate) mod queries {
    use super::*;

    /// Check command legality without mutating authoritative state.
    ///
    /// During the staged migration this delegates to the existing deterministic
    /// rules implementation on a cloned state, guaranteeing one source of truth
    /// instead of reimplementing legality in a second query path.
    #[allow(
        dead_code,
        reason = "query boundary is introduced before UI and AI consumers migrate"
    )]
    pub(crate) fn validate_command(
        state: &MatchState,
        side: Side,
        command: &GameCommand,
    ) -> Result<(), RuleViolation> {
        state.validate_game_command(side, command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_session::Phase;

    #[test]
    fn advance_ai_is_application_orchestration_not_a_game_command() {
        assert_eq!(
            GameCommand::try_from(MatchActionRequest::AdvanceAi),
            Err(NonGameCommand::AdvanceAi)
        );
    }

    #[test]
    fn legacy_not_adjacent_error_maps_to_command_specific_rules() {
        let movement = GameCommand::MovePiece {
            piece_id: "piece".to_string(),
            to: HexCoord { q: 1, r: 0 },
        };
        let attack = GameCommand::Attack {
            attacker_id: "attacker".to_string(),
            target_id: "target".to_string(),
        };

        assert_eq!(
            RuleViolation::for_command(&movement, MatchError::NotAdjacent).rule_id(),
            RuleId::MovementDestinationAdjacent
        );
        assert_eq!(
            RuleViolation::for_command(&attack, MatchError::NotAdjacent).rule_id(),
            RuleId::AttackTargetInRange
        );
    }

    #[test]
    fn rejected_movement_command_has_stable_rule_id_and_does_not_mutate_state() {
        let game = MatchState::new();
        let before = game
            .to_snapshot_json()
            .expect("initial state should serialize");
        let command = GameCommand::MovePiece {
            piece_id: "missing-piece".to_string(),
            to: HexCoord { q: 0, r: 0 },
        };

        let violation = queries::validate_command(&game, Side::Player, &command)
            .expect_err("missing piece should be illegal");

        assert_eq!(violation.rule_id(), RuleId::PieceExists);
        assert_eq!(violation.rule_id().as_str(), "piece.exists");
        assert_eq!(
            game.to_snapshot_json()
                .expect("unchanged state should serialize"),
            before
        );
    }

    #[test]
    fn finish_phases_and_end_turn_use_tabletop_commands() {
        let mut game = MatchState::new();

        let decision = actions::decide(&game, Side::Player, GameCommand::FinishMovement, 1)
            .expect("finishing movement should be legal");
        assert_eq!(game.phase, Phase::Movement);
        actions::evolve(&mut game, decision);
        assert_eq!(game.phase, Phase::Attack);

        let decision = actions::decide(&game, Side::Player, GameCommand::FinishAttacks, 2)
            .expect("finishing attacks should be legal");
        actions::evolve(&mut game, decision);
        assert_eq!(game.phase, Phase::CardPlay);

        let decision = actions::decide(&game, Side::Player, GameCommand::EndTurn, 3)
            .expect("ending card play should be legal");
        assert_eq!(game.active_side, Side::Player);
        actions::evolve(&mut game, decision);
        assert_eq!(game.active_side, Side::Opponent);
    }

    #[test]
    fn end_turn_in_wrong_phase_is_rejected_without_mutation() {
        let game = MatchState::new();
        let before = game
            .to_snapshot_json()
            .expect("initial state should serialize");

        let violation = queries::validate_command(&game, Side::Player, &GameCommand::EndTurn)
            .expect_err("ending the turn during movement should be illegal");

        assert_eq!(violation.rule_id(), RuleId::PhaseAllowsCommand);
        assert_eq!(
            game.to_snapshot_json()
                .expect("unchanged state should serialize"),
            before
        );
    }

    #[test]
    fn attack_in_wrong_phase_is_rejected_without_mutation() {
        let game = MatchState::new();
        let before = game
            .to_snapshot_json()
            .expect("initial state should serialize");
        let command = GameCommand::Attack {
            attacker_id: game.player.hero.id.clone(),
            target_id: game.opponent.hero.id.clone(),
        };

        let violation = queries::validate_command(&game, Side::Player, &command)
            .expect_err("attacking during movement should be illegal");

        assert_eq!(violation.rule_id(), RuleId::PhaseAllowsCommand);
        assert_eq!(
            game.to_snapshot_json()
                .expect("unchanged state should serialize"),
            before
        );
    }

    #[test]
    fn legal_movement_is_decided_before_it_is_committed() {
        let mut game = MatchState::new();
        let from = game.player.hero.position;
        let to = empty_adjacent_hex(&game, from);
        let command = GameCommand::MovePiece {
            piece_id: game.player.hero.id.clone(),
            to,
        };

        let decision = actions::decide(&game, Side::Player, command, 1)
            .expect("adjacent empty movement should be legal");

        assert_eq!(game.player.hero.position, from);
        let frames = actions::evolve(&mut game, decision);
        assert_eq!(game.player.hero.position, to);
        assert!(!frames.is_empty());
    }

    #[test]
    fn legal_attack_runs_through_the_same_decide_and_evolve_boundary() {
        let mut game = MatchState::new();
        game.phase = Phase::Attack;
        game.player.hero.position = HexCoord { q: 0, r: 0 };
        game.opponent.hero.position = HexCoord { q: 1, r: 0 };
        game.player.hero.ap_remaining = game.player.hero.max_ap.max(1);
        game.player.hero.has_attacked = false;
        let opponent_hp = game.opponent.hero.hp;
        let command = GameCommand::Attack {
            attacker_id: game.player.hero.id.clone(),
            target_id: game.opponent.hero.id.clone(),
        };

        let decision = actions::decide(&game, Side::Player, command, 1)
            .expect("adjacent attack should be legal");
        assert_eq!(game.opponent.hero.hp, opponent_hp);

        actions::evolve(&mut game, decision);
        assert!(game.opponent.hero.hp < opponent_hp);
        assert!(game.player.hero.has_attacked);
    }

    fn empty_adjacent_hex(game: &MatchState, from: HexCoord) -> HexCoord {
        const DIRECTIONS: [(i32, i32); 6] = [
            (1, 0),
            (1, -1),
            (0, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
        ];

        DIRECTIONS
            .into_iter()
            .map(|(dq, dr)| HexCoord {
                q: from.q + dq,
                r: from.r + dr,
            })
            .find(|candidate| {
                let on_board = game.board.tiles.iter().any(|tile| tile.coord == *candidate);
                let hero_occupied = game.player.hero.position == *candidate
                    || game.opponent.hero.position == *candidate
                    || game
                        .player_two
                        .as_ref()
                        .is_some_and(|player| player.hero.position == *candidate)
                    || game
                        .opponent_two
                        .as_ref()
                        .is_some_and(|player| player.hero.position == *candidate);
                let unit_occupied = game
                    .board
                    .units
                    .iter()
                    .any(|unit| unit.position == *candidate);

                on_board && !hero_occupied && !unit_occupied
            })
            .expect("initial hero should have an adjacent empty board hex")
    }
}
