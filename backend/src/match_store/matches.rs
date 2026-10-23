use rusqlite::{OptionalExtension, Transaction, params};

use crate::deck_library;
use crate::match_session::{Card, HeroType, MatchProgressionLoadout, MatchState, Side};

use super::ids::*;
use super::{MatchStoreError, SqliteMatchStore, StoredMatch, event_store, replays};

impl SqliteMatchStore {
    #[allow(
        dead_code,
        reason = "kept as the default store API for tests and callers"
    )]
    pub fn create_match_for_user(
        &mut self,
        player_hero_type: HeroType,
        owner_user_id: Option<i64>,
    ) -> Result<StoredMatch, MatchStoreError> {
        let starter = deck_library::starter_deck_snapshot();
        let player_deck = deck_library::deck_from_snapshot(Side::Player, &starter)?;
        let opponent_deck = deck_library::deck_from_snapshot(Side::Opponent, &starter)?;
        self.create_match_for_user_with_decks(
            player_hero_type,
            HeroType::Runekeeper,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
            starter.name,
            owner_user_id,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "match creation stores mirrored player/opponent decks and progression loadouts"
    )]
    pub fn create_match_for_user_with_decks(
        &mut self,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
        player_deck_name: String,
        owner_user_id: Option<i64>,
    ) -> Result<StoredMatch, MatchStoreError> {
        for attempt in 0..8 {
            let id = readable_match_id(attempt);
            let state = MatchState::new_with_progression_loadouts(
                player_hero_type,
                opponent_hero_type,
                player_deck.clone(),
                opponent_deck.clone(),
                player_progression.clone(),
                opponent_progression.clone(),
            );
            let snapshot = state.to_snapshot_json()?;

            let transaction = self.connection.transaction()?;
            let inserted = transaction.execute(
                "
                INSERT OR IGNORE INTO matches (
                    id,
                    snapshot_json,
                    initial_snapshot_json,
                    mode,
                    owner_user_id,
                    player_deck_name,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?2, 'solo', ?3, ?4, unixepoch(), unixepoch())
                ",
                params![id, snapshot, owner_user_id, player_deck_name],
            )?;

            if inserted == 1 {
                record_match_start(&transaction, &id, &state)?;
                transaction.commit()?;
                return Ok(StoredMatch { id, state });
            }
            transaction.commit()?;
        }

        let id = readable_match_id(99);
        let state = MatchState::new_with_progression_loadouts(
            player_hero_type,
            opponent_hero_type,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
        );
        let snapshot = state.to_snapshot_json()?;
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO matches (
                id,
                snapshot_json,
                initial_snapshot_json,
                mode,
                owner_user_id,
                player_deck_name,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?2, 'solo', ?3, ?4, unixepoch(), unixepoch())
            ON CONFLICT(id) DO UPDATE SET
                snapshot_json = excluded.snapshot_json,
                initial_snapshot_json = excluded.initial_snapshot_json,
                mode = excluded.mode,
                owner_user_id = excluded.owner_user_id,
                player_deck_name = excluded.player_deck_name,
                updated_at = unixepoch()
            ",
            params![id, snapshot, owner_user_id, player_deck_name],
        )?;
        record_match_start(&transaction, &id, &state)?;
        transaction.commit()?;
        Ok(StoredMatch { id, state })
    }

    #[cfg(any(test, debug_assertions))]
    pub fn create_scenario_match(
        &mut self,
        scenario_id: &str,
        state: MatchState,
    ) -> Result<StoredMatch, MatchStoreError> {
        for attempt in 0..8 {
            let id = scenario_match_id(scenario_id, attempt);
            let snapshot = state.to_snapshot_json()?;

            let transaction = self.connection.transaction()?;
            let inserted = transaction.execute(
                "
                INSERT OR IGNORE INTO matches (
                    id,
                    snapshot_json,
                    initial_snapshot_json,
                    mode,
                    owner_user_id,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?2, 'solo', NULL, unixepoch(), unixepoch())
                ",
                params![id, snapshot],
            )?;

            if inserted == 1 {
                record_match_start(&transaction, &id, &state)?;
                transaction.commit()?;
                return Ok(StoredMatch { id, state });
            }

            transaction.commit()?;
        }

        Err(MatchStoreError::Sqlite(
            rusqlite::Error::ExecuteReturnedResults,
        ))
    }

    pub fn load_match(&self, id: &str) -> Result<Option<StoredMatch>, MatchStoreError> {
        let snapshot: Option<String> = self
            .connection
            .query_row(
                "SELECT snapshot_json FROM matches WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;

        snapshot
            .map(|snapshot| {
                MatchState::from_snapshot_json(&snapshot).map(|state| StoredMatch {
                    id: id.to_string(),
                    state,
                })
            })
            .transpose()
            .map_err(MatchStoreError::from)
    }

    pub fn match_owner_user_id(&self, id: &str) -> Result<Option<Option<i64>>, MatchStoreError> {
        self.connection
            .query_row(
                "SELECT owner_user_id FROM matches WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(MatchStoreError::from)
    }
}

/// Records the start of a new match: replay frame 0 and the genesis event, in the caller's
/// transaction alongside the `matches` row.
pub(super) fn record_match_start(
    transaction: &Transaction<'_>,
    id: &str,
    state: &MatchState,
) -> Result<(), MatchStoreError> {
    replays::insert_initial_frame(transaction, id, state)?;
    event_store::insert_genesis(transaction, id, state, 0)
}
