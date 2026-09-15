use serde::{Deserialize, Serialize};

use super::{HexBoard, MatchMode, MatchState, Phase, PlayerState, Side, StackItem};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MatchSnapshot {
    #[serde(default = "default_match_mode")]
    mode: MatchMode,
    round: u32,
    phase: Phase,
    #[serde(default = "default_active_side")]
    active_side: Side,
    #[serde(default)]
    priority_side: Option<Side>,
    player: PlayerState,
    opponent: PlayerState,
    #[serde(default)]
    player_two: Option<PlayerState>,
    #[serde(default)]
    opponent_two: Option<PlayerState>,
    board: HexBoard,
    #[serde(default)]
    action_stack: Vec<StackItem>,
    log: Vec<String>,
    winner: Option<Side>,
    #[serde(default)]
    priority_passes: Vec<Side>,
    #[serde(default = "default_next_stack_item_id")]
    next_stack_item_id: u32,
    next_unit_id: u32,
    #[serde(default = "default_next_item_id")]
    next_item_id: u32,
    #[serde(default = "default_next_building_id")]
    next_building_id: u32,
}

impl From<&MatchState> for MatchSnapshot {
    fn from(match_state: &MatchState) -> Self {
        Self {
            round: match_state.round,
            mode: match_state.mode,
            phase: match_state.phase.clone(),
            active_side: match_state.active_side,
            priority_side: match_state.priority_side,
            player: match_state.player.clone(),
            opponent: match_state.opponent.clone(),
            player_two: match_state.player_two.clone(),
            opponent_two: match_state.opponent_two.clone(),
            board: match_state.board.clone(),
            action_stack: match_state.action_stack.clone(),
            log: match_state.log.clone(),
            winner: match_state.winner,
            priority_passes: match_state.priority_passes.clone(),
            next_stack_item_id: match_state.next_stack_item_id,
            next_unit_id: match_state.next_unit_id,
            next_item_id: match_state.next_item_id,
            next_building_id: match_state.next_building_id,
        }
    }
}

impl From<MatchSnapshot> for MatchState {
    fn from(snapshot: MatchSnapshot) -> Self {
        let mut board = snapshot.board;
        board.migrate_legacy_mana_sources();
        board.remove_legacy_outer_natural_mana_wells();
        Self {
            round: snapshot.round,
            mode: snapshot.mode,
            phase: snapshot.phase,
            active_side: snapshot.active_side,
            priority_side: snapshot.priority_side,
            player: snapshot.player,
            opponent: snapshot.opponent,
            player_two: snapshot.player_two,
            opponent_two: snapshot.opponent_two,
            board,
            action_stack: snapshot.action_stack,
            log: snapshot.log,
            winner: snapshot.winner,
            priority_passes: snapshot.priority_passes,
            next_stack_item_id: snapshot.next_stack_item_id,
            next_unit_id: snapshot.next_unit_id,
            next_item_id: snapshot.next_item_id,
            next_building_id: snapshot.next_building_id,
        }
    }
}

fn default_match_mode() -> MatchMode {
    MatchMode::Solo
}

fn default_active_side() -> Side {
    Side::Player
}

fn default_next_stack_item_id() -> u32 {
    1
}

fn default_next_item_id() -> u32 {
    1
}

fn default_next_building_id() -> u32 {
    1
}
