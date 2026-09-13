use crate::rules_kernel::{GameCommand, RuleViolation};

use super::{MatchState, Side};

impl MatchState {
    /// Validate a command against authoritative rules without mutating this state.
    ///
    /// The clone is intentional during the staged migration: query legality and
    /// command execution share one implementation rather than drifting apart.
    #[allow(
        dead_code,
        reason = "query boundary is introduced before UI and AI consumers migrate"
    )]
    pub(crate) fn validate_game_command(
        &self,
        side: Side,
        command: &GameCommand,
    ) -> Result<(), RuleViolation> {
        let mut candidate = self.clone();
        candidate
            .apply_game_command_recording_for_side(side, command.clone(), 0)
            .map(|_| ())
            .map_err(|error| RuleViolation::for_command(command, error))
    }
}
