//! SQLite adapter for match persistence.
//!
//! The interface is durable persistence operations: create and load matches, Shared match seat
//! records, replay reads, and the event-sourced command path (`load_event_sourced_match` /
//! `append_event_and_projection`). Action indexes, replay frame indexes, and projection updates
//! are internal write protocol owned by the submodules below.

use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::deck_library::{DeckLibraryError, DeckRecipeSnapshot};
use crate::identity;
use crate::match_session::{HeroType, MatchProgressionLoadout, MatchState, ReplayEvent, Side};
use crate::progression;

mod db_values;
mod event_store;
mod ids;
mod matches;
mod replays;
mod schema;
mod shared_matches;
pub(crate) use db_values::side_from_db;

pub const MATCH_DATABASE_PATH_ENV: &str = "RUNE_LANES_DB_PATH";

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
    EventSourcing(rune_lanes_core::event_sourcing::EventSourcingError),
    EventStore(String),
    VersionConflict {
        expected: rune_lanes_core::event_sourcing::AggregateVersion,
        actual: rune_lanes_core::event_sourcing::AggregateVersion,
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

impl From<rune_lanes_core::event_sourcing::EventSourcingError> for MatchStoreError {
    fn from(error: rune_lanes_core::event_sourcing::EventSourcingError) -> Self {
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

    /// Shared connection for the sibling SQLite modules (identity, decks, progression, workshop,
    /// preferences) that persist in the same database. Match persistence goes through the
    /// methods on this store instead.
    pub(crate) fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}

pub fn database_path_from_environment() -> PathBuf {
    database_path_from_override(env::var_os(MATCH_DATABASE_PATH_ENV))
}

pub fn database_path_from_override(value: Option<OsString>) -> PathBuf {
    value
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("data/rune-lanes.sqlite3"))
}

#[cfg(test)]
mod tests;
