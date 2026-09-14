use rune_lanes_core::MatchState;
use rune_lanes_core::event_sourcing::{
    AggregateSnapshot, AggregateVersion, EventEnvelope, EventSourcedMatch, MatchEvent,
};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};

use crate::match_session::{RecordedReplayFrame, Side};
use crate::progression;

use super::db_values::SideDbValue;
use super::{MatchStoreError, SqliteMatchStore, insert_replay_frame, next_frame_index};

const SNAPSHOT_INTERVAL: u64 = 32;

impl SqliteMatchStore {
    pub(crate) fn load_event_sourced_match(
        &mut self,
        id: &str,
    ) -> Result<Option<(EventSourcedMatch, u32)>, MatchStoreError> {
        self.ensure_event_stream(id)?;

        let events = load_events(&self.connection, id)?;
        if events.is_empty() {
            return Ok(None);
        }
        let next_action_index = next_action_index_from_events(&events)?;
        let snapshot = load_snapshot(&self.connection, id)?;
        let aggregate = if let Some(snapshot) = snapshot {
            let last_version = events
                .last()
                .map(|event| event.aggregate_version)
                .ok_or_else(|| {
                    MatchStoreError::EventStore("event stream disappeared".to_string())
                })?;
            if snapshot.aggregate_version > last_version {
                return Err(MatchStoreError::EventStore(format!(
                    "snapshot version {} is newer than stream version {} for {id}",
                    snapshot.aggregate_version.0, last_version.0
                )));
            }
            let tail = events
                .iter()
                .filter(|event| event.aggregate_version > snapshot.aggregate_version)
                .cloned()
                .collect::<Vec<_>>();
            EventSourcedMatch::rehydrate_from_snapshot(&snapshot, &tail)?
        } else {
            EventSourcedMatch::rehydrate(&events)?
        };

        Ok(Some((aggregate, next_action_index)))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "atomic event + compatibility projection commit"
    )]
    pub(crate) fn append_event_and_projection(
        &mut self,
        id: &str,
        event: &EventEnvelope,
        aggregate_snapshot: &AggregateSnapshot,
        request_json: &str,
        state: &MatchState,
        frames: &[RecordedReplayFrame],
        forfeit_winner: Option<Side>,
    ) -> Result<Option<AggregateVersion>, MatchStoreError> {
        let expected =
            AggregateVersion(event.aggregate_version.0.checked_sub(1).ok_or_else(|| {
                MatchStoreError::EventStore("cannot append aggregate version zero".to_string())
            })?);
        let command_id = event_command_id(event).ok_or_else(|| {
            MatchStoreError::EventStore("non-genesis event has no command id".to_string())
        })?;
        let action_index = event_action_index(event).ok_or_else(|| {
            MatchStoreError::EventStore("non-genesis event has no action index".to_string())
        })?;

        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        let existing_version = transaction
            .query_row(
                "SELECT aggregate_version FROM match_events WHERE match_id = ?1 AND command_id = ?2",
                params![id, command_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if let Some(existing_version) = existing_version {
            let existing_version = stored_version(existing_version, id)?;
            return Ok(Some(existing_version));
        }

        let actual_raw = transaction.query_row(
            "SELECT COALESCE(MAX(aggregate_version), 0) FROM match_events WHERE match_id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )?;
        let actual = stored_version(actual_raw, id)?;
        if actual != expected {
            return Err(MatchStoreError::VersionConflict { expected, actual });
        }

        insert_event(&transaction, id, event)?;
        transaction.execute(
            "INSERT INTO match_actions (match_id, action_index, request_json, accepted_at) VALUES (?1, ?2, ?3, unixepoch())",
            params![id, i64::from(action_index), request_json],
        )?;

        let next_frame = next_frame_index(&transaction, id)?;
        for (offset, frame) in frames.iter().enumerate() {
            let event_json = serde_json::to_string(&frame.event)?;
            insert_replay_frame(
                &transaction,
                id,
                next_frame + offset as u32,
                frame,
                &event_json,
            )?;
        }

        let completed_at = if state.winner.is_some() {
            "completed_at = COALESCE(completed_at, unixepoch()),"
        } else {
            ""
        };
        transaction.execute(
            &format!(
                "UPDATE matches SET snapshot_json = ?2, {completed_at} updated_at = unixepoch() WHERE id = ?1"
            ),
            params![id, &aggregate_snapshot.state_snapshot_json],
        )?;

        if state.winner.is_some() {
            if let Some(winner) = forfeit_winner {
                transaction.execute(
                    "UPDATE shared_matches SET status = 'forfeited', forfeit_winner = ?2, updated_at = unixepoch() WHERE match_id = ?1",
                    params![id, winner.to_db()],
                )?;
            } else {
                transaction.execute(
                    "UPDATE shared_matches SET status = CASE status WHEN 'forfeited' THEN 'forfeited' ELSE 'completed' END, updated_at = unixepoch() WHERE match_id = ?1",
                    params![id],
                )?;
            }
            progression::award_completed_match_in_transaction(&transaction, id)?;
        }

        if event.aggregate_version.0 % SNAPSHOT_INTERVAL == 0 || state.winner.is_some() {
            upsert_snapshot(&transaction, id, aggregate_snapshot)?;
        }

        transaction.commit()?;
        Ok(None)
    }

    fn ensure_event_stream(&mut self, id: &str) -> Result<(), MatchStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let event_count = transaction.query_row(
            "SELECT COUNT(*) FROM match_events WHERE match_id = ?1",
            params![id],
            |row| row.get::<_, i64>(0),
        )?;
        if event_count == 0 {
            let legacy = transaction
                .query_row(
                    "SELECT matches.snapshot_json, (SELECT COALESCE(MAX(match_actions.action_index) + 1, 0) FROM match_actions WHERE match_actions.match_id = matches.id) FROM matches WHERE matches.id = ?1",
                    params![id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?;
            let Some((snapshot_json, next_action_index)) = legacy else {
                transaction.commit()?;
                return Ok(());
            };
            let next_action_index = u32::try_from(next_action_index).map_err(|_| {
                MatchStoreError::EventStore(format!("legacy action index is out of range for {id}"))
            })?;
            let state = MatchState::from_snapshot_json(&snapshot_json)?;
            insert_genesis(&transaction, id, &state, next_action_index)?;
        }
        transaction.commit()?;
        Ok(())
    }
}

pub(super) fn insert_genesis(
    transaction: &Transaction<'_>,
    id: &str,
    state: &MatchState,
    next_action_index: u32,
) -> Result<(), MatchStoreError> {
    let existing = transaction.query_row(
        "SELECT COUNT(*) FROM match_events WHERE match_id = ?1",
        params![id],
        |row| row.get::<_, i64>(0),
    )?;
    if existing != 0 {
        return Err(MatchStoreError::EventStore(format!(
            "refusing to replace existing event stream for {id}"
        )));
    }

    let (aggregate, genesis) =
        EventSourcedMatch::create_migration_genesis(state.clone(), next_action_index)?;
    insert_event(transaction, id, &genesis)?;
    upsert_snapshot(transaction, id, &aggregate.snapshot()?)?;
    Ok(())
}

fn load_events(
    connection: &rusqlite::Connection,
    id: &str,
) -> Result<Vec<EventEnvelope>, MatchStoreError> {
    let mut statement = connection.prepare(
        "SELECT aggregate_version, event_schema_version, ruleset_version, command_id, action_index, event_json FROM match_events WHERE match_id = ?1 ORDER BY aggregate_version",
    )?;
    let mut rows = statement.query(params![id])?;
    let mut events = Vec::new();
    while let Some(row) = rows.next()? {
        let stored_aggregate = stored_version(row.get::<_, i64>(0)?, id)?;
        let stored_schema = u16::try_from(row.get::<_, i64>(1)?).map_err(|_| {
            MatchStoreError::EventStore(format!("event schema version is out of range for {id}"))
        })?;
        let stored_ruleset = row.get::<_, String>(2)?;
        let stored_command_id = row.get::<_, Option<String>>(3)?;
        let stored_action_index = row
            .get::<_, Option<i64>>(4)?
            .map(u32::try_from)
            .transpose()
            .map_err(|_| {
                MatchStoreError::EventStore(format!("action index is out of range for {id}"))
            })?;
        let event_json = row.get::<_, String>(5)?;
        let event: EventEnvelope = serde_json::from_str(&event_json)?;

        if event.aggregate_version != stored_aggregate
            || event.event_schema_version.0 != stored_schema
            || event.ruleset_version.as_str() != stored_ruleset
            || event_command_id(&event).map(str::to_owned) != stored_command_id
            || event_action_index(&event) != stored_action_index
        {
            return Err(MatchStoreError::EventStore(format!(
                "event envelope metadata does not match stored columns for {id} at version {}",
                stored_aggregate.0
            )));
        }
        events.push(event);
    }
    Ok(events)
}

fn load_snapshot(
    connection: &rusqlite::Connection,
    id: &str,
) -> Result<Option<AggregateSnapshot>, MatchStoreError> {
    let row = connection
        .query_row(
            "SELECT aggregate_version, snapshot_schema_version, event_schema_version, ruleset_version, snapshot_json FROM match_event_snapshots WHERE match_id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    let Some((aggregate_version, snapshot_schema, event_schema, ruleset, snapshot_json)) = row
    else {
        return Ok(None);
    };
    let snapshot: AggregateSnapshot = serde_json::from_str(&snapshot_json)?;
    if snapshot.aggregate_version != stored_version(aggregate_version, id)?
        || i64::from(snapshot.snapshot_schema_version.0) != snapshot_schema
        || i64::from(snapshot.event_schema_version.0) != event_schema
        || snapshot.ruleset_version.as_str() != ruleset
    {
        return Err(MatchStoreError::EventStore(format!(
            "snapshot envelope metadata does not match stored columns for {id}"
        )));
    }
    Ok(Some(snapshot))
}

fn insert_event(
    transaction: &Transaction<'_>,
    id: &str,
    event: &EventEnvelope,
) -> Result<(), MatchStoreError> {
    let event_json = serde_json::to_string(event)?;
    transaction.execute(
        "INSERT INTO match_events (match_id, aggregate_version, event_schema_version, ruleset_version, command_id, action_index, event_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())",
        params![
            id,
            i64::try_from(event.aggregate_version.0).map_err(|_| MatchStoreError::EventStore("aggregate version exceeds SQLite integer range".to_string()))?,
            i64::from(event.event_schema_version.0),
            event.ruleset_version.as_str(),
            event_command_id(event),
            event_action_index(event).map(i64::from),
            event_json,
        ],
    )?;
    Ok(())
}

fn upsert_snapshot(
    transaction: &Transaction<'_>,
    id: &str,
    snapshot: &AggregateSnapshot,
) -> Result<(), MatchStoreError> {
    let snapshot_json = serde_json::to_string(snapshot)?;
    transaction.execute(
        "INSERT INTO match_event_snapshots (match_id, aggregate_version, snapshot_schema_version, event_schema_version, ruleset_version, snapshot_json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch()) ON CONFLICT(match_id) DO UPDATE SET aggregate_version = excluded.aggregate_version, snapshot_schema_version = excluded.snapshot_schema_version, event_schema_version = excluded.event_schema_version, ruleset_version = excluded.ruleset_version, snapshot_json = excluded.snapshot_json, updated_at = unixepoch()",
        params![
            id,
            i64::try_from(snapshot.aggregate_version.0).map_err(|_| MatchStoreError::EventStore("snapshot aggregate version exceeds SQLite integer range".to_string()))?,
            i64::from(snapshot.snapshot_schema_version.0),
            i64::from(snapshot.event_schema_version.0),
            snapshot.ruleset_version.as_str(),
            snapshot_json,
        ],
    )?;
    Ok(())
}

fn event_command_id(event: &EventEnvelope) -> Option<&str> {
    match &event.event {
        MatchEvent::Created { .. } => None,
        MatchEvent::CommandAccepted { command_id, .. }
        | MatchEvent::ForfeitAccepted { command_id, .. }
        | MatchEvent::AiTurnFinished { command_id, .. } => Some(command_id.as_str()),
    }
}

fn event_action_index(event: &EventEnvelope) -> Option<u32> {
    match &event.event {
        MatchEvent::Created { .. } => None,
        MatchEvent::CommandAccepted { action_index, .. }
        | MatchEvent::ForfeitAccepted { action_index, .. }
        | MatchEvent::AiTurnFinished { action_index, .. } => Some(*action_index),
    }
}

fn next_action_index_from_events(events: &[EventEnvelope]) -> Result<u32, MatchStoreError> {
    let Some(first) = events.first() else {
        return Err(MatchStoreError::EventStore(
            "event stream is empty".to_string(),
        ));
    };
    let MatchEvent::Created {
        next_action_index, ..
    } = first.event
    else {
        return Err(MatchStoreError::EventStore(
            "event stream is missing genesis".to_string(),
        ));
    };
    let mut next = *next_action_index;
    for event in &events[1..] {
        let Some(action_index) = event_action_index(event) else {
            return Err(MatchStoreError::EventStore(
                "non-genesis event is missing action index".to_string(),
            ));
        };
        next = next.max(
            action_index
                .checked_add(1)
                .ok_or_else(|| MatchStoreError::EventStore("action index overflow".to_string()))?,
        );
    }
    Ok(next)
}

fn stored_version(value: i64, id: &str) -> Result<AggregateVersion, MatchStoreError> {
    let value = u64::try_from(value).map_err(|_| {
        MatchStoreError::EventStore(format!("negative aggregate version stored for {id}"))
    })?;
    Ok(AggregateVersion(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_session::HeroType;

    #[test]
    fn new_match_has_authoritative_genesis_beside_projection_rows() {
        let path = std::env::temp_dir().join(format!(
            "rune-lanes-event-store-genesis-{}.sqlite3",
            rand::random::<u128>()
        ));
        let mut store = SqliteMatchStore::new(path).expect("store should open");
        let created = store
            .create_match_for_user(HeroType::Pyromancer, None)
            .expect("match should create");
        let event_count: i64 = store
            .connection_mut()
            .query_row(
                "SELECT COUNT(*) FROM match_events WHERE match_id = ?1",
                params![&created.id],
                |row| row.get(0),
            )
            .expect("event count should load");
        let action_count: i64 = store
            .connection_mut()
            .query_row(
                "SELECT COUNT(*) FROM match_actions WHERE match_id = ?1",
                params![&created.id],
                |row| row.get(0),
            )
            .expect("action count should load");
        assert_eq!(event_count, 1);
        assert_eq!(action_count, 0);
    }

    #[test]
    fn legacy_projection_bootstrap_uses_snapshot_and_only_carries_index_forward() {
        let path = std::env::temp_dir().join(format!(
            "rune-lanes-event-store-legacy-{}.sqlite3",
            rand::random::<u128>()
        ));
        let mut store = SqliteMatchStore::new(path).expect("store should open");
        let created = store
            .create_match_for_user(HeroType::Pyromancer, None)
            .expect("match should create");
        store
            .connection_mut()
            .execute(
                "DELETE FROM match_event_snapshots WHERE match_id = ?1",
                params![&created.id],
            )
            .unwrap();
        store
            .connection_mut()
            .execute(
                "DELETE FROM match_events WHERE match_id = ?1",
                params![&created.id],
            )
            .unwrap();
        store
            .connection_mut()
            .execute(
                "INSERT INTO match_actions (match_id, action_index, request_json, accepted_at) VALUES (?1, 7, '{}', unixepoch())",
                params![&created.id],
            )
            .unwrap();

        let (aggregate, next_action_index) = store
            .load_event_sourced_match(&created.id)
            .expect("legacy stream should bootstrap")
            .expect("match should exist");
        assert_eq!(aggregate.version(), AggregateVersion(1));
        assert_eq!(aggregate.state(), &created.state);
        assert_eq!(next_action_index, 8);
    }
}
