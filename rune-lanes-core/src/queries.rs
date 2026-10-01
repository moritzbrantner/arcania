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
    CommandProjection { viewer_side: Side },
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
    CommandProjection { projection: MatchCommandProjection },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandAvailability {
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection: Option<MatchError>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CardCommandAvailability {
    pub card_id: String,
    pub allowed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rejection: Option<MatchError>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MatchCommandProjection {
    pub viewer_side: Side,
    pub proactive_card_phases: Vec<Phase>,
    pub legal_commands: Vec<GameCommand>,
    pub cards: Vec<CardCommandAvailability>,
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
            GameQuery::CommandProjection { viewer_side } => GameQueryResult::CommandProjection {
                projection: queries.command_projection(viewer_side),
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
        let mut view = self.state.public_value_for_side(viewer_side);
        let projection = self.command_projection(viewer_side);
        view["legalCommands"] = serde_json::json!(&projection.legal_commands);
        view["commandProjection"] = serde_json::json!(projection);
        view
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
        self.command_projection(side).legal_commands
    }

    /// Derive command and Card availability together from the real command path.
    /// Only the viewer's hand contributes Card commands or rejection information.
    #[must_use]
    pub fn command_projection(&self, viewer_side: Side) -> MatchCommandProjection {
        let participant = match viewer_side {
            Side::Player => Some(&self.state.player),
            Side::Opponent => Some(&self.state.opponent),
            Side::PlayerTwo => self.state.player_two.as_ref(),
            Side::OpponentTwo => self.state.opponent_two.as_ref(),
        };
        let mut cards: Vec<_> = participant
            .into_iter()
            .flat_map(|participant| &participant.hand)
            .map(|card| CardCommandAvailability {
                card_id: card.id.clone(),
                allowed: false,
                rejection: None,
            })
            .collect();
        let mut legal_commands = Vec::new();
        for command in self.candidate_commands(viewer_side) {
            let availability = self.command_availability(viewer_side, &command);
            if let GameCommand::PlayCard { card_id, .. } = &command
                && let Some(card) = cards.iter_mut().find(|card| &card.card_id == card_id)
                && !card.allowed
            {
                if availability.allowed {
                    card.allowed = true;
                    card.rejection = None;
                } else if card.rejection.is_none() {
                    card.rejection = availability.rejection;
                }
            }
            if availability.allowed {
                legal_commands.push(command);
            }
        }
        for card in &mut cards {
            if !card.allowed && card.rejection.is_none() {
                card.rejection = Some(MatchError::InvalidTarget);
            }
        }
        let proactive_card_phases = if self.state.ruleset.turn.movement_card_play {
            vec![Phase::Movement, Phase::CardPlay]
        } else {
            vec![Phase::CardPlay]
        };
        MatchCommandProjection {
            viewer_side,
            proactive_card_phases,
            legal_commands,
            cards,
        }
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
    fn command_projection_is_viewer_scoped_read_only_and_matches_real_commands() {
        let deck = |side: Side| {
            let mut card = crate::starter_card_templates()
                .into_iter()
                .find(|card| card.template_id == "ember-squire")
                .unwrap();
            card.id = format!("{side:?}-card");
            vec![card]
        };
        let state = MatchState::new_shared_two_v_two_with_progression_loadouts(
            crate::HeroType::Runekeeper,
            crate::HeroType::Runekeeper,
            crate::HeroType::Runekeeper,
            crate::HeroType::Runekeeper,
            deck(Side::Player),
            deck(Side::Opponent),
            deck(Side::PlayerTwo),
            deck(Side::OpponentTwo),
            crate::MatchProgressionLoadout::default(),
            crate::MatchProgressionLoadout::default(),
            crate::MatchProgressionLoadout::default(),
            crate::MatchProgressionLoadout::default(),
        );
        let before = state.to_snapshot_json().unwrap();
        for side in [
            Side::Player,
            Side::Opponent,
            Side::PlayerTwo,
            Side::OpponentTwo,
        ] {
            let projection = state.queries().command_projection(side);
            assert_eq!(projection.viewer_side, side);
            assert_eq!(
                projection.proactive_card_phases,
                [Phase::Movement, Phase::CardPlay]
            );
            assert_eq!(projection.cards.len(), 1);
            assert_eq!(projection.cards[0].card_id, format!("{side:?}-card"));
            for command in &projection.legal_commands {
                command
                    .check(
                        &state,
                        CommandContext {
                            side,
                            action_index: 0,
                        },
                    )
                    .unwrap();
                if let GameCommand::PlayCard { card_id, .. } = command {
                    assert_eq!(card_id, &projection.cards[0].card_id);
                }
            }
            assert_eq!(projection.cards[0].allowed, side == Side::Player);
            assert_eq!(
                projection.cards[0].rejection,
                if side == Side::Player {
                    None
                } else {
                    Some(MatchError::NotActiveSide)
                }
            );
            let public = state.queries().public_match(side);
            assert_eq!(
                public["commandProjection"],
                serde_json::to_value(&projection).unwrap()
            );
            assert_eq!(
                public["legalCommands"],
                public["commandProjection"]["legalCommands"]
            );
            assert_eq!(state.to_snapshot_json().unwrap(), before);
        }
        assert!(!before.contains("commandProjection"));
        assert!(
            !state
                .replay_value(crate::ReplayVisibility::Public)
                .to_string()
                .contains("commandProjection")
        );
    }

    #[test]
    fn card_projection_reports_real_phase_mana_target_and_priority_rejections() {
        let mut state = MatchState::new_with_seed(7);
        state.mode = crate::MatchMode::Shared;
        let mut card = crate::starter_card_templates()
            .into_iter()
            .find(|card| card.template_id == "ember-squire")
            .unwrap();
        card.id = "projection-unit".into();
        state.player.hand = vec![card];
        state.phase = Phase::Attack;
        assert_eq!(
            state.queries().command_projection(Side::Player).cards[0].rejection,
            Some(MatchError::WrongPhase)
        );
        state.phase = Phase::Movement;
        state.player.mana = 0;
        assert_eq!(
            state.queries().command_projection(Side::Player).cards[0].rejection,
            Some(MatchError::NotEnoughMana)
        );
        state.player.mana = 3;
        state.board.tiles.clear();
        assert_eq!(
            state.queries().command_projection(Side::Player).cards[0].rejection,
            Some(MatchError::InvalidTarget)
        );
        state.board = crate::HexBoard::new(3);
        let mut low = crate::starter_card_templates()
            .into_iter()
            .find(|card| card.template_id == "runic-insight")
            .unwrap();
        low.id = "low-priority".into();
        state.player.hand.push(low);
        state
            .apply_action_recording_for_side(
                Side::Player,
                crate::MatchActionRequest::PlayCard {
                    card_id: "projection-unit".into(),
                    target: ActionTarget::Hex {
                        coord: crate::HexCoord { q: 0, r: 2 },
                    },
                },
                0,
            )
            .unwrap();
        let mut response = crate::starter_card_templates()
            .into_iter()
            .find(|card| card.template_id == "quick-salve")
            .unwrap();
        response.id = "response".into();
        state.opponent.hand = vec![response];
        state.opponent.hero.hp -= 2;
        assert!(state.queries().command_projection(Side::Opponent).cards[0].allowed);
        state
            .apply_action_recording_for_side(
                Side::Opponent,
                crate::MatchActionRequest::PlayCard {
                    card_id: "response".into(),
                    target: ActionTarget::Piece {
                        piece_id: state.opponent.hero.id.clone(),
                    },
                },
                1,
            )
            .unwrap();
        assert_eq!(
            state.queries().command_projection(Side::Player).cards[0].rejection,
            Some(MatchError::PriorityTooLow)
        );

        let mut legacy = MatchState::new_with_seed(7);
        legacy.ruleset = crate::rules::LEGACY_RULESET;
        let projection = legacy.queries().command_projection(Side::Player);
        assert_eq!(projection.proactive_card_phases, [Phase::CardPlay]);
        assert!(
            !projection
                .legal_commands
                .iter()
                .any(|command| matches!(command, GameCommand::PlayCard { .. }))
        );
    }

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
