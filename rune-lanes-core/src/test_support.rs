use crate::{MatchState, RecordedReplayFrame, Side};

/// Test-fixture escape hatch for technical adapter tests.
///
/// Production code must use the event-sourced command/application boundary.
pub fn forfeit_match(
    state: &mut MatchState,
    winner: Side,
    action_index: u32,
) -> Vec<RecordedReplayFrame> {
    state.forfeit_recording(winner, action_index)
}
