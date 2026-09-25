use std::error::Error;
use std::fmt;

use rand::RngCore;
use rand::rngs::OsRng;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};

use rune_lanes_core::{
    CardDefinition, CardDefinitionValidationError, PublishedCardRevision,
    PublishedCardRevisionError,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCardDraftRequest {
    pub definition: CardDefinition,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCardDraftRequest {
    pub version: u64,
    pub definition: CardDefinition,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishCardDraftRequest {
    pub version: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDraft {
    pub id: i64,
    pub version: u64,
    pub catalog_id: String,
    pub definition: CardDefinition,
    pub validation_errors: Vec<CardDefinitionValidationError>,
    pub source_revision: Option<u32>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDraftListResponse {
    pub drafts: Vec<CardDraft>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedCardRevisionRecord {
    pub revision: PublishedCardRevision,
    pub created_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRevisionHistoryResponse {
    pub card_id: String,
    pub revisions: Vec<PublishedCardRevisionRecord>,
}

#[derive(Debug)]
pub enum CardWorkshopError {
    Sqlite(rusqlite::Error),
    Snapshot(serde_json::Error),
    NotFound,
    VersionConflict {
        expected: u64,
        current: u64,
    },
    InvalidDefinition {
        errors: Vec<CardDefinitionValidationError>,
    },
    CardIdConflict(String),
    PublishedCardIdImmutable {
        current: String,
        requested: String,
    },
    InvalidPublishedRevision(PublishedCardRevisionError),
}

impl fmt::Display for CardWorkshopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sqlite(error) => write!(
                formatter,
                "could not access card workshop database: {error}"
            ),
            Self::Snapshot(error) => {
                write!(formatter, "could not read card workshop content: {error}")
            }
            Self::NotFound => formatter.write_str("Card workshop content was not found."),
            Self::VersionConflict { expected, current } => write!(
                formatter,
                "Card draft changed since version {expected}; current version is {current}."
            ),
            Self::InvalidDefinition { errors } => write!(
                formatter,
                "Card definition has {} validation error(s).",
                errors.len()
            ),
            Self::CardIdConflict(card_id) => {
                write!(
                    formatter,
                    "Card id {card_id} is already used by another custom card."
                )
            }
            Self::PublishedCardIdImmutable { current, requested } => write!(
                formatter,
                "Published card id cannot change from {current} to {requested}."
            ),
            Self::InvalidPublishedRevision(error) => error.fmt(formatter),
        }
    }
}

impl Error for CardWorkshopError {}

impl From<rusqlite::Error> for CardWorkshopError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for CardWorkshopError {
    fn from(error: serde_json::Error) -> Self {
        Self::Snapshot(error)
    }
}

impl From<PublishedCardRevisionError> for CardWorkshopError {
    fn from(error: PublishedCardRevisionError) -> Self {
        Self::InvalidPublishedRevision(error)
    }
}

pub struct CardWorkshop<'a> {
    connection: &'a mut Connection,
}

impl<'a> CardWorkshop<'a> {
    pub fn new(connection: &'a mut Connection) -> Self {
        Self { connection }
    }

    pub fn list_drafts_for_user(
        &self,
        user_id: i64,
    ) -> Result<CardDraftListResponse, CardWorkshopError> {
        let mut statement = self.connection.prepare(
            "
            SELECT id, version, catalog_id, definition_json, source_revision, created_at, updated_at
            FROM card_drafts
            WHERE user_id = ?1
            ORDER BY updated_at DESC, id DESC
            ",
        )?;
        let rows = statement.query_map(params![user_id], read_draft_row)?;
        let mut drafts = Vec::new();
        for row in rows {
            drafts.push(row?);
        }
        Ok(CardDraftListResponse { drafts })
    }

    pub fn load_draft_for_user(
        &self,
        user_id: i64,
        draft_id: i64,
    ) -> Result<Option<CardDraft>, CardWorkshopError> {
        load_draft_for_user(self.connection, user_id, draft_id)
    }

    pub fn create_draft_for_user(
        &mut self,
        user_id: i64,
        request: CreateCardDraftRequest,
    ) -> Result<CardDraft, CardWorkshopError> {
        self.insert_draft(user_id, request.definition, new_custom_catalog_id(), None)
    }

    pub fn update_draft_for_user(
        &mut self,
        user_id: i64,
        draft_id: i64,
        request: UpdateCardDraftRequest,
    ) -> Result<CardDraft, CardWorkshopError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = load_draft_for_user(&transaction, user_id, draft_id)?
            .ok_or(CardWorkshopError::NotFound)?;
        if current.version != request.version {
            return Err(CardWorkshopError::VersionConflict {
                expected: request.version,
                current: current.version,
            });
        }

        if current.definition.id != request.definition.id {
            let has_publication = transaction.query_row(
                "
                SELECT EXISTS(
                    SELECT 1
                    FROM card_revisions
                    WHERE core_card_id = ?1
                    LIMIT 1
                )
                ",
                params![current.catalog_id],
                |row| row.get::<_, i64>(0),
            )? == 1;
            if has_publication {
                return Err(CardWorkshopError::PublishedCardIdImmutable {
                    current: current.definition.id,
                    requested: request.definition.id,
                });
            }
        }

        let definition_json = serde_json::to_string(&request.definition)?;
        let changed = transaction.execute(
            "
            UPDATE card_drafts
            SET definition_json = ?4,
                version = version + 1,
                updated_at = unixepoch()
            WHERE id = ?1 AND user_id = ?2 AND version = ?3
            ",
            params![draft_id, user_id, request.version, definition_json],
        )?;
        if changed == 0 {
            let current = load_draft_for_user(&transaction, user_id, draft_id)?
                .ok_or(CardWorkshopError::NotFound)?;
            return Err(CardWorkshopError::VersionConflict {
                expected: request.version,
                current: current.version,
            });
        }

        let updated = load_draft_for_user(&transaction, user_id, draft_id)?
            .ok_or(CardWorkshopError::NotFound)?;
        transaction.commit()?;
        Ok(updated)
    }

    pub fn delete_draft_for_user(
        &mut self,
        user_id: i64,
        draft_id: i64,
    ) -> Result<bool, CardWorkshopError> {
        Ok(self.connection.execute(
            "DELETE FROM card_drafts WHERE id = ?1 AND user_id = ?2",
            params![draft_id, user_id],
        )? == 1)
    }

    pub fn publish_draft_for_user(
        &mut self,
        user_id: i64,
        draft_id: i64,
        request: PublishCardDraftRequest,
    ) -> Result<PublishedCardRevisionRecord, CardWorkshopError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) =
            published_for_source_version(&transaction, user_id, draft_id, request.version)?
        {
            transaction.commit()?;
            return Ok(existing);
        }

        let draft = load_draft_for_user(&transaction, user_id, draft_id)?
            .ok_or(CardWorkshopError::NotFound)?;
        if draft.version != request.version {
            return Err(CardWorkshopError::VersionConflict {
                expected: request.version,
                current: draft.version,
            });
        }

        let validation_errors = draft.definition.validation_errors();
        if !validation_errors.is_empty() {
            return Err(CardWorkshopError::InvalidDefinition {
                errors: validation_errors,
            });
        }

        let existing_catalog_id: Option<String> = transaction
            .query_row(
                "
                SELECT core_card_id
                FROM card_revisions
                WHERE user_id = ?1 AND card_id = ?2
                LIMIT 1
                ",
                params![user_id, draft.definition.id],
                |row| row.get(0),
            )
            .optional()?;
        if existing_catalog_id
            .as_deref()
            .is_some_and(|catalog_id| catalog_id != draft.catalog_id)
        {
            return Err(CardWorkshopError::CardIdConflict(
                draft.definition.id.clone(),
            ));
        }

        let latest_revision: Option<u32> = transaction.query_row(
            "
            SELECT MAX(revision)
            FROM card_revisions
            WHERE core_card_id = ?1
            ",
            params![draft.catalog_id],
            |row| row.get(0),
        )?;
        let revision_number = latest_revision.unwrap_or(0).saturating_add(1);

        let local_card_id = draft.definition.id.clone();
        let mut published_definition = draft.definition;
        published_definition.id = draft.catalog_id.clone();
        let revision = PublishedCardRevision::new(published_definition, revision_number)?;
        let revision_json = serde_json::to_string(&revision)?;
        transaction.execute(
            "
            INSERT INTO card_revisions (
                user_id,
                card_id,
                core_card_id,
                revision,
                revision_json,
                source_draft_id,
                source_draft_version,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, unixepoch())
            ",
            params![
                user_id,
                local_card_id,
                revision.id().card_id(),
                revision.id().revision(),
                revision_json,
                draft_id,
                draft.version
            ],
        )?;
        let created_at = transaction.query_row(
            "
            SELECT created_at
            FROM card_revisions
            WHERE core_card_id = ?1 AND revision = ?2
            ",
            params![revision.id().card_id(), revision.id().revision()],
            |row| row.get(0),
        )?;
        transaction.commit()?;

        Ok(PublishedCardRevisionRecord {
            revision,
            created_at,
        })
    }

    pub fn revision_history_for_user(
        &self,
        user_id: i64,
        card_id: &str,
    ) -> Result<CardRevisionHistoryResponse, CardWorkshopError> {
        let mut statement = self.connection.prepare(
            "
            SELECT revision_json, created_at
            FROM card_revisions
            WHERE user_id = ?1 AND card_id = ?2
            ORDER BY revision DESC
            ",
        )?;
        let rows = statement.query_map(params![user_id, card_id], |row| {
            let revision_json: String = row.get(0)?;
            let created_at: i64 = row.get(1)?;
            Ok((revision_json, created_at))
        })?;

        let mut revisions = Vec::new();
        for row in rows {
            let (revision_json, created_at) = row?;
            revisions.push(PublishedCardRevisionRecord {
                revision: serde_json::from_str(&revision_json)?,
                created_at,
            });
        }

        Ok(CardRevisionHistoryResponse {
            card_id: card_id.to_string(),
            revisions,
        })
    }

    pub fn fork_revision_for_user(
        &mut self,
        user_id: i64,
        card_id: &str,
        revision: u32,
    ) -> Result<CardDraft, CardWorkshopError> {
        let revision_json: Option<String> = self
            .connection
            .query_row(
                "
                SELECT revision_json
                FROM card_revisions
                WHERE user_id = ?1 AND card_id = ?2 AND revision = ?3
                ",
                params![user_id, card_id, revision],
                |row| row.get(0),
            )
            .optional()?;
        let revision_json = revision_json.ok_or(CardWorkshopError::NotFound)?;
        let published: PublishedCardRevision = serde_json::from_str(&revision_json)?;
        let mut definition = published.definition().clone();
        definition.id = card_id.to_string();
        self.insert_draft(
            user_id,
            definition,
            published.id().card_id().to_string(),
            Some(published.id().revision()),
        )
    }

    fn insert_draft(
        &mut self,
        user_id: i64,
        definition: CardDefinition,
        catalog_id: String,
        source_revision: Option<u32>,
    ) -> Result<CardDraft, CardWorkshopError> {
        let definition_json = serde_json::to_string(&definition)?;
        self.connection.execute(
            "
            INSERT INTO card_drafts (
                user_id,
                version,
                catalog_id,
                definition_json,
                source_revision,
                created_at,
                updated_at
            )
            VALUES (?1, 1, ?2, ?3, ?4, unixepoch(), unixepoch())
            ",
            params![user_id, catalog_id, definition_json, source_revision],
        )?;
        let draft_id = self.connection.last_insert_rowid();
        self.load_draft_for_user(user_id, draft_id)?
            .ok_or(CardWorkshopError::NotFound)
    }

}

