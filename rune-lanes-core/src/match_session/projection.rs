use serde_json::json;

use super::{HexBoard, MatchState, PlayerState, Side, apply_ap_delta};

impl MatchState {
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

    pub(super) fn public_participants_for_side(&self, viewer_side: Side) -> Vec<serde_json::Value> {
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

    pub(super) fn public_board(&self) -> HexBoard {
        let mut board = self.board.clone();
        for unit in &mut board.units {
            let bonus = self.aura_stat_bonus_for(unit.side, unit.position, false);
            unit.attack = unit.attack.saturating_add(bonus.attack).max(0);
            unit.armor = unit.armor.saturating_add(bonus.armor).max(1);
            unit.max_armor = unit.max_armor.saturating_add(bonus.armor).max(1);
            apply_ap_delta(&mut unit.ap_remaining, &mut unit.max_ap, bonus.max_ap);
        }
        board
    }

    /// Read-only participant access for application projections.
    pub fn participant_for_public(&self, side: Side) -> Option<&PlayerState> {
        match side {
            Side::Player => Some(&self.player),
            Side::Opponent => Some(&self.opponent),
            Side::PlayerTwo => self.player_two.as_ref(),
            Side::OpponentTwo => self.opponent_two.as_ref(),
        }
    }
}
