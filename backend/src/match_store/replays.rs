use rusqlite::{OptionalExtension, Rows, Transaction, params};

use crate::match_session::{MatchState, RecordedReplayFrame, ReplayEvent};

use super::{
    MatchStoreError, SqliteMatchStore, StoredMatchSummary, StoredReplay, StoredReplayFrame,
};

impl SqliteMatchStore {
    pub fn list_replayable_matches_for_user(
        &self,
        user_id: i64,
    ) -> Result<Vec<StoredMatchSummary>, MatchStoreError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                matches.id,
                matches.snapshot_json,
                matches.created_at,
                matches.updated_at,
                matches.player_deck_name,
                COUNT(match_replay_frames.frame_index) AS frame_count
            FROM matches
            JOIN match_replay_frames ON match_replay_frames.match_id = matches.id
            LEFT JOIN shared_matches ON shared_matches.match_id = matches.id
            WHERE matches.initial_snapshot_json IS NOT NULL
                AND (
                    matches.mode = 'solo'
                    OR shared_matches.status IN ('completed', 'forfeited')
                )
                AND (
                    matches.owner_user_id = ?1
                    OR (
                        matches.mode = 'shared'
                        AND shared_matches.status IN ('completed', 'forfeited')
                        AND EXISTS (
                            SELECT 1
                            FROM match_seats
                            WHERE match_seats.match_id = matches.id
                                AND match_seats.participant_user_id = ?1
                        )
                    )
                )
            GROUP BY matches.id
            HAVING frame_count > 0
            ORDER BY matches.updated_at DESC
            ",
        )?;
        summaries_from_rows(statement.query(params![user_id])?)
    }

    pub fn list_profile_matches(
        &self,
        user_id: i64,
    ) -> Result<Vec<StoredMatchSummary>, MatchStoreError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                matches.id,
                matches.snapshot_json,
                matches.created_at,
                matches.updated_at,
                matches.player_deck_name,
                COUNT(match_replay_frames.frame_index) AS frame_count
            FROM matches
            JOIN match_replay_frames ON match_replay_frames.match_id = matches.id
            LEFT JOIN shared_matches ON shared_matches.match_id = matches.id
            WHERE matches.initial_snapshot_json IS NOT NULL
                AND (
                    matches.owner_user_id = ?1
                    OR (
                        matches.mode = 'shared'
                        AND shared_matches.status IN ('completed', 'forfeited')
                        AND EXISTS (
                            SELECT 1
                            FROM match_seats
                            WHERE match_seats.match_id = matches.id
                                AND match_seats.participant_user_id = ?1
                        )
                    )
                )
            GROUP BY matches.id
            HAVING frame_count > 0
            ORDER BY matches.updated_at DESC
            ",
        )?;
        summaries_from_rows(statement.query(params![user_id])?)
    }

    pub fn load_replay(&self, id: &str) -> Result<Option<StoredReplay>, MatchStoreError> {
        let summary = self.load_replay_summary(id)?;
        let Some(summary) = summary else {
            return Ok(None);
        };

        let mut statement = self.connection.prepare(
            "
            SELECT frame_index, action_index, event_json, snapshot_json
            FROM match_replay_frames
            WHERE match_id = ?1
            ORDER BY frame_index ASC
            ",
        )?;
        let rows = statement.query_map(params![id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut frames = Vec::new();
        for row in rows {
            let (frame_index, action_index, event_json, snapshot_json) = row?;
            let event = serde_json::from_str::<ReplayEvent>(&event_json)?;
            let state = MatchState::from_snapshot_json(&snapshot_json)?;
            frames.push(StoredReplayFrame {
                frame_index: frame_index as u32,
                action_index: action_index.map(|index| index as u32),
                event,
                state,
            });
        }

        if frames.is_empty() {
            return Ok(None);
        }

        Ok(Some(StoredReplay { summary, frames }))
    }

    fn load_replay_summary(&self, id: &str) -> Result<Option<StoredMatchSummary>, MatchStoreError> {
        let row: Option<(String, String, i64, i64, Option<String>, i64)> = self
            .connection
            .query_row(
                "
                SELECT
                    matches.id,
                    matches.snapshot_json,
                    matches.created_at,
                    matches.updated_at,
                    matches.player_deck_name,
                    COUNT(match_replay_frames.frame_index) AS frame_count
                FROM matches
                LEFT JOIN match_replay_frames ON match_replay_frames.match_id = matches.id
                WHERE matches.id = ?1
                    AND matches.initial_snapshot_json IS NOT NULL
                GROUP BY matches.id
                HAVING frame_count > 0
                ",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                },
            )
            .optional()?;

        row.map(
            |(id, snapshot, created_at, updated_at, player_deck_name, frame_count)| {
                MatchState::from_snapshot_json(&snapshot).map(|state| StoredMatchSummary {
                    id,
                    created_at,
                    updated_at,
                    frame_count: frame_count as usize,
                    player_deck_name,
                    state,
                })
            },
        )
        .transpose()
        .map_err(MatchStoreError::from)
    }
}

/// Writes frame 0 of a new match's replay from its starting state.
pub(super) fn insert_initial_frame(
    transaction: &Transaction<'_>,
    id: &str,
    state: &MatchState,
) -> Result<(), MatchStoreError> {
    insert_frame(transaction, id, 0, &state.initial_replay_frame())
}

/// Appends server-recorded frames after the last stored frame, keeping frame indexes dense.
pub(super) fn append_frames(
    transaction: &Transaction<'_>,
    id: &str,
    frames: &[RecordedReplayFrame],
) -> Result<(), MatchStoreError> {
    let next = transaction.query_row(
        "
        SELECT COALESCE(MAX(frame_index) + 1, 0)
        FROM match_replay_frames
        WHERE match_id = ?1
        ",
        params![id],
        |row| row.get::<_, i64>(0),
    )? as u32;
    for (offset, frame) in frames.iter().enumerate() {
        insert_frame(transaction, id, next + offset as u32, frame)?;
    }
    Ok(())
}

fn insert_frame(
    transaction: &Transaction<'_>,
    id: &str,
    frame_index: u32,
    frame: &RecordedReplayFrame,
) -> Result<(), MatchStoreError> {
    let event_json = serde_json::to_string(&frame.event)?;
    transaction.execute(
        "
        INSERT INTO match_replay_frames (
            match_id,
            frame_index,
            action_index,
            event_json,
            snapshot_json,
            created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, unixepoch())
        ",
        params![
            id,
            i64::from(frame_index),
            frame.action_index.map(i64::from),
            event_json,
            frame.snapshot_json
        ],
    )?;
    Ok(())
}

fn summaries_from_rows(mut rows: Rows<'_>) -> Result<Vec<StoredMatchSummary>, MatchStoreError> {
    let mut summaries = Vec::new();
    while let Some(row) = rows.next()? {
        let snapshot = row.get::<_, String>(1)?;
        summaries.push(StoredMatchSummary {
            id: row.get(0)?,
            created_at: row.get(2)?,
            updated_at: row.get(3)?,
            player_deck_name: row.get(4)?,
            frame_count: row.get::<_, i64>(5)? as usize,
            state: MatchState::from_snapshot_json(&snapshot)?,
        });
    }
    Ok(summaries)
}
