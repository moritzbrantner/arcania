use rusqlite::{Connection, OptionalExtension, Transaction, params};

use crate::match_session::{HeroType, MatchState, Side};
use crate::match_store::side_from_db;

use super::catalog::{
    appearance_definitions_for_hero, hero_type_from_db, hero_type_to_db, rune_definitions,
    summary_for_xp,
};
use super::{
    HeroProgression, HeroProgressionDelta, MatchRewardSummary, MatchUnlockCallout,
    ProgressionDelta, ProgressionError, ProgressionModule, ProgressionSummary,
};

const COMPLETION_XP: i64 = 100;
const WIN_BONUS_XP: i64 = 50;

impl ProgressionModule<'_> {
    pub fn match_reward_summary(
        &self,
        user_id: i64,
        match_id: &str,
        side: Side,
    ) -> Result<Option<MatchRewardSummary>, ProgressionError> {
        let Some(target) = self.match_award(user_id, match_id)? else {
            return Ok(None);
        };

        let account_awards = self.awards_for_user(user_id)?;
        let current_account_xp = self.total_xp(user_id)?;
        let total_awarded_account_xp: i64 =
            account_awards.iter().map(|award| award.account_xp).sum();
        let baseline_account_xp = current_account_xp - total_awarded_account_xp;
        let account_xp_before =
            baseline_account_xp + prior_account_xp(&account_awards, &target.match_id);
        let account_before = summary_for_xp(account_xp_before);
        let account_after = summary_for_xp(account_xp_before + target.account_xp);

        let hero_awards: Vec<&MatchAwardRow> = account_awards
            .iter()
            .filter(|award| award.hero_type == target.hero_type)
            .collect();
        let current_hero_xp = self.hero_xp(user_id, target.hero_type)?;
        let total_awarded_hero_xp: i64 = hero_awards.iter().map(|award| award.hero_xp).sum();
        let baseline_hero_xp = current_hero_xp - total_awarded_hero_xp;
        let hero_xp_before = baseline_hero_xp + prior_hero_xp(&hero_awards, &target.match_id);
        let hero_before =
            self.hero_progression_from_xp(user_id, target.hero_type, hero_xp_before)?;
        let hero_after = self.hero_progression_from_xp(
            user_id,
            target.hero_type,
            hero_xp_before + target.hero_xp,
        )?;

        let unlocks = reward_unlocks(
            target.hero_type,
            &account_before,
            &account_after,
            &hero_before,
            &hero_after,
        );

        Ok(Some(MatchRewardSummary {
            side,
            hero_type: target.hero_type,
            won: target.won,
            account_xp_gained: target.account_xp,
            hero_xp_gained: target.hero_xp,
            win_bonus_xp: if target.won { WIN_BONUS_XP } else { 0 },
            account: ProgressionDelta {
                before: account_before,
                after: account_after,
            },
            hero: HeroProgressionDelta {
                hero_type: target.hero_type,
                before: hero_before,
                after: hero_after,
            },
            unlocks,
        }))
    }

    fn match_award(
        &self,
        user_id: i64,
        match_id: &str,
    ) -> Result<Option<MatchAwardRow>, ProgressionError> {
        self.connection
            .query_row(
                "
                SELECT match_id, account_xp, hero_type, hero_xp, won, awarded_at
                FROM match_xp_awards
                WHERE user_id = ?1 AND match_id = ?2
                ",
                params![user_id, match_id],
                match_award_from_row,
            )
            .optional()
            .map_err(ProgressionError::from)
    }

    fn awards_for_user(&self, user_id: i64) -> Result<Vec<MatchAwardRow>, ProgressionError> {
        let mut statement = self.connection.prepare(
            "
            SELECT match_id, account_xp, hero_type, hero_xp, won, awarded_at
            FROM match_xp_awards
            WHERE user_id = ?1
            ORDER BY awarded_at ASC, match_id ASC
            ",
        )?;
        let rows = statement.query_map(params![user_id], match_award_from_row)?;
        let mut awards = Vec::new();
        for row in rows {
            awards.push(row?);
        }
        Ok(awards)
    }
}

#[allow(dead_code, reason = "kept for tests and direct progression callers")]
pub fn award_completed_match(
    connection: &mut Connection,
    match_id: &str,
) -> Result<(), ProgressionError> {
    award_completed_match_on_connection(connection, match_id)
}

pub fn award_completed_match_in_transaction(
    transaction: &Transaction<'_>,
    match_id: &str,
) -> Result<(), ProgressionError> {
    award_completed_match_on_connection(transaction, match_id)
}

