use std::collections::HashSet;

use rusqlite::{OptionalExtension, params};

use crate::match_session::{HeroType, MatchProgressionLoadout};

use super::catalog::{effects_for, hero_type_to_db, hero_types, root_skill_id, rune_definition};
use super::{
    ProgressionError, ProgressionModule, ProgressionResponse, SaveRuneLoadoutRequest,
    SavedRuneLoadout,
};

impl ProgressionModule<'_> {
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

    fn active_skill_ids(
        &self,
        user_id: i64,
        hero_type: HeroType,
    ) -> Result<Vec<String>, ProgressionError> {
        let mut skill_ids = vec![root_skill_id(hero_type).to_string()];
        skill_ids.extend(self.unlocked_skill_ids(user_id, hero_type)?);
        Ok(skill_ids)
    }

    pub(super) fn load_saved_loadouts(
        &self,
        user_id: i64,
    ) -> Result<Vec<SavedRuneLoadout>, ProgressionError> {
        let mut loadouts = Vec::new();
        for hero_type in hero_types() {
            loadouts.push(SavedRuneLoadout {
                hero_type,
                rune_ids: self.saved_rune_ids(user_id, hero_type)?.unwrap_or_default(),
            });
        }
        Ok(loadouts)
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
