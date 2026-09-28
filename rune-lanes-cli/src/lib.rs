//! Deterministic match script runner for Rune Lanes.
//!
//! The runner is a thin application adapter over `rune-lanes-core`: every
//! command enters through `EventSourcedMatch::decide`/`evolve` (the same path
//! the backend persists), and every legality answer comes from the query
//! facade. It owns no rules of its own.

mod expect;
mod render;
mod script;
mod setup;

use rune_lanes_core::commands::GameCommand;
use rune_lanes_core::event_sourcing::{
    AggregateVersion, CommandDecision, CommandId, CommandMetadata, EventEnvelope,
    EventSourcedMatch, EventSourcingError,
};
use rune_lanes_core::{MatchError, MatchState, ReplayEvent, Side, SoloAiPolicy};

pub use script::{ScriptError, parse_script};
pub use setup::{build_setup, setup_catalog};

use script::{ScriptEntry, ScriptLine};

/// Upper bound on AI steps for one `{"advanceAi": ..., "untilDone": true}`.
const MAX_AI_STEPS_PER_LINE: u32 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

/// Result of playing a script: rendered output plus the number of failed
/// expectations (non-zero means the script failed as a test).
#[derive(Clone, Debug)]
pub struct PlayReport {
    pub output: String,
    pub failed_expectations: usize,
}

#[derive(Debug)]
pub enum PlayError {
    Script(ScriptError),
    Setup(String),
    Engine(String),
}

impl std::fmt::Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Script(error) => write!(f, "{error}"),
            Self::Setup(message) => write!(f, "setup: {message}"),
            Self::Engine(message) => write!(f, "engine: {message}"),
        }
    }
}

impl std::error::Error for PlayError {}

impl From<ScriptError> for PlayError {
    fn from(error: ScriptError) -> Self {
        Self::Script(error)
    }
}

/// One recorded step of a run. Rendering (text or JSON) is derived from these
/// records so both formats always describe the same execution.
#[derive(Clone, Debug)]
pub(crate) enum Step {
    Command {
        label: String,
        side: Side,
        by_ai: bool,
        command: Option<GameCommand>,
        outcome: Outcome,
    },
    AiTurnFinished {
        label: String,
        side: Side,
        outcome: Outcome,
    },
    Expect {
        label: String,
        checks: usize,
        failures: Vec<String>,
    },
}

#[derive(Clone, Debug)]
pub(crate) enum Outcome {
    Accepted {
        event: Box<EventEnvelope>,
        replay_events: Vec<ReplayEvent>,
        state: Box<MatchState>,
    },
    Rejected(MatchError),
}

pub(crate) struct Run {
    pub setup: String,
    pub initial_state: MatchState,
    pub steps: Vec<Step>,
    pub final_state: MatchState,
    pub final_version: AggregateVersion,
    pub legal_side: Side,
    pub legal_commands: Vec<GameCommand>,
}

/// Play `script` from `setup` (or from the script's own `{"setup": ...}`
/// header when `setup` is `None`).
pub fn play(
    setup: Option<&str>,
    script: &str,
    format: OutputFormat,
) -> Result<PlayReport, PlayError> {
    let lines = parse_script(script)?;
    let (setup_label, body) = match (setup, lines.first()) {
        (
            Some(setup),
            Some(ScriptLine {
                entry: ScriptEntry::Setup(_),
                ..
            }),
        ) => (setup.to_string(), &lines[1..]),
        (Some(setup), _) => (setup.to_string(), &lines[..]),
        (
            None,
            Some(ScriptLine {
                entry: ScriptEntry::Setup(setup),
                ..
            }),
        ) => (setup.clone(), &lines[1..]),
        (None, _) => {
            return Err(PlayError::Setup(
                "no setup given; pass one on the command line or start the script with {\"setup\": \"<scenario>\"}".to_string(),
            ));
        }
    };
    let initial_state = build_setup(&setup_label).map_err(PlayError::Setup)?;
    let run = execute(setup_label, initial_state, body)?;
    let failed_expectations = run
        .steps
        .iter()
        .filter(|step| matches!(step, Step::Expect { failures, .. } if !failures.is_empty()))
        .count();
    let output = match format {
        OutputFormat::Text => render::text(&run),
        OutputFormat::Json => render::json(&run),
    };
    Ok(PlayReport {
        output,
        failed_expectations,
    })
}

