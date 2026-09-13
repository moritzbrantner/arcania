use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

mod actions;
mod board;
#[path = "card_interactions.rs"]
mod card_interactions;
mod construction;
mod effects;
mod ids;
mod queries;
mod replay;
#[cfg(any(test, debug_assertions))]
pub mod scenarios;
mod serialization;
mod solo_ai_policy;
mod stack;
mod turn_flow;
mod types;
pub use replay::*;
pub use types::*;
pub(crate) use types::{DestroyedUnit, PieceView, StatBonus};

use board::piece_can_attack;
use serialization::MatchSnapshot;
#[cfg(test)]
pub(crate) use solo_ai_policy::SoloAiRuleId;
pub(crate) use solo_ai_policy::{AiPolicyConfig, SoloAiPolicy, default_policy_config_path};
use solo_ai_policy::{SoloAiActionIntent, SoloAiCarriedItem, SoloAiDecision, SoloAiView};

const BOARD_RADIUS: i32 = 3;
const TWO_V_TWO_BOARD_RADIUS: i32 = 4;
const HERO_MANA: u8 = 3;
const OPENING_HAND_SIZE: usize = 7;

enum AiDecisionApplication {
    Applied,
    FinishTurn,
    Illegal(MatchError),
}

impl MatchState {
    #[allow(dead_code, reason = "kept as the default rules-engine constructor")]
    pub fn new() -> Self {
        Self::new_with_player_hero_type(HeroType::default())
    }

