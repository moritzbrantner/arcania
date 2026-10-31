use super::{
    ActionTarget, Building, BuildingEffect, CardKind, CardSummary, HexCoord, MatchError,
    MatchState, Phase, RecordedReplayFrame, ReplayEvent, Side, StackAction, StackItem, StatBonus,
    Team, card_interactions,
};

impl MatchState {
    pub(super) fn push_stack_item(
        &mut self,
        side: Side,
        priority: u8,
        action: StackAction,
    ) -> StackItem {
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

    pub(super) fn pass_priority_for_side(
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

    pub(super) fn resolve_all_stack(
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
        let attack_range = self.ruleset.turn.default_attack_range;
        let Some(mut unit) = card_interactions::summon_unit_from_card(
            &card,
            side,
            || self.next_unit_id(side),
            coord,
            &progression,
            is_first_summoned_unit,
        ) else {
            return;
        };
        unit.attack_range = attack_range;
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

    pub(super) fn resolve_item_activation(
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
                if let Some(applied) = self.add_stat_marker_to_carrier(&carrier_id, marker) {
                    self.record_replay_frame(
                        frames,
                        action_index,
                        ReplayEvent::PieceBuffed {
                            side,
                            piece_id: carrier_id,
                            attack_delta: applied.attack_delta,
                            armor_delta: applied.armor_delta,
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
                let _ = self.apply_stat_bonus_to_piece(
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
}
