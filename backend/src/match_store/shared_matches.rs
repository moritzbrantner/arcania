use rusqlite::{OptionalExtension, Transaction, params};

use crate::deck_library::{self, DeckRecipeSnapshot};
use crate::match_session::{HeroType, MatchProgressionLoadout, MatchState, Side, Team};

use super::db_values::*;
use super::ids::*;
use super::matches::record_match_start;
use super::{
    CreatedSharedMatch, MatchStoreError, SharedMatchFormat, SharedMatchStatus, SqliteMatchStore,
    StoredSharedMatch, StoredSharedSeat,
};

type ReadySharedLoadouts = (
    HeroType,
    HeroType,
    DeckRecipeSnapshot,
    DeckRecipeSnapshot,
    MatchProgressionLoadout,
    MatchProgressionLoadout,
);

impl SqliteMatchStore {
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
                record_match_start(&transaction, id, &state)?;
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
            record_match_start(&transaction, id, &state)?;
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
