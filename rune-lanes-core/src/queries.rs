use serde::{Deserialize, Serialize};

use crate::commands::{CommandContext, GameCommand};
use crate::event_sourcing::EventSourcedMatch;
#[cfg(test)]
use crate::rules::CURRENT_RULESET;
use crate::rules::RuneLanesRuleset;
use crate::{ActionTarget, MatchError, MatchState, Phase, PlayerState, Side};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameQuery {
    Round,
    Phase,
    ActiveSide,
    Winner,
    Ruleset,
    PublicMatch { viewer_side: Side },
    CommandAvailability { side: Side, command: GameCommand },
    LegalCommands { side: Side },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GameQueryResult {
    Round { round: u32 },
    Phase { phase: Phase },
    ActiveSide { side: Side },
    Winner { winner: Option<Side> },
    Ruleset { ruleset: RuneLanesRuleset },
    PublicMatch { match_state: serde_json::Value },
    CommandAvailability { availability: CommandAvailability },
    LegalCommands { commands: Vec<GameCommand> },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandAvailability {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection: Option<MatchError>,
}

#[derive(Clone, Copy, Debug)]
pub struct MatchQueries<'a> {
    state: &'a MatchState,
}

impl MatchState {
    #[must_use]
    pub fn queries(&self) -> MatchQueries<'_> {
        MatchQueries { state: self }
    }

    #[must_use]
    pub fn query(&self, query: GameQuery) -> GameQueryResult {
        let queries = self.queries();
        match query {
            GameQuery::Round => GameQueryResult::Round {
                round: queries.round(),
            },
            GameQuery::Phase => GameQueryResult::Phase {
                phase: queries.phase(),
            },
            GameQuery::ActiveSide => GameQueryResult::ActiveSide {
                side: queries.active_side(),
            },
            GameQuery::Winner => GameQueryResult::Winner {
                winner: queries.winner(),
            },
            GameQuery::Ruleset => GameQueryResult::Ruleset {
                ruleset: queries.ruleset(),
            },
            GameQuery::PublicMatch { viewer_side } => GameQueryResult::PublicMatch {
                match_state: queries.public_match(viewer_side),
            },
            GameQuery::CommandAvailability { side, command } => {
                GameQueryResult::CommandAvailability {
                    availability: queries.command_availability(side, &command),
                }
            }
            GameQuery::LegalCommands { side } => GameQueryResult::LegalCommands {
                commands: queries.legal_commands(side),
            },
        }
    }
}

impl EventSourcedMatch {
    #[must_use]
    pub fn queries(&self) -> MatchQueries<'_> {
        self.state().queries()
    }

    #[must_use]
    pub fn query(&self, query: GameQuery) -> GameQueryResult {
        self.state().query(query)
    }
}

impl GameQuery {
    #[must_use]
    pub fn execute(self, state: &MatchState) -> GameQueryResult {
        state.query(self)
    }
}

