use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::cqrs::{CommandContext, GameCommand};
use crate::{MatchError, MatchState, RecordedReplayFrame, Side};

pub const CURRENT_EVENT_SCHEMA_VERSION: EventSchemaVersion = EventSchemaVersion(1);
pub const CURRENT_SNAPSHOT_SCHEMA_VERSION: SnapshotSchemaVersion = SnapshotSchemaVersion(1);
pub const CURRENT_RULESET_VERSION: &str = "rune-lanes-rules-v1";

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AggregateVersion(pub u64);

impl AggregateVersion {
    pub const ZERO: Self = Self(0);

    pub fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventSchemaVersion(pub u16);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotSchemaVersion(pub u16);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CommandId(String);

impl CommandId {
    pub fn new(value: impl Into<String>) -> Result<Self, EventSourcingError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(EventSourcingError::EmptyCommandId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RulesetVersion(String);

impl RulesetVersion {
    pub fn current() -> Self {
        Self(CURRENT_RULESET_VERSION.to_string())
    }

    pub fn new(value: impl Into<String>) -> Result<Self, EventSourcingError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(EventSourcingError::EmptyRulesetVersion);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandMetadata {
    pub command_id: CommandId,
    pub expected_version: AggregateVersion,
    pub side: Side,
    pub action_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MatchEvent {
    /// Event-sourcing migration genesis. The snapshot is the complete private
    /// authoritative state, not the public match projection. Later schemas may
    /// replace this with a fully structured setup event without changing the
    /// immutability contract of schema v1 streams.
    Created {
        initial_snapshot_json: String,
        /// Next compatibility projection action index at the migration boundary.
        /// This preserves legacy projection numbering without making action rows
        /// part of authoritative aggregate recovery.
        next_action_index: u32,
    },
    /// Fact that a tabletop command was accepted by the pinned ruleset.
    /// Rehydration evolves this event by deterministically replaying the command;
    /// it never re-runs application/auth/transport decisions.
    CommandAccepted {
        command_id: CommandId,
        side: Side,
        action_index: u32,
        command: GameCommand,
    },
    /// The application established that a disconnected opponent may be forfeited;
    /// the state transition itself remains a core-owned domain fact.
    ForfeitAccepted {
        command_id: CommandId,
        winner: Side,
        action_index: u32,
    },
    /// The solo AI policy selected no further concrete action in card-play.
    /// Scheduling AI remains application orchestration; finishing the turn is a
    /// deterministic core transition recorded as its own fact.
    AiTurnFinished {
        command_id: CommandId,
        side: Side,
        action_index: u32,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub aggregate_version: AggregateVersion,
    pub event_schema_version: EventSchemaVersion,
    pub ruleset_version: RulesetVersion,
    pub event: MatchEvent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandDecision {
    Append(EventEnvelope),
    AlreadyApplied { aggregate_version: AggregateVersion },
}

#[derive(Clone, Debug)]
pub struct EvolutionOutcome {
    pub replay_frames: Vec<RecordedReplayFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppliedCommandReceipt {
    pub command_id: CommandId,
    pub aggregate_version: AggregateVersion,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AggregateSnapshot {
    pub aggregate_version: AggregateVersion,
    pub snapshot_schema_version: SnapshotSchemaVersion,
    pub event_schema_version: EventSchemaVersion,
    pub ruleset_version: RulesetVersion,
    pub state_snapshot_json: String,
    pub applied_commands: Vec<AppliedCommandReceipt>,
}

#[derive(Clone, Debug)]
pub struct EventSourcedMatch {
    state: MatchState,
    version: AggregateVersion,
    ruleset_version: RulesetVersion,
    applied_commands: HashMap<CommandId, AggregateVersion>,
}

impl EventSourcedMatch {
    pub fn create(initial_state: MatchState) -> Result<(Self, EventEnvelope), EventSourcingError> {
        Self::create_migration_genesis(initial_state, 0)
    }

    pub fn create_migration_genesis(
        initial_state: MatchState,
        next_action_index: u32,
    ) -> Result<(Self, EventEnvelope), EventSourcingError> {
        let ruleset_version = RulesetVersion::current();
        let event = EventEnvelope {
            aggregate_version: AggregateVersion(1),
            event_schema_version: CURRENT_EVENT_SCHEMA_VERSION,
            ruleset_version: ruleset_version.clone(),
            event: MatchEvent::Created {
                initial_snapshot_json: initial_state.to_snapshot_json()?,
                next_action_index,
            },
        };
        let aggregate = Self::rehydrate(std::slice::from_ref(&event))?;
        Ok((aggregate, event))
    }

    pub fn state(&self) -> &MatchState {
        &self.state
    }

    pub fn version(&self) -> AggregateVersion {
        self.version
    }

    pub fn ruleset_version(&self) -> &RulesetVersion {
        &self.ruleset_version
    }

    /// Validate intent without mutating authoritative state.
    pub fn decide(
        &self,
        metadata: CommandMetadata,
        command: GameCommand,
    ) -> Result<CommandDecision, EventSourcingError> {
        if let Some(version) = self.validate_command_metadata(&metadata)? {
            return Ok(CommandDecision::AlreadyApplied {
                aggregate_version: version,
            });
        }

        // Validation intentionally runs on a clone. The authoritative aggregate
        // only changes when the resulting event is evolved after a successful
        // append to the event store.
        let mut candidate = self.state.clone();
        command.clone().execute_compatibility(
            &mut candidate,
            CommandContext {
                side: metadata.side,
                action_index: metadata.action_index,
            },
        )?;

        Ok(CommandDecision::Append(EventEnvelope {
            aggregate_version: self.version.next(),
            event_schema_version: CURRENT_EVENT_SCHEMA_VERSION,
            ruleset_version: self.ruleset_version.clone(),
            event: MatchEvent::CommandAccepted {
                command_id: metadata.command_id,
                side: metadata.side,
                action_index: metadata.action_index,
                command,
            },
        }))
    }

    pub fn decide_forfeit(
        &self,
        metadata: CommandMetadata,
        winner: Side,
    ) -> Result<CommandDecision, EventSourcingError> {
        if let Some(version) = self.validate_command_metadata(&metadata)? {
            return Ok(CommandDecision::AlreadyApplied {
                aggregate_version: version,
            });
        }
        if self.state.winner.is_some() {
            return Err(EventSourcingError::Rule(MatchError::MatchOver));
        }

        Ok(CommandDecision::Append(EventEnvelope {
            aggregate_version: self.version.next(),
            event_schema_version: CURRENT_EVENT_SCHEMA_VERSION,
            ruleset_version: self.ruleset_version.clone(),
            event: MatchEvent::ForfeitAccepted {
                command_id: metadata.command_id,
                winner,
                action_index: metadata.action_index,
            },
        }))
    }

    pub fn decide_ai_turn_finished(
        &self,
        metadata: CommandMetadata,
    ) -> Result<CommandDecision, EventSourcingError> {
        if let Some(version) = self.validate_command_metadata(&metadata)? {
            return Ok(CommandDecision::AlreadyApplied {
                aggregate_version: version,
            });
        }

        let mut candidate = self.state.clone();
        candidate.finish_solo_ai_turn_recording(metadata.side, metadata.action_index)?;

        Ok(CommandDecision::Append(EventEnvelope {
            aggregate_version: self.version.next(),
            event_schema_version: CURRENT_EVENT_SCHEMA_VERSION,
            ruleset_version: self.ruleset_version.clone(),
            event: MatchEvent::AiTurnFinished {
                command_id: metadata.command_id,
                side: metadata.side,
                action_index: metadata.action_index,
            },
        }))
    }

    /// Evolve authoritative state from an already accepted/persisted event.
    pub fn evolve(
        &mut self,
        envelope: &EventEnvelope,
    ) -> Result<EvolutionOutcome, EventSourcingError> {
        self.validate_envelope(envelope)?;
        let expected_next = self.version.next();
        if envelope.aggregate_version != expected_next {
            return Err(EventSourcingError::EventVersionGap {
                expected: expected_next,
                actual: envelope.aggregate_version,
            });
        }

        let command_id = match &envelope.event {
            MatchEvent::Created { .. } => {
                return Err(EventSourcingError::UnexpectedGenesisEvent {
                    aggregate_version: envelope.aggregate_version,
                });
            }
            MatchEvent::CommandAccepted { command_id, .. }
            | MatchEvent::ForfeitAccepted { command_id, .. }
            | MatchEvent::AiTurnFinished { command_id, .. } => command_id,
        };

        if let Some(existing_version) = self.applied_commands.get(command_id) {
            return Err(EventSourcingError::DuplicateCommandEvent {
                command_id: command_id.clone(),
                first_version: *existing_version,
                duplicate_version: envelope.aggregate_version,
            });
        }

        let replay_frames = match &envelope.event {
            MatchEvent::Created { .. } => unreachable!("genesis rejected above"),
            MatchEvent::CommandAccepted {
                side,
                action_index,
                command,
                ..
            } => {
                command
                    .clone()
                    .execute_compatibility(
                        &mut self.state,
                        CommandContext {
                            side: *side,
                            action_index: *action_index,
                        },
                    )?
                    .replay_frames
            }
            MatchEvent::ForfeitAccepted {
                winner,
                action_index,
                ..
            } => self.state.forfeit_recording(*winner, *action_index),
            MatchEvent::AiTurnFinished {
                side, action_index, ..
            } => self
                .state
                .finish_solo_ai_turn_recording(*side, *action_index)?,
        };

        self.version = envelope.aggregate_version;
        self.applied_commands
            .insert(command_id.clone(), envelope.aggregate_version);

        Ok(EvolutionOutcome { replay_frames })
    }

    pub fn rehydrate(events: &[EventEnvelope]) -> Result<Self, EventSourcingError> {
        let first = events
            .first()
            .ok_or(EventSourcingError::MissingGenesisEvent)?;
        if first.aggregate_version != AggregateVersion(1) {
            return Err(EventSourcingError::EventVersionGap {
                expected: AggregateVersion(1),
                actual: first.aggregate_version,
            });
        }
        validate_schema_and_rules(first)?;
        let MatchEvent::Created {
            initial_snapshot_json,
            ..
        } = &first.event
        else {
            return Err(EventSourcingError::MissingGenesisEvent);
        };

        let mut aggregate = Self {
            state: MatchState::from_snapshot_json(initial_snapshot_json)?,
            version: first.aggregate_version,
            ruleset_version: first.ruleset_version.clone(),
            applied_commands: HashMap::new(),
        };

        for event in &events[1..] {
            aggregate.evolve(event)?;
        }
        Ok(aggregate)
    }

    pub fn snapshot(&self) -> Result<AggregateSnapshot, EventSourcingError> {
        let mut applied_commands = self
            .applied_commands
            .iter()
            .map(|(command_id, aggregate_version)| AppliedCommandReceipt {
                command_id: command_id.clone(),
                aggregate_version: *aggregate_version,
            })
            .collect::<Vec<_>>();
        applied_commands.sort_by_key(|receipt| receipt.aggregate_version);

        Ok(AggregateSnapshot {
            aggregate_version: self.version,
            snapshot_schema_version: CURRENT_SNAPSHOT_SCHEMA_VERSION,
            event_schema_version: CURRENT_EVENT_SCHEMA_VERSION,
            ruleset_version: self.ruleset_version.clone(),
            state_snapshot_json: self.state.to_snapshot_json()?,
            applied_commands,
        })
    }

    pub fn rehydrate_from_snapshot(
        snapshot: &AggregateSnapshot,
        tail_events: &[EventEnvelope],
    ) -> Result<Self, EventSourcingError> {
        if snapshot.snapshot_schema_version != CURRENT_SNAPSHOT_SCHEMA_VERSION {
            return Err(EventSourcingError::UnsupportedSnapshotSchema {
                expected: CURRENT_SNAPSHOT_SCHEMA_VERSION,
                actual: snapshot.snapshot_schema_version,
            });
        }
        if snapshot.event_schema_version != CURRENT_EVENT_SCHEMA_VERSION {
            return Err(EventSourcingError::UnsupportedEventSchema {
                expected: CURRENT_EVENT_SCHEMA_VERSION,
                actual: snapshot.event_schema_version,
            });
        }
        let current_ruleset = RulesetVersion::current();
        if snapshot.ruleset_version != current_ruleset {
            return Err(EventSourcingError::UnsupportedRuleset {
                expected: current_ruleset,
                actual: snapshot.ruleset_version.clone(),
            });
        }

        let mut applied_commands = HashMap::new();
        for receipt in &snapshot.applied_commands {
            if receipt.aggregate_version > snapshot.aggregate_version {
                return Err(EventSourcingError::CorruptSnapshot(
                    "command receipt is newer than snapshot aggregate version".to_string(),
                ));
            }
            if applied_commands
                .insert(receipt.command_id.clone(), receipt.aggregate_version)
                .is_some()
            {
                return Err(EventSourcingError::CorruptSnapshot(
                    "duplicate command receipt".to_string(),
                ));
            }
        }

        let mut aggregate = Self {
            state: MatchState::from_snapshot_json(&snapshot.state_snapshot_json)?,
            version: snapshot.aggregate_version,
            ruleset_version: snapshot.ruleset_version.clone(),
            applied_commands,
        };
        for event in tail_events {
            aggregate.evolve(event)?;
        }
        Ok(aggregate)
    }

    fn validate_command_metadata(
        &self,
        metadata: &CommandMetadata,
    ) -> Result<Option<AggregateVersion>, EventSourcingError> {
        if let Some(version) = self.applied_commands.get(&metadata.command_id) {
            return Ok(Some(*version));
        }
        if metadata.expected_version != self.version {
            return Err(EventSourcingError::VersionConflict {
                expected: metadata.expected_version,
                actual: self.version,
            });
        }
        Ok(None)
    }

    fn validate_envelope(&self, envelope: &EventEnvelope) -> Result<(), EventSourcingError> {
        validate_schema_and_rules(envelope)?;
        if envelope.ruleset_version != self.ruleset_version {
            return Err(EventSourcingError::RulesetChangedWithinStream {
                expected: self.ruleset_version.clone(),
                actual: envelope.ruleset_version.clone(),
            });
        }
        Ok(())
    }
}

fn validate_schema_and_rules(envelope: &EventEnvelope) -> Result<(), EventSourcingError> {
    if envelope.event_schema_version != CURRENT_EVENT_SCHEMA_VERSION {
        return Err(EventSourcingError::UnsupportedEventSchema {
            expected: CURRENT_EVENT_SCHEMA_VERSION,
            actual: envelope.event_schema_version,
        });
    }
    let current_ruleset = RulesetVersion::current();
    if envelope.ruleset_version != current_ruleset {
        return Err(EventSourcingError::UnsupportedRuleset {
            expected: current_ruleset,
            actual: envelope.ruleset_version.clone(),
        });
    }
    Ok(())
}

#[derive(Debug)]
pub enum EventSourcingError {
    EmptyCommandId,
    EmptyRulesetVersion,
    MissingGenesisEvent,
    UnexpectedGenesisEvent {
        aggregate_version: AggregateVersion,
    },
    UnsupportedEventSchema {
        expected: EventSchemaVersion,
        actual: EventSchemaVersion,
    },
    UnsupportedSnapshotSchema {
        expected: SnapshotSchemaVersion,
        actual: SnapshotSchemaVersion,
    },
    UnsupportedRuleset {
        expected: RulesetVersion,
        actual: RulesetVersion,
    },
    RulesetChangedWithinStream {
        expected: RulesetVersion,
        actual: RulesetVersion,
    },
    VersionConflict {
        expected: AggregateVersion,
        actual: AggregateVersion,
    },
    EventVersionGap {
        expected: AggregateVersion,
        actual: AggregateVersion,
    },
    DuplicateCommandEvent {
        command_id: CommandId,
        first_version: AggregateVersion,
        duplicate_version: AggregateVersion,
    },
    CorruptSnapshot(String),
    Rule(MatchError),
    Serialization(serde_json::Error),
}

impl fmt::Display for EventSourcingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommandId => write!(f, "command id must not be empty"),
            Self::EmptyRulesetVersion => write!(f, "ruleset version must not be empty"),
            Self::MissingGenesisEvent => {
                write!(f, "event stream has no MatchCreated genesis event")
            }
            Self::UnexpectedGenesisEvent { aggregate_version } => write!(
                f,
                "genesis event cannot appear at aggregate version {}",
                aggregate_version.0
            ),
            Self::UnsupportedEventSchema { expected, actual } => write!(
                f,
                "unsupported event schema version {}; expected {}",
                actual.0, expected.0
            ),
            Self::UnsupportedSnapshotSchema { expected, actual } => write!(
                f,
                "unsupported snapshot schema version {}; expected {}",
                actual.0, expected.0
            ),
            Self::UnsupportedRuleset { expected, actual } => write!(
                f,
                "unsupported ruleset {}; expected {}",
                actual.as_str(),
                expected.as_str()
            ),
            Self::RulesetChangedWithinStream { expected, actual } => write!(
                f,
                "ruleset changed inside event stream from {} to {}",
                expected.as_str(),
                actual.as_str()
            ),
            Self::VersionConflict { expected, actual } => write!(
                f,
                "aggregate version conflict: expected {}, actual {}",
                expected.0, actual.0
            ),
            Self::EventVersionGap { expected, actual } => write!(
                f,
                "event stream version gap: expected {}, actual {}",
                expected.0, actual.0
            ),
            Self::DuplicateCommandEvent {
                command_id,
                first_version,
                duplicate_version,
            } => write!(
                f,
                "command {} appears at aggregate versions {} and {}",
                command_id.as_str(),
                first_version.0,
                duplicate_version.0
            ),
            Self::CorruptSnapshot(reason) => write!(f, "corrupt aggregate snapshot: {reason}"),
            Self::Rule(error) => write!(f, "rule rejected event-sourced command: {error}"),
            Self::Serialization(error) => write!(f, "event-sourcing serialization failed: {error}"),
        }
    }
}

impl Error for EventSourcingError {}

impl From<MatchError> for EventSourcingError {
    fn from(error: MatchError) -> Self {
        Self::Rule(error)
    }
}

impl From<serde_json::Error> for EventSourcingError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialization(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HexCoord;
    use crate::cqrs::GameCommand;

    fn metadata(
        id: &str,
        expected_version: AggregateVersion,
        action_index: u32,
    ) -> CommandMetadata {
        CommandMetadata {
            command_id: CommandId::new(id).expect("command id should be valid"),
            expected_version,
            side: Side::Player,
            action_index,
        }
    }

    fn append(decision: CommandDecision) -> EventEnvelope {
        match decision {
            CommandDecision::Append(event) => event,
            CommandDecision::AlreadyApplied { .. } => panic!("expected append decision"),
        }
    }

    #[test]
    fn decide_does_not_mutate_until_event_is_evolved() {
        let initial = MatchState::new_with_seed(7);
        let (mut aggregate, genesis) = EventSourcedMatch::create(initial).expect("create works");
        let before = aggregate
            .state()
            .to_snapshot_json()
            .expect("snapshot should serialize");

        let event = append(
            aggregate
                .decide(
                    metadata("phase-1", aggregate.version(), 0),
                    GameCommand::StartAttackPhase,
                )
                .expect("command should be accepted"),
        );

        assert_eq!(aggregate.version(), AggregateVersion(1));
        assert_eq!(
            aggregate.state().to_snapshot_json().unwrap(),
            before,
            "decide must not mutate authoritative state"
        );
        assert_eq!(genesis.aggregate_version, AggregateVersion(1));

        aggregate.evolve(&event).expect("event should evolve");
        assert_eq!(aggregate.version(), AggregateVersion(2));
    }

    #[test]
    fn rejected_command_emits_no_event_and_keeps_version() {
        let (aggregate, _) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let error = aggregate
            .decide(
                metadata("bad-move", aggregate.version(), 0),
                GameCommand::MovePiece {
                    piece_id: "missing-piece".to_string(),
                    to: HexCoord { q: 0, r: 0 },
                },
            )
            .expect_err("invalid command should be rejected");

        assert!(matches!(error, EventSourcingError::Rule(_)));
        assert_eq!(aggregate.version(), AggregateVersion(1));
    }

    #[test]
    fn full_stream_rehydrates_to_identical_authoritative_snapshot() {
        let (mut aggregate, genesis) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let attack_phase = append(
            aggregate
                .decide(
                    metadata("phase-attack", aggregate.version(), 0),
                    GameCommand::StartAttackPhase,
                )
                .expect("attack phase should be accepted"),
        );
        aggregate
            .evolve(&attack_phase)
            .expect("event should evolve");
        let card_play = append(
            aggregate
                .decide(
                    metadata("phase-cards", aggregate.version(), 1),
                    GameCommand::StartCardPlay,
                )
                .expect("card play should be accepted"),
        );
        aggregate.evolve(&card_play).expect("event should evolve");

        let rehydrated = EventSourcedMatch::rehydrate(&[genesis, attack_phase, card_play])
            .expect("stream should rehydrate");

        assert_eq!(
            rehydrated.state().to_snapshot_json().unwrap(),
            aggregate.state().to_snapshot_json().unwrap()
        );
        assert_eq!(rehydrated.version(), aggregate.version());
    }

    #[test]
    fn duplicate_command_id_is_idempotent_after_successful_evolution() {
        let (mut aggregate, _) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let command_id = CommandId::new("same-command").unwrap();
        let event = append(
            aggregate
                .decide(
                    CommandMetadata {
                        command_id: command_id.clone(),
                        expected_version: aggregate.version(),
                        side: Side::Player,
                        action_index: 0,
                    },
                    GameCommand::StartAttackPhase,
                )
                .expect("first command should append"),
        );
        aggregate.evolve(&event).expect("event should evolve");

        let duplicate = aggregate
            .decide(
                CommandMetadata {
                    command_id,
                    expected_version: aggregate.version(),
                    side: Side::Player,
                    action_index: 0,
                },
                GameCommand::StartAttackPhase,
            )
            .expect("duplicate should be idempotent");

        assert_eq!(
            duplicate,
            CommandDecision::AlreadyApplied {
                aggregate_version: AggregateVersion(2)
            }
        );
    }

    #[test]
    fn stale_expected_version_fails_closed() {
        let (aggregate, _) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let error = aggregate
            .decide(
                metadata("stale", AggregateVersion::ZERO, 0),
                GameCommand::StartAttackPhase,
            )
            .expect_err("stale writer must fail");

        assert!(matches!(error, EventSourcingError::VersionConflict { .. }));
    }

    #[test]
    fn snapshot_plus_tail_matches_full_event_replay() {
        let (mut aggregate, genesis) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let attack_phase = append(
            aggregate
                .decide(
                    metadata("attack", aggregate.version(), 0),
                    GameCommand::StartAttackPhase,
                )
                .expect("attack phase should be accepted"),
        );
        aggregate
            .evolve(&attack_phase)
            .expect("event should evolve");
        let snapshot = aggregate.snapshot().expect("snapshot should serialize");

        let card_play = append(
            aggregate
                .decide(
                    metadata("cards", aggregate.version(), 1),
                    GameCommand::StartCardPlay,
                )
                .expect("card phase should be accepted"),
        );
        aggregate.evolve(&card_play).expect("event should evolve");

        let full = EventSourcedMatch::rehydrate(&[genesis, attack_phase, card_play.clone()])
            .expect("full stream should rehydrate");
        let from_snapshot = EventSourcedMatch::rehydrate_from_snapshot(&snapshot, &[card_play])
            .expect("snapshot plus tail should rehydrate");

        assert_eq!(
            full.state().to_snapshot_json().unwrap(),
            from_snapshot.state().to_snapshot_json().unwrap()
        );
        assert_eq!(full.version(), from_snapshot.version());
    }

    #[test]
    fn incompatible_snapshot_schema_fails_closed() {
        let (aggregate, _) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let mut snapshot = aggregate.snapshot().expect("snapshot should serialize");
        snapshot.snapshot_schema_version = SnapshotSchemaVersion(99);

        let error = EventSourcedMatch::rehydrate_from_snapshot(&snapshot, &[])
            .expect_err("unsupported snapshot must fail");
        assert!(matches!(
            error,
            EventSourcingError::UnsupportedSnapshotSchema { .. }
        ));
    }

    #[test]
    fn ruleset_change_inside_stream_fails_closed() {
        let (aggregate, _) =
            EventSourcedMatch::create(MatchState::new_with_seed(7)).expect("create works");
        let mut event = append(
            aggregate
                .decide(
                    metadata("ruleset-change", aggregate.version(), 0),
                    GameCommand::StartAttackPhase,
                )
                .expect("command should be accepted"),
        );
        event.ruleset_version = RulesetVersion::new("future-rules").unwrap();

        let mut aggregate = aggregate;
        let error = aggregate
            .evolve(&event)
            .expect_err("mixed rulesets must fail closed");
        assert!(matches!(
            error,
            EventSourcingError::UnsupportedRuleset { .. }
        ));
    }
}