/// The side whose input the match is waiting for: the priority holder while a
/// stack is pending, otherwise the active side.
pub(crate) fn acting_side(state: &MatchState) -> Side {
    state.priority_side.unwrap_or(state.active_side)
}

struct Session {
    aggregate: EventSourcedMatch,
    action_index: u32,
    last_result: Option<Result<(), MatchError>>,
}

impl Session {
    fn metadata(&self, side: Side, id: &str) -> Result<CommandMetadata, PlayError> {
        Ok(CommandMetadata {
            command_id: CommandId::new(id).map_err(engine)?,
            expected_version: self.aggregate.version(),
            side,
            action_index: self.action_index,
        })
    }

    /// Submit a decision result to the aggregate exactly like the backend's
    /// event-sourced command path: decide, then evolve the accepted event.
    fn settle(
        &mut self,
        decision: Result<CommandDecision, EventSourcingError>,
    ) -> Result<Outcome, PlayError> {
        let envelope = match decision {
            Ok(CommandDecision::Append(envelope)) => envelope,
            Ok(CommandDecision::AlreadyApplied { aggregate_version }) => {
                return Err(PlayError::Engine(format!(
                    "command id already applied at version {}",
                    aggregate_version.0
                )));
            }
            Err(EventSourcingError::Rule(error)) => {
                self.last_result = Some(Err(error.clone()));
                return Ok(Outcome::Rejected(error));
            }
            Err(error) => return Err(engine(error)),
        };
        let evolution = self.aggregate.evolve(&envelope).map_err(engine)?;
        self.action_index += 1;
        self.last_result = Some(Ok(()));
        Ok(Outcome::Accepted {
            event: Box::new(envelope),
            replay_events: evolution
                .replay_frames
                .into_iter()
                .map(|frame| frame.event)
                .collect(),
            state: Box::new(self.aggregate.state().clone()),
        })
    }
}

fn engine(error: impl std::fmt::Display) -> PlayError {
    PlayError::Engine(error.to_string())
}

fn execute(
    setup: String,
    initial_state: MatchState,
    lines: &[ScriptLine],
) -> Result<Run, PlayError> {
    let (aggregate, _genesis) = EventSourcedMatch::create(initial_state.clone()).map_err(engine)?;
    let mut session = Session {
        aggregate,
        action_index: 0,
        last_result: None,
    };
    let mut steps = Vec::new();

    for ScriptLine { line, entry } in lines {
        match entry {
            ScriptEntry::Setup(_) => unreachable!("parser only accepts setup as the first line"),
            ScriptEntry::Command { side, command } => {
                let side = side.unwrap_or_else(|| acting_side(session.aggregate.state()));
                let metadata = session.metadata(side, &format!("script:{line}"))?;
                let decision = session.aggregate.decide(metadata, command.clone());
                let outcome = session.settle(decision)?;
                steps.push(Step::Command {
                    label: line.to_string(),
                    side,
                    by_ai: false,
                    command: Some(command.clone()),
                    outcome,
                });
            }
            ScriptEntry::AdvanceAi { side, until_done } => {
                advance_ai(&mut session, &mut steps, *line, *side, *until_done)?;
            }
            ScriptEntry::Expect(expectation) => {
                let failures = expect::evaluate(
                    expectation,
                    session.aggregate.state(),
                    session.last_result.as_ref(),
                );
                steps.push(Step::Expect {
                    label: line.to_string(),
                    checks: expectation.checks.len(),
                    failures,
                });
            }
        }
    }

    let final_state = session.aggregate.state().clone();
    let legal_side = acting_side(&final_state);
    let legal_commands = final_state.queries().legal_commands(legal_side);
    Ok(Run {
        setup,
        initial_state,
        steps,
        final_version: session.aggregate.version(),
        final_state,
        legal_side,
        legal_commands,
    })
}