fn load_draft_for_user(
    connection: &Connection,
    user_id: i64,
    draft_id: i64,
) -> Result<Option<CardDraft>, CardWorkshopError> {
    connection
        .query_row(
            "
            SELECT id, version, catalog_id, definition_json, source_revision, created_at, updated_at
            FROM card_drafts
            WHERE id = ?1 AND user_id = ?2
            ",
            params![draft_id, user_id],
            read_draft_row,
        )
        .optional()
        .map_err(CardWorkshopError::from)
}

fn published_for_source_version(
    connection: &Connection,
    user_id: i64,
    draft_id: i64,
    draft_version: u64,
) -> Result<Option<PublishedCardRevisionRecord>, CardWorkshopError> {
    let row: Option<(String, i64)> = connection
        .query_row(
            "
            SELECT revision_json, created_at
            FROM card_revisions
            WHERE user_id = ?1 AND source_draft_id = ?2 AND source_draft_version = ?3
            ",
            params![user_id, draft_id, draft_version],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    row.map(|(revision_json, created_at)| {
        Ok(PublishedCardRevisionRecord {
            revision: serde_json::from_str(&revision_json)?,
            created_at,
        })
    })
    .transpose()
}

fn read_draft_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CardDraft> {
    let definition_json: String = row.get(3)?;
    let definition: CardDefinition = serde_json::from_str(&definition_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let validation_errors = definition.validation_errors();
    Ok(CardDraft {
        id: row.get(0)?,
        version: row.get(1)?,
        catalog_id: row.get(2)?,
        definition,
        validation_errors,
        source_revision: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn new_custom_catalog_id() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    let mut id = String::with_capacity("custom-".len() + bytes.len() * 2);
    id.push_str("custom-");
    for byte in bytes {
        id.push_str(&format!("{byte:02x}"));
    }
    id
}

pub fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS card_drafts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            catalog_id TEXT NOT NULL,
            definition_json TEXT NOT NULL,
            source_revision INTEGER,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS card_drafts_user_id_idx
            ON card_drafts(user_id);

        CREATE TABLE IF NOT EXISTS card_revisions (
            user_id INTEGER NOT NULL,
            card_id TEXT NOT NULL,
            core_card_id TEXT NOT NULL,
            revision INTEGER NOT NULL,
            revision_json TEXT NOT NULL,
            source_draft_id INTEGER NOT NULL,
            source_draft_version INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (core_card_id, revision),
            UNIQUE (user_id, source_draft_id, source_draft_version),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS card_revisions_user_card_idx
            ON card_revisions(user_id, card_id, revision DESC);
        CREATE INDEX IF NOT EXISTS card_revisions_core_card_idx
            ON card_revisions(core_card_id, revision DESC);
        ",
    )?;
    Ok(())
}
