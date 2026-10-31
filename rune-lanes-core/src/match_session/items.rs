use super::{
    AppliedStatDelta, CarriedItem, HexCoord, MatchState, RecordedReplayFrame, ReplayEvent, Side,
    StatMarker, apply_ap_delta, card_interactions,
};

impl MatchState {
    pub(super) fn pick_up_dropped_items_at(
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
        let mut remaining_capacity = usize::from(self.ruleset.turn.max_carried_items)
            .saturating_sub(
                self.carrier_item_count(carrier_id)
                    .unwrap_or(usize::from(self.ruleset.turn.max_carried_items)),
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

    pub(super) fn carrier_item_count(&self, carrier_id: &str) -> Option<usize> {
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

    pub(super) fn carried_item(&self, carrier_id: &str, item_id: &str) -> Option<CarriedItem> {
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

    pub(super) fn carrier_name(&self, carrier_id: &str) -> Option<String> {
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

    pub(super) fn add_carried_item_to_carrier(
        &mut self,
        carrier_id: &str,
        item: CarriedItem,
    ) -> bool {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == carrier_id {
                let max_items = usize::from(self.ruleset.turn.max_carried_items);
                let hero = &mut self.player_mut(side).hero;
                if hero.items.len() >= max_items {
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
            if unit.items.len() >= usize::from(self.ruleset.turn.max_carried_items) {
                return false;
            }
            card_interactions::apply_item_passive(unit, &item.passive);
            unit.items.push(item);
            return true;
        }
        false
    }

    pub(super) fn mark_carried_item_used(&mut self, carrier_id: &str, item_id: &str) -> bool {
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

    pub(super) fn add_stat_marker_to_carrier(
        &mut self,
        carrier_id: &str,
        marker: StatMarker,
    ) -> Option<AppliedStatDelta> {
        for side in self.participant_sides() {
            if self.player_ref(side).hero.id == carrier_id {
                let hero = &mut self.player_mut(side).hero;
                let previous_attack = hero.attack;
                let previous_shield = hero.shield;
                hero.attack = hero.attack.saturating_add(marker.attack).max(0);
                if marker.armor > 0 {
                    hero.shield = hero.shield.saturating_add(marker.armor);
                }
                apply_ap_delta(&mut hero.ap_remaining, &mut hero.max_ap, marker.max_ap);
                hero.stat_markers.push(marker);
                return Some(AppliedStatDelta {
                    attack_delta: hero.attack.saturating_sub(previous_attack),
                    armor_delta: hero.shield.saturating_sub(previous_shield),
                });
            }
        }
        if let Some(unit) = self
            .board
            .units
            .iter_mut()
            .find(|unit| unit.id == carrier_id)
        {
            let previous_attack = unit.attack;
            let previous_armor = unit.armor;
            unit.attack = unit.attack.saturating_add(marker.attack).max(0);
            unit.armor = unit.armor.saturating_add(marker.armor).max(1);
            unit.max_armor = unit.max_armor.saturating_add(marker.armor).max(1);
            apply_ap_delta(&mut unit.ap_remaining, &mut unit.max_ap, marker.max_ap);
            unit.stat_markers.push(marker);
            return Some(AppliedStatDelta {
                attack_delta: unit.attack.saturating_sub(previous_attack),
                armor_delta: unit.armor.saturating_sub(previous_armor),
            });
        }
        None
    }

    pub(super) fn next_stat_marker_id(&self, carrier_id: &str, source_item_id: &str) -> String {
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
}
