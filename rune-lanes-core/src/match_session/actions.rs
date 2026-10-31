use super::{
    ActionTarget, BuildingEffect, HexCoord, MatchActionRequest, MatchError, MatchMode, MatchState,
    Phase, RecordedReplayFrame, ReplayEvent, Side, StackAction, card_interactions,
    piece_can_attack,
};

impl MatchState {
    #[cfg(test)]
    pub(crate) fn apply_action(&mut self, request: MatchActionRequest) -> Result<(), MatchError> {
        self.apply_action_internal(Side::Player, request, None)
            .map(|_| ())
    }

    #[cfg(test)]
    pub(crate) fn apply_action_recording(
        &mut self,
        request: MatchActionRequest,
        action_index: u32,
    ) -> Result<Vec<RecordedReplayFrame>, MatchError> {
        self.apply_action_recording_for_side(Side::Player, request, action_index)
    }

    pub(crate) fn apply_action_recording_for_side(
        &mut self,
        side: Side,
        request: MatchActionRequest,
        action_index: u32,
    ) -> Result<Vec<RecordedReplayFrame>, MatchError> {
        self.apply_action_internal(side, request, Some(action_index))
    }

    pub(crate) fn forfeit_recording(
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
                } else if side == Side::Opponent {
                    self.finish_ai_turn(side, &mut frames, action_index);
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

    pub(super) fn require_phase(&self, phase: Phase) -> Result<(), MatchError> {
        if self.phase != phase {
            return Err(MatchError::WrongPhase);
        }
        Ok(())
    }

    pub(super) fn move_piece_for_side(
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

    pub(super) fn attack_for_side(
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

    pub(super) fn activate_item_for_side(
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

    pub(super) fn truncate_log(&mut self) {
        self.log.truncate(12);
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
