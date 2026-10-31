use super::{
    BuildingEffect, CardSummary, DroppedItem, MatchError, MatchState, Phase, RecordedReplayFrame,
    ReplayEvent, Side, Team, mana_with_progression, piece_can_attack,
};

impl MatchState {
    pub(super) fn end_player_turn(
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

    pub(super) fn end_shared_turn(
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

    pub(super) fn start_attack_phase_for_side(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        self.require_phase(Phase::Movement)?;
        self.transition_to_phase(side, Phase::Attack, frames, action_index);
        Ok(())
    }

    pub(super) fn start_card_play_for_side(
        &mut self,
        side: Side,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        if self.ruleset.turn.movement_card_play {
            self.require_phase(Phase::Attack)?;
        } else if self.phase != Phase::Movement && self.phase != Phase::Attack {
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
        self.record_replay_frame(
            frames,
            action_index,
            ReplayEvent::PhaseChanged { side, phase },
        );
    }

    pub(super) fn auto_start_card_play_if_attacks_exhausted(
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

    pub(super) fn side_has_legal_attack(&self, side: Side) -> bool {
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
            self.ruleset
                .turn
                .base_hero_mana
                .saturating_add(self.occupied_mana_sources(side)),
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

    pub(super) fn start_turn(
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
        if should_draw && let Some(card) = self.player_mut(side).draw() {
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

    pub(super) fn check_winner(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) {
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
}
