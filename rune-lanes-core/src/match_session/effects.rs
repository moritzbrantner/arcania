use super::{
    Card, Hero, MatchState, RecordedReplayFrame, ReplayEvent, Side, StatBonus, card_interactions,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AppliedStatDelta {
    pub(super) attack_delta: i32,
    pub(super) armor_delta: i32,
}

impl MatchState {
    pub(super) fn damage_piece(&mut self, piece_id: &str, amount: i32) {
        let amount = amount.max(0);
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                damage_hero(&mut self.player_mut(side).hero, amount);
                return;
            }
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.armor = unit.armor.saturating_sub(amount);
        }
    }

    pub(super) fn damage_pieces(
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

    pub(super) fn draw_cards_for_side(&mut self, side: Side, amount: u8) -> Vec<Card> {
        let player = self.player_mut(side);
        (0..amount).filter_map(|_| player.draw()).collect()
    }

    pub(super) fn heal_piece(&mut self, piece_id: &str, amount: i32) {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                let hero = &mut self.player_mut(side).hero;
                hero.hp = hero.hp.saturating_add(amount).min(hero.max_hp);
                return;
            }
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            unit.armor = unit.armor.saturating_add(amount).min(unit.max_armor);
        }
    }

    pub(super) fn apply_stat_bonus_to_piece(
        &mut self,
        piece_id: &str,
        bonus: StatBonus,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Option<AppliedStatDelta> {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == piece_id {
                return self.apply_stat_bonus_to_hero(side, bonus, frames, action_index);
            }
        }
        if !card_interactions::target_policy_allows(bonus.targets, false) {
            return None;
        }
        if let Some(unit) = self.board.units.iter_mut().find(|unit| unit.id == piece_id) {
            let previous_attack = unit.attack;
            let previous_armor = unit.armor;
            unit.attack = unit.attack.saturating_add(bonus.attack).max(0);
            unit.armor = unit.armor.saturating_add(bonus.armor).max(1);
            unit.max_armor = unit.max_armor.saturating_add(bonus.armor).max(1);
            apply_ap_delta(&mut unit.ap_remaining, &mut unit.max_ap, bonus.max_ap);
            return Some(AppliedStatDelta {
                attack_delta: unit.attack.saturating_sub(previous_attack),
                armor_delta: unit.armor.saturating_sub(previous_armor),
            });
        }
        None
    }

    fn apply_stat_bonus_to_hero(
        &mut self,
        side: Side,
        bonus: StatBonus,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Option<AppliedStatDelta> {
        if !card_interactions::target_policy_allows(bonus.targets, true)
            || (bonus.attack == 0 && bonus.max_ap == 0 && bonus.armor <= 0)
        {
            return None;
        }
        let (hero_id, applied) = {
            let hero = &mut self.player_mut(side).hero;
            let previous_attack = hero.attack;
            let previous_shield = hero.shield;
            hero.attack = hero.attack.saturating_add(bonus.attack).max(0);
            apply_ap_delta(&mut hero.ap_remaining, &mut hero.max_ap, bonus.max_ap);
            if bonus.armor > 0 {
                hero.shield = hero.shield.saturating_add(bonus.armor);
            }
            (
                hero.id.clone(),
                AppliedStatDelta {
                    attack_delta: hero.attack.saturating_sub(previous_attack),
                    armor_delta: hero.shield.saturating_sub(previous_shield),
                },
            )
        };
        if applied.armor_delta > 0 {
            self.record_replay_frame(
                frames,
                action_index,
                ReplayEvent::HeroShielded {
                    side,
                    hero_id,
                    amount: applied.armor_delta,
                },
            );
        }
        Some(applied)
    }
}

fn damage_hero(hero: &mut Hero, amount: i32) {
    let shield_damage = hero.shield.min(amount);
    hero.shield = hero.shield.saturating_sub(shield_damage);
    hero.hp = hero.hp.saturating_sub(amount.saturating_sub(shield_damage));
}

pub(super) fn apply_ap_delta(ap_remaining: &mut u8, max_ap: &mut u8, delta: i8) {
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
