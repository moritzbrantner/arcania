use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::deck_library::{self, DeckLibraryError, DeckRecipeSnapshot};
use crate::identity;
use crate::match_session::{
    Card, HeroType, MatchActionRequest, MatchProgressionLoadout, MatchState, RecordedReplayFrame,
    ReplayEvent, Side, Team,
};
use crate::progression;

mod db_values;
mod event_store;
mod ids;
mod matches;
mod replays;
mod rows;
mod schema;
mod shared_matches;
use db_values::*;
use ids::*;

pub const MATCH_DATABASE_PATH_ENV: &str = "ARCANIA_DB_PATH";

type ReadySharedLoadouts = (
    HeroType,
    HeroType,
    DeckRecipeSnapshot,
    DeckRecipeSnapshot,
    MatchProgressionLoadout,
    MatchProgressionLoadout,
);

#[derive(Clone, Debug)]
pub struct StoredMatch {
    pub id: String,
    pub state: MatchState,
}

#[derive(Clone, Debug)]
pub struct StoredMatchSummary {
    pub id: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub frame_count: usize,
    pub player_deck_name: Option<String>,
    pub state: MatchState,
}

#[derive(Clone, Debug)]
pub struct StoredReplayFrame {
    pub frame_index: u32,
    pub action_index: Option<u32>,
    pub event: ReplayEvent,
    pub state: MatchState,
}

#[derive(Clone, Debug)]
pub struct StoredReplay {
    pub summary: StoredMatchSummary,
    pub frames: Vec<StoredReplayFrame>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedMatchStatus {
    Setup,
    Active,
    Completed,
    Forfeited,
}

#[derive(Clone, Debug)]
#[allow(
    dead_code,
    reason = "loaded for future shared setup visibility and frozen recipe audits"
)]
pub struct StoredSharedSeat {
    pub side: Side,
    pub seat_token: String,
    pub participant_user_id: Option<i64>,
    pub hero_type: Option<HeroType>,
    pub deck_recipe_name: Option<String>,
    pub deck_recipe_snapshot: Option<DeckRecipeSnapshot>,
    pub progression_loadout: MatchProgressionLoadout,
    pub joined_at: Option<i64>,
    pub disconnected_at: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct StoredSharedMatch {
    pub match_id: String,
    pub format: SharedMatchFormat,
    pub status: SharedMatchStatus,
    pub viewer_seat: StoredSharedSeat,
    pub opposing_seat: StoredSharedSeat,
    pub seats: Vec<StoredSharedSeat>,
    pub state: Option<MatchState>,
}

#[derive(Clone, Debug)]
pub struct CreatedSharedMatch {
    pub match_id: String,
    pub format: SharedMatchFormat,
    pub player_token: String,
    pub opponent_token: String,
    pub player_two_token: Option<String>,
    pub opponent_two_token: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedMatchFormat {
    Duel,
    TwoVTwo,
}

pub struct SqliteMatchStore {
    connection: Connection,
}

#[derive(Debug)]
pub enum MatchStoreError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Snapshot(serde_json::Error),
    Identity(identity::IdentityError),
    Deck(DeckLibraryError),
    Progression(progression::ProgressionError),
    EventSourcing(arcania_core::event_sourcing::EventSourcingError),
    EventStore(String),
    VersionConflict {
        expected: arcania_core::event_sourcing::AggregateVersion,
        actual: arcania_core::event_sourcing::AggregateVersion,
    },
}

impl fmt::Display for MatchStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not prepare match database: {error}"),
            Self::Sqlite(error) => write!(f, "could not access match database: {error}"),
            Self::Snapshot(error) => write!(f, "could not read persisted match snapshot: {error}"),
            Self::Identity(error) => write!(f, "{error}"),
            Self::Deck(error) => write!(f, "{error}"),
            Self::Progression(error) => write!(f, "{error}"),
            Self::EventSourcing(error) => write!(f, "event-sourced match failed: {error}"),
            Self::EventStore(reason) => write!(f, "event store is inconsistent: {reason}"),
            Self::VersionConflict { expected, actual } => write!(
                f,
                "event store version conflict: expected {}, actual {}",
                expected.0, actual.0
            ),
        }
    }
}

impl Error for MatchStoreError {}

impl From<std::io::Error> for MatchStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for MatchStoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for MatchStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::Snapshot(error)
    }
}