impl MatchQueries<'_> {
    #[must_use]
    pub fn round(&self) -> u32 {
        self.state.round
    }

    #[must_use]
    pub fn phase(&self) -> Phase {
        self.state.phase.clone()
    }

    #[must_use]
    pub fn active_side(&self) -> Side {
        self.state.active_side
    }

    #[must_use]
    pub fn winner(&self) -> Option<Side> {
        self.state.winner
    }

    #[must_use]
    pub fn ruleset(&self) -> RuneLanesRuleset {
        self.state.ruleset
    }

    #[must_use]
    pub fn public_match(&self, viewer_side: Side) -> serde_json::Value {
        self.state.public_value_for_side(viewer_side)
    }

    #[must_use]
    pub fn command_availability(&self, side: Side, command: &GameCommand) -> CommandAvailability {
        match command.check(
            self.state,
            CommandContext {
                side,
                action_index: 0,
            },
        ) {
            Ok(()) => CommandAvailability {
                allowed: true,
                rejection: None,
            },
            Err(error) => CommandAvailability {
                allowed: false,
                rejection: Some(error),
            },
        }
    }

    /// Every concrete command `side` could submit right now, in a stable order.
    ///
    /// Candidates are enumerated structurally (each Card in hand against every
    /// Hex and piece, each own piece against every Hex and piece, each Carried
    /// Item and Building, and the turn-flow commands) and then filtered through
    /// [`Self::command_availability`]. The enumeration encodes no rules of its
    /// own, so legality stays owned by the real `GameCommand` path.
    #[must_use]
    pub fn legal_commands(&self, side: Side) -> Vec<GameCommand> {
        self.candidate_commands(side)
            .into_iter()
            .filter(|command| self.command_availability(side, command).allowed)
            .collect()
    }

    fn candidate_commands(&self, side: Side) -> Vec<GameCommand> {
        let state = self.state;
        let participants: Vec<&PlayerState> = [
            Some(&state.player),
            Some(&state.opponent),
            state.player_two.as_ref(),
            state.opponent_two.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        let hexes: Vec<_> = state.board.tiles.iter().map(|tile| tile.coord).collect();
        let piece_ids: Vec<&str> = participants
            .iter()
            .map(|participant| participant.hero.id.as_str())
            .chain(state.board.units.iter().map(|unit| unit.id.as_str()))
            .collect();
        let targets: Vec<ActionTarget> = hexes
            .iter()
            .map(|coord| ActionTarget::Hex { coord: *coord })
            .chain(piece_ids.iter().map(|id| ActionTarget::Piece {
                piece_id: (*id).to_string(),
            }))
            .collect();

        let mut commands = Vec::new();
        let Some(owner) = participants
            .iter()
            .find(|participant| participant.side == side)
        else {
            return commands;
        };

        for card in &owner.hand {
            commands.extend(targets.iter().map(|target| GameCommand::PlayCard {
                card_id: card.id.clone(),
                target: target.clone(),
            }));
        }

        let own_pieces = std::iter::once((owner.hero.id.as_str(), &owner.hero.items)).chain(
            state
                .board
                .units
                .iter()
                .filter(|unit| unit.side == side)
                .map(|unit| (unit.id.as_str(), &unit.items)),
        );
        for (piece_id, items) in own_pieces {
            commands.extend(hexes.iter().map(|coord| GameCommand::MovePiece {
                piece_id: piece_id.to_string(),
                to: *coord,
            }));
            commands.extend(piece_ids.iter().map(|target_id| GameCommand::Attack {
                attacker_id: piece_id.to_string(),
                target_id: (*target_id).to_string(),
            }));
            for item in items {
                commands.extend(
                    std::iter::once(None)
                        .chain(targets.iter().cloned().map(Some))
                        .map(|target| GameCommand::ActivateItem {
                            carrier_id: piece_id.to_string(),
                            item_id: item.id.clone(),
                            target,
                        }),
                );
            }
        }

        commands.extend(state.board.buildings.iter().map(|building| {
            GameCommand::ActivateBuilding {
                building_id: building.id.clone(),
            }
        }));
        commands.extend([
            GameCommand::StartAttackPhase,
            GameCommand::StartCardPlay,
            GameCommand::EndTurn,
            GameCommand::PassPriority,
        ]);
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_roundtrip_as_stable_tagged_application_contract() {
        let query = GameQuery::CommandAvailability {
            side: Side::Player,
            command: GameCommand::StartAttackPhase,
        };
        let encoded = serde_json::to_value(&query).unwrap();
        let decoded: GameQuery = serde_json::from_value(encoded.clone()).unwrap();

        assert_eq!(decoded, query);
        assert_eq!(encoded["kind"], "commandAvailability");
        assert_eq!(encoded["command"]["type"], "startAttackPhase");
    }

    #[test]
    fn query_facade_is_read_only_and_exposes_current_ruleset() {
        let state = MatchState::new_with_seed(7);
        let before = state.to_snapshot_json().unwrap();

        assert_eq!(state.queries().round(), 1);
        assert_eq!(state.queries().ruleset(), CURRENT_RULESET);
        assert_eq!(
            state.queries().public_match(Side::Player)["activeSide"],
            "player"
        );
        assert_eq!(state.to_snapshot_json().unwrap(), before);
    }

    #[test]
    fn command_availability_reuses_the_authoritative_command_rules() {
        let state = MatchState::new_with_seed(7);

        let legal = state
            .queries()
            .command_availability(Side::Player, &GameCommand::StartAttackPhase);
        let illegal = state
            .queries()
            .command_availability(Side::Opponent, &GameCommand::StartAttackPhase);

        assert_eq!(
            legal,
            CommandAvailability {
                allowed: true,
                rejection: None,
            }
        );
        assert_eq!(
            illegal,
            CommandAvailability {
                allowed: false,
                rejection: Some(MatchError::NotActiveSide),
            }
        );
    }

    #[test]
    fn legal_commands_agree_with_command_availability() {
        let state = MatchState::new_with_seed(7);
        let before = state.to_snapshot_json().unwrap();

        let legal = state.queries().legal_commands(Side::Player);

        assert!(legal.contains(&GameCommand::StartAttackPhase));
        assert!(!legal.contains(&GameCommand::EndTurn));
        assert!(
            legal
                .iter()
                .any(|command| matches!(command, GameCommand::MovePiece { .. }))
        );
        assert!(legal.iter().all(|command| {
            state
                .queries()
                .command_availability(Side::Player, command)
                .allowed
        }));
        assert!(state.queries().legal_commands(Side::Opponent).is_empty());
        assert_eq!(state.to_snapshot_json().unwrap(), before);
    }

    #[test]
    fn query_dispatch_returns_transport_safe_rules_and_availability() {
        let state = MatchState::new_with_seed(7);
        let rules = serde_json::to_value(state.query(GameQuery::Ruleset)).unwrap();
        let availability = serde_json::to_value(state.query(GameQuery::CommandAvailability {
            side: Side::Opponent,
            command: GameCommand::StartAttackPhase,
        }))
        .unwrap();

        assert_eq!(rules["kind"], "ruleset");
        assert_eq!(rules["ruleset"]["arena"]["duelRadius"], 3);
        assert_eq!(availability["kind"], "commandAvailability");
        assert_eq!(availability["availability"]["allowed"], false);
        assert_eq!(availability["availability"]["rejection"], "notActiveSide");
    }
}
