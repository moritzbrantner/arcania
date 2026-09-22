use rune_lanes_core::commands::GameCommand;
use rune_lanes_core::event_sourcing::{
    CommandDecision, CommandId, CommandMetadata, EventSourcedMatch,
};
use rusqlite::params;

use crate::match_access::Actor;
use crate::match_commands::MatchCommands;
use crate::match_session::{HeroType, MatchActionRequest, Side};
use crate::match_store::{MatchStoreError, SqliteMatchStore};

fn test_db_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "rune-lanes-event-store-{name}-{}.sqlite3",
        rand::random::<u128>()
    ))
}

#[test]
fn authoritative_state_rehydrates_without_action_replay_or_match_snapshot_projections() {
    let path = test_db_path("projection-independence");
    let mut store = SqliteMatchStore::new(path).expect("store should open");
    let created = store
        .create_match_for_user(HeroType::Pyromancer, None)
        .expect("match should create");

    let applied = MatchCommands::new(&mut store)
        .apply_solo_action(
            Actor::Anonymous,
            &created.id,
            MatchActionRequest::StartCardPlay,
        )
        .expect("command should append an authoritative event");
    let expected_state = applied
        .stored_match
        .state
        .to_snapshot_json()
        .expect("state should serialize");

    store
        .connection_mut()
        .execute(
            "DELETE FROM match_actions WHERE match_id = ?1",
            params![&created.id],
        )
        .unwrap();
    store
        .connection_mut()
        .execute(
            "DELETE FROM match_replay_frames WHERE match_id = ?1",
            params![&created.id],
        )
        .unwrap();
    store
        .connection_mut()
        .execute(
            "UPDATE matches SET snapshot_json = initial_snapshot_json WHERE id = ?1",
            params![&created.id],
        )
        .unwrap();

    let (rehydrated, next_action_index) = store
        .load_event_sourced_match(&created.id)
        .expect("event stream should load")
        .expect("match should exist");

    assert_eq!(
        rehydrated.state().to_snapshot_json().unwrap(),
        expected_state,
        "authoritative recovery must not depend on compatibility projections"
    );
    assert_eq!(next_action_index, 1);
}

#[test]
fn stale_append_fails_before_writing_projection_rows() {
    let path = test_db_path("optimistic-conflict");
    let mut store = SqliteMatchStore::new(path).expect("store should open");
    let created = store
        .create_match_for_user(HeroType::Pyromancer, None)
        .expect("match should create");
    let (base, action_index) = store
        .load_event_sourced_match(&created.id)
        .expect("event stream should load")
        .expect("match should exist");

    let first_event = append_decision(
        &base,
        CommandMetadata {
            command_id: CommandId::new("first-writer").unwrap(),
            expected_version: base.version(),
            side: Side::Player,
            action_index,
        },
        GameCommand::StartCardPlay,
    );
    let mut first_candidate = base.clone();
    let first_outcome = first_candidate
        .evolve(&first_event)
        .expect("first event should evolve");
    let first_snapshot = first_candidate.snapshot().unwrap();
    store
        .append_event_and_projection(
            &created.id,
            &first_event,
            &first_snapshot,
            r#"{"type":"startCardPlay"}"#,
            first_candidate.state(),
            &first_outcome.replay_frames,
            None,
        )
        .expect("first writer should append");

    let stale_event = append_decision(
        &base,
        CommandMetadata {
            command_id: CommandId::new("stale-writer").unwrap(),
            expected_version: base.version(),
            side: Side::Player,
            action_index,
        },
        GameCommand::StartAttackPhase,
    );
    let mut stale_candidate = base;
    let stale_outcome = stale_candidate
        .evolve(&stale_event)
        .expect("stale candidate is locally valid");
    let stale_snapshot = stale_candidate.snapshot().unwrap();
    let error = store
        .append_event_and_projection(
            &created.id,
            &stale_event,
            &stale_snapshot,
            r#"{"type":"startAttackPhase"}"#,
            stale_candidate.state(),
            &stale_outcome.replay_frames,
            None,
        )
        .expect_err("stale writer must fail closed");
    assert!(matches!(error, MatchStoreError::VersionConflict { .. }));

    let event_count: i64 = store
        .connection_mut()
        .query_row(
            "SELECT COUNT(*) FROM match_events WHERE match_id = ?1",
            params![&created.id],
            |row| row.get(0),
        )
        .unwrap();
    let action_count: i64 = store
        .connection_mut()
        .query_row(
            "SELECT COUNT(*) FROM match_actions WHERE match_id = ?1",
            params![&created.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(event_count, 2, "genesis plus the first accepted command");
    assert_eq!(action_count, 1, "stale writes must not leak projections");
}

fn append_decision(
    aggregate: &EventSourcedMatch,
    metadata: CommandMetadata,
    command: GameCommand,
) -> rune_lanes_core::event_sourcing::EventEnvelope {
    match aggregate
        .decide(metadata, command)
        .expect("command should be valid against the loaded aggregate")
    {
        CommandDecision::Append(event) => event,
        CommandDecision::AlreadyApplied { .. } => panic!("fresh command id should append"),
    }
}
