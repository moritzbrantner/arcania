use crate::match_session::{AiAdvanceOutcome, Phase};

use super::*;

#[derive(Clone, Debug)]
pub(super) struct SimulationGameResult {
    pub(super) spec: GameSpec,
    pub(super) outcome: GameOutcome,
    pub(super) action_count: u32,
    pub(super) frames: Vec<RecordedReplayFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum GameOutcome {
    CandidateWin,
    BaselineWin,
    Draw,
    Timeout,
    IllegalAction(String),
}

pub(super) fn run_game(
    spec: &GameSpec,
    baseline_policy: &crate::match_session::SoloAiPolicy,
    candidate_policy: &crate::match_session::SoloAiPolicy,
    setup: &SimulationSetup,
    max_actions: u32,
) -> Result<SimulationGameResult, AiLabError> {
    let mut game = setup.new_match(spec)?;
    let mut frames = vec![game.initial_replay_frame()];
    let mut action_count = 0;
    while game.phase != Phase::MatchOver && action_count < max_actions {
        let side = if !game.action_stack.is_empty() {
            game.priority_side.ok_or_else(|| {
                AiLabError::Config("stack is pending without priority".to_string())
            })?
        } else {
            game.active_side
        };
        let policy = if side == spec.candidate_side {
            candidate_policy
        } else {
            baseline_policy
        };
        let mut action_frames = Vec::new();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            game.advance_ai_for_side_with_policy_strict(
                side,
                policy,
                &mut action_frames,
                Some(action_count),
            )
        }));
        match result {
            Ok(Ok(AiAdvanceOutcome::IllegalIntent { reason })) => {
                return Ok(SimulationGameResult {
                    spec: spec.clone(),
                    outcome: GameOutcome::IllegalAction(reason),
                    action_count,
                    frames,
                });
            }
            Ok(Ok(_)) => {
                frames.extend(action_frames);
                action_count += 1;
            }
            Ok(Err(error)) => {
                return Ok(SimulationGameResult {
                    spec: spec.clone(),
                    outcome: GameOutcome::IllegalAction(error.to_string()),
                    action_count,
                    frames,
                });
            }
            Err(_) => {
                return Ok(SimulationGameResult {
                    spec: spec.clone(),
                    outcome: GameOutcome::IllegalAction("AI action panicked".to_string()),
                    action_count,
                    frames,
                });
            }
        }
    }
    let outcome = match game.winner {
        Some(winner) if winner == spec.candidate_side => GameOutcome::CandidateWin,
        Some(_) => GameOutcome::BaselineWin,
        None if action_count >= max_actions => GameOutcome::Timeout,
        None => GameOutcome::Draw,
    };
    Ok(SimulationGameResult {
        spec: spec.clone(),
        outcome,
        action_count,
        frames,
    })
}