impl From<identity::IdentityError> for MatchStoreError {
    fn from(error: identity::IdentityError) -> Self {
        Self::Identity(error)
    }
}

impl From<DeckLibraryError> for MatchStoreError {
    fn from(error: DeckLibraryError) -> Self {
        Self::Deck(error)
    }
}

impl From<progression::ProgressionError> for MatchStoreError {
    fn from(error: progression::ProgressionError) -> Self {
        Self::Progression(error)
    }
}

impl From<arcania_core::event_sourcing::EventSourcingError> for MatchStoreError {
    fn from(error: arcania_core::event_sourcing::EventSourcingError) -> Self {
        Self::EventSourcing(error)
    }
}

impl SqliteMatchStore {
    pub fn from_environment() -> Result<Self, MatchStoreError> {
        Self::new(database_path_from_environment())
    }

    pub fn new(path: impl AsRef<Path>) -> Result<Self, MatchStoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        schema::prepare_connection(&connection)?;

        Ok(Self { connection })
    }

    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }

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
            let initial_frame = state.initial_replay_frame();
            let event_json = serde_json::to_string(&initial_frame.event)?;

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
                insert_replay_frame(&transaction, &id, 0, &initial_frame, &event_json)?;
                event_store::insert_genesis(&transaction, &id, &state, 0)?;
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
        let initial_frame = state.initial_replay_frame();
        let event_json = serde_json::to_string(&initial_frame.event)?;
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
        insert_replay_frame(&transaction, &id, 0, &initial_frame, &event_json)?;
        event_store::insert_genesis(&transaction, &id, &state, 0)?;
        transaction.commit()?;
        Ok(StoredMatch { id, state })
    }

    pub fn create_shared_match(
        &mut self,
        creator_user_id: Option<i64>,
        format: SharedMatchFormat,
    ) -> Result<CreatedSharedMatch, MatchStoreError> {
        for attempt in 0..8 {
            let match_id = readable_match_id(attempt);
            let player_token = random_seat_token();
            let opponent_token = random_seat_token();
            let player_two_token = (format == SharedMatchFormat::TwoVTwo).then(random_seat_token);
            let opponent_two_token = (format == SharedMatchFormat::TwoVTwo).then(random_seat_token);
            let transaction = self.connection.transaction()?;
            let inserted = transaction.execute(
                "
                INSERT OR IGNORE INTO shared_matches (
                    match_id,
                    status,
                    creator_user_id,
                    format,
                    created_at,
                    updated_at
                )
                VALUES (?1, 'setup', ?2, ?3, unixepoch(), unixepoch())
                ",
                params![match_id, creator_user_id, format.as_str()],
            )?;

            if inserted == 1 {
                insert_shared_seat(
                    &transaction,
                    &match_id,
                    Side::Player,
                    &player_token,
                    None,
                    false,
                )?;
                insert_shared_seat(
                    &transaction,
                    &match_id,
                    Side::Opponent,
                    &opponent_token,
                    None,
                    false,
                )?;
                if let Some(player_two_token) = &player_two_token {
                    insert_shared_seat(
                        &transaction,
                        &match_id,
                        Side::PlayerTwo,
                        player_two_token,
                        None,
                        false,
                    )?;
                }
                if let Some(opponent_two_token) = &opponent_two_token {
                    insert_shared_seat(
                        &transaction,
                        &match_id,
                        Side::OpponentTwo,
                        opponent_two_token,
                        None,
                        false,
                    )?;
                }
                transaction.commit()?;
                return Ok(CreatedSharedMatch {
                    match_id,
                    format,
                    player_token,
                    opponent_token,
                    player_two_token,
                    opponent_two_token,
                });
            }

            transaction.commit()?;
        }

        Err(MatchStoreError::Sqlite(
            rusqlite::Error::ExecuteReturnedResults,
        ))
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
            let initial_frame = state.initial_replay_frame();
            let event_json = serde_json::to_string(&initial_frame.event)?;

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
                insert_replay_frame(&transaction, &id, 0, &initial_frame, &event_json)?;
                event_store::insert_genesis(&transaction, &id, &state, 0)?;
                transaction.commit()?;
                return Ok(StoredMatch { id, state });
            }

            transaction.commit()?;
        }

        Err(MatchStoreError::Sqlite(
            rusqlite::Error::ExecuteReturnedResults,
        ))
    }

    pub fn load_shared_match_for_seat(
        &self,
        id: &str,
        seat_token: &str,
    ) -> Result<Option<StoredSharedMatch>, MatchStoreError> {
        let row: Option<(String, String, String)> = self
            .connection
            .query_row(
                "
                SELECT match_id, status, format
                FROM shared_matches
                WHERE match_id = ?1
                ",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let Some((match_id, status, format)) = row else {
            return Ok(None);
        };

        let seats = self.load_shared_seats(&match_id)?;
        let Some(viewer_seat) = seats
            .iter()
            .find(|seat| seat.seat_token == seat_token)
            .cloned()
        else {
            return Ok(None);
        };
        let opposing_seat = seats
            .iter()
            .find(|seat| seat.side.team() != viewer_seat.side.team())
            .cloned()
            .expect("shared match should have an opposing seat");
        let state = self.load_match(&match_id)?.map(|stored| stored.state);

        Ok(Some(StoredSharedMatch {
            match_id,
            format: SharedMatchFormat::from_db(&format),
            status: SharedMatchStatus::from_db(&status),
            viewer_seat,
            opposing_seat,
            seats,
            state,
        }))
    }

    pub fn join_shared_match(
        &mut self,
        id: &str,
        seat_token: &str,
        hero_type: HeroType,
        deck_recipe: DeckRecipeSnapshot,
        progression_loadout: MatchProgressionLoadout,
        participant_user_id: Option<i64>,
    ) -> Result<Option<StoredSharedMatch>, MatchStoreError> {
        let Some(shared) = self.load_shared_match_for_seat(id, seat_token)? else {
            return Ok(None);
        };
        if shared.status != SharedMatchStatus::Setup {
            return Ok(Some(shared));
        }

        let transaction = self.connection.transaction()?;
        transaction.execute(
            "
            UPDATE match_seats
            SET hero_type = ?3,
                deck_recipe_name = ?4,
                deck_recipe_snapshot_json = ?5,
                participant_user_id = COALESCE(participant_user_id, ?6),
                progression_loadout_json = ?7,
                joined_at = COALESCE(joined_at, unixepoch()),
                last_seen_at = unixepoch(),
                disconnected_at = NULL
            WHERE match_id = ?1 AND seat_token = ?2
            ",
            params![
                id,
                seat_token,
                hero_type.to_db(),
                deck_recipe.name,
                serde_json::to_string(&deck_recipe)?,
                participant_user_id,
                serde_json::to_string(&progression_loadout)?
            ],
        )?;

        if shared.format == SharedMatchFormat::TwoVTwo {
            if let Some(loadouts) = ready_shared_two_v_two_loadouts(&transaction, id)? {
                let player_deck =
                    deck_library::deck_from_snapshot(Side::Player, &loadouts.player.1)?;
                let opponent_deck =
                    deck_library::deck_from_snapshot(Side::Opponent, &loadouts.opponent.1)?;
                let player_two_deck =
                    deck_library::deck_from_snapshot(Side::PlayerTwo, &loadouts.player_two.1)?;
                let opponent_two_deck =
                    deck_library::deck_from_snapshot(Side::OpponentTwo, &loadouts.opponent_two.1)?;
                let state = MatchState::new_shared_two_v_two_with_progression_loadouts(
                    loadouts.player.0,
                    loadouts.opponent.0,
                    loadouts.player_two.0,
                    loadouts.opponent_two.0,
                    player_deck,
                    opponent_deck,
                    player_two_deck,
                    opponent_two_deck,
                    loadouts.player.2,
                    loadouts.opponent.2,
                    loadouts.player_two.2,
                    loadouts.opponent_two.2,
                );
                let snapshot = state.to_snapshot_json()?;
                let initial_frame = state.initial_replay_frame();
                let event_json = serde_json::to_string(&initial_frame.event)?;
                transaction.execute(
                    "
                    INSERT INTO matches (
                        id,
                        snapshot_json,
                        initial_snapshot_json,
                        mode,
                        created_at,
                        updated_at
                    )
                    VALUES (?1, ?2, ?2, 'shared', unixepoch(), unixepoch())
                    ",
                    params![id, snapshot],
                )?;
                insert_replay_frame(&transaction, id, 0, &initial_frame, &event_json)?;
                event_store::insert_genesis(&transaction, id, &state, 0)?;
                transaction.execute(
                    "
                    UPDATE shared_matches
                    SET status = 'active',
                        updated_at = unixepoch()
                    WHERE match_id = ?1
                    ",
                    params![id],
                )?;
            } else {
                transaction.execute(
                    "
                    UPDATE shared_matches
                    SET updated_at = unixepoch()
                    WHERE match_id = ?1
                    ",
                    params![id],
                )?;
            }
        } else if let Some((
            player_hero_type,
            opponent_hero_type,
            player_recipe,
            opponent_recipe,
            player_progression,
            opponent_progression,
        )) = ready_shared_loadouts(&transaction, id)?
        {
            let player_deck = deck_library::deck_from_snapshot(Side::Player, &player_recipe)?;
            let opponent_deck = deck_library::deck_from_snapshot(Side::Opponent, &opponent_recipe)?;
            let state = MatchState::new_shared_with_progression_loadouts(
                player_hero_type,
                opponent_hero_type,
                player_deck,
                opponent_deck,
                player_progression,
                opponent_progression,
            );
            let snapshot = state.to_snapshot_json()?;
            let initial_frame = state.initial_replay_frame();
            let event_json = serde_json::to_string(&initial_frame.event)?;
            transaction.execute(
                "
                INSERT INTO matches (
                    id,
                    snapshot_json,
                    initial_snapshot_json,
                    mode,
                    created_at,
                    updated_at
                )
                VALUES (?1, ?2, ?2, 'shared', unixepoch(), unixepoch())
                ",
                params![id, snapshot],
            )?;
            insert_replay_frame(&transaction, id, 0, &initial_frame, &event_json)?;
            event_store::insert_genesis(&transaction, id, &state, 0)?;
            transaction.execute(
                "
                UPDATE shared_matches
                SET status = 'active',
                    updated_at = unixepoch()
                WHERE match_id = ?1
                ",
                params![id],
            )?;
        } else {
            transaction.execute(
                "
                UPDATE shared_matches
                SET updated_at = unixepoch()
                WHERE match_id = ?1
                ",
                params![id],
            )?;
        }

        transaction.commit()?;

        self.load_shared_match_for_seat(id, seat_token)
    }

    pub fn mark_shared_seat_seen(
        &mut self,
        id: &str,
        seat_token: &str,
    ) -> Result<(), MatchStoreError> {
        self.connection.execute(
            "
            UPDATE match_seats
            SET last_seen_at = unixepoch(),
                disconnected_at = NULL
            WHERE match_id = ?1 AND seat_token = ?2
            ",
            params![id, seat_token],
        )?;
        Ok(())
    }

    pub fn mark_shared_seat_disconnected(
        &mut self,
        id: &str,
        seat_token: &str,
    ) -> Result<(), MatchStoreError> {
        self.connection.execute(
            "
            UPDATE match_seats
            SET disconnected_at = COALESCE(disconnected_at, unixepoch())
            WHERE match_id = ?1 AND seat_token = ?2
            ",
            params![id, seat_token],
        )?;
        Ok(())
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

    pub fn completed_shared_participant_side(
        &self,
        match_id: &str,
        user_id: i64,
    ) -> Result<Option<Side>, MatchStoreError> {
        let side = self
            .connection
            .query_row(
                "
                SELECT match_seats.side
                FROM match_seats
                JOIN shared_matches ON shared_matches.match_id = match_seats.match_id
                WHERE match_seats.match_id = ?1
                    AND match_seats.participant_user_id = ?2
                    AND shared_matches.status IN ('completed', 'forfeited')
                ",
                params![match_id, user_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(side.and_then(|side| side_from_db(&side)))
    }

    pub fn next_action_index(&self, id: &str) -> Result<u32, MatchStoreError> {
        let next = self.connection.query_row(
            "
            SELECT COALESCE(MAX(action_index) + 1, 0)
            FROM match_actions
            WHERE match_id = ?1
            ",
            params![id],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(next as u32)
    }

    pub fn save_action_and_replay_frames(
        &mut self,
        id: &str,
        action_index: u32,
        action: &MatchActionRequest,
        state: &MatchState,
        frames: &[RecordedReplayFrame],
    ) -> Result<(), MatchStoreError> {
        let request_json = serde_json::to_string(action)?;
        self.save_action_json_and_replay_frames(
            id,
            action_index,
            &request_json,
            state,
            frames,
            None,
        )
    }

    #[allow(dead_code, reason = "kept for tests and non-command store callers")]
    pub fn save_custom_action_and_replay_frames(
        &mut self,
        id: &str,
        action_index: u32,
        request_json: &str,
        state: &MatchState,
        frames: &[RecordedReplayFrame],
    ) -> Result<(), MatchStoreError> {
        self.save_action_json_and_replay_frames(id, action_index, request_json, state, frames, None)
    }

    pub fn save_forfeit_action_and_replay_frames(
        &mut self,
        id: &str,
        action_index: u32,
        request_json: &str,
        state: &MatchState,
        frames: &[RecordedReplayFrame],
        winner: Side,
    ) -> Result<(), MatchStoreError> {
        self.save_action_json_and_replay_frames(
            id,
            action_index,
            request_json,
            state,
            frames,
            Some(winner),
        )
    }

    fn save_action_json_and_replay_frames(
        &mut self,
        id: &str,
        action_index: u32,
        request_json: &str,
        state: &MatchState,
        frames: &[RecordedReplayFrame],
        forfeit_winner: Option<Side>,
    ) -> Result<(), MatchStoreError> {
        let snapshot = state.to_snapshot_json()?;
        let completed_at = if state.winner.is_some() {
            "completed_at = COALESCE(completed_at, unixepoch()),"
        } else {
            ""
        };
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO match_actions (match_id, action_index, request_json, accepted_at)
            VALUES (?1, ?2, ?3, unixepoch())
            ",
            params![id, i64::from(action_index), request_json],
        )?;

        let next_frame_index = next_frame_index(&transaction, id)?;
        for (offset, frame) in frames.iter().enumerate() {
            let event_json = serde_json::to_string(&frame.event)?;
            insert_replay_frame(
                &transaction,
                id,
                next_frame_index + offset as u32,
                frame,
                &event_json,
            )?;
        }

        transaction.execute(
            &format!(
                "
                UPDATE matches
                SET snapshot_json = ?2,
                    {completed_at}
                    updated_at = unixepoch()
                WHERE id = ?1
                "
            ),
            params![id, snapshot],
        )?;
        if state.winner.is_some() {
            if let Some(winner) = forfeit_winner {
                transaction.execute(
                    "
                    UPDATE shared_matches
                    SET status = 'forfeited',
                        forfeit_winner = ?2,
                        updated_at = unixepoch()
                    WHERE match_id = ?1
                    ",
                    params![id, winner.to_db()],
                )?;
            } else {
                transaction.execute(
                    "
                    UPDATE shared_matches
                    SET status = CASE status
                            WHEN 'forfeited' THEN 'forfeited'
                            ELSE 'completed'
                        END,
                        updated_at = unixepoch()
                    WHERE match_id = ?1
                    ",
                    params![id],
                )?;
            }
            progression::award_completed_match_in_transaction(&transaction, id)?;
        }
        transaction.commit()?;
        Ok(())
    }

    #[allow(dead_code, reason = "kept for tests and legacy direct store callers")]
    pub fn mark_shared_match_forfeited(
        &mut self,
        id: &str,
        winner: Side,
    ) -> Result<(), MatchStoreError> {
        self.connection.execute(
            "
            UPDATE shared_matches
            SET status = 'forfeited',
                forfeit_winner = ?2,
                updated_at = unixepoch()
            WHERE match_id = ?1
            ",
            params![id, winner.to_db()],
        )?;
        Ok(())
    }

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
        let mut rows = statement.query(params![user_id])?;

        let mut summaries = Vec::new();
        while let Some(row) = rows.next()? {
            let id = row.get::<_, String>(0)?;
            let snapshot = row.get::<_, String>(1)?;
            let created_at = row.get::<_, i64>(2)?;
            let updated_at = row.get::<_, i64>(3)?;
            let player_deck_name = row.get::<_, Option<String>>(4)?;
            let frame_count = row.get::<_, i64>(5)?;
            let state = MatchState::from_snapshot_json(&snapshot)?;
            summaries.push(StoredMatchSummary {
                id,
                created_at,
                updated_at,
                frame_count: frame_count as usize,
                player_deck_name,
                state,
            });
        }

        Ok(summaries)
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
        let mut rows = statement.query(params![user_id])?;

        let mut summaries = Vec::new();
        while let Some(row) = rows.next()? {
            let id = row.get::<_, String>(0)?;
            let snapshot = row.get::<_, String>(1)?;
            let created_at = row.get::<_, i64>(2)?;
            let updated_at = row.get::<_, i64>(3)?;
            let player_deck_name = row.get::<_, Option<String>>(4)?;
            let frame_count = row.get::<_, i64>(5)?;
            let state = MatchState::from_snapshot_json(&snapshot)?;
            summaries.push(StoredMatchSummary {
                id,
                created_at,
                updated_at,
                frame_count: frame_count as usize,
                player_deck_name,
                state,
            });
        }

        Ok(summaries)
    }

    pub fn viewer_context_for_user(
        &self,
        match_id: &str,
        user_id: i64,
    ) -> Result<Option<(Team, Option<String>)>, MatchStoreError> {
        let seat: Option<(String, Option<String>)> = self.connection.query_row(
            "SELECT side, deck_recipe_name FROM match_seats WHERE match_id = ?1 AND participant_user_id = ?2",
            params![match_id, user_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        Ok(seat
            .and_then(|(side, deck_name)| side_from_db(&side).map(|side| (side.team(), deck_name))))
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

    fn load_shared_seats(&self, match_id: &str) -> Result<Vec<StoredSharedSeat>, MatchStoreError> {
        let mut statement = self.connection.prepare(
            "
            SELECT
                side,
                seat_token,
                participant_user_id,
                hero_type,
                deck_recipe_name,
                deck_recipe_snapshot_json,
                progression_loadout_json,
                joined_at,
                last_seen_at,
                disconnected_at
            FROM match_seats
            WHERE match_id = ?1
            ",
        )?;
        let rows = statement.query_map(params![match_id], |row| {
            let side = row.get::<_, String>(0)?;
            let hero_type = row.get::<_, Option<String>>(3)?;
            let snapshot_json = row.get::<_, Option<String>>(5)?;
            let progression_loadout_json = row.get::<_, Option<String>>(6)?;
            let deck_recipe_snapshot = snapshot_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            let progression_loadout = progression_loadout_json
                .as_deref()
                .map(serde_json::from_str)
                .transpose()
                .map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?
                .unwrap_or_default();
            Ok(StoredSharedSeat {
                side: side_from_db(&side).expect("stored side should be valid"),
                seat_token: row.get(1)?,
                participant_user_id: row.get(2)?,
                hero_type: hero_type.as_deref().and_then(hero_type_from_db),
                deck_recipe_name: row.get(4)?,
                deck_recipe_snapshot,
                progression_loadout,
                joined_at: row.get(7)?,
                disconnected_at: row.get(9)?,
            })
        })?;

        let mut seats = Vec::new();
        for row in rows {
            seats.push(row?);
        }
        Ok(seats)
    }
}

fn insert_shared_seat(
    transaction: &Transaction<'_>,
    match_id: &str,
    side: Side,
    seat_token: &str,
    hero_type: Option<HeroType>,
    joined: bool,
) -> Result<(), MatchStoreError> {
    let joined_expr = if joined { "unixepoch()" } else { "NULL" };
    transaction.execute(
        &format!(
            "
            INSERT INTO match_seats (
                match_id,
                side,
                seat_token,
                hero_type,
                joined_at,
                last_seen_at
            )
            VALUES (?1, ?2, ?3, ?4, {joined_expr}, {joined_expr})
            "
        ),
        params![
            match_id,
            side.to_db(),
            seat_token,
            hero_type.map(HeroType::to_db)
        ],
    )?;
    Ok(())
}

fn ready_shared_loadouts(
    transaction: &Transaction<'_>,
    match_id: &str,
) -> Result<Option<ReadySharedLoadouts>, MatchStoreError> {
    let mut statement = transaction.prepare(
        "
        SELECT side, hero_type, deck_recipe_snapshot_json, progression_loadout_json, joined_at
        FROM match_seats
        WHERE match_id = ?1
        ",
    )?;
    let rows = statement.query_map(params![match_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<i64>>(4)?,
        ))
    })?;

    let mut player_hero_type = None;
    let mut opponent_hero_type = None;
    let mut player_deck = None;
    let mut opponent_deck = None;
    let mut player_progression = None;
    let mut opponent_progression = None;
    for row in rows {
        let (side, hero_type, deck_snapshot_json, progression_loadout_json, joined_at) = row?;
        if joined_at.is_none() {
            continue;
        }
        let Some(hero_type) = hero_type.as_deref().and_then(hero_type_from_db) else {
            continue;
        };
        let Some(deck_snapshot_json) = deck_snapshot_json else {
            continue;
        };
        let deck_snapshot = serde_json::from_str::<DeckRecipeSnapshot>(&deck_snapshot_json)?;
        let progression_loadout = progression_loadout_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default();
        match side_from_db(&side) {
            Some(Side::Player) => {
                player_hero_type = Some(hero_type);
                player_deck = Some(deck_snapshot);
                player_progression = Some(progression_loadout);
            }
            Some(Side::Opponent) => {
                opponent_hero_type = Some(hero_type);
                opponent_deck = Some(deck_snapshot);
                opponent_progression = Some(progression_loadout);
            }
            Some(Side::PlayerTwo) | Some(Side::OpponentTwo) => {}
            None => {}
        }
    }

    Ok(
        match (
            player_hero_type,
            opponent_hero_type,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
        ) {
            (
                Some(player_hero_type),
                Some(opponent_hero_type),
                Some(player_deck),
                Some(opponent_deck),
                Some(player_progression),
                Some(opponent_progression),
            ) => Some((
                player_hero_type,
                opponent_hero_type,
                player_deck,
                opponent_deck,
                player_progression,
                opponent_progression,
            )),
            _ => None,
        },
    )
}

type ReadySeatLoadout = (HeroType, DeckRecipeSnapshot, MatchProgressionLoadout);

struct ReadyTwoVTwoLoadouts {
    player: ReadySeatLoadout,
    opponent: ReadySeatLoadout,
    player_two: ReadySeatLoadout,
    opponent_two: ReadySeatLoadout,
}

fn ready_shared_two_v_two_loadouts(
    transaction: &Transaction<'_>,
    match_id: &str,
) -> Result<Option<ReadyTwoVTwoLoadouts>, MatchStoreError> {
    let mut statement = transaction.prepare(
        "
        SELECT side, hero_type, deck_recipe_snapshot_json, progression_loadout_json, joined_at
        FROM match_seats
        WHERE match_id = ?1
        ",
    )?;
    let rows = statement.query_map(params![match_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, Option<i64>>(4)?,
        ))
    })?;

    let mut player = None;
    let mut opponent = None;
    let mut player_two = None;
    let mut opponent_two = None;

    for row in rows {
        let (side, hero_type, deck_snapshot_json, progression_loadout_json, joined_at) = row?;
        if joined_at.is_none() {
            continue;
        }
        let Some(hero_type) = hero_type.as_deref().and_then(hero_type_from_db) else {
            continue;
        };
        let Some(deck_snapshot_json) = deck_snapshot_json else {
            continue;
        };
        let deck_snapshot = serde_json::from_str::<DeckRecipeSnapshot>(&deck_snapshot_json)?;
        let progression_loadout = progression_loadout_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default();
        let loadout = (hero_type, deck_snapshot, progression_loadout);
        match side_from_db(&side) {
            Some(Side::Player) => player = Some(loadout),
            Some(Side::Opponent) => opponent = Some(loadout),
            Some(Side::PlayerTwo) => player_two = Some(loadout),
            Some(Side::OpponentTwo) => opponent_two = Some(loadout),
            None => {}
        }
    }

    Ok(match (player, opponent, player_two, opponent_two) {
        (Some(player), Some(opponent), Some(player_two), Some(opponent_two)) => {
            Some(ReadyTwoVTwoLoadouts {
                player,
                opponent,
                player_two,
                opponent_two,
            })
        }
        _ => None,
    })
}

fn next_frame_index(transaction: &Transaction<'_>, id: &str) -> Result<u32, MatchStoreError> {
    let next = transaction.query_row(
        "
        SELECT COALESCE(MAX(frame_index) + 1, 0)
        FROM match_replay_frames
        WHERE match_id = ?1
        ",
        params![id],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(next as u32)
}

fn insert_replay_frame(
    transaction: &Transaction<'_>,
    id: &str,
    frame_index: u32,
    frame: &RecordedReplayFrame,
    event_json: &str,
) -> Result<(), MatchStoreError> {
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

pub fn database_path_from_environment() -> PathBuf {
    database_path_from_override(env::var_os(MATCH_DATABASE_PATH_ENV))
}

pub fn database_path_from_override(value: Option<OsString>) -> PathBuf {
    value
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/arcania.sqlite3"))
}

#[cfg(test)]
mod tests;
