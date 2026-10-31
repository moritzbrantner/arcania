use super::{
    ActionTarget, Card, CardKind, CardSummary, HexCoord, MatchError, MatchMode, MatchState, Phase,
    RecordedReplayFrame, ReplayEvent, Side, StackAction, StatBonus, Team, card_interactions,
};

impl MatchState {
    pub(super) fn play_card_for_side(
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
            if self.phase != Phase::CardPlay
                && !(self.ruleset.turn.movement_card_play && self.phase == Phase::Movement)
            {
                return Err(MatchError::WrongPhase);
            }
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
                    .is_some_and(|count| count >= usize::from(self.ruleset.turn.max_carried_items))
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

    pub(super) fn apply_resolved_spell(
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
                if let Some(applied) = self.apply_stat_bonus_to_piece(
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
                            attack_delta: applied.attack_delta,
                            armor_delta: applied.armor_delta,
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
}