fn award_completed_match_on_connection(
    connection: &Connection,
    match_id: &str,
) -> Result<(), ProgressionError> {
    let row: Option<(String, Option<i64>, String)> = connection
        .query_row(
            "
            SELECT snapshot_json, owner_user_id, mode
            FROM matches
            WHERE id = ?1
            ",
            params![match_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((snapshot_json, owner_user_id, mode)) = row else {
        return Ok(());
    };
    let match_state = MatchState::from_snapshot_json(&snapshot_json)?;
    let Some(winner) = match_state.winner else {
        return Ok(());
    };

    let participants = if mode == "shared" {
        shared_participants(connection, match_id, &match_state)?
    } else {
        owner_user_id
            .map(|user_id| {
                vec![AwardParticipant {
                    user_id,
                    side: Side::Player,
                    hero_type: match_state.player.hero.hero_type,
                }]
            })
            .unwrap_or_default()
    };

    for participant in participants {
        let won = participant.side.team() == winner.team();
        let xp = COMPLETION_XP + if won { WIN_BONUS_XP } else { 0 };
        let inserted = connection.execute(
            "
            INSERT OR IGNORE INTO match_xp_awards (
                match_id,
                user_id,
                account_xp,
                hero_type,
                hero_xp,
                won,
                awarded_at
            )
            VALUES (?1, ?2, ?3, ?4, ?3, ?5, unixepoch())
            ",
            params![
                match_id,
                participant.user_id,
                xp,
                hero_type_to_db(participant.hero_type),
                won
            ],
        )?;
        if inserted == 0 {
            continue;
        }
        connection.execute(
            "
            UPDATE users
            SET total_xp = total_xp + ?2
            WHERE id = ?1
            ",
            params![participant.user_id, xp],
        )?;
        connection.execute(
            "
            INSERT INTO hero_mastery (user_id, hero_type, xp)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(user_id, hero_type) DO UPDATE SET
                xp = hero_mastery.xp + excluded.xp
            ",
            params![
                participant.user_id,
                hero_type_to_db(participant.hero_type),
                xp
            ],
        )?;
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct MatchAwardRow {
    match_id: String,
    account_xp: i64,
    hero_type: HeroType,
    hero_xp: i64,
    won: bool,
    awarded_at: i64,
}

fn match_award_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MatchAwardRow> {
    let hero_type: String = row.get(2)?;
    Ok(MatchAwardRow {
        match_id: row.get(0)?,
        account_xp: row.get(1)?,
        hero_type: hero_type_from_db(&hero_type).expect("stored hero type should be valid"),
        hero_xp: row.get(3)?,
        won: row.get(4)?,
        awarded_at: row.get(5)?,
    })
}

fn prior_account_xp(awards: &[MatchAwardRow], match_id: &str) -> i64 {
    let Some(target) = awards.iter().find(|award| award.match_id == match_id) else {
        return 0;
    };

    awards
        .iter()
        .filter(|award| {
            (award.awarded_at, award.match_id.as_str())
                < (target.awarded_at, target.match_id.as_str())
        })
        .map(|award| award.account_xp)
        .sum()
}

fn prior_hero_xp(awards: &[&MatchAwardRow], match_id: &str) -> i64 {
    let Some(target) = awards.iter().find(|award| award.match_id == match_id) else {
        return 0;
    };

    awards
        .iter()
        .filter(|award| {
            (award.awarded_at, award.match_id.as_str())
                < (target.awarded_at, target.match_id.as_str())
        })
        .map(|award| award.hero_xp)
        .sum()
}

fn reward_unlocks(
    hero_type: HeroType,
    account_before: &ProgressionSummary,
    account_after: &ProgressionSummary,
    hero_before: &HeroProgression,
    hero_after: &HeroProgression,
) -> Vec<MatchUnlockCallout> {
    let mut unlocks = Vec::new();

    for level in (account_before.level + 1)..=account_after.level {
        unlocks.push(MatchUnlockCallout::AccountLevel { level });
    }

    if account_after.rune_slots > account_before.rune_slots {
        unlocks.push(MatchUnlockCallout::RuneSlotUnlocked {
            rune_slots: account_after.rune_slots,
        });
    }

    for rune in rune_definitions() {
        if account_before.level < rune.unlock_level && account_after.level >= rune.unlock_level {
            unlocks.push(MatchUnlockCallout::RuneUnlocked {
                rune_id: rune.id,
                name: rune.name,
            });
        }
    }

    for level in (hero_before.level + 1)..=hero_after.level {
        unlocks.push(MatchUnlockCallout::HeroMasteryLevel { hero_type, level });
    }

    for appearance in appearance_definitions_for_hero(hero_type) {
        let Some(unlock_level) = appearance.unlock_level else {
            continue;
        };
        if hero_before.level < unlock_level && hero_after.level >= unlock_level {
            unlocks.push(MatchUnlockCallout::HeroAppearanceUnlocked {
                hero_type,
                appearance_id: appearance.id,
                name: appearance.name,
            });
        }
    }

    if hero_after.total_skill_points > hero_before.total_skill_points {
        unlocks.push(MatchUnlockCallout::SkillPointUnlocked {
            hero_type,
            skill_points: hero_after.total_skill_points - hero_before.total_skill_points,
        });
    }

    unlocks
}

struct AwardParticipant {
    user_id: i64,
    side: Side,
    hero_type: HeroType,
}

fn shared_participants(
    connection: &Connection,
    match_id: &str,
    match_state: &MatchState,
) -> Result<Vec<AwardParticipant>, ProgressionError> {
    let mut statement = connection.prepare(
        "
        SELECT side, participant_user_id
        FROM match_seats
        WHERE match_id = ?1 AND participant_user_id IS NOT NULL
        ",
    )?;
    let rows = statement.query_map(params![match_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut participants = Vec::new();
    for row in rows {
        let (side, user_id) = row?;
        let Some(side) = side_from_db(&side) else {
            continue;
        };
        let hero_type = match side {
            Side::Player => match_state.player.hero.hero_type,
            Side::Opponent => match_state.opponent.hero.hero_type,
            Side::PlayerTwo => match_state
                .player_two
                .as_ref()
                .map(|participant| participant.hero.hero_type)
                .unwrap_or(match_state.player.hero.hero_type),
            Side::OpponentTwo => match_state
                .opponent_two
                .as_ref()
                .map(|participant| participant.hero.hero_type)
                .unwrap_or(match_state.opponent.hero.hero_type),
        };
        participants.push(AwardParticipant {
            user_id,
            side,
            hero_type,
        });
    }
    Ok(participants)
}
