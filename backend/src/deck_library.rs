use std::error::Error;
use std::fmt;

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::deck_recipe_legality::{self, DeckRecipeLegalityError, normalize_requested_cards};
use crate::match_session::{Card, HeroType, Side};

pub const DECK_LIMIT_PER_ACCOUNT: usize = 30;
pub use crate::deck_recipe_legality::{
    DeckCardCount, DeckCardCountRequest, DeckLegality, DeckRules, validate_recipe,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckRecipeSnapshot {
    pub name: String,
    pub cards: Vec<DeckCardCount>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckRecipeSummary {
    pub id: i64,
    pub name: String,
    pub is_default: bool,
    pub hero_type: HeroType,
    pub rune_ids: Vec<String>,
    pub cards: Vec<DeckCardCount>,
    pub legality: DeckLegality,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckListResponse {
    pub rules: DeckRules,
    pub decks: Vec<DeckRecipeSummary>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDeckRecipe {
    pub id: String,
    pub name: String,
    pub hero_type: HeroType,
    pub cards: Vec<DeckCardCount>,
    pub legality: DeckLegality,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDeckListResponse {
    pub rules: DeckRules,
    pub decks: Vec<SystemDeckRecipe>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDeckRequest {
    pub name: String,
    pub cards: Vec<DeckCardCountRequest>,
    #[serde(default)]
    pub is_default: Option<bool>,
    #[serde(default, alias = "wizardType")]
    pub hero_type: Option<HeroType>,
    #[serde(default)]
    pub rune_ids: Vec<String>,
}

#[derive(Debug)]
pub enum DeckLibraryError {
    Sqlite(rusqlite::Error),
    Snapshot(serde_json::Error),
    UnknownTemplate(String),
    NegativeCount(String),
    EmptyName,
    NameTooLong,
    TooManyDecks,
    NotFound,
    IllegalRecipe(Vec<String>),
    UnknownSystemDeck(String),
}

impl fmt::Display for DeckLibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(f, "could not access deck database: {error}"),
            Self::Snapshot(error) => write!(f, "could not read deck recipe snapshot: {error}"),
            Self::UnknownTemplate(template_id) => {
                write!(f, "Unknown card template: {template_id}")
            }
            Self::NegativeCount(template_id) => {
                write!(f, "Card count must not be negative: {template_id}")
            }
            Self::EmptyName => write!(f, "Deck name is required."),
            Self::NameTooLong => write!(f, "Deck name must be 40 characters or fewer."),
            Self::TooManyDecks => write!(f, "Deck library limit reached."),
            Self::NotFound => write!(f, "Deck recipe was not found."),
            Self::IllegalRecipe(messages) => {
                write!(f, "Deck recipe is not legal: {}", messages.join(" "))
            }
            Self::UnknownSystemDeck(deck_id) => write!(f, "Unknown system deck: {deck_id}"),
        }
    }
}

impl Error for DeckLibraryError {}

impl From<rusqlite::Error> for DeckLibraryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for DeckLibraryError {
    fn from(error: serde_json::Error) -> Self {
        Self::Snapshot(error)
    }
}

impl From<DeckRecipeLegalityError> for DeckLibraryError {
    fn from(error: DeckRecipeLegalityError) -> Self {
        match error {
            DeckRecipeLegalityError::UnknownTemplate(template_id) => {
                Self::UnknownTemplate(template_id)
            }
            DeckRecipeLegalityError::NegativeCount(template_id) => Self::NegativeCount(template_id),
        }
    }
}

pub struct DeckLibrary<'a> {
    connection: &'a mut Connection,
}

impl<'a> DeckLibrary<'a> {
    pub fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    pub fn list_for_user(&mut self, user_id: i64) -> Result<DeckListResponse, DeckLibraryError> {
        self.ensure_starter_deck(user_id)?;
        Ok(DeckListResponse {
            rules: deck_rules(),
            decks: self.load_user_decks(user_id)?,
        })
    }

    pub fn load_for_user(
        &mut self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<Option<DeckRecipeSummary>, DeckLibraryError> {
        self.ensure_starter_deck(user_id)?;
        self.load_user_deck(user_id, deck_id)
    }

    pub fn load_public_for_user(
        &self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<Option<DeckRecipeSummary>, DeckLibraryError> {
        self.load_user_deck(user_id, deck_id)
    }

    pub fn create_for_user(
        &mut self,
        user_id: i64,
        request: SaveDeckRequest,
    ) -> Result<DeckRecipeSummary, DeckLibraryError> {
        self.ensure_deck_limit(user_id)?;
        let name = normalize_deck_name(&request.name)?;
        let cards = normalize_requested_cards(request.cards)?;
        let is_default = request.is_default.unwrap_or(false);
        let hero_type = request.hero_type.unwrap_or_default();
        let rune_ids_json = serde_json::to_string(&request.rune_ids)?;

        let transaction = self.connection.transaction()?;
        if is_default {
            clear_default_deck(&transaction, user_id)?;
        }
        transaction.execute(
            "
            INSERT INTO deck_recipes (
                user_id,
                name,
                is_default,
                hero_type,
                rune_ids_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, unixepoch(), unixepoch())
            ",
            params![
                user_id,
                name,
                is_default,
                hero_type_to_db(hero_type),
                rune_ids_json
            ],
        )?;
        let deck_id = transaction.last_insert_rowid();
        replace_deck_cards(&transaction, deck_id, &cards)?;
        transaction.commit()?;

        self.load_user_deck(user_id, deck_id)?
            .ok_or(DeckLibraryError::NotFound)
    }

    pub fn update_for_user(
        &mut self,
        user_id: i64,
        deck_id: i64,
        request: SaveDeckRequest,
    ) -> Result<Option<DeckRecipeSummary>, DeckLibraryError> {
        let name = normalize_deck_name(&request.name)?;
        let cards = normalize_requested_cards(request.cards)?;
        let hero_type = request.hero_type.unwrap_or_default();
        let rune_ids_json = serde_json::to_string(&request.rune_ids)?;
        let exists = self.deck_exists_for_user(user_id, deck_id)?;
        if !exists {
            return Ok(None);
        }

        let transaction = self.connection.transaction()?;
        if request.is_default.unwrap_or(false) {
            clear_default_deck(&transaction, user_id)?;
        }
        transaction.execute(
            "
            UPDATE deck_recipes
            SET name = ?3,
                is_default = CASE WHEN ?4 THEN 1 ELSE is_default END,
                hero_type = ?5,
                rune_ids_json = ?6,
                updated_at = unixepoch()
            WHERE id = ?1 AND user_id = ?2
            ",
            params![
                deck_id,
                user_id,
                name,
                request.is_default.unwrap_or(false),
                hero_type_to_db(hero_type),
                rune_ids_json,
            ],
        )?;
        replace_deck_cards(&transaction, deck_id, &cards)?;
        transaction.commit()?;

        self.load_user_deck(user_id, deck_id)
    }

    pub fn duplicate_for_user(
        &mut self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<Option<DeckRecipeSummary>, DeckLibraryError> {
        self.ensure_deck_limit(user_id)?;
        let Some(source) = self.load_user_deck(user_id, deck_id)? else {
            return Ok(None);
        };
        let request = SaveDeckRequest {
            name: format!("{} Copy", source.name).chars().take(40).collect(),
            cards: source
                .cards
                .iter()
                .map(|card| DeckCardCountRequest {
                    template_id: card.template_id.clone(),
                    count: i32::from(card.count),
                })
                .collect(),
            is_default: Some(false),
            hero_type: Some(source.hero_type),
            rune_ids: source.rune_ids,
        };
        self.create_for_user(user_id, request).map(Some)
    }

    pub fn delete_for_user(
        &mut self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<bool, DeckLibraryError> {
        let Some(deck) = self.load_user_deck(user_id, deck_id)? else {
            return Ok(false);
        };
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM deck_recipe_cards WHERE deck_id = ?1",
            params![deck_id],
        )?;
        let deleted = transaction.execute(
            "DELETE FROM deck_recipes WHERE id = ?1 AND user_id = ?2",
            params![deck_id, user_id],
        )?;
        transaction.commit()?;

        if deleted == 1 && deck.is_default {
            self.promote_oldest_legal_deck(user_id)?;
        }

        Ok(deleted == 1)
    }

    pub fn legal_snapshot_for_user(
        &mut self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<Option<DeckRecipeSnapshot>, DeckLibraryError> {
        let Some(deck) = self.load_for_user(user_id, deck_id)? else {
            return Ok(None);
        };
        if !deck.legality.legal {
            return Err(DeckLibraryError::IllegalRecipe(deck.legality.messages));
        }
        Ok(Some(DeckRecipeSnapshot {
            name: deck.name,
            cards: deck.cards,
        }))
    }

    pub fn configuration_for_user(
        &mut self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<Option<(HeroType, Vec<String>)>, DeckLibraryError> {
        Ok(self
            .load_for_user(user_id, deck_id)?
            .map(|deck| (deck.hero_type, deck.rune_ids)))
    }

    fn ensure_starter_deck(&mut self, user_id: i64) -> Result<(), DeckLibraryError> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM deck_recipes WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Ok(());
        }

        let starter = starter_deck_snapshot();
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "
            INSERT INTO deck_recipes (
                user_id,
                name,
                is_default,
                hero_type,
                rune_ids_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, 1, ?3, '[]', unixepoch(), unixepoch())
            ",
            params![user_id, starter.name, hero_type_to_db(HeroType::Runekeeper)],
        )?;
        let deck_id = transaction.last_insert_rowid();
        replace_deck_cards(&transaction, deck_id, &starter.cards)?;
        transaction.commit()?;
        Ok(())
    }

    fn ensure_deck_limit(&self, user_id: i64) -> Result<(), DeckLibraryError> {
        let count: i64 = self.connection.query_row(
            "SELECT COUNT(*) FROM deck_recipes WHERE user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        if count as usize >= DECK_LIMIT_PER_ACCOUNT {
            return Err(DeckLibraryError::TooManyDecks);
        }
        Ok(())
    }

    fn deck_exists_for_user(&self, user_id: i64, deck_id: i64) -> Result<bool, DeckLibraryError> {
        let exists: Option<i64> = self
            .connection
            .query_row(
                "SELECT id FROM deck_recipes WHERE id = ?1 AND user_id = ?2",
                params![deck_id, user_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(exists.is_some())
    }

    fn load_user_decks(&self, user_id: i64) -> Result<Vec<DeckRecipeSummary>, DeckLibraryError> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, name, is_default, hero_type, rune_ids_json, created_at, updated_at
            FROM deck_recipes
            WHERE user_id = ?1
            ORDER BY is_default DESC, updated_at DESC, id ASC
            ",
        )?;
        let rows = statement.query_map(params![user_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? == 1,
                hero_type_from_db(&row.get::<_, String>(3)?),
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })?;

        let mut decks = Vec::new();
        for row in rows {
            let (id, name, is_default, hero_type, rune_ids_json, created_at, updated_at) = row?;
            let cards = load_deck_cards(self.connection, id)?;
            decks.push(DeckRecipeSummary {
                id,
                name,
                is_default,
                hero_type,
                rune_ids: serde_json::from_str(&rune_ids_json)?,
                legality: validate_recipe(&cards),
                cards,
                created_at,
                updated_at,
            });
        }
        Ok(decks)
    }

    fn load_user_deck(
        &self,
        user_id: i64,
        deck_id: i64,
    ) -> Result<Option<DeckRecipeSummary>, DeckLibraryError> {
        let row = self
            .connection
            .query_row(
                "
            SELECT id, name, is_default, hero_type, rune_ids_json, created_at, updated_at
            FROM deck_recipes
            WHERE id = ?1 AND user_id = ?2
            ",
                params![deck_id, user_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)? == 1,
                        hero_type_from_db(&row.get::<_, String>(3)?),
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, i64>(6)?,
                    ))
                },
            )
            .optional()?;

        row.map(
            |(id, name, is_default, hero_type, rune_ids_json, created_at, updated_at)| {
                let cards = load_deck_cards(self.connection, id)?;
                Ok(DeckRecipeSummary {
                    id,
                    name,
                    is_default,
                    hero_type,
                    rune_ids: serde_json::from_str(&rune_ids_json)?,
                    legality: validate_recipe(&cards),
                    cards,
                    created_at,
                    updated_at,
                })
            },
        )
        .transpose()
    }

    fn promote_oldest_legal_deck(&mut self, user_id: i64) -> Result<(), DeckLibraryError> {
        let decks = self.load_user_decks(user_id)?;
        let Some(deck) = decks.into_iter().find(|deck| deck.legality.legal) else {
            return Ok(());
        };
        self.connection.execute(
            "UPDATE deck_recipes SET is_default = CASE WHEN id = ?2 THEN 1 ELSE 0 END WHERE user_id = ?1",
            params![user_id, deck.id],
        )?;
        Ok(())
    }
}

pub fn migrate(connection: &Connection) -> Result<(), DeckLibraryError> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS deck_recipes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0,
            hero_type TEXT NOT NULL DEFAULT 'runekeeper',
            rune_ids_json TEXT NOT NULL DEFAULT '[]',
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS deck_recipe_cards (
            deck_id INTEGER NOT NULL,
            template_id TEXT NOT NULL,
            count INTEGER NOT NULL,
            PRIMARY KEY (deck_id, template_id),
            FOREIGN KEY (deck_id) REFERENCES deck_recipes(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS deck_recipes_user_id_idx
            ON deck_recipes(user_id);
        ",
    )?;
    let hero_type_added = add_column_if_missing(
        connection,
        "deck_recipes",
        "hero_type",
        "TEXT NOT NULL DEFAULT 'runekeeper'",
    )?;
    let rune_ids_added = add_column_if_missing(
        connection,
        "deck_recipes",
        "rune_ids_json",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    let had_legacy_hero_type = column_exists(connection, "deck_recipes", "wizard_type")?;
    if had_legacy_hero_type {
        connection.execute_batch(
            "
            UPDATE deck_recipes
            SET hero_type = COALESCE(wizard_type, hero_type);
            ",
        )?;
        drop_column_if_exists(connection, "deck_recipes", "wizard_type")?;
    }
    if hero_type_added && !had_legacy_hero_type {
        connection.execute_batch(
            "
            UPDATE deck_recipes
            SET hero_type = COALESCE((
                SELECT users.preferred_hero_type
                FROM users
                WHERE users.id = deck_recipes.user_id
            ), hero_type);
            ",
        )?;
    }
    if rune_ids_added && table_exists(connection, "wizard_rune_loadouts")? {
        connection.execute_batch(
            "
            UPDATE deck_recipes
            SET rune_ids_json = COALESCE((
                SELECT wizard_rune_loadouts.rune_ids_json
                FROM wizard_rune_loadouts
                WHERE wizard_rune_loadouts.user_id = deck_recipes.user_id
                  AND wizard_rune_loadouts.wizard_type = deck_recipes.hero_type
            ), rune_ids_json);
            ",
        )?;
    }
    if rune_ids_added && table_exists(connection, "hero_rune_loadouts")? {
        connection.execute_batch(
            "
            UPDATE deck_recipes
            SET rune_ids_json = COALESCE((
                SELECT hero_rune_loadouts.rune_ids_json
                FROM hero_rune_loadouts
                WHERE hero_rune_loadouts.user_id = deck_recipes.user_id
                  AND hero_rune_loadouts.hero_type = deck_recipes.hero_type
            ), rune_ids_json);
            ",
        )?;
    }
    Ok(())
}

pub fn deck_rules() -> DeckRules {
    deck_recipe_legality::deck_rules(DECK_LIMIT_PER_ACCOUNT)
}

pub fn system_deck_response() -> SystemDeckListResponse {
    SystemDeckListResponse {
        rules: deck_rules(),
        decks: system_decks(),
    }
}

pub(crate) fn ai_lab_system_decks() -> Vec<SystemDeckRecipe> {
    system_decks()
}

pub fn starter_deck_snapshot() -> DeckRecipeSnapshot {
    let starter = system_deck_by_id("balanced-starter").expect("starter system deck should exist");
    DeckRecipeSnapshot {
        name: starter.name,
        cards: starter.cards,
    }
}

pub fn system_deck_by_id(deck_id: &str) -> Option<SystemDeckRecipe> {
    system_decks().into_iter().find(|deck| deck.id == deck_id)
}

pub fn system_deck_snapshot(deck_id: &str) -> Result<DeckRecipeSnapshot, DeckLibraryError> {
    let Some(deck) = system_deck_by_id(deck_id) else {
        return Err(DeckLibraryError::UnknownSystemDeck(deck_id.to_string()));
    };
    Ok(DeckRecipeSnapshot {
        name: deck.name,
        cards: deck.cards,
    })
}

pub fn deck_from_snapshot(
    side: Side,
    snapshot: &DeckRecipeSnapshot,
) -> Result<Vec<Card>, DeckLibraryError> {
    deck_from_counts(side, &snapshot.cards)
}

pub fn deck_from_counts(
    side: Side,
    counts: &[DeckCardCount],
) -> Result<Vec<Card>, DeckLibraryError> {
    let counts: Vec<_> = counts
        .iter()
        .map(|card| rune_lanes_core::deck_library::DeckCardCount {
            template_id: card.template_id.clone(),
            count: card.count,
        })
        .collect();
    rune_lanes_core::deck_library::deck_from_counts(side, &counts).map_err(|error| match error {
        rune_lanes_core::deck_library::DeckLibraryError::UnknownTemplate(id) => {
            DeckLibraryError::UnknownTemplate(id)
        }
    })
}

fn system_decks() -> Vec<SystemDeckRecipe> {
    rune_lanes_core::deck_library::system_deck_recipes()
        .iter()
        .map(|recipe| {
            let cards: Vec<_> = recipe
                .cards
                .iter()
                .map(|card| DeckCardCount {
                    template_id: card.template_id.clone(),
                    count: card.count,
                })
                .collect();
            SystemDeckRecipe {
                id: recipe.id.clone(),
                name: recipe.name.clone(),
                hero_type: recipe.hero_type,
                legality: validate_recipe(&cards),
                cards,
            }
        })
        .collect()
}

fn normalize_deck_name(name: &str) -> Result<String, DeckLibraryError> {
    let normalized = name.trim();
    if normalized.is_empty() {
        return Err(DeckLibraryError::EmptyName);
    }
    if normalized.chars().count() > 40 {
        return Err(DeckLibraryError::NameTooLong);
    }
    Ok(normalized.to_string())
}

fn replace_deck_cards(
    transaction: &rusqlite::Transaction<'_>,
    deck_id: i64,
    cards: &[DeckCardCount],
) -> Result<(), DeckLibraryError> {
    transaction.execute(
        "DELETE FROM deck_recipe_cards WHERE deck_id = ?1",
        params![deck_id],
    )?;
    for card in cards {
        transaction.execute(
            "
            INSERT INTO deck_recipe_cards (deck_id, template_id, count)
            VALUES (?1, ?2, ?3)
            ",
            params![deck_id, card.template_id, i64::from(card.count)],
        )?;
    }
    Ok(())
}

fn clear_default_deck(
    transaction: &rusqlite::Transaction<'_>,
    user_id: i64,
) -> Result<(), DeckLibraryError> {
    transaction.execute(
        "UPDATE deck_recipes SET is_default = 0 WHERE user_id = ?1",
        params![user_id],
    )?;
    Ok(())
}

fn add_column_if_missing(
    connection: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<bool, DeckLibraryError> {
    let columns = connection
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if columns.iter().any(|existing| existing == column) {
        return Ok(false);
    }
    connection.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {definition};"
    ))?;
    Ok(true)
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, DeckLibraryError> {
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
) -> Result<bool, DeckLibraryError> {
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
) -> Result<(), DeckLibraryError> {
    if column_exists(connection, table, column)? {
        let _ = connection.execute(&format!("ALTER TABLE {table} DROP COLUMN {column}"), []);
    }
    Ok(())
}

fn hero_type_to_db(hero_type: HeroType) -> &'static str {
    match hero_type {
        HeroType::Runekeeper => "runekeeper",
        HeroType::Pyromancer => "pyromancer",
        HeroType::Chronomancer => "chronomancer",
        HeroType::Warden => "warden",
        HeroType::Battlemage => "battlemage",
        HeroType::Barbarian => "barbarian",
        HeroType::Archer => "archer",
        HeroType::Builder => "builder",
    }
}

fn hero_type_from_db(value: &str) -> HeroType {
    match value {
        "pyromancer" => HeroType::Pyromancer,
        "chronomancer" => HeroType::Chronomancer,
        "warden" => HeroType::Warden,
        "battlemage" => HeroType::Battlemage,
        "barbarian" => HeroType::Barbarian,
        "archer" => HeroType::Archer,
        "builder" => HeroType::Builder,
        _ => HeroType::Runekeeper,
    }
}

fn load_deck_cards(
    connection: &Connection,
    deck_id: i64,
) -> Result<Vec<DeckCardCount>, DeckLibraryError> {
    let mut statement = connection.prepare(
        "
        SELECT template_id, count
        FROM deck_recipe_cards
        WHERE deck_id = ?1
        ORDER BY template_id ASC
        ",
    )?;
    let rows = statement.query_map(params![deck_id], |row| {
        Ok(DeckCardCount {
            template_id: row.get(0)?,
            count: row.get::<_, i64>(1)? as u16,
        })
    })?;
    let mut cards = Vec::new();
    for row in rows {
        cards.push(row?);
    }
    Ok(cards)
}

#[cfg(test)]
fn catalog_contains_all_system_recipe_cards() -> bool {
    let templates: std::collections::HashMap<_, _> = rune_lanes_core::starter_card_templates()
        .into_iter()
        .map(|card| (card.template_id, card.name))
        .collect();
    system_decks().iter().all(|deck| {
        deck.cards
            .iter()
            .all(|card| templates.contains_key(&card.template_id))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_and_system_decks_are_legal() {
        assert!(catalog_contains_all_system_recipe_cards());
        for deck in system_decks() {
            assert!(
                deck.legality.legal,
                "{} should be legal: {:?}",
                deck.name, deck.legality.messages
            );
            assert_eq!(deck.legality.total_cards, 60);
        }
        assert!(validate_recipe(&starter_deck_snapshot().cards).legal);
    }

    #[test]
    fn player_and_ai_system_loadouts_use_the_same_core_recipes() {
        let player = system_deck_response().decks;
        let ai = ai_lab_system_decks();
        let configured = rune_lanes_core::deck_library::system_deck_recipes();
        assert_eq!(player.len(), configured.len());
        assert_eq!(
            serde_json::to_value(&player).unwrap(),
            serde_json::to_value(ai).unwrap()
        );
        for (projected, recipe) in player.iter().zip(configured) {
            assert_eq!(projected.id, recipe.id);
            assert_eq!(projected.name, recipe.name);
            assert_eq!(projected.hero_type, recipe.hero_type);
            assert_eq!(
                serde_json::to_value(&projected.cards).unwrap(),
                serde_json::to_value(&recipe.cards).unwrap()
            );
            assert!(projected.legality.legal);
            let snapshot = system_deck_snapshot(&recipe.id).unwrap();
            let hosted_cards = deck_from_snapshot(Side::Player, &snapshot).unwrap();
            let core_cards =
                rune_lanes_core::deck_library::deck_from_counts(Side::Player, &recipe.cards)
                    .unwrap();
            assert_eq!(
                serde_json::to_value(hosted_cards).unwrap(),
                serde_json::to_value(core_cards).unwrap()
            );
        }
    }

    #[test]
    fn materialization_preserves_legacy_order_and_unknown_template_errors() {
        let cards = deck_from_counts(
            Side::Opponent,
            &[
                DeckCardCount {
                    template_id: "spark-jolt".to_string(),
                    count: 1,
                },
                DeckCardCount {
                    template_id: "ember-squire".to_string(),
                    count: 1,
                },
            ],
        )
        .unwrap();
        assert_eq!(
            cards
                .iter()
                .map(|card| card.id.as_str())
                .collect::<Vec<_>>(),
            vec!["o-0-spark-jolt", "o-0-ember-squire"]
        );
        assert!(matches!(deck_from_counts(Side::Player, &[
            DeckCardCount { template_id: "missing".to_string(), count: 1 },
        ]), Err(DeckLibraryError::UnknownTemplate(id)) if id == "missing"));
    }

    #[test]
    fn validator_rejects_too_few_cards() {
        let legality = validate_recipe(&[DeckCardCount {
            template_id: "ember-squire".to_string(),
            count: 5,
        }]);
        assert!(!legality.legal);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("at least 60"))
        );
    }

    #[test]
    fn validator_rejects_copy_and_rarity_caps() {
        let legality = validate_recipe(&[
            DeckCardCount {
                template_id: "ember-squire".to_string(),
                count: 6,
            },
            DeckCardCount {
                template_id: "blade-dancer".to_string(),
                count: 5,
            },
            DeckCardCount {
                template_id: "iron-colossus".to_string(),
                count: 4,
            },
            DeckCardCount {
                template_id: "shield-adept".to_string(),
                count: 20,
            },
            DeckCardCount {
                template_id: "starfire-bolt".to_string(),
                count: 10,
            },
        ]);

        assert!(!legality.legal);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("Basic cards"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("Advanced cards"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("Rare cards"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("at most 24"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("at most 12"))
        );
    }

    #[test]
    fn validator_rejects_unknown_templates() {
        let legality = validate_recipe(&[DeckCardCount {
            template_id: "missing-card".to_string(),
            count: 60,
        }]);
        assert!(!legality.legal);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("not in the card catalog"))
        );
    }
}
