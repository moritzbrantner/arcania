use std::collections::HashSet;

use super::{
    AiAdvanceOutcome, BuildingEffect, MatchError, MatchMode, MatchState, Phase, PieceView,
    RecordedReplayFrame, ReplayEvent, Side, SoloAiActionIntent, SoloAiCarriedItem, SoloAiDecision,
    SoloAiPolicy, SoloAiView,
};

enum AiDecisionApplication {
    Applied,
    FinishTurn,
    Illegal(MatchError),
}

impl MatchState {
    pub(super) fn advance_solo_ai(
        &mut self,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        let policy = SoloAiPolicy::default();
        self.advance_ai_for_side_with_policy(Side::Opponent, &policy, frames, action_index)
    }

    pub fn advance_ai_for_side_with_policy(
        &mut self,
        side: Side,
        policy: &SoloAiPolicy,
        frames: &mut Vec<RecordedReplayFrame>,
        action_index: Option<u32>,
    ) -> Result<(), MatchError> {
        self.advance_ai_for_side_with_policy_mode(side, policy, frames, action_index, false)
            .map(|_| ())
    }

    pub fn advance_ai_for_side_with_policy_strict(
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
            if let Some((card_id, target)) = self.solo_ai_response_card(side, policy) {
                self.play_card_for_side(side, card_id, target, frames, action_index)?;
                self.truncate_log();
                return Ok(AiAdvanceOutcome::ActionApplied);
            }
            self.pass_priority_for_side(side, frames, action_index)?;
            return Ok(AiAdvanceOutcome::PriorityPassed);
        }

        if self.active_side != side {
            return Err(MatchError::AiUnavailable);
        }

        let decision = self.solo_ai_phase_decision(side, policy)?;
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
            SoloAiDecision::StartAttackPhase => self
                .start_attack_phase_for_side(side, frames, action_index)
                .map(|()| AiDecisionApplication::Applied)
                .unwrap_or_else(AiDecisionApplication::Illegal),
            SoloAiDecision::StartCardPlay => self
                .start_card_play_for_side(side, frames, action_index)
                .map(|()| AiDecisionApplication::Applied)
                .unwrap_or_else(AiDecisionApplication::Illegal),
            SoloAiDecision::FinishTurn => AiDecisionApplication::FinishTurn,
        }
    }

    pub(super) fn finish_ai_turn(
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

    pub(super) fn solo_ai_view_for_side(&self, side: Side) -> SoloAiView {
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
}