/// Mirror the backend's `advanceAi` orchestration: ask the baseline Solo AI
/// policy for one concrete command and submit it through `decide`, or record
/// `AiTurnFinished` when the policy has nothing left in Card Play.
fn advance_ai(
    session: &mut Session,
    steps: &mut Vec<Step>,
    line: usize,
    side: Side,
    until_done: bool,
) -> Result<(), PlayError> {
    let policy = SoloAiPolicy::baseline();
    for step in 1..=MAX_AI_STEPS_PER_LINE {
        let label = if until_done {
            format!("{line}.{step}")
        } else {
            line.to_string()
        };
        let command_id = format!("script:{line}:ai:{step}");
        let next = session
            .aggregate
            .state()
            .next_solo_ai_game_command_with_policy(side, &policy);
        match next {
            Ok(Some(command)) => {
                let metadata = session.metadata(side, &command_id)?;
                let decision = session.aggregate.decide(metadata, command.clone());
                let outcome = session.settle(decision)?;
                let rejected = matches!(outcome, Outcome::Rejected(_));
                steps.push(Step::Command {
                    label,
                    side,
                    by_ai: true,
                    command: Some(command),
                    outcome,
                });
                if rejected {
                    return Ok(());
                }
            }
            Ok(None) => {
                let metadata = session.metadata(side, &command_id)?;
                let decision = session.aggregate.decide_ai_turn_finished(metadata);
                let outcome = session.settle(decision)?;
                steps.push(Step::AiTurnFinished {
                    label,
                    side,
                    outcome,
                });
                return Ok(());
            }
            Err(error) => {
                // The AI cannot act for `side` now. That is only worth a step
                // when it was the first attempt on this line.
                if step == 1 {
                    session.last_result = Some(Err(error.clone()));
                    steps.push(Step::Command {
                        label,
                        side,
                        by_ai: true,
                        command: None,
                        outcome: Outcome::Rejected(error),
                    });
                }
                return Ok(());
            }
        }
        if !until_done {
            return Ok(());
        }
    }
    Err(PlayError::Engine(format!(
        "advanceAi on line {line} did not settle within {MAX_AI_STEPS_PER_LINE} steps"
    )))
}

pub const USAGE: &str = "\
Usage:
  rune-lanes play [<setup>] <script.jsonl> [--json]
  rune-lanes setups

<setup> is a dev match scenario id (see `rune-lanes setups`) or seed:<n> for a
fresh seeded Solo match. It may be omitted when the script's first line is
{\"setup\": \"<setup>\"}; a setup given on the command line wins.
Exit codes: 0 all expectations passed, 1 an expectation failed, 2 usage,
script, setup or engine error.";

/// CLI entry point: returns the process exit code and writes to the given
/// streams so the binary stays a two-line shim.
pub fn run_cli(
    args: &[String],
    stdout: &mut dyn std::io::Write,
    stderr: &mut dyn std::io::Write,
) -> i32 {
    let mut json = false;
    let mut positional = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "-h" | "--help" => {
                let _ = writeln!(stdout, "{USAGE}");
                return 0;
            }
            _ => positional.push(arg.as_str()),
        }
    }
    let format = if json {
        OutputFormat::Json
    } else {
        OutputFormat::Text
    };
    match positional.as_slice() {
        ["setups"] => {
            for (id, description) in setup_catalog() {
                let _ = writeln!(stdout, "{id:<22} {description}");
            }
            0
        }
        ["play", script_path] => play_file(None, script_path, format, stdout, stderr),
        ["play", setup, script_path] => play_file(Some(setup), script_path, format, stdout, stderr),
        _ => {
            let _ = writeln!(stderr, "{USAGE}");
            2
        }
    }
}

fn play_file(
    setup: Option<&str>,
    script_path: &str,
    format: OutputFormat,
    stdout: &mut dyn std::io::Write,
    stderr: &mut dyn std::io::Write,
) -> i32 {
    let script = match std::fs::read_to_string(script_path) {
        Ok(script) => script,
        Err(error) => {
            let _ = writeln!(stderr, "cannot read {script_path}: {error}");
            return 2;
        }
    };
    match play(setup, &script, format) {
        Ok(report) => {
            let _ = write!(stdout, "{}", report.output);
            i32::from(report.failed_expectations > 0)
        }
        Err(error) => {
            let _ = writeln!(stderr, "{error}");
            2
        }
    }
}