    pub fn new_with_player_hero_type(player_hero_type: HeroType) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_and_player_hero_type(seed, player_hero_type)
    }

    #[allow(
        dead_code,
        reason = "kept as a progression-free constructor for tests and callers"
    )]
    pub fn new_with_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
    ) -> Self {
        Self::new_with_progression_loadouts(
            player_hero_type,
            opponent_hero_type,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    pub fn new_with_progression_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Solo,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
        )
    }

    #[allow(
        dead_code,
        reason = "kept as the default shared rules-engine constructor"
    )]
    pub fn new_shared_with_hero_types(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_and_mode(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Shared,
        )
    }

    #[allow(
        dead_code,
        reason = "kept as a progression-free constructor for tests and callers"
    )]
    pub fn new_shared_with_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
    ) -> Self {
        Self::new_shared_with_progression_loadouts(
            player_hero_type,
            opponent_hero_type,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    pub fn new_shared_with_progression_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Shared,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
        )
    }

    pub fn new_shared_two_v_two_with_progression_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_two_hero_type: HeroType,
        opponent_two_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_two_deck: Vec<Card>,
        opponent_two_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
        player_two_progression: MatchProgressionLoadout,
        opponent_two_progression: MatchProgressionLoadout,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Shared,
            TWO_V_TWO_BOARD_RADIUS,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
            Some((
                player_two_hero_type,
                player_two_deck,
                player_two_progression,
            )),
            Some((
                opponent_two_hero_type,
                opponent_two_deck,
                opponent_two_progression,
            )),
        )
    }

    pub fn to_snapshot_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&MatchSnapshot::from(self))
    }

    pub fn from_snapshot_json(snapshot: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str::<MatchSnapshot>(snapshot).map(Self::from)
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

    #[allow(
        dead_code,
        reason = "kept as the deterministic rules-engine test constructor"
    )]
    fn new_with_seed(seed: u64) -> Self {
        Self::new_with_seed_and_player_hero_type(seed, HeroType::default())
    }

    fn new_with_seed_and_player_hero_type(seed: u64, player_hero_type: HeroType) -> Self {
        Self::new_with_seed_hero_types_and_mode(
            seed,
            player_hero_type,
            HeroType::Runekeeper,
            MatchMode::Solo,
        )
    }

    fn new_with_seed_hero_types_and_mode(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        mode: MatchMode,
    ) -> Self {
        let player_deck = crate::deck_library::deck_from_snapshot(
            Side::Player,
            &crate::deck_library::starter_deck_snapshot(),
        )
        .expect("starter player deck should be valid");
        let opponent_deck = crate::deck_library::deck_from_snapshot(
            Side::Opponent,
            &crate::deck_library::starter_deck_snapshot(),
        )
        .expect("starter opponent deck should be valid");
        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            mode,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "test and store constructors pass mirrored player/opponent setup explicitly"
    )]
    fn new_with_seed_hero_types_mode_and_decks(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        mode: MatchMode,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
    ) -> Self {
        Self::new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
            seed,
            player_hero_type,
            opponent_hero_type,
            mode,
            BOARD_RADIUS,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
            None,
            None,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "2v2 setup passes mirrored per-seat setup explicitly"
    )]
    fn new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        mode: MatchMode,
        board_radius: i32,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
        player_two: Option<(HeroType, Vec<Card>, MatchProgressionLoadout)>,
        opponent_two: Option<(HeroType, Vec<Card>, MatchProgressionLoadout)>,
    ) -> Self {
        let mut game = Self {
            mode,
            round: 1,
            phase: Phase::Movement,
            active_side: Side::Player,
            player: PlayerState::new(
                Side::Player,
                board_radius,
                seed ^ 0xA11C_E551_1234_5678,
                player_hero_type,
                player_deck,
                player_progression,
            ),
            opponent: PlayerState::new(
                Side::Opponent,
                board_radius,
                seed ^ 0x0B0E_1234_9876_5432,
                opponent_hero_type,
                opponent_deck,
                opponent_progression,
            ),
            player_two: player_two.map(|(hero_type, deck, progression)| {
                PlayerState::new(
                    Side::PlayerTwo,
                    board_radius,
                    seed ^ 0xA11C_E552_2234_5678,
                    hero_type,
                    deck,
                    progression,
                )
            }),
            opponent_two: opponent_two.map(|(hero_type, deck, progression)| {
                PlayerState::new(
                    Side::OpponentTwo,
                    board_radius,
                    seed ^ 0x0B0E_2234_9876_5432,
                    hero_type,
                    deck,
                    progression,
                )
            }),
            board: HexBoard::new(board_radius),
            action_stack: Vec::new(),
            priority_side: None,
            log: vec!["The heroes enter the hex arena.".to_string()],
            winner: None,
            priority_passes: Vec::new(),
            next_stack_item_id: 1,
            next_unit_id: 1,
            next_item_id: 1,
            next_building_id: 1,
        };

        for _ in 0..opening_hand_size(&game.player.progression) {
            game.player.draw();
        }
        for _ in 0..opening_hand_size(&game.opponent.progression) {
            game.opponent.draw();
        }
        if let Some(player_two) = &mut game.player_two {
            for _ in 0..opening_hand_size(&player_two.progression) {
                player_two.draw();
            }
        }
        if let Some(opponent_two) = &mut game.opponent_two {
            for _ in 0..opening_hand_size(&opponent_two.progression) {
                opponent_two.draw();
            }
        }
        let mut ignored_frames = Vec::new();
        game.start_turn(Side::Player, &mut ignored_frames, None);

        game
    }

    pub(crate) fn new_ai_lab_with_seed_and_decks(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
    ) -> Self {
        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Solo,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    #[allow(dead_code, reason = "kept as the non-recording rules-engine API")]
    pub fn apply_action(&mut self, request: MatchActionRequest) -> Result<(), MatchError> {
        self.apply_action_internal(Side::Player, request, None)
            .map(|_| ())
    }

    pub fn apply_action_recording(
        &mut self,
        request: MatchActionRequest,
        action_index: u32,
    ) -> Result<Vec<RecordedReplayFrame>, MatchError> {
        self.apply_action_recording_for_side(Side::Player, request, action_index)
    }

    pub fn apply_action_recording_for_side(
        &mut self,
        side: Side,
        request: MatchActionRequest,
        action_index: u32,
    ) -> Result<Vec<RecordedReplayFrame>, MatchError> {
        self.apply_action_internal(side, request, Some(action_index))
    }

    pub fn forfeit_recording(
        &mut self,
        winner: Side,
        action_index: u32,
    ) -> Vec<RecordedReplayFrame> {
        let mut frames = Vec::new();
        if self.phase == Phase::MatchOver {
            return frames;
        }

        self.phase = Phase::MatchOver;
        self.winner = Some(winner);
        self.log
            .insert(0, format!("{} wins by forfeit.", winner.label()));
        self.truncate_log();
        self.record_replay_frame(
            &mut frames,
            Some(action_index),
            ReplayEvent::MatchEnded { winner },
        );
        frames
    }

    pub fn public_value_for_side(&self, viewer_side: Side) -> serde_json::Value {
        let mut value = json!({
            "mode": self.mode,
            "format": self.format(),
            "round": self.round,
            "phase": self.phase,
            "activeSide": self.active_side,
            "prioritySide": self.priority_side,
            "player": self.player.replay_value(viewer_side == Side::Player),
            "opponent": self.opponent.replay_value(viewer_side == Side::Opponent),
            "playerTwo": self.player_two.as_ref().map(|player| {
                player.replay_value(viewer_side == Side::PlayerTwo || viewer_side.team() == player.side.team())
            }),
            "opponentTwo": self.opponent_two.as_ref().map(|player| {
                player.replay_value(viewer_side == Side::OpponentTwo || viewer_side.team() == player.side.team())
            }),
            "participants": self.public_participants_for_side(viewer_side),
            "board": self.board,
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

    pub fn format(&self) -> &'static str {
        if self.player_two.is_some() || self.opponent_two.is_some() {
            "twoVTwo"
        } else {
            "duel"
        }
    }

    fn public_participants_for_side(&self, viewer_side: Side) -> Vec<serde_json::Value> {
        self.participants()
            .into_iter()
            .map(|participant| {
                participant.replay_value(
                    participant.side == viewer_side
                        || participant.side.team() == viewer_side.team(),
                )
            })
            .collect()
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

    fn apply_action_internal(
        &mut self,
        side: Side,
        request: MatchActionRequest,
        action_index: Option<u32>,
    ) -> Result<Vec<RecordedReplayFrame>, MatchError> {
        if self.phase == Phase::MatchOver {
            return Err(MatchError::MatchOver);
        }

        let mut frames = Vec::new();
        match request {
            MatchActionRequest::PlayCard { card_id, target } => {
                self.play_card_for_side(side, card_id, target, &mut frames, action_index)
            }
            MatchActionRequest::MovePiece { piece_id, to } => {
                self.require_turn_action_side(side)?;
                self.require_phase(Phase::Movement)?;
                self.move_piece_for_side(side, &piece_id, to, &mut frames, action_index)
            }
            MatchActionRequest::Attack {
                attacker_id,
                target_id,
            } => {
                self.require_turn_action_side(side)?;
                self.require_phase(Phase::Attack)?;
                self.attack_for_side(side, &attacker_id, &target_id, &mut frames, action_index)
            }
            MatchActionRequest::ActivateItem {
                carrier_id,
                item_id,
                target,
            } => self.activate_item_for_side(
                side,
                &carrier_id,
                &item_id,
                target,
                &mut frames,
                action_index,
            ),
            MatchActionRequest::ActivateBuilding { building_id } => {
                self.require_turn_action_side(side)?;
                self.activate_building_for_side(side, &building_id, &mut frames, action_index)
            }
            MatchActionRequest::StartAttackPhase => {
                self.require_turn_action_side(side)?;
                self.start_attack_phase_for_side(side, &mut frames, action_index)
            }
            MatchActionRequest::StartCardPlay => {
                self.require_turn_action_side(side)?;
                self.start_card_play_for_side(side, &mut frames, action_index)
            }
            MatchActionRequest::EndTurn => {
                self.require_turn_action_side(side)?;
                self.require_phase(Phase::CardPlay)?;
                if !self.action_stack.is_empty() {
                    return Err(MatchError::StackPending);
                }
                if self.mode == MatchMode::Shared {
                    self.end_shared_turn(side, &mut frames, action_index);
                } else {
                    self.end_player_turn(&mut frames, action_index);
                }
                Ok(())
            }
            MatchActionRequest::PassPriority => {
                self.pass_priority_for_side(side, &mut frames, action_index)
            }
            MatchActionRequest::AdvanceAi => self.advance_solo_ai(&mut frames, action_index),
        }?;

        Ok(frames)
    }

    fn require_turn_action_side(&self, side: Side) -> Result<(), MatchError> {
        if self.player_ref(side).knocked_out {
            return Err(MatchError::NotActiveSide);
        }
        if !self.action_stack.is_empty() {
            return Err(MatchError::StackPending);
        }
        if side != self.active_side {
            return Err(MatchError::NotActiveSide);
        }
        Ok(())
    }

    fn require_phase(&self, phase: Phase) -> Result<(), MatchError> {
        if self.phase != phase {
            return Err(MatchError::WrongPhase);
        }
        Ok(())
    }

    fn record_replay_frame(
        &self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
        event: ReplayEvent,
    ) {
        frames.push(RecordedReplayFrame {
            action_index,
            event,
            snapshot_json: self
                .to_snapshot_json()
                .expect("match snapshot should serialize for replay"),
        });
    }

    fn end_player_turn(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        self.log.insert(0, "You ended your turn.".to_string());
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::TurnEnded {
                side: Side::Player,
                round: self.round,
            },
        );
        self.start_turn(Side::Opponent, frames, action_index);
        self.truncate_log();
    }

    fn end_shared_turn(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        self.log
            .insert(0, format!("{} ended their turn.", side.label()));
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::TurnEnded {
                side,
                round: self.round,
            },
        );

        let next_side = self.next_turn_side_after(side);
        if self
            .participant_sides()
            .iter()
            .position(|candidate| *candidate == next_side)
            .unwrap_or_default()
            <= self
                .participant_sides()
                .iter()
                .position(|candidate| *candidate == side)
                .unwrap_or_default()
        {
            self.round += 1;
            self.log.insert(0, format!("Round {} begins.", self.round));
            self.truncate_log();
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::RoundStarted { round: self.round },
            );
        }

        self.start_turn(next_side, frames, action_index);
        self.truncate_log();
    }

    fn start_attack_phase_for_side(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        self.require_phase(Phase::Movement)?;
        self.transition_to_phase(side, Phase::Attack, frames, action_index);
        Ok(())
    }

    fn start_card_play_for_side(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        if self.phase != Phase::Movement && self.phase != Phase::Attack {
            return Err(MatchError::WrongPhase);
        }
        self.transition_to_phase(side, Phase::CardPlay, frames, action_index);
        Ok(())
    }

    fn transition_to_phase(
        &mut self,
        side: Side,
        phase: Phase,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        if self.phase == phase || self.phase == Phase::MatchOver {
            return;
        }
        self.phase = phase.clone();
        self.record_replay_frame(frames, action_index, ReplayEvent::PhaseChanged { side, phase });
    }

    fn auto_start_card_play_if_attacks_exhausted(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        if self.phase != Phase::Attack || !self.action_stack.is_empty() {
            return;
        }
        if !self.side_has_legal_attack(self.active_side) {
            self.transition_to_phase(self.active_side, Phase::CardPlay, frames, action_index);
        }
    }

    fn side_has_legal_attack(&self, side: Side) -> bool {
        let targets = self.pieces_for_opposing_team(side);
        self.pieces_for_side(side).into_iter().any(|attacker| {
            attacker.ap_remaining > 0
                && !attacker.has_attacked
                && targets
                    .iter()
                    .any(|target| piece_can_attack(&attacker, target))
        })
    }

    fn refresh_unit_armor_for_turn(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let refreshed_units: Vec<_> = self
            .board
            .units
            .iter_mut()
            .filter(|unit| unit.side == side && unit.armor > 0 && unit.armor < unit.max_armor)
            .map(|unit| {
                let amount = unit.max_armor - unit.armor;
                unit.armor = unit.max_armor;
                (unit.id.clone(), amount)
            })
            .collect();

        for (unit_id, amount) in refreshed_units {
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::UnitArmorRefreshed {
                    side,
                    unit_id,
                    amount,
                },
            );
        }
    }

    fn refresh_mana_from_sources(&mut self, side: Side) {
        let mana = mana_with_progression(
            HERO_MANA.saturating_add(self.occupied_mana_sources(side)),
            self.player_ref(side).progression.effects.mana_delta,
        );
        let player = self.player_mut(side);
        player.max_mana = mana;
        player.mana = mana;
    }

    fn occupied_mana_sources(&self, side: Side) -> u8 {
        let amount = self
            .board
            .buildings
            .iter()
            .filter_map(|building| match building.effect {
                BuildingEffect::TurnStartMana { amount } => Some((building.position, amount)),
                _ => None,
            })
            .filter_map(|(position, amount)| {
                self.piece_view_at(position)
                    .is_some_and(|piece| piece.side == side)
                    .then_some(amount)
            })
            .fold(0_u16, |total, amount| {
                total.saturating_add(u16::from(amount))
            });
        let legacy_amount = self
            .board
            .mana_sources
            .iter()
            .filter(|source| {
                self.piece_view_at(**source)
                    .is_some_and(|piece| piece.side == side)
            })
            .count()
            .min(usize::from(u8::MAX)) as u16;
        amount.saturating_add(legacy_amount).min(u16::from(u8::MAX)) as u8
    }

    fn has_building(&self, coord: HexCoord) -> bool {
        self.board
            .buildings
            .iter()
            .any(|building| building.position == coord)
            || self.board.mana_sources.contains(&coord)
    }

    fn public_board(&self) -> HexBoard {
        let mut board = self.board.clone();
        for unit in &mut board.units {
            let bonus = self.aura_stat_bonus_for(unit.side, unit.position, false);
            unit.attack += bonus.attack;
            unit.armor += bonus.armor;
            unit.max_armor += bonus.armor;
            apply_ap_delta(&mut unit.ap_remaining, &mut unit.max_ap, bonus.max_ap);
        }
        board
    }

    fn play_card_for_side(
        &mut self,
        side: Side,
        card_id: String,
        target: ActionTarget,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        let stack_was_empty = self.action_stack.is_empty();
        let card = {
            if self.player_ref(side).knocked_out {
                return Err(MatchError::NotActiveSide);
            }
            let player = self.player_ref(side);
            player
                .hand
                .iter()
                .find(|card| card.id == card_id)
                .cloned()
                .ok_or(MatchError::CardNotFound)?
        };

        {
            let player = self.player_ref(side);
            if player.mana < card.cost {
                return Err(MatchError::NotEnoughMana);
            }
        }

        if stack_was_empty {
            if side != self.active_side {
                return Err(MatchError::NotActiveSide);
            }
            self.require_phase(Phase::CardPlay)?;
        } else {
            if self.priority_side != Some(side) {
                return Err(MatchError::NotPrioritySide);
            }
            let CardKind::Spell { priority, .. } = &card.kind else {
                return Err(MatchError::StackPending);
            };
            let pending_priority = self
                .action_stack
                .last()
                .map(|item| item.priority)
                .unwrap_or_default();
            if *priority <= pending_priority {
                return Err(MatchError::PriorityTooLow);
            }
        }

        match &card.kind {
            CardKind::Unit { .. } => {
                let hero_position = self.player_ref(side).hero.position;
                let planned_unit_play = card_interactions::plan_unit_play(
                    &card,
                    target,
                    hero_position,
                    &self.board,
                    |coord| self.is_occupied(coord),
                )?;

                self.spend_card_resources(side, &card_id, &card)?;
                self.log.insert(
                    0,
                    format!("{} put {} on the stack.", side.label(), card.name),
                );
                let item = self.push_stack_item(
                    side,
                    0,
                    StackAction::PlayUnit {
                        card: CardSummary::from(&card),
                        coord: planned_unit_play.coord,
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::CardPlayed {
                        side,
                        card: CardSummary::from(&card),
                        target: ActionTarget::Hex {
                            coord: planned_unit_play.coord,
                        },
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::ActionQueued { side, item },
                );
            }
            CardKind::Item { .. } => {
                let ActionTarget::Piece { piece_id } = target else {
                    return Err(MatchError::InvalidTarget);
                };
                let target = self
                    .piece_view(&piece_id)
                    .ok_or(MatchError::PieceNotFound)?;
                let caster_position = self.player_ref(side).hero.position;
                let planned_item_play = card_interactions::plan_item_play(
                    &card,
                    ActionTarget::Piece { piece_id },
                    side,
                    caster_position,
                    &target,
                )?;
                if self
                    .carrier_item_count(&planned_item_play.carrier_id)
                    .is_some_and(|count| count >= MAX_CARRIED_ITEMS)
                {
                    return Err(MatchError::InvalidTarget);
                }

                self.spend_card_resources(side, &card_id, &card)?;
                self.log.insert(
                    0,
                    format!("{} put {} on the stack.", side.label(), card.name),
                );
                let item = self.push_stack_item(
                    side,
                    0,
                    StackAction::EquipItem {
                        card: CardSummary::from(&card),
                        carrier_id: planned_item_play.carrier_id.clone(),
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::CardPlayed {
                        side,
                        card: CardSummary::from(&card),
                        target: ActionTarget::Piece {
                            piece_id: planned_item_play.carrier_id,
                        },
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::ActionQueued { side, item },
                );
            }
            CardKind::Spell { .. } => {
                let ActionTarget::Piece { piece_id } = target else {
                    return Err(MatchError::InvalidTarget);
                };
                let target = self
                    .piece_view(&piece_id)
                    .ok_or(MatchError::PieceNotFound)?;
                let caster_position = self.player_ref(side).hero.position;
                let planned_spell_play = card_interactions::plan_spell_play(
                    &card,
                    ActionTarget::Piece { piece_id },
                    side,
                    caster_position,
                    &self.player_ref(side).hero.id,
                    &target,
                )?;
                self.spend_card_resources(side, &card_id, &card)?;
                self.log.insert(
                    0,
                    format!("{} put {} on the stack.", side.label(), card.name),
                );
                let item = self.push_stack_item(
                    side,
                    planned_spell_play.priority,
                    StackAction::CastSpell {
                        card: CardSummary::from(&card),
                        target_id: planned_spell_play.target_id.clone(),
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::CardPlayed {
                        side,
                        card: CardSummary::from(&card),
                        target: ActionTarget::Piece {
                            piece_id: planned_spell_play.target_id,
                        },
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::ActionQueued { side, item },
                );
            }
            CardKind::Building { .. } | CardKind::ManaSource => {
                let coord = self.validate_building_target(side, target)?;

                self.spend_card_resources(side, &card_id, &card)?;
                self.log.insert(
                    0,
                    format!("{} put {} on the stack.", side.label(), card.name),
                );
                let item = self.push_stack_item(
                    side,
                    0,
                    StackAction::BuildBuilding {
                        card: CardSummary::from(&card),
                        coord,
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::CardPlayed {
                        side,
                        card: CardSummary::from(&card),
                        target: ActionTarget::Hex { coord },
                    },
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::ActionQueued { side, item },
                );
            }
        }

        if self.mode == MatchMode::Solo && side == Side::Player && stack_was_empty {
            self.resolve_all_stack(frames, action_index);
        }
        self.check_winner(frames, action_index);
        self.truncate_log();
        Ok(())
    }

    fn validate_building_target(
        &self,
        side: Side,
        target: ActionTarget,
    ) -> Result<HexCoord, MatchError> {
        let ActionTarget::Hex { coord } = target else {
            return Err(MatchError::InvalidTarget);
        };
        if !self.board.is_valid(coord) {
            return Err(MatchError::InvalidHex);
        }
        if self.is_occupied(coord) {
            return Err(MatchError::OccupiedHex);
        }
        if self.has_building(coord) {
            return Err(MatchError::InvalidTarget);
        }
        if self.player_ref(side).hero.position.distance(coord) != 1 {
            return Err(MatchError::InvalidTarget);
        }

        Ok(coord)
    }

    fn spend_card_resources(
        &mut self,
        side: Side,
        card_id: &str,
        card: &Card,
    ) -> Result<(), MatchError> {
        let player = self.player_mut(side);
        let hand_index = player
            .hand
            .iter()
            .position(|candidate| candidate.id == card_id)
            .ok_or(MatchError::CardNotFound)?;

        if player.mana < card.cost {
            return Err(MatchError::NotEnoughMana);
        }

        player.mana -= card.cost;
        let card = player.hand.remove(hand_index);
        player.discard.push(card);
        player.discard_count = player.discard.len();
        Ok(())
    }

    fn apply_resolved_spell(
        &mut self,
        side: Side,
        resolved_spell: card_interactions::ResolvedSpell,
        card_name: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        match resolved_spell.effect {
            card_interactions::ResolvedSpellEffect::Heal { piece_id, amount } => {
                self.heal_piece(&piece_id, amount);
                self.log.insert(
                    0,
                    format!("{} cast {} to heal {}.", side.label(), card_name, piece_id),
                );
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::PieceHealed {
                        side,
                        piece_id,
                        amount,
                    },
                );
            }
            card_interactions::ResolvedSpellEffect::Buff {
                piece_id,
                attack,
                armor,
                max_ap,
                targets,
            } => {
                if self.apply_stat_bonus_to_piece(
                    &piece_id,
                    StatBonus {
                        attack,
                        armor,
                        max_ap,
                        targets,
                    },
                    frames,
                    action_index,
                ) {
                    self.log.insert(
                        0,
                        format!("{} cast {} on {}.", side.label(), card_name, piece_id),
                    );
                    self.record_replay_frame(
                        frames,
                        action_index,
                        ReplayEvent::PieceBuffed {
                            side,
                            piece_id,
                            attack_delta: attack,
                            armor_delta: armor,
                        },
                    );
                }
            }
            card_interactions::ResolvedSpellEffect::Damage { piece_ids, amount } => {
                self.damage_pieces(side, piece_ids, amount, frames, action_index);
                self.log_spell_resolution(side, card_name, resolved_spell.log);
            }
            card_interactions::ResolvedSpellEffect::Draw { amount } => {
                let drawn = self.draw_cards_for_side(side, amount);
                for card in drawn {
                    self.record_replay_frame(
                        frames,
                        action_index,
                        ReplayEvent::CardDrawn {
                            side,
                            card: Some(CardSummary::from(&card)),
                            hidden: side.team() == Team::Opponent,
                        },
                    );
                }
                self.log_spell_resolution(side, card_name, resolved_spell.log);
            }
        }
    }

    fn log_spell_resolution(
        &mut self,
        side: Side,
        card_name: &str,
        log: card_interactions::SpellLog,
    ) {
        let message = match log {
            card_interactions::SpellLog::Heal { piece_id } => {
                format!("{} cast {} to heal {}.", side.label(), card_name, piece_id)
            }
            card_interactions::SpellLog::Buff { piece_id } => {
                format!("{} cast {} on {}.", side.label(), card_name, piece_id)
            }
            card_interactions::SpellLog::Damage { piece_id } => {
                format!("{} cast {} at {}.", side.label(), card_name, piece_id)
            }
            card_interactions::SpellLog::Draw { amount } => {
                format!("{} cast {} to draw {}.", side.label(), card_name, amount)
            }
            card_interactions::SpellLog::AreaDamage { piece_id } => {
                format!("{} cast {} around {}.", side.label(), card_name, piece_id)
            }
            card_interactions::SpellLog::LineDamage => {
                format!("{} cast {} down a line.", side.label(), card_name)
            }
        };

        self.log.insert(0, message);
    }

    fn push_stack_item(&mut self, side: Side, priority: u8, action: StackAction) -> StackItem {
        let item = StackItem {
            id: format!("stack-{}", self.next_stack_item_id),
            side,
            priority,
            action,
        };
        self.next_stack_item_id += 1;
        self.action_stack.push(item.clone());
        self.priority_passes.clear();
        self.priority_side =
            self.next_priority_side_for_team(side.opposing_team(), Some(side), &[]);
        item
    }

    fn pass_priority_for_side(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        if self.action_stack.is_empty() {
            return Err(MatchError::EmptyStack);
        }
        if self.priority_side != Some(side) {
            return Err(MatchError::NotPrioritySide);
        }

        if self.format() == "twoVTwo" {
            self.priority_passes.push(side);
            if let Some(next_priority_side) =
                self.next_priority_side_for_team(side.team(), Some(side), &self.priority_passes)
            {
                self.priority_side = Some(next_priority_side);
                return Ok(());
            }
        }

        self.resolve_top_stack_item(frames, action_index);
        self.check_winner(frames, action_index);
        self.auto_start_card_play_if_attacks_exhausted(frames, action_index);
        self.truncate_log();
        Ok(())
    }

    fn resolve_all_stack(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        while !self.action_stack.is_empty() && self.phase != Phase::MatchOver {
            self.resolve_top_stack_item(frames, action_index);
            self.check_winner(frames, action_index);
            self.auto_start_card_play_if_attacks_exhausted(frames, action_index);
        }
    }

    fn resolve_top_stack_item(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let Some(item) = self.action_stack.pop() else {
            self.priority_side = None;
            return;
        };

        match item.action {
            StackAction::PlayUnit { card, coord } => {
                self.resolve_unit_card(item.side, card, coord, frames, action_index);
            }
            StackAction::CastSpell { card, target_id } => {
                self.resolve_spell_card(item.side, card, &target_id, frames, action_index);
            }
            StackAction::MovePiece { piece_id, from, to } => {
                self.resolve_move(item.side, &piece_id, from, to, frames, action_index);
            }
            StackAction::Attack {
                attacker_id,
                target_id,
            } => {
                self.resolve_attack(item.side, &attacker_id, &target_id, frames, action_index);
            }
            StackAction::EquipItem { card, carrier_id } => {
                self.resolve_item_card(item.side, card, &carrier_id, frames, action_index);
            }
            StackAction::BuildManaSource { card, coord }
            | StackAction::BuildBuilding { card, coord } => {
                self.resolve_building_card(item.side, card, coord, frames, action_index);
            }
            StackAction::ActivateItem {
                carrier_id,
                item_id,
                target,
            } => {
                self.resolve_item_activation(
                    item.side,
                    &carrier_id,
                    &item_id,
                    target,
                    frames,
                    action_index,
                );
            }
            StackAction::ActivateBuilding {
                building_id,
                occupant_id,
            } => {
                self.resolve_building_activation(
                    item.side,
                    &building_id,
                    &occupant_id,
                    frames,
                    action_index,
                );
            }
        }

        self.priority_passes.clear();
        self.priority_side = self.action_stack.last().and_then(|item| {
            self.next_priority_side_for_team(item.side.opposing_team(), Some(item.side), &[])
        });
    }

    fn resolve_building_card(
        &mut self,
        side: Side,
        card: CardSummary,
        coord: HexCoord,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        if !self.board.is_valid(coord) || self.is_occupied(coord) || self.has_building(coord) {
            self.log.insert(
                0,
                format!("{} resolved with no legal building hex.", card.name),
            );
            self.truncate_log();
            return;
        }

        let building_id = self.next_building_id(side);
        let effect = match card.kind {
            CardKind::Building { effect } => effect,
            CardKind::ManaSource => BuildingEffect::TurnStartMana { amount: 1 },
            _ => return,
        };
        self.board.buildings.push(Building {
            id: building_id.clone(),
            template_id: card.template_id,
            name: card.name.clone(),
            position: coord,
            effect: effect.clone(),
            activated_this_turn: false,
        });
        self.log.insert(
            0,
            format!(
                "{} built {} at q {}, r {}.",
                side.label(),
                card.name,
                coord.q,
                coord.r
            ),
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::BuildingBuilt {
                side,
                building_id: building_id.clone(),
                name: card.name.clone(),
                coord,
            },
        );
        if matches!(effect, BuildingEffect::TurnStartMana { .. }) {
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::ManaSourceBuilt { side, coord },
            );
        }
    }

    fn resolve_unit_card(
        &mut self,
        side: Side,
        card: CardSummary,
        coord: HexCoord,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let hero_position = self.player_ref(side).hero.position;
        if !card_interactions::can_resolve_unit_play(coord, hero_position, &self.board, |coord| {
            self.is_occupied(coord)
        }) {
            self.log
                .insert(0, format!("{} could not resolve.", card.name));
            return;
        }

        let progression = self.player_ref(side).progression.clone();
        let is_first_summoned_unit = self.player_ref(side).summoned_unit_count == 0;
        let Some(unit) = card_interactions::summon_unit_from_card(
            &card,
            side,
            || self.next_unit_id(side),
            coord,
            &progression,
            is_first_summoned_unit,
        ) else {
            return;
        };
        let unit_id = unit.id.clone();
        let unit_name = unit.name.clone();
        let unit_position = unit.position;
        self.board.units.push(unit);
        self.player_mut(side).summoned_unit_count += 1;
        self.log
            .insert(0, format!("{} summoned {}.", side.label(), card.name));
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::UnitSummoned {
                side,
                unit_id: unit_id.clone(),
                name: unit_name,
                position: unit_position,
            },
        );
        self.pick_up_dropped_items_at(side, &unit_id, unit_position, frames, action_index);
    }

    fn resolve_spell_card(
        &mut self,
        side: Side,
        card: CardSummary,
        target_id: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let CardKind::Spell { effect, .. } = &card.kind else {
            return;
        };
        let Some(target) = self.piece_view(target_id) else {
            self.log
                .insert(0, format!("{} had no legal target.", card.name));
            return;
        };
        let caster_position = self.player_ref(side).hero.position;
        let caster_hero_id = self.player_ref(side).hero.id.clone();
        if card_interactions::validate_spell_target(
            side,
            effect,
            caster_position,
            &caster_hero_id,
            &target,
        )
        .is_err()
        {
            self.log
                .insert(0, format!("{} had no legal target.", card.name));
            return;
        }

        let enemy_pieces = self.pieces_for_opposing_team(side);
        let progression = self.player_ref(side).progression.clone();
        let Ok(resolved_spell) = card_interactions::resolve_spell(
            &card,
            side,
            caster_position,
            &caster_hero_id,
            &target,
            &enemy_pieces,
            &progression,
        ) else {
            self.log
                .insert(0, format!("{} had no legal target.", card.name));
            return;
        };

        self.apply_resolved_spell(side, resolved_spell, &card.name, frames, action_index);
    }

    fn resolve_item_card(
        &mut self,
        side: Side,
        card: CardSummary,
        carrier_id: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let item_id = self.next_item_id(side);
        let Some(carrier) = self.piece_view(carrier_id) else {
            self.log
                .insert(0, format!("{} had no legal carrier.", card.name));
            return;
        };
        if carrier.side.team() != side.team() {
            self.log
                .insert(0, format!("{} had no legal carrier.", card.name));
            return;
        }

        let card_name = card.name.clone();
        let Ok(item) = card_interactions::equip_item_from_card(card, item_id.clone()) else {
            return;
        };
        if !self.add_carried_item_to_carrier(carrier_id, item) {
            return;
        }
        self.log.insert(
            0,
            format!(
                "{} equipped {} to {}.",
                side.label(),
                card_name,
                self.carrier_name(carrier_id)
                    .unwrap_or_else(|| carrier_id.to_string())
            ),
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ItemEquipped {
                side,
                carrier_id: carrier_id.to_string(),
                item_id,
                name: card_name,
            },
        );
    }

    fn resolve_item_activation(
        &mut self,
        side: Side,
        carrier_id: &str,
        item_id: &str,
        target: Option<ActionTarget>,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let Some(carrier) = self.piece_view(carrier_id) else {
            self.log.insert(
                0,
                format!("Item activation by {} had no legal carrier.", carrier_id),
            );
            return;
        };
        if carrier.side != side {
            self.log.insert(
                0,
                format!("Item activation by {} had no legal carrier.", carrier_id),
            );
            return;
        }
        let Some(item) = self.carried_item(carrier_id, item_id) else {
            self.log.insert(
                0,
                format!("Item activation by {} had no legal item.", carrier_id),
            );
            return;
        };
        let target_piece = target.as_ref().and_then(|target| match target {
            ActionTarget::Piece { piece_id } => self.piece_view(piece_id),
            ActionTarget::Hex { .. } => None,
        });

        let Ok(resolved_item_activation) = card_interactions::resolve_item_activation(
            &carrier,
            &item,
            item_id,
            target_piece.as_ref(),
        ) else {
            self.log.insert(
                0,
                format!("Item activation by {} had no legal item.", carrier_id),
            );
            return;
        };

        match resolved_item_activation.effect {
            card_interactions::ResolvedItemActiveEffect::HealCarrier { carrier_id, amount } => {
                self.heal_piece(&carrier_id, amount);
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::PieceHealed {
                        side,
                        piece_id: carrier_id,
                        amount,
                    },
                );
            }
            card_interactions::ResolvedItemActiveEffect::DamageTarget { target_id, amount } => {
                self.damage_pieces(side, vec![target_id], amount, frames, action_index);
            }
            card_interactions::ResolvedItemActiveEffect::Draw { amount } => {
                let drawn = self.draw_cards_for_side(side, amount);
                for card in drawn {
                    self.record_replay_frame(
                        frames,
                        action_index,
                        ReplayEvent::CardDrawn {
                            side,
                            card: Some(CardSummary::from(&card)),
                            hidden: false,
                        },
                    );
                }
            }
            card_interactions::ResolvedItemActiveEffect::StatMarker {
                carrier_id,
                mut marker,
            } => {
                marker.id = self.next_stat_marker_id(&carrier_id, &marker.source_item_id);
                let attack = marker.attack;
                let armor = marker.armor;
                if self.add_stat_marker_to_carrier(&carrier_id, marker) {
                    self.record_replay_frame(
                        frames,
                        action_index,
                        ReplayEvent::PieceBuffed {
                            side,
                            piece_id: carrier_id,
                            attack_delta: attack,
                            armor_delta: armor,
                        },
                    );
                }
            }
        }

        self.log.insert(
            0,
            format!(
                "{} activated {}.",
                side.label(),
                resolved_item_activation.item_name
            ),
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ItemActivated {
                side,
                carrier_id: carrier_id.to_string(),
                item_id: item_id.to_string(),
                name: resolved_item_activation.item_name,
            },
        );
    }

    fn resolve_building_activation(
        &mut self,
        side: Side,
        building_id: &str,
        occupant_id: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let Some(building) = self
            .board
            .buildings
            .iter()
            .find(|building| building.id == building_id)
            .cloned()
        else {
            self.log.insert(
                0,
                format!("Building activation by {occupant_id} had no building."),
            );
            return;
        };
        let Some(occupant) = self.piece_view_at(building.position) else {
            self.log
                .insert(0, format!("{} had no occupant.", building.name));
            return;
        };
        if occupant.id != occupant_id || occupant.side != side {
            self.log
                .insert(0, format!("{} had no legal occupant.", building.name));
            return;
        }

        match building.effect {
            BuildingEffect::ActivatedDamageLine { range, amount } => {
                let targets = self.best_building_line_targets(side, building.position, range);
                self.damage_pieces(side, targets, amount, frames, action_index);
            }
            BuildingEffect::ActivatedHeal {
                range,
                amount,
                targets,
            } => {
                if let Some(piece_id) =
                    self.best_building_heal_target(side, building.position, range, targets)
                {
                    self.heal_piece(&piece_id, amount);
                    self.record_replay_frame(
                        frames,
                        action_index,
                        ReplayEvent::PieceHealed {
                            side,
                            piece_id,
                            amount,
                        },
                    );
                }
            }
            BuildingEffect::ActivatedStatBonus {
                targets,
                attack,
                armor,
                max_ap,
                ..
            } => {
                self.apply_stat_bonus_to_piece(
                    occupant_id,
                    StatBonus {
                        attack,
                        armor,
                        max_ap,
                        targets,
                    },
                    frames,
                    action_index,
                );
            }
            BuildingEffect::TurnStartMana { .. } | BuildingEffect::AuraStatBonus { .. } => {}
        }

        self.log
            .insert(0, format!("{} activated {}.", side.label(), building.name));
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::BuildingActivated {
                side,
                building_id: building.id,
                name: building.name,
                occupant_id: occupant_id.to_string(),
            },
        );
    }

    fn move_piece_for_side(
        &mut self,
        side: Side,
        piece_id: &str,
        to: HexCoord,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        let piece = self.piece_view(piece_id).ok_or(MatchError::PieceNotFound)?;
        if piece.side != side {
            return Err(MatchError::NotYourPiece);
        }
        if piece.ap_remaining == 0 {
            return Err(MatchError::NoActionPoints);
        }
        if !self.board.is_valid(to) {
            return Err(MatchError::InvalidHex);
        }
        if !piece.position.is_adjacent(to) {
            return Err(MatchError::NotAdjacent);
        }
        if self.is_occupied(to) {
            return Err(MatchError::OccupiedHex);
        }

        if let Some(hero_side) = self
            .participant_sides()
            .into_iter()
            .find(|candidate| self.player_ref(*candidate).hero.id == piece_id)
        {
            self.player_mut(hero_side).hero.ap_remaining -= 1;
        } else if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.ap_remaining -= 1;
        }

        self.log
            .insert(0, format!("{} put a move on the stack.", side.label()));
        let item = self.push_stack_item(
            side,
            0,
            StackAction::MovePiece {
                piece_id: piece_id.to_string(),
                from: piece.position,
                to,
            },
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ActionQueued { side, item },
        );
        if self.mode == MatchMode::Solo && side == Side::Player {
            self.resolve_all_stack(frames, action_index);
        }
        self.truncate_log();
        Ok(())
    }

    fn resolve_move(
        &mut self,
        side: Side,
        piece_id: &str,
        from: HexCoord,
        to: HexCoord,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let Some(piece) = self.piece_view(piece_id) else {
            self.log
                .insert(0, format!("Move by {} had no legal piece.", piece_id));
            return;
        };
        if piece.side != side
            || piece.position != from
            || !self.board.is_valid(to)
            || self.is_occupied(to)
        {
            self.log
                .insert(0, format!("Move by {} had no legal path.", piece_id));
            return;
        }

        if let Some(hero_side) = self
            .participant_sides()
            .into_iter()
            .find(|candidate| self.player_ref(*candidate).hero.id == piece_id)
        {
            self.player_mut(hero_side).hero.position = to;
        } else if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.position = to;
        }

        self.log
            .insert(0, format!("{} moved {}.", side.label(), piece_id));
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::PieceMoved {
                side,
                piece_id: piece_id.to_string(),
                from,
                to,
            },
        );
        self.pick_up_dropped_items_at(side, piece_id, to, frames, action_index);
    }

    fn attack_for_side(
        &mut self,
        side: Side,
        attacker_id: &str,
        target_id: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        let attacker = self
            .piece_view(attacker_id)
            .ok_or(MatchError::PieceNotFound)?;
        let target = self
            .piece_view(target_id)
            .ok_or(MatchError::PieceNotFound)?;

        if attacker.side != side {
            return Err(MatchError::NotYourPiece);
        }
        if target.side.team() == side.team() {
            return Err(MatchError::InvalidTarget);
        }
        if attacker.ap_remaining == 0 {
            return Err(MatchError::NoActionPoints);
        }
        if attacker.has_attacked {
            return Err(MatchError::AlreadyAttacked);
        }
        if !piece_can_attack(&attacker, &target) {
            return Err(MatchError::NotAdjacent);
        }

        self.mark_attacker_spent(attacker_id);
        self.log
            .insert(0, format!("{} put an attack on the stack.", side.label()));
        let item = self.push_stack_item(
            side,
            0,
            StackAction::Attack {
                attacker_id: attacker_id.to_string(),
                target_id: target_id.to_string(),
            },
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ActionQueued { side, item },
        );
        if self.mode == MatchMode::Solo && side == Side::Player {
            self.resolve_all_stack(frames, action_index);
        }
        self.check_winner(frames, action_index);
        self.truncate_log();
        Ok(())
    }

    fn activate_item_for_side(
        &mut self,
        side: Side,
        carrier_id: &str,
        item_id: &str,
        target: Option<ActionTarget>,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        if self.player_ref(side).knocked_out {
            return Err(MatchError::NotActiveSide);
        }
        let carrier = self
            .piece_view(carrier_id)
            .ok_or(MatchError::PieceNotFound)?;
        if carrier.side != side {
            return Err(MatchError::NotYourPiece);
        }
        if carrier.ap_remaining == 0 {
            return Err(MatchError::NoActionPoints);
        }
        let item = self
            .carried_item(carrier_id, item_id)
            .ok_or(MatchError::ItemNotFound)?;
        let active = item.active.as_ref().ok_or(MatchError::InvalidTarget)?;
        let priority = card_interactions::item_active_priority(active);
        if item.active_used_this_turn {
            return Err(MatchError::ItemExhausted);
        }
        if let Some(pending) = self.action_stack.last() {
            if self.priority_side != Some(side) {
                return Err(MatchError::NotPrioritySide);
            }
            if priority <= pending.priority {
                return Err(MatchError::PriorityTooLow);
            }
        } else if side != self.active_side {
            return Err(MatchError::NotActiveSide);
        } else if self.phase == Phase::MatchOver {
            return Err(MatchError::MatchOver);
        }

        let target_piece = target.as_ref().and_then(|target| match target {
            ActionTarget::Piece { piece_id } => self.piece_view(piece_id),
            ActionTarget::Hex { .. } => None,
        });
        card_interactions::resolve_item_activation(
            &carrier,
            &item,
            item_id,
            target_piece.as_ref(),
        )?;

        if !self.mark_carried_item_used(carrier_id, item_id) {
            return Err(MatchError::ItemNotFound);
        }

        self.spend_piece_ap(carrier_id);
        self.log.insert(
            0,
            format!("{} put {}'s item on the stack.", side.label(), carrier.id),
        );
        let stack_item = self.push_stack_item(
            side,
            priority,
            StackAction::ActivateItem {
                carrier_id: carrier_id.to_string(),
                item_id: item_id.to_string(),
                target,
            },
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ActionQueued {
                side,
                item: stack_item,
            },
        );
        if self.mode == MatchMode::Solo && side == Side::Player {
            self.resolve_all_stack(frames, action_index);
        }
        self.truncate_log();
        Ok(())
    }

    fn activate_building_for_side(
        &mut self,
        side: Side,
        building_id: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        let building_index = self
            .board
            .buildings
            .iter()
            .position(|building| building.id == building_id)
            .ok_or(MatchError::BuildingNotFound)?;
        if !building_effect_is_activated(&self.board.buildings[building_index].effect) {
            return Err(MatchError::InvalidTarget);
        }
        if self.board.buildings[building_index].activated_this_turn {
            return Err(MatchError::BuildingExhausted);
        }

        let occupant = self
            .piece_view_at(self.board.buildings[building_index].position)
            .ok_or(MatchError::InvalidTarget)?;
        if occupant.side != side {
            return Err(MatchError::NotYourPiece);
        }
        if occupant.ap_remaining == 0 {
            return Err(MatchError::NoActionPoints);
        }

        self.spend_piece_ap(&occupant.id);
        self.board.buildings[building_index].activated_this_turn = true;
        self.log.insert(
            0,
            format!(
                "{} put {} on the stack.",
                side.label(),
                self.board.buildings[building_index].name
            ),
        );
        let stack_item = self.push_stack_item(
            side,
            0,
            StackAction::ActivateBuilding {
                building_id: building_id.to_string(),
                occupant_id: occupant.id,
            },
        );
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ActionQueued {
                side,
                item: stack_item,
            },
        );
        if self.mode == MatchMode::Solo && side == Side::Player {
            self.resolve_all_stack(frames, action_index);
        }
        self.truncate_log();
        Ok(())
    }

    fn spend_piece_ap(&mut self, piece_id: &str) {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                let hero = &mut self.player_mut(side).hero;
                hero.ap_remaining = hero.ap_remaining.saturating_sub(1);
                return;
            }
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.ap_remaining = unit.ap_remaining.saturating_sub(1);
        }
    }

    fn resolve_attack(
        &mut self,
        side: Side,
        attacker_id: &str,
        target_id: &str,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let Some(attacker) = self.piece_view(attacker_id) else {
            self.log.insert(
                0,
                format!("Attack by {} had no legal attacker.", attacker_id),
            );
            return;
        };
        let Some(target) = self.piece_view(target_id) else {
            self.log
                .insert(0, format!("Attack by {} had no legal target.", attacker_id));
            return;
        };

        if attacker.side != side
            || target.side.team() == side.team()
            || !piece_can_attack(&attacker, &target)
        {
            self.log
                .insert(0, format!("Attack by {} had no legal target.", attacker_id));
            return;
        }

        let counter_damage_to_attacker = if piece_can_attack(&target, &attacker) {
            target.attack
        } else {
            0
        };
        self.damage_piece(target_id, attacker.attack);
        if counter_damage_to_attacker > 0 {
            self.damage_piece(attacker_id, counter_damage_to_attacker);
        }
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::PieceAttacked {
                side,
                attacker_id: attacker_id.to_string(),
                target_id: target_id.to_string(),
                damage_to_target: attacker.attack,
                counter_damage_to_attacker,
            },
        );
        let destroyed = self.remove_dead_units(frames, action_index);
        self.advance_surviving_melee_attacker_after_kill(
            side,
            attacker_id,
            &attacker,
            &target,
            &destroyed,
            frames,
            action_index,
        );
        self.apply_barbarian_combat_mana(
            &attacker,
            &target,
            target_id,
            attacker_id,
            counter_damage_to_attacker,
            &destroyed,
            frames,
            action_index,
        );
        self.log.insert(
            0,
            format!(
                "{} attacked {} with {}.",
                side.label(),
                target_id,
                attacker_id
            ),
        );
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "melee advance depends on the resolved attack context"
    )]
    fn advance_surviving_melee_attacker_after_kill(
        &mut self,
        side: Side,
        attacker_id: &str,
        attacker: &PieceView,
        target: &PieceView,
        destroyed: &[DestroyedUnit],
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        if target.is_hero || attacker.position.distance(target.position) != 1 {
            return;
        }
        if !destroyed
            .iter()
            .any(|destroyed_unit| destroyed_unit.unit_id == target.id)
        {
            return;
        }
        let Some(surviving_attacker) = self.piece_view(attacker_id) else {
            return;
        };
        if self.is_occupied(target.position) {
            return;
        }

        if let Some(hero_side) = self
            .participant_sides()
            .into_iter()
            .find(|candidate| self.player_ref(*candidate).hero.id == attacker_id)
        {
            self.player_mut(hero_side).hero.position = target.position;
        } else if let Some(unit) = self
            .board
            .units
            .iter_mut()
            .find(|unit| unit.id == attacker_id)
        {
            unit.position = target.position;
        }

        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::PieceMoved {
                side,
                piece_id: attacker_id.to_string(),
                from: surviving_attacker.position,
                to: target.position,
            },
        );
        self.pick_up_dropped_items_at(side, attacker_id, target.position, frames, action_index);
    }

    fn advance_solo_ai(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        let policy = SoloAiPolicy::default();
        self.advance_ai_for_side_with_policy(Side::Opponent, &policy, frames, action_index)
    }

    pub(crate) fn advance_ai_for_side_with_policy(
        &mut self,
        side: Side,
        policy: &SoloAiPolicy,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        self.advance_ai_for_side_with_policy_mode(side, policy, frames, action_index, false)
            .map(|_| ())
    }

    pub(crate) fn advance_ai_for_side_with_policy_strict(
        &mut self,
        side: Side,
        policy: &SoloAiPolicy,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<AiAdvanceOutcome, MatchError> {
        self.advance_ai_for_side_with_policy_mode(side, policy, frames, action_index, true)
    }

    fn advance_ai_for_side_with_policy_mode(
        &mut self,
        side: Side,
        policy: &SoloAiPolicy,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
        strict: bool,
    ) -> Result<AiAdvanceOutcome, MatchError> {
        if self.mode != MatchMode::Solo {
            return Err(MatchError::AiUnavailable);
        }

        if !self.action_stack.is_empty() {
            if self.priority_side != Some(side) {
                return Err(MatchError::NotPrioritySide);
            }
            self.pass_priority_for_side(side, frames, action_index)?;
            return Ok(AiAdvanceOutcome::PriorityPassed);
        }

        if self.active_side != side {
            return Err(MatchError::AiUnavailable);
        }

        let decision = match self.ai_phase_decision(side, policy, frames, action_index)? {
            Some(outcome) => return Ok(outcome),
            None => policy.decide(&self.solo_ai_view_for_side(side)),
        };
        let outcome = match self.apply_ai_decision_for_side(side, decision, frames, action_index) {
            AiDecisionApplication::Applied => {
                self.check_winner(frames, action_index);
                AiAdvanceOutcome::ActionApplied
            }
            AiDecisionApplication::FinishTurn => {
                self.finish_ai_turn(side, frames, action_index);
                AiAdvanceOutcome::FinishedTurn
            }
            AiDecisionApplication::Illegal(error) if strict => AiAdvanceOutcome::IllegalIntent {
                reason: error.to_string(),
            },
            AiDecisionApplication::Illegal(_) => {
                self.finish_ai_turn(side, frames, action_index);
                AiAdvanceOutcome::FinishedTurn
            }
        };

        self.truncate_log();
        Ok(outcome)
    }

    fn ai_phase_decision(
        &mut self,
        side: Side,
        policy: &SoloAiPolicy,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<Option<AiAdvanceOutcome>, MatchError> {
        match self.phase {
            Phase::Movement => {
                if self.side_has_legal_attack(side) {
                    self.start_attack_phase_for_side(side, frames, action_index)?;
                    return Ok(Some(AiAdvanceOutcome::ActionApplied));
                }
                let decision = policy.decide(&self.solo_ai_view_for_side(side));
                match decision {
                    SoloAiDecision::TakeAction(SoloAiActionIntent::MovePiece { .. })
                    | SoloAiDecision::TakeAction(SoloAiActionIntent::ActivateItem { .. }) => {
                        Ok(None)
                    }
                    SoloAiDecision::TakeAction(SoloAiActionIntent::Attack { .. }) => {
                        self.start_attack_phase_for_side(side, frames, action_index)?;
                        Ok(Some(AiAdvanceOutcome::ActionApplied))
                    }
                    _ => {
                        self.start_card_play_for_side(side, frames, action_index)?;
                        Ok(Some(AiAdvanceOutcome::ActionApplied))
                    }
                }
            }
            Phase::Attack => {
                let decision = policy.decide(&self.solo_ai_view_for_side(side));
                match decision {
                    SoloAiDecision::TakeAction(SoloAiActionIntent::Attack { .. })
                    | SoloAiDecision::TakeAction(SoloAiActionIntent::ActivateItem { .. }) => {
                        Ok(None)
                    }
                    _ => {
                        self.start_card_play_for_side(side, frames, action_index)?;
                        Ok(Some(AiAdvanceOutcome::ActionApplied))
                    }
                }
            }
            Phase::CardPlay => {
                let decision = policy.decide(&self.solo_ai_view_for_side(side));
                match decision {
                    SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard { .. })
                    | SoloAiDecision::TakeAction(SoloAiActionIntent::ActivateItem { .. }) => {
                        Ok(None)
                    }
                    _ => {
                        self.finish_ai_turn(side, frames, action_index);
                        Ok(Some(AiAdvanceOutcome::FinishedTurn))
                    }
                }
            }
            Phase::MatchOver => Err(MatchError::MatchOver),
        }
    }

    fn apply_ai_decision_for_side(
        &mut self,
        side: Side,
        decision: SoloAiDecision,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> AiDecisionApplication {
        match decision {
            SoloAiDecision::TakeAction(SoloAiActionIntent::Attack {
                attacker_id,
                target_id,
            }) => {
                match self.attack_for_side(side, &attacker_id, &target_id, frames, action_index) {
                    Ok(()) => AiDecisionApplication::Applied,
                    Err(error) => AiDecisionApplication::Illegal(error),
                }
            }
            SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard { card_id, target }) => self
                .play_card_for_side(side, card_id, target, frames, action_index)
                .map(|()| AiDecisionApplication::Applied)
                .unwrap_or_else(AiDecisionApplication::Illegal),
            SoloAiDecision::TakeAction(SoloAiActionIntent::MovePiece { piece_id, to }) => self
                .move_piece_for_side(side, &piece_id, to, frames, action_index)
                .map(|()| AiDecisionApplication::Applied)
                .unwrap_or_else(AiDecisionApplication::Illegal),
            SoloAiDecision::TakeAction(SoloAiActionIntent::ActivateItem {
                carrier_id,
                item_id,
                target,
            }) => self
                .activate_item_for_side(side, &carrier_id, &item_id, target, frames, action_index)
                .map(|()| AiDecisionApplication::Applied)
                .unwrap_or_else(AiDecisionApplication::Illegal),
            SoloAiDecision::FinishTurn => AiDecisionApplication::FinishTurn,
        }
    }

    fn finish_ai_turn(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let message = if side == Side::Opponent {
            "Opponent ended their turn.".to_string()
        } else {
            "Player ended their turn.".to_string()
        };
        self.log.insert(0, message);
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::TurnEnded {
                side,
                round: self.round,
            },
        );

        if self.phase != Phase::MatchOver {
            if side == Side::Opponent {
                self.round += 1;
            }
            self.start_turn(side.opponent(), frames, action_index);
            if side == Side::Opponent {
                self.log.insert(0, format!("Round {} begins.", self.round));
                self.truncate_log();
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::RoundStarted { round: self.round },
                );
            }
        }
    }

    fn start_turn(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        self.active_side = side;
        self.phase = Phase::Movement;
        self.refresh_mana_from_sources(side);
        let should_draw = self.player_ref(side).has_started_first_turn;
        {
            let player = self.player_mut(side);
            player.hero.ap_remaining = player.hero.max_ap;
            player.hero.has_attacked = false;
            for item in &mut player.hero.items {
                item.active_used_this_turn = false;
            }
            if !should_draw {
                player.has_started_first_turn = true;
            }
        }
        for unit in self.board.units.iter_mut().filter(|unit| unit.side == side) {
            unit.ap_remaining = unit.max_ap;
            unit.has_attacked = false;
            for item in &mut unit.items {
                item.active_used_this_turn = false;
            }
        }
        for building in &mut self.board.buildings {
            building.activated_this_turn = false;
        }
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::TurnStarted {
                side,
                round: self.round,
            },
        );
        self.refresh_unit_armor_for_turn(side, frames, action_index);
        if should_draw {
            if let Some(card) = self.player_mut(side).draw() {
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::CardDrawn {
                        side,
                        card: Some(CardSummary::from(&card)),
                        hidden: side.team() == Team::Opponent,
                    },
                );
            }
        }
    }

    fn player_ref(&self, side: Side) -> &PlayerState {
        match side {
            Side::Player => &self.player,
            Side::Opponent => &self.opponent,
            Side::PlayerTwo => self
                .player_two
                .as_ref()
                .expect("player two should exist in a 2v2 match"),
            Side::OpponentTwo => self
                .opponent_two
                .as_ref()
                .expect("opponent two should exist in a 2v2 match"),
        }
    }

    fn player_mut(&mut self, side: Side) -> &mut PlayerState {
        match side {
            Side::Player => &mut self.player,
            Side::Opponent => &mut self.opponent,
            Side::PlayerTwo => self
                .player_two
                .as_mut()
                .expect("player two should exist in a 2v2 match"),
            Side::OpponentTwo => self
                .opponent_two
                .as_mut()
                .expect("opponent two should exist in a 2v2 match"),
        }
    }

    fn participants(&self) -> Vec<&PlayerState> {
        let mut participants = vec![&self.player, &self.opponent];
        if let Some(player_two) = &self.player_two {
            participants.push(player_two);
        }
        if let Some(opponent_two) = &self.opponent_two {
            participants.push(opponent_two);
        }
        participants
    }

    fn participant_sides(&self) -> Vec<Side> {
        self.participants()
            .into_iter()
            .map(|participant| participant.side)
            .collect()
    }

    fn active_participant_sides(&self) -> Vec<Side> {
        self.participants()
            .into_iter()
            .filter(|participant| !participant.knocked_out)
            .map(|participant| participant.side)
            .collect()
    }

    fn next_turn_side_after(&self, side: Side) -> Side {
        let order = self.participant_sides();
        let Some(index) = order.iter().position(|candidate| *candidate == side) else {
            return Side::Player;
        };
        for offset in 1..=order.len() {
            let candidate = order[(index + offset) % order.len()];
            if !self.player_ref(candidate).knocked_out {
                return candidate;
            }
        }
        side
    }

    fn next_priority_side_for_team(
        &self,
        team: Team,
        after: Option<Side>,
        passed: &[Side],
    ) -> Option<Side> {
        let order = self.participant_sides();
        if order.is_empty() {
            return None;
        }
        let start_index = after
            .and_then(|side| order.iter().position(|candidate| *candidate == side))
            .map(|index| index + 1)
            .unwrap_or(0);
        for offset in 0..order.len() {
            let candidate = order[(start_index + offset) % order.len()];
            let participant = self.player_ref(candidate);
            if participant.knocked_out || candidate.team() != team || passed.contains(&candidate) {
                continue;
            }
            return Some(candidate);
        }
        None
    }

    pub(crate) fn player_mut_for_ai_lab(&mut self, side: Side) -> &mut PlayerState {
        self.player_mut(side)
    }

    pub(crate) fn participant_for_public(&self, side: Side) -> Option<&PlayerState> {
        match side {
            Side::Player => Some(&self.player),
            Side::Opponent => Some(&self.opponent),
            Side::PlayerTwo => self.player_two.as_ref(),
            Side::OpponentTwo => self.opponent_two.as_ref(),
        }
    }

    fn solo_ai_view_for_side(&self, side: Side) -> SoloAiView {
        let opposing_side = side.opponent();
        let opponent_hero = PieceView::from(&self.player_ref(side).hero);
        let player_hero = PieceView::from(&self.player_ref(opposing_side).hero);
        let opponent_units: Vec<_> = self
            .board
            .units
            .iter()
            .filter(|unit| unit.side == side)
            .map(PieceView::from)
            .collect();
        let player_units: Vec<_> = self
            .board
            .units
            .iter()
            .filter(|unit| unit.side == opposing_side)
            .map(PieceView::from)
            .collect();
        let mut opponent_pieces = vec![opponent_hero.clone()];
        opponent_pieces.extend(opponent_units.clone());
        let mut player_pieces = vec![player_hero.clone()];
        player_pieces.extend(player_units.clone());
        let occupied_hexes = opponent_pieces
            .iter()
            .chain(player_pieces.iter())
            .map(|piece| piece.position)
            .collect();
        let valid_hexes = self.board.tiles.iter().map(|tile| tile.coord).collect();
        let mana_sources = self
            .board
            .buildings
            .iter()
            .filter(|building| matches!(building.effect, BuildingEffect::TurnStartMana { .. }))
            .map(|building| building.position)
            .collect();
        let mut damaged_piece_ids = HashSet::new();
        for piece in opponent_pieces.iter().chain(player_pieces.iter()) {
            if self.piece_is_damaged(&piece.id) {
                damaged_piece_ids.insert(piece.id.clone());
            }
        }
        let mut opponent_carried_items = Vec::new();
        for participant in self.participants() {
            if participant.side == side {
                let carrier = PieceView::from(&participant.hero);
                for item in &participant.hero.items {
                    if let Some(active) = &item.active {
                        opponent_carried_items.push(SoloAiCarriedItem {
                            carrier: carrier.clone(),
                            item_id: item.id.clone(),
                            active: active.clone(),
                            active_used_this_turn: item.active_used_this_turn,
                        });
                    }
                }
            }
        }
        for unit in self.board.units.iter().filter(|unit| unit.side == side) {
            let carrier = PieceView::from(unit);
            for item in &unit.items {
                if let Some(active) = &item.active {
                    opponent_carried_items.push(SoloAiCarriedItem {
                        carrier: carrier.clone(),
                        item_id: item.id.clone(),
                        active: active.clone(),
                        active_used_this_turn: item.active_used_this_turn,
                    });
                }
            }
        }

        SoloAiView {
            controlled_side: side,
            opponent_hero,
            player_hero,
            opponent_pieces,
            player_pieces,
            opponent_units,
            player_units,
            opponent_carried_items,
            opponent_hand: self.player_ref(side).hand.clone(),
            opponent_mana: self.player_ref(side).mana,
            opponent_deck_count: self.player_ref(side).deck_count,
            opponent_discard_count: self.player_ref(side).discard_count,
            valid_hexes,
            occupied_hexes,
            mana_sources,
            damaged_piece_ids,
        }
    }

    fn pieces_for_side(&self, side: Side) -> Vec<PieceView> {
        let participant = self.player_ref(side);
        let mut pieces = if participant.knocked_out {
            Vec::new()
        } else {
            vec![self.apply_aura_to_piece_view(PieceView::from(&participant.hero))]
        };
        pieces.extend(
            self.board
                .units
                .iter()
                .filter(|unit| unit.side == side)
                .map(PieceView::from)
                .map(|piece| self.apply_aura_to_piece_view(piece)),
        );
        pieces
    }

    fn pieces_for_team(&self, team: Team) -> Vec<PieceView> {
        self.active_participant_sides()
            .into_iter()
            .filter(|side| side.team() == team)
            .flat_map(|side| self.pieces_for_side(side))
            .collect()
    }

    fn pieces_for_opposing_team(&self, side: Side) -> Vec<PieceView> {
        self.pieces_for_team(side.opposing_team())
    }

    fn piece_view_at(&self, coord: HexCoord) -> Option<PieceView> {
        for participant in self.participants() {
            if !participant.knocked_out && participant.hero.position == coord {
                return Some(self.apply_aura_to_piece_view(PieceView::from(&participant.hero)));
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.position == coord)
            .map(PieceView::from)
            .map(|piece| self.apply_aura_to_piece_view(piece))
    }

    fn piece_view(&self, piece_id: &str) -> Option<PieceView> {
        for participant in self.participants() {
            if !participant.knocked_out && participant.hero.id == piece_id {
                return Some(self.apply_aura_to_piece_view(PieceView::from(&participant.hero)));
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == piece_id)
            .map(PieceView::from)
            .map(|piece| self.apply_aura_to_piece_view(piece))
    }

    fn apply_aura_to_piece_view(&self, mut piece: PieceView) -> PieceView {
        let bonus = self.aura_stat_bonus_for(piece.side, piece.position, piece.is_hero);
        piece.attack += bonus.attack;
        piece
    }

    fn aura_stat_bonus_for(&self, side: Side, position: HexCoord, is_hero: bool) -> StatBonus {
        let mut bonus = StatBonus {
            attack: 0,
            armor: 0,
            max_ap: 0,
            targets: BuffTargetPolicy::UnitsAndHeroes,
        };
        for building in &self.board.buildings {
            let BuildingEffect::AuraStatBonus {
                range,
                targets,
                attack,
                armor,
                max_ap,
                ..
            } = building.effect
            else {
                continue;
            };
            if building.position.distance(position) > i32::from(range) {
                continue;
            }
            if self.building_occupant_side(building.position) != Some(side) {
                continue;
            }
            if !card_interactions::target_policy_allows(targets, is_hero) {
                continue;
            }
            bonus.attack += attack;
            bonus.armor += armor;
            bonus.max_ap += max_ap;
        }
        bonus
    }

    fn building_occupant_side(&self, position: HexCoord) -> Option<Side> {
        for participant in self.participants() {
            if !participant.knocked_out && participant.hero.position == position {
                return Some(participant.side);
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.position == position)
            .map(|unit| unit.side)
    }

    fn best_building_line_targets(&self, side: Side, origin: HexCoord, range: u8) -> Vec<String> {
        HexCoord::directions()
            .into_iter()
            .map(|direction| {
                self.pieces_for_opposing_team(side)
                    .into_iter()
                    .filter(|piece| {
                        origin.distance(piece.position) <= i32::from(range)
                            && origin.direction_to(piece.position) == Some(direction)
                    })
                    .map(|piece| piece.id)
                    .collect::<Vec<_>>()
            })
            .max_by_key(Vec::len)
            .unwrap_or_default()
    }

    fn best_building_heal_target(
        &self,
        side: Side,
        origin: HexCoord,
        range: u8,
        targets: BuffTargetPolicy,
    ) -> Option<String> {
        self.pieces_for_team(side.team())
            .into_iter()
            .filter(|piece| origin.distance(piece.position) <= i32::from(range))
            .filter(|piece| card_interactions::target_policy_allows(targets, piece.is_hero))
            .filter(|piece| self.piece_is_damaged(&piece.id))
            .min_by_key(|piece| origin.distance(piece.position))
            .map(|piece| piece.id)
    }

    fn mark_attacker_spent(&mut self, piece_id: &str) {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                let hero = &mut self.player_mut(side).hero;
                hero.ap_remaining -= 1;
                hero.has_attacked = true;
                return;
            }
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.ap_remaining -= 1;
            unit.has_attacked = true;
        }
    }

    fn damage_piece(&mut self, piece_id: &str, amount: i32) {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                damage_hero(&mut self.player_mut(side).hero, amount);
                return;
            }
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.armor -= amount;
        }
    }

    fn damage_pieces(
        &mut self,
        side: Side,
        piece_ids: Vec<String>,
        amount: i32,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        for piece_id in piece_ids {
            self.damage_piece(&piece_id, amount);
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::PieceDamaged {
                    side,
                    piece_id,
                    amount,
                },
            );
        }
        self.remove_dead_units(frames, action_index);
    }

    fn draw_cards_for_side(&mut self, side: Side, amount: u8) -> Vec<Card> {
        let player = self.player_mut(side);
        (0..amount).filter_map(|_| player.draw()).collect()
    }

    fn heal_piece(&mut self, piece_id: &str, amount: i32) {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                let hero = &mut self.player_mut(side).hero;
                hero.hp = (hero.hp + amount).min(hero.max_hp);
                return;
            }
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.armor = (unit.armor + amount).min(unit.max_armor);
        }
    }

    fn apply_stat_bonus_to_piece(
        &mut self,
        piece_id: &str,
        bonus: StatBonus,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> bool {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                return self.apply_stat_bonus_to_hero(side, bonus, frames, action_index);
            }
        }
        if !card_interactions::target_policy_allows(bonus.targets, false) {
            return false;
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.attack += bonus.attack;
            unit.armor += bonus.armor;
            unit.max_armor += bonus.armor;
            apply_ap_delta(&mut unit.ap_remaining, &mut unit.max_ap, bonus.max_ap);
            return true;
        }
        false
    }

    fn apply_stat_bonus_to_hero(
        &mut self,
        side: Side,
        bonus: StatBonus,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> bool {
        if !card_interactions::target_policy_allows(bonus.targets, true) {
            return false;
        }
        let mut shielded_hero_id = None;
        {
            let hero = &mut self.player_mut(side).hero;
            hero.attack += bonus.attack;
            apply_ap_delta(&mut hero.ap_remaining, &mut hero.max_ap, bonus.max_ap);
            if bonus.armor > 0 {
                hero.shield = hero.shield.saturating_add(bonus.armor);
                shielded_hero_id = Some(hero.id.clone());
            }
        }
        if let Some(hero_id) = shielded_hero_id {
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::HeroShielded {
                    side,
                    hero_id,
                    amount: bonus.armor,
                },
            );
        }
        true
    }

    fn piece_is_damaged(&self, piece_id: &str) -> bool {
        for participant in self.participants() {
            if participant.hero.id == piece_id {
                return participant.hero.hp < participant.hero.max_hp;
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == piece_id)
            .is_some_and(|unit| unit.armor < unit.max_armor)
    }

    fn remove_dead_units(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Vec<DestroyedUnit> {
        let destroyed: Vec<_> = self
            .board
            .units
            .iter()
            .filter(|unit| unit.armor <= 0)
            .map(|unit| {
                (
                    unit.side,
                    unit.id.clone(),
                    unit.name.clone(),
                    unit.position,
                    unit.items.clone(),
                )
            })
            .collect();
        self.board.units.retain(|unit| unit.armor > 0);
        let mut destroyed_units = Vec::new();
        for (side, unit_id, name, position, items) in destroyed {
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::UnitDestroyed {
                    side,
                    unit_id: unit_id.clone(),
                    name,
                },
            );
            destroyed_units.push(DestroyedUnit {
                side,
                unit_id: unit_id.clone(),
            });
            for item in items {
                let dropped_id = format!("dropped-{}", item.id);
                self.board.dropped_items.push(DroppedItem {
                    id: dropped_id,
                    position,
                    item: item.clone(),
                });
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::ItemDropped {
                        side,
                        carrier_id: unit_id.clone(),
                        item_id: item.id,
                        name: item.name,
                        position,
                    },
                );
            }
        }
        destroyed_units
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "combat attribution needs both sides of the resolved attack"
    )]
    fn apply_barbarian_combat_mana(
        &mut self,
        attacker: &PieceView,
        target: &PieceView,
        target_id: &str,
        attacker_id: &str,
        counter_damage_to_attacker: i32,
        destroyed: &[DestroyedUnit],
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        for destroyed_unit in destroyed {
            if destroyed_unit.unit_id == target_id {
                self.apply_barbarian_kill_mana(attacker, destroyed_unit, frames, action_index);
            }
            if counter_damage_to_attacker > 0 && destroyed_unit.unit_id == attacker_id {
                self.apply_barbarian_kill_mana(target, destroyed_unit, frames, action_index);
            }
        }
    }

    fn apply_barbarian_kill_mana(
        &mut self,
        killer: &PieceView,
        destroyed_unit: &DestroyedUnit,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        if destroyed_unit.side == killer.side || !self.is_barbarian_hero(&killer.id) {
            return;
        }

        let amount = 2;
        let player = self.player_mut(killer.side);
        player.mana = player.mana.saturating_add(amount);
        self.log
            .insert(0, format!("Barbarian gained {amount} mana from the kill."));
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::ManaGained {
                side: killer.side,
                amount,
                source: ReplayManaSource::BarbarianKill {
                    hero_id: killer.id.clone(),
                    unit_id: destroyed_unit.unit_id.clone(),
                },
            },
        );
    }

    fn is_barbarian_hero(&self, piece_id: &str) -> bool {
        self.player.hero.id == piece_id && self.player.hero.hero_type == HeroType::Barbarian
            || self.opponent.hero.id == piece_id
                && self.opponent.hero.hero_type == HeroType::Barbarian
    }

    fn pick_up_dropped_items_at(
        &mut self,
        side: Side,
        carrier_id: &str,
        position: HexCoord,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let Some(carrier) = self.piece_view(carrier_id) else {
            return;
        };
        if carrier.side != side || carrier.position != position {
            return;
        }

        let mut picked_up = Vec::new();
        let mut remaining_capacity = MAX_CARRIED_ITEMS.saturating_sub(
            self.carrier_item_count(carrier_id)
                .unwrap_or(MAX_CARRIED_ITEMS),
        );
        self.board.dropped_items.retain(|dropped_item| {
            if dropped_item.position == position && remaining_capacity > 0 {
                picked_up.push(dropped_item.item.clone());
                remaining_capacity -= 1;
                false
            } else {
                true
            }
        });

        for item in picked_up {
            let item_id = item.id.clone();
            let item_name = item.name.clone();
            if !self.add_carried_item_to_carrier(carrier_id, item) {
                continue;
            }
            self.log
                .insert(0, format!("{} picked up {}.", carrier_id, item_name));
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::ItemEquipped {
                    side,
                    carrier_id: carrier_id.to_string(),
                    item_id,
                    name: item_name,
                },
            );
        }
    }

    fn carrier_item_count(&self, carrier_id: &str) -> Option<usize> {
        for participant in self.participants() {
            if participant.hero.id == carrier_id {
                return Some(participant.hero.items.len());
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == carrier_id)
            .map(|unit| unit.items.len())
    }

    fn carried_item(&self, carrier_id: &str, item_id: &str) -> Option<CarriedItem> {
        for participant in self.participants() {
            if participant.hero.id == carrier_id {
                return participant
                    .hero
                    .items
                    .iter()
                    .find(|item| item.id == item_id)
                    .cloned();
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == carrier_id)
            .and_then(|unit| unit.items.iter().find(|item| item.id == item_id).cloned())
    }

    fn carrier_name(&self, carrier_id: &str) -> Option<String> {
        for participant in self.participants() {
            if participant.hero.id == carrier_id {
                return Some(format!("{:?}", participant.hero.hero_type));
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == carrier_id)
            .map(|unit| unit.name.clone())
    }

    fn add_carried_item_to_carrier(&mut self, carrier_id: &str, item: CarriedItem) -> bool {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == carrier_id {
                let hero = &mut self.player_mut(side).hero;
                if hero.items.len() >= MAX_CARRIED_ITEMS {
                    return false;
                }
                card_interactions::apply_item_passive_to_hero(hero, &item.passive);
                hero.items.push(item);
                return true;
            }
        }
        if let Some(unit) = self
            .board
            .units
            .iter_mut()
            .find(|unit| unit.id == carrier_id)
        {
            if unit.items.len() >= MAX_CARRIED_ITEMS {
                return false;
            }
            card_interactions::apply_item_passive(unit, &item.passive);
            unit.items.push(item);
            return true;
        }
        false
    }

    fn mark_carried_item_used(&mut self, carrier_id: &str, item_id: &str) -> bool {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == carrier_id {
                let Some(item) = self
                    .player_mut(side)
                    .hero
                    .items
                    .iter_mut()
                    .find(|item| item.id == item_id)
                else {
                    return false;
                };
                item.active_used_this_turn = true;
                return true;
            }
        }
        if let Some(item) = self
            .board
            .units
            .iter_mut()
            .find(|unit| unit.id == carrier_id)
            .and_then(|unit| unit.items.iter_mut().find(|item| item.id == item_id))
        {
            item.active_used_this_turn = true;
            return true;
        }
        false
    }

    fn add_stat_marker_to_carrier(&mut self, carrier_id: &str, marker: StatMarker) -> bool {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == carrier_id {
                let hero = &mut self.player_mut(side).hero;
                hero.attack += marker.attack;
                if marker.armor > 0 {
                    hero.shield = hero.shield.saturating_add(marker.armor);
                }
                apply_ap_delta(&mut hero.ap_remaining, &mut hero.max_ap, marker.max_ap);
                hero.stat_markers.push(marker);
                return true;
            }
        }
        if let Some(unit) = self
            .board
            .units
            .iter_mut()
            .find(|unit| unit.id == carrier_id)
        {
            unit.attack += marker.attack;
            unit.armor += marker.armor;
            unit.max_armor += marker.armor;
            apply_ap_delta(&mut unit.ap_remaining, &mut unit.max_ap, marker.max_ap);
            unit.stat_markers.push(marker);
            return true;
        }
        false
    }

    fn next_stat_marker_id(&self, carrier_id: &str, source_item_id: &str) -> String {
        for participant in self.participants() {
            if participant.hero.id == carrier_id {
                return format!(
                    "{source_item_id}-marker-{}",
                    participant.hero.stat_markers.len() + 1
                );
            }
        }
        let count = self
            .board
            .units
            .iter()
            .find(|unit| unit.id == carrier_id)
            .map(|unit| unit.stat_markers.len() + 1)
            .unwrap_or(1);
        format!("{source_item_id}-marker-{count}")
    }

    fn check_winner(&mut self, frames: &mut Vec<RecordedReplayFrame>, action_index: Option<u32>) {
        if self.phase == Phase::MatchOver {
            return;
        }

        self.knock_out_defeated_heroes(frames, action_index);

        let player_team_out = self
            .participants()
            .into_iter()
            .filter(|participant| participant.side.team() == Team::Player)
            .all(|participant| participant.knocked_out);
        let opponent_team_out = self
            .participants()
            .into_iter()
            .filter(|participant| participant.side.team() == Team::Opponent)
            .all(|participant| participant.knocked_out);

        let winner = match (player_team_out, opponent_team_out) {
            (true, true) => Some(Side::Player),
            (false, true) => Some(Side::Player),
            (true, false) => Some(Side::Opponent),
            (false, false) => None,
        };

        if let Some(winner) = winner {
            self.phase = Phase::MatchOver;
            self.winner = Some(winner);
            self.log.insert(0, format!("{} wins.", winner.label()));
            self.truncate_log();
            self.record_replay_frame(frames, action_index, ReplayEvent::MatchEnded { winner });
        }
    }

    fn knock_out_defeated_heroes(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
        let defeated_sides: Vec<_> = self
            .participant_sides()
            .into_iter()
            .filter(|side| {
                let participant = self.player_ref(*side);
                !participant.knocked_out && participant.hero.hp <= 0
            })
            .collect();

        for side in defeated_sides {
            let (hero_id, position, items) = {
                let hero = &self.player_ref(side).hero;
                (hero.id.clone(), hero.position, hero.items.clone())
            };
            {
                let participant = self.player_mut(side);
                participant.knocked_out = true;
                participant.hero.knocked_out = true;
                participant.hero.ap_remaining = 0;
                participant.hero.items.clear();
            }
            for item in items {
                let dropped_id = format!("dropped-{}", item.id);
                self.board.dropped_items.push(DroppedItem {
                    id: dropped_id,
                    position,
                    item: item.clone(),
                });
                self.record_replay_frame(
                    frames,
                    action_index,
                    ReplayEvent::ItemDropped {
                        side,
                        carrier_id: hero_id.clone(),
                        item_id: item.id,
                        name: item.name,
                        position,
                    },
                );
            }
            self.board.units.retain(|unit| unit.side != side);
            self.log
                .insert(0, format!("{} was knocked out.", side.label()));
        }
    }

    fn is_occupied(&self, coord: HexCoord) -> bool {
        self.participants()
            .into_iter()
            .any(|participant| !participant.knocked_out && participant.hero.position == coord)
            || self.board.units.iter().any(|unit| unit.position == coord)
    }

    fn next_unit_id(&mut self, side: Side) -> String {
        let prefix = match side {
            Side::Player => "P",
            Side::Opponent => "O",
            Side::PlayerTwo => "P2",
            Side::OpponentTwo => "O2",
        };
        let id = format!("{prefix}{}", self.next_unit_id);
        self.next_unit_id += 1;
        id
    }

    fn next_item_id(&mut self, side: Side) -> String {
        let prefix = match side {
            Side::Player => "PI",
            Side::Opponent => "OI",
            Side::PlayerTwo => "P2I",
            Side::OpponentTwo => "O2I",
        };
        let id = format!("{prefix}{}", self.next_item_id);
        self.next_item_id += 1;
        id
    }

    fn next_building_id(&mut self, side: Side) -> String {
        let prefix = match side {
            Side::Player => "PB",
            Side::Opponent => "OB",
            Side::PlayerTwo => "P2B",
            Side::OpponentTwo => "O2B",
        };
        let id = format!("{prefix}{}", self.next_building_id);
        self.next_building_id += 1;
        id
    }

    fn truncate_log(&mut self) {
        self.log.truncate(12);
    }
}

