use rusqlite::Connection;

use crate::{card_workshop, deck_library, identity, progression};

use super::MatchStoreError;

pub(super) fn prepare_connection(connection: &Connection) -> Result<(), MatchStoreError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS matches (
            id TEXT PRIMARY KEY NOT NULL,
            snapshot_json TEXT NOT NULL,
            initial_snapshot_json TEXT,
            completed_at INTEGER,
            mode TEXT NOT NULL DEFAULT 'solo',
            owner_user_id INTEGER,
            player_deck_name TEXT,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        );
        CREATE TABLE IF NOT EXISTS match_actions (
            match_id TEXT NOT NULL,
            action_index INTEGER NOT NULL,
            request_json TEXT NOT NULL,
            accepted_at INTEGER NOT NULL,
            PRIMARY KEY (match_id, action_index)
        );
        CREATE TABLE IF NOT EXISTS match_replay_frames (
            match_id TEXT NOT NULL,
            frame_index INTEGER NOT NULL,
            action_index INTEGER,
            event_json TEXT NOT NULL,
            snapshot_json TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (match_id, frame_index)
        );
        CREATE TABLE IF NOT EXISTS match_events (
    match_id TEXT NOT NULL,
    aggregate_version INTEGER NOT NULL,
    event_schema_version INTEGER NOT NULL,
    ruleset_version TEXT NOT NULL,
    command_id TEXT,
    action_index INTEGER,
    event_json TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (match_id, aggregate_version)
);
CREATE UNIQUE INDEX IF NOT EXISTS match_events_command_id
    ON match_events(match_id, command_id)
    WHERE command_id IS NOT NULL;
CREATE TABLE IF NOT EXISTS match_event_snapshots (
    match_id TEXT PRIMARY KEY NOT NULL,
    aggregate_version INTEGER NOT NULL,
    snapshot_schema_version INTEGER NOT NULL,
    event_schema_version INTEGER NOT NULL,
    ruleset_version TEXT NOT NULL,
    snapshot_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
        CREATE TABLE IF NOT EXISTS shared_matches (
            match_id TEXT PRIMARY KEY NOT NULL,
            status TEXT NOT NULL,
            creator_user_id INTEGER,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            forfeit_winner TEXT,
            format TEXT NOT NULL DEFAULT 'duel'
        );
        CREATE TABLE IF NOT EXISTS match_seats (
            match_id TEXT NOT NULL,
            side TEXT NOT NULL,
            seat_token TEXT NOT NULL UNIQUE,
            hero_type TEXT,
            joined_at INTEGER,
            last_seen_at INTEGER,
            disconnected_at INTEGER,
            deck_recipe_name TEXT,
            deck_recipe_snapshot_json TEXT,
            progression_loadout_json TEXT,
            PRIMARY KEY (match_id, side)
        );
        ",
    )?;
    identity::migrate(connection)?;
    card_workshop::migrate(connection)?;
    deck_library::migrate(connection)?;
    progression::migrate(connection)?;
    #[cfg(debug_assertions)]
    {
        let experienced_user_id = identity::seed_experienced_local_account(connection)?;
        progression::seed_experienced_local_mastery(connection, experienced_user_id)?;
    }
    add_column_if_missing(connection, "matches", "initial_snapshot_json", "TEXT")?;
    add_column_if_missing(connection, "matches", "completed_at", "INTEGER")?;
    add_column_if_missing(
        connection,
        "matches",
        "mode",
        "TEXT NOT NULL DEFAULT 'solo'",
    )?;
    add_column_if_missing(connection, "matches", "owner_user_id", "INTEGER")?;
    add_column_if_missing(connection, "matches", "player_deck_name", "TEXT")?;
    add_column_if_missing(connection, "shared_matches", "creator_user_id", "INTEGER")?;
    add_column_if_missing(
        connection,
        "shared_matches",
        "format",
        "TEXT NOT NULL DEFAULT 'duel'",
    )?;
    let had_legacy_seat_hero = column_exists(connection, "match_seats", "wizard_type")?;
    add_column_if_missing(connection, "match_seats", "hero_type", "TEXT")?;
    add_column_if_missing(connection, "match_seats", "participant_user_id", "INTEGER")?;
    add_column_if_missing(connection, "match_seats", "deck_recipe_name", "TEXT")?;
    add_column_if_missing(
        connection,
        "match_seats",
        "deck_recipe_snapshot_json",
        "TEXT",
    )?;
    add_column_if_missing(
        connection,
        "match_seats",
        "progression_loadout_json",
        "TEXT",
    )?;
    discard_legacy_hero_matches(connection, had_legacy_seat_hero)?;
    drop_column_if_exists(connection, "match_seats", "wizard_type")?;

    Ok(())
}

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), MatchStoreError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }

    connection.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
        [],
    )?;
    Ok(())
}

fn column_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, MatchStoreError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn drop_column_if_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<(), MatchStoreError> {
    if column_exists(connection, table, column)? {
        let _ = connection.execute(&format!("ALTER TABLE {table} DROP COLUMN {column}"), []);
    }
    Ok(())
}

fn discard_legacy_hero_matches(
    connection: &Connection,
    had_legacy_seat_hero: bool,
) -> Result<(), MatchStoreError> {
    let has_legacy_snapshots = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM matches WHERE snapshot_json LIKE '%\"wizard\"%' OR snapshot_json LIKE '%wizardType%' LIMIT 1)",
        [],
        |row| row.get::<_, i64>(0),
    )? == 1;

    if had_legacy_seat_hero || has_legacy_snapshots {
        connection.execute_batch(
            "
                        DELETE FROM match_event_snapshots;
    DELETE FROM match_events;
    DELETE FROM match_replay_frames;
    DELETE FROM match_actions;
    DELETE FROM match_seats;
            DELETE FROM shared_matches;
            DELETE FROM matches;
            ",
        )?;
    }

    Ok(())
}
