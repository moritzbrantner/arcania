use super::{
    DestroyedUnit, DroppedItem, HeroType, HexCoord, MatchState, PieceView, RecordedReplayFrame,
    ReplayEvent, ReplayManaSource, Side, piece_can_attack,
};

impl MatchState {
    pub(super) fn resolve_move(
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

    pub(super) fn spend_piece_ap(&mut self, piece_id: &str) {
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

    pub(super) fn resolve_attack(
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

    pub(super) fn mark_attacker_spent(&mut self, piece_id: &str) {
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

    pub(super) fn remove_dead_units(
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
}