fn default_attack_range() -> u8 {
    1
}

impl PlayerState {
    fn replay_value(&self, expose_hand: bool) -> serde_json::Value {
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

fn opening_hand_size(progression: &MatchProgressionLoadout) -> usize {
    OPENING_HAND_SIZE + usize::from(progression.effects.opening_hand_delta)
}

fn mana_with_progression(base: u8, delta: i8) -> u8 {
    let value = i16::from(base) + i16::from(delta);
    value.clamp(0, i16::from(u8::MAX)) as u8
}

fn damage_hero(hero: &mut Hero, amount: i32) {
    let shield_damage = hero.shield.min(amount);
    hero.shield -= shield_damage;
    hero.hp -= amount - shield_damage;
}

fn apply_ap_delta(ap_remaining: &mut u8, max_ap: &mut u8, delta: i8) {
    if delta >= 0 {
        let amount = delta as u8;
        *ap_remaining = ap_remaining.saturating_add(amount);
        *max_ap = max_ap.saturating_add(amount);
    } else {
        let amount = delta.unsigned_abs();
        *ap_remaining = ap_remaining.saturating_sub(amount);
        *max_ap = max_ap.saturating_sub(amount);
    }
}

fn building_effect_is_activated(effect: &BuildingEffect) -> bool {
    matches!(
        effect,
        BuildingEffect::ActivatedDamageLine { .. }
            | BuildingEffect::ActivatedHeal { .. }
            | BuildingEffect::ActivatedStatBonus { .. }
    )
}

impl From<&Hero> for PieceView {
    fn from(hero: &Hero) -> Self {
        Self {
            id: hero.id.clone(),
            side: hero.side,
            position: hero.position,
            attack: hero.attack,
            attack_range: hero.attack_range,
            ap_remaining: hero.ap_remaining,
            has_attacked: hero.has_attacked,
            is_hero: true,
        }
    }
}

impl From<&Unit> for PieceView {
    fn from(unit: &Unit) -> Self {
        Self {
            id: unit.id.clone(),
            side: unit.side,
            position: unit.position,
            attack: unit.attack,
            attack_range: unit.attack_range,
            ap_remaining: unit.ap_remaining,
            has_attacked: unit.has_attacked,
            is_hero: false,
        }
    }
}

impl Side {
    fn label(self) -> &'static str {
        match self {
            Self::Player => "Player 1",
            Self::Opponent => "Opponent",
            Self::PlayerTwo => "Player 2",
            Self::OpponentTwo => "Opponent 2",
        }
    }

    pub(crate) fn opponent(self) -> Self {
        match self {
            Self::Player => Self::Opponent,
            Self::Opponent => Self::Player,
            Self::PlayerTwo => Self::OpponentTwo,
            Self::OpponentTwo => Self::PlayerTwo,
        }
    }

    pub(crate) fn team(self) -> Team {
        match self {
            Self::Player | Self::PlayerTwo => Team::Player,
            Self::Opponent | Self::OpponentTwo => Team::Opponent,
        }
    }

    pub(crate) fn opposing_team(self) -> Team {
        match self.team() {
            Team::Player => Team::Opponent,
            Team::Opponent => Team::Player,
        }
    }
}

impl Side {
    pub(crate) fn card_prefix(self) -> &'static str {
        match self {
            Self::Player => "p",
            Self::Opponent => "o",
            Self::PlayerTwo => "p2",
            Self::OpponentTwo => "o2",
        }
    }

    pub(crate) fn label_for_response(self) -> &'static str {
        match self {
            Self::Player => "Player 1",
            Self::Opponent => "Opponent 1",
            Self::PlayerTwo => "Player 2",
            Self::OpponentTwo => "Opponent 2",
        }
    }
}

#[cfg(test)]
mod tests;
