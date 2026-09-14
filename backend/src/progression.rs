use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

use crate::match_session::{
    HeroType, MatchProgressionEffects, MatchProgressionLoadout, MatchState, Side,
};

mod awards;
mod catalog;
mod loadouts;
pub use catalog::summary_for_xp;
use catalog::*;

const COMPLETION_XP: i64 = 100;
const WIN_BONUS_XP: i64 = 50;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionSummary {
    pub total_xp: i64,
    pub level: u32,
    pub current_level_xp: i64,
    pub next_level_xp: i64,
    pub xp_into_level: i64,
    pub xp_to_next_level: i64,
    pub rune_slots: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionResponse {
    pub account: ProgressionSummary,
    pub runes: Vec<RuneDefinition>,
    pub heroes: Vec<HeroProgression>,
    pub skill_trees: Vec<HeroSkillTree>,
    pub loadouts: Vec<SavedRuneLoadout>,
    pub hero_appearances: Vec<HeroAppearanceProgression>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchRewardSummary {
    pub side: Side,
    pub hero_type: HeroType,
    pub won: bool,
    pub account_xp_gained: i64,
    pub hero_xp_gained: i64,
    pub win_bonus_xp: i64,
    pub account: ProgressionDelta,
    pub hero: HeroProgressionDelta,
    pub unlocks: Vec<MatchUnlockCallout>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressionDelta {
    pub before: ProgressionSummary,
    pub after: ProgressionSummary,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroProgressionDelta {
    pub hero_type: HeroType,
    pub before: HeroProgression,
    pub after: HeroProgression,
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum MatchUnlockCallout {
    AccountLevel {
        level: u32,
    },
    RuneUnlocked {
        rune_id: &'static str,
        name: &'static str,
    },
    RuneSlotUnlocked {
        rune_slots: usize,
    },
    HeroMasteryLevel {
        hero_type: HeroType,
        level: u32,
    },
    SkillPointUnlocked {
        hero_type: HeroType,
        skill_points: usize,
    },
    HeroAppearanceUnlocked {
        hero_type: HeroType,
        appearance_id: &'static str,
        name: &'static str,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuneDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub text: &'static str,
    pub unlock_level: u32,
    pub unlocked: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroProgression {
    pub hero_type: HeroType,
    pub xp: i64,
    pub level: u32,
    pub current_level_xp: i64,
    pub next_level_xp: i64,
    pub xp_into_level: i64,
    pub xp_to_next_level: i64,
    pub total_skill_points: usize,
    pub spent_skill_points: usize,
    pub available_skill_points: usize,
    pub unlocked_skill_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroSkillTree {
    pub hero_type: HeroType,
    pub nodes: Vec<SkillNodeDefinition>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillNodeDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub text: &'static str,
    pub root: bool,
    pub prerequisite_id: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedRuneLoadout {
    pub hero_type: HeroType,
    pub rune_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroAppearanceProgression {
    pub hero_type: HeroType,
    pub selected_appearance_id: String,
    pub appearances: Vec<HeroAppearanceDefinition>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroAppearanceDefinition {
    pub id: &'static str,
    pub hero_type: HeroType,
    pub name: &'static str,
    pub text: &'static str,
    pub unlock_level: Option<u32>,
    pub unlocked: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHeroAppearanceRequest {
    pub appearance_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveRuneLoadoutRequest {
    pub rune_ids: Vec<String>,
}

pub struct ProgressionModule<'a> {
    connection: &'a mut Connection,
}

#[derive(Debug)]
pub enum ProgressionError {
    Sqlite(rusqlite::Error),
    Snapshot(serde_json::Error),
    UnknownRune(String),
    LockedRune(String),
    DuplicateRune(String),
    TooManyRunes {
        requested: usize,
        allowed: usize,
    },
    UnknownSkill(String),
    SkillAlreadyUnlocked(String),
    SkillPrerequisiteMissing(String),
    NotEnoughSkillPoints,
    UnknownHeroAppearance(String),
    LockedHeroAppearance(String),
    MismatchedHeroAppearance {
        hero_type: HeroType,
        appearance_id: String,
    },
}

impl fmt::Display for ProgressionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "could not access progression database: {error}"),
            Self::Snapshot(error) => {
                write!(f, "could not read persisted progression snapshot: {error}")
            }
            Self::UnknownRune(rune_id) => write!(f, "Unknown rune: {rune_id}"),
            Self::LockedRune(rune_id) => write!(f, "Rune is not unlocked: {rune_id}"),
            Self::DuplicateRune(rune_id) => write!(f, "Rune may only be equipped once: {rune_id}"),
            Self::TooManyRunes { requested, allowed } => {
                write!(
                    f,
                    "Too many runes selected: requested {requested}, allowed {allowed}"
                )
            }
            Self::UnknownSkill(node_id) => write!(f, "Unknown skill: {node_id}"),
            Self::SkillAlreadyUnlocked(node_id) => {
                write!(f, "Skill is already unlocked: {node_id}")
            }
            Self::SkillPrerequisiteMissing(node_id) => {
                write!(f, "Skill prerequisite is not unlocked: {node_id}")
            }
            Self::NotEnoughSkillPoints => write!(f, "Not enough skill points."),
            Self::UnknownHeroAppearance(appearance_id) => {
                write!(f, "Unknown hero appearance: {appearance_id}")
            }
            Self::LockedHeroAppearance(appearance_id) => {
                write!(f, "Hero appearance is not unlocked: {appearance_id}")
            }
            Self::MismatchedHeroAppearance {
                hero_type,
                appearance_id,
            } => write!(
                f,
                "Hero appearance {appearance_id} does not belong to {hero_type:?}"
            ),
        }
    }
}

impl Error for ProgressionError {}

impl From<rusqlite::Error> for ProgressionError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for ProgressionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Snapshot(error)
    }
}

pub fn default_hero_appearance_id(hero_type: HeroType) -> String {
    base_hero_appearance_id(hero_type).to_string()
}

impl<'a> ProgressionModule<'a> {
    pub fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    pub fn load_for_user(&mut self, user_id: i64) -> Result<ProgressionResponse, ProgressionError> {
        let total_xp = self.total_xp(user_id)?;
        let account = summary_for_xp(total_xp);
        let runes = rune_definitions()
            .into_iter()
            .map(|rune| RuneDefinition {
                unlocked: account.level >= rune.unlock_level,
                ..rune
            })
            .collect();
        let mut heroes = Vec::new();
        for hero_type in hero_types() {
            heroes.push(self.hero_progression(user_id, hero_type)?);
        }
        Ok(ProgressionResponse {
            account,
            runes,
            heroes,
            skill_trees: skill_trees(),
            loadouts: self.load_saved_loadouts(user_id)?,
            hero_appearances: self.load_hero_appearances(user_id)?,
        })
    }

    pub fn progression_summary(
        &self,
        user_id: i64,
    ) -> Result<ProgressionSummary, ProgressionError> {
        self.total_xp(user_id).map(summary_for_xp)
    }

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

    pub fn unlock_skill(
        &mut self,
        user_id: i64,
        hero_type: HeroType,
        node_id: &str,
    ) -> Result<ProgressionResponse, ProgressionError> {
        let Some(node) = skill_node(hero_type, node_id) else {
            return Err(ProgressionError::UnknownSkill(node_id.to_string()));
        };
        if node.root {
            return Err(ProgressionError::SkillAlreadyUnlocked(node_id.to_string()));
        }
        let progression = self.hero_progression(user_id, hero_type)?;
        if progression
            .unlocked_skill_ids
            .iter()
            .any(|unlocked| unlocked == node_id)
        {
            return Err(ProgressionError::SkillAlreadyUnlocked(node_id.to_string()));
        }
        if progression.available_skill_points == 0 {
            return Err(ProgressionError::NotEnoughSkillPoints);
        }
        if let Some(prerequisite_id) = node.prerequisite_id {
            let prerequisite_unlocked = prerequisite_id == root_skill_id(hero_type)
                || progression
                    .unlocked_skill_ids
                    .iter()
                    .any(|unlocked| unlocked == prerequisite_id);
            if !prerequisite_unlocked {
                return Err(ProgressionError::SkillPrerequisiteMissing(
                    prerequisite_id.to_string(),
                ));
            }
        }

        self.connection.execute(
            "
            INSERT INTO hero_skill_unlocks (user_id, hero_type, node_id, unlocked_at)
            VALUES (?1, ?2, ?3, unixepoch())
            ",
            params![user_id, hero_type_to_db(hero_type), node_id],
        )?;
        self.load_for_user(user_id)
    }

    pub fn respec_hero(
        &mut self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<ProgressionResponse, ProgressionError> {
        self.connection.execute(
            "
            DELETE FROM hero_skill_unlocks
            WHERE user_id = ?1 AND hero_type = ?2
            ",
            params![user_id, hero_type_to_db(hero_type)],
        )?;
        self.load_for_user(user_id)
    }

    pub fn save_rune_loadout(
        &mut self,
        user_id: i64,
        hero_type: HeroType,
        request: SaveRuneLoadoutRequest,
    ) -> Result<ProgressionResponse, ProgressionError> {
        self.validate_rune_ids(user_id, &request.rune_ids)?;
        let rune_ids_json = serde_json::to_string(&request.rune_ids)?;
        self.connection.execute(
            "
            INSERT INTO hero_rune_loadouts (user_id, hero_type, rune_ids_json, updated_at)
            VALUES (?1, ?2, ?3, unixepoch())
            ON CONFLICT(user_id, hero_type) DO UPDATE SET
                rune_ids_json = excluded.rune_ids_json,
                updated_at = unixepoch()
            ",
            params![user_id, hero_type_to_db(hero_type), rune_ids_json],
        )?;
        self.load_for_user(user_id)
    }

    pub fn save_hero_appearance_selection(
        &mut self,
        user_id: i64,
        hero_type: HeroType,
        request: SaveHeroAppearanceRequest,
    ) -> Result<ProgressionResponse, ProgressionError> {
        let Some(definition) = hero_appearance_definition_any(&request.appearance_id) else {
            return Err(ProgressionError::UnknownHeroAppearance(
                request.appearance_id,
            ));
        };
        if definition.hero_type != hero_type {
            return Err(ProgressionError::MismatchedHeroAppearance {
                hero_type,
                appearance_id: request.appearance_id,
            });
        }

        let hero_level = self.hero_progression(user_id, hero_type)?.level;
        if !appearance_unlocked(hero_level, &definition) {
            return Err(ProgressionError::LockedHeroAppearance(
                request.appearance_id,
            ));
        }

        self.connection.execute(
            "
            INSERT INTO hero_appearance_selections (user_id, hero_type, appearance_id, updated_at)
            VALUES (?1, ?2, ?3, unixepoch())
            ON CONFLICT(user_id, hero_type) DO UPDATE SET
                appearance_id = excluded.appearance_id,
                updated_at = unixepoch()
            ",
            params![user_id, hero_type_to_db(hero_type), definition.id],
        )?;
        self.load_for_user(user_id)
    }

    pub fn selected_hero_appearance_id(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Option<String>, ProgressionError> {
        let Some(selected) = self.saved_hero_appearance_id(user_id, hero_type)? else {
            return Ok(None);
        };
        let Some(definition) = hero_appearance_definition(hero_type, &selected) else {
            return Ok(None);
        };
        let hero_level = summary_for_xp(self.hero_xp(user_id, hero_type)?).level;
        if appearance_unlocked(hero_level, &definition) {
            Ok(Some(selected))
        } else {
            Ok(None)
        }
    }

    pub fn match_loadout(
        &mut self,
        user_id: Option<i64>,
        hero_type: HeroType,
        requested_rune_ids: Option<Vec<String>>,
    ) -> Result<MatchProgressionLoadout, ProgressionError> {
        let Some(user_id) = user_id else {
            return Ok(MatchProgressionLoadout::default());
        };
        let rune_ids = match requested_rune_ids {
            Some(rune_ids) => rune_ids,
            None => self.saved_rune_ids(user_id, hero_type)?.unwrap_or_default(),
        };
        self.validate_rune_ids(user_id, &rune_ids)?;
        let skill_ids = self.active_skill_ids(user_id, hero_type)?;
        Ok(MatchProgressionLoadout {
            rune_ids: rune_ids.clone(),
            skill_ids: skill_ids.clone(),
            effects: effects_for(&rune_ids, &skill_ids),
        })
    }

    fn total_xp(&self, user_id: i64) -> Result<i64, ProgressionError> {
        self.connection
            .query_row(
                "SELECT total_xp FROM users WHERE id = ?1",
                params![user_id],
                |row| row.get(0),
            )
            .map_err(ProgressionError::from)
    }

    fn hero_progression(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<HeroProgression, ProgressionError> {
        let xp = self.hero_xp(user_id, hero_type)?;
        self.hero_progression_from_xp(user_id, hero_type, xp)
    }

    fn hero_progression_from_xp(
        &self,
        user_id: i64,
        hero_type: HeroType,
        xp: i64,
    ) -> Result<HeroProgression, ProgressionError> {
        let level_summary = summary_for_xp(xp);
        let unlocked_skill_ids = self.unlocked_skill_ids(user_id, hero_type)?;
        let spent_skill_points = unlocked_skill_ids.len();
        let total_skill_points = level_summary.level.saturating_sub(1) as usize;
        let available_skill_points = total_skill_points.saturating_sub(spent_skill_points);
        Ok(HeroProgression {
            hero_type,
            xp,
            level: level_summary.level,
            current_level_xp: level_summary.current_level_xp,
            next_level_xp: level_summary.next_level_xp,
            xp_into_level: level_summary.xp_into_level,
            xp_to_next_level: level_summary.xp_to_next_level,
            total_skill_points,
            spent_skill_points,
            available_skill_points,
            unlocked_skill_ids,
        })
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

    fn hero_xp(&self, user_id: i64, hero_type: HeroType) -> Result<i64, ProgressionError> {
        self.connection
            .query_row(
                "
                SELECT xp
                FROM hero_mastery
                WHERE user_id = ?1 AND hero_type = ?2
                ",
                params![user_id, hero_type_to_db(hero_type)],
                |row| row.get(0),
            )
            .optional()
            .map(|xp| xp.unwrap_or(0))
            .map_err(ProgressionError::from)
    }

    fn unlocked_skill_ids(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Vec<String>, ProgressionError> {
        let mut statement = self.connection.prepare(
            "
            SELECT node_id
            FROM hero_skill_unlocks
            WHERE user_id = ?1 AND hero_type = ?2
            ORDER BY unlocked_at ASC, node_id ASC
            ",
        )?;
        let rows = statement.query_map(params![user_id, hero_type_to_db(hero_type)], |row| {
            row.get::<_, String>(0)
        })?;
        let mut node_ids = Vec::new();
        for row in rows {
            node_ids.push(row?);
        }
        Ok(node_ids)
    }

    fn active_skill_ids(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Vec<String>, ProgressionError> {
        let mut skill_ids = vec![root_skill_id(hero_type).to_string()];
        skill_ids.extend(self.unlocked_skill_ids(user_id, hero_type)?);
        Ok(skill_ids)
    }

    fn load_saved_loadouts(&self, user_id: i64) -> Result<Vec<SavedRuneLoadout>, ProgressionError> {
        let mut loadouts = Vec::new();
        for hero_type in hero_types() {
            loadouts.push(SavedRuneLoadout {
                hero_type,
                rune_ids: self.saved_rune_ids(user_id, hero_type)?.unwrap_or_default(),
            });
        }
        Ok(loadouts)
    }

    fn load_hero_appearances(
        &self,
        user_id: i64,
    ) -> Result<Vec<HeroAppearanceProgression>, ProgressionError> {
        let mut appearances = Vec::new();
        for hero_type in hero_types() {
            let hero_level = self.hero_progression(user_id, hero_type)?.level;
            let definitions: Vec<_> = appearance_definitions_for_hero(hero_type)
                .into_iter()
                .map(|definition| HeroAppearanceDefinition {
                    unlocked: appearance_unlocked(hero_level, &definition),
                    ..definition
                })
                .collect();
            let saved = self.saved_hero_appearance_id(user_id, hero_type)?;
            let selected_appearance_id = saved
                .filter(|appearance_id| {
                    definitions
                        .iter()
                        .any(|definition| definition.id == appearance_id && definition.unlocked)
                })
                .unwrap_or_else(|| base_hero_appearance_id(hero_type).to_string());
            appearances.push(HeroAppearanceProgression {
                hero_type,
                selected_appearance_id,
                appearances: definitions,
            });
        }
        Ok(appearances)
    }

    fn saved_hero_appearance_id(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Option<String>, ProgressionError> {
        self.connection
            .query_row(
                "
                SELECT appearance_id
                FROM hero_appearance_selections
                WHERE user_id = ?1 AND hero_type = ?2
                ",
                params![user_id, hero_type_to_db(hero_type)],
                |row| row.get(0),
            )
            .optional()
            .map_err(ProgressionError::from)
    }

    fn saved_rune_ids(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Option<Vec<String>>, ProgressionError> {
        let rune_ids_json: Option<String> = self
            .connection
            .query_row(
                "
                SELECT rune_ids_json
                FROM hero_rune_loadouts
                WHERE user_id = ?1 AND hero_type = ?2
                ",
                params![user_id, hero_type_to_db(hero_type)],
                |row| row.get(0),
            )
            .optional()?;

        rune_ids_json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(ProgressionError::from)
    }

    fn validate_rune_ids(&self, user_id: i64, rune_ids: &[String]) -> Result<(), ProgressionError> {
        let allowed = self.progression_summary(user_id)?.rune_slots;
        if rune_ids.len() > allowed {
            return Err(ProgressionError::TooManyRunes {
                requested: rune_ids.len(),
                allowed,
            });
        }

        let level = self.progression_summary(user_id)?.level;
        let mut seen = HashSet::new();
        for rune_id in rune_ids {
            let Some(rune) = rune_definition(rune_id) else {
                return Err(ProgressionError::UnknownRune(rune_id.clone()));
            };
            if !seen.insert(rune_id) {
                return Err(ProgressionError::DuplicateRune(rune_id.clone()));
            }
            if level < rune.unlock_level {
                return Err(ProgressionError::LockedRune(rune_id.clone()));
            }
        }
        Ok(())
    }
}

pub fn migrate(connection: &Connection) -> Result<(), ProgressionError> {
    add_column_if_missing(
        connection,
        "users",
        "total_xp",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS match_xp_awards (
            match_id TEXT NOT NULL,
            user_id INTEGER NOT NULL,
            account_xp INTEGER NOT NULL,
            hero_type TEXT NOT NULL,
            hero_xp INTEGER NOT NULL,
            won INTEGER NOT NULL,
            awarded_at INTEGER NOT NULL,
            PRIMARY KEY (match_id, user_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS hero_mastery (
            user_id INTEGER NOT NULL,
            hero_type TEXT NOT NULL,
            xp INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (user_id, hero_type),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS hero_skill_unlocks (
            user_id INTEGER NOT NULL,
            hero_type TEXT NOT NULL,
            node_id TEXT NOT NULL,
            unlocked_at INTEGER NOT NULL,
            PRIMARY KEY (user_id, hero_type, node_id),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS hero_rune_loadouts (
            user_id INTEGER NOT NULL,
            hero_type TEXT NOT NULL,
            rune_ids_json TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (user_id, hero_type),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS hero_appearance_selections (
            user_id INTEGER NOT NULL,
            hero_type TEXT NOT NULL,
            appearance_id TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (user_id, hero_type),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        ",
    )?;
    if table_exists(connection, "wizard_mastery")? {
        connection.execute_batch(
            "
            INSERT OR IGNORE INTO hero_mastery (user_id, hero_type, xp)
            SELECT user_id, wizard_type, xp
            FROM wizard_mastery;
            DROP TABLE wizard_mastery;
            ",
        )?;
    }
    if table_exists(connection, "wizard_skill_unlocks")? {
        connection.execute_batch(
            "
            INSERT OR IGNORE INTO hero_skill_unlocks (user_id, hero_type, node_id, unlocked_at)
            SELECT user_id, wizard_type, node_id, unlocked_at
            FROM wizard_skill_unlocks;
            DROP TABLE wizard_skill_unlocks;
            ",
        )?;
    }
    if table_exists(connection, "wizard_rune_loadouts")? {
        connection.execute_batch(
            "
            INSERT OR IGNORE INTO hero_rune_loadouts (user_id, hero_type, rune_ids_json, updated_at)
            SELECT user_id, wizard_type, rune_ids_json, updated_at
            FROM wizard_rune_loadouts;
            DROP TABLE wizard_rune_loadouts;
            ",
        )?;
    }
    if column_exists(connection, "match_xp_awards", "wizard_type")? {
        add_column_if_missing(
            connection,
            "match_xp_awards",
            "hero_type",
            "TEXT NOT NULL DEFAULT 'runekeeper'",
        )?;
        connection.execute(
            "
            UPDATE match_xp_awards
            SET hero_type = COALESCE(wizard_type, hero_type)
            ",
            [],
        )?;
        drop_column_if_exists(connection, "match_xp_awards", "wizard_type")?;
    }
    if column_exists(connection, "match_xp_awards", "wizard_xp")? {
        add_column_if_missing(
            connection,
            "match_xp_awards",
            "hero_xp",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        connection.execute(
            "
            UPDATE match_xp_awards
            SET hero_xp = COALESCE(wizard_xp, hero_xp)
            ",
            [],
        )?;
        drop_column_if_exists(connection, "match_xp_awards", "wizard_xp")?;
    }
    Ok(())
}

#[cfg(debug_assertions)]
pub fn seed_experienced_local_mastery(
    connection: &Connection,
    user_id: i64,
) -> Result<(), ProgressionError> {
    for hero_type in hero_types() {
        connection.execute(
            "
            INSERT INTO hero_mastery (user_id, hero_type, xp)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(user_id, hero_type) DO UPDATE SET
                xp = excluded.xp
            ",
            params![
                user_id,
                hero_type_to_db(hero_type),
                crate::identity::EXPERIENCED_LOCAL_XP
            ],
        )?;
    }
    Ok(())
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

fn side_from_db(value: &str) -> Option<Side> {
    match value {
        "player" => Some(Side::Player),
        "opponent" => Some(Side::Opponent),
        _ => None,
    }
}

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), ProgressionError> {
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

fn table_exists(connection: &Connection, table: &str) -> Result<bool, ProgressionError> {
    let exists: Option<i64> = connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )
        .optional()?;
    Ok(exists.is_some())
}

fn column_exists(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, ProgressionError> {
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
) -> Result<(), ProgressionError> {
    if column_exists(connection, table, column)? {
        let _ = connection.execute(&format!("ALTER TABLE {table} DROP COLUMN {column}"), []);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
