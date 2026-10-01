use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use super::{CardTransferPreview, CardWorkshop, CardWorkshopError};
use rune_lanes_core::{
    CardCatalog, CardRevisionId, CardRevisionImportStatus, CardRevisionTransfer, CardTransferError,
    PublishedCardRevision, starter_card_catalog,
};
use rusqlite::{Connection, TransactionBehavior, params};

impl CardWorkshop<'_> {
    pub fn export_revision_for_user(
        &self,
        user_id: i64,
        id: &CardRevisionId,
    ) -> Result<CardRevisionTransfer, CardWorkshopError> {
        let catalogs = load_card_catalogs(self.connection, user_id, Some(id))?;
        Ok(CardRevisionTransfer::export(&catalogs.account, id)?)
    }

    pub fn preview_transfer_for_user(
        &self,
        user_id: i64,
        transfer: &CardRevisionTransfer,
    ) -> Result<CardTransferPreview, CardWorkshopError> {
        preview_transfer(self.connection, user_id, transfer)
    }

    pub fn import_transfer_for_user(
        &mut self,
        user_id: i64,
        transfer: CardRevisionTransfer,
    ) -> Result<CardTransferPreview, CardWorkshopError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let preview = preview_transfer(&transaction, user_id, &transfer)?;
        match preview.status {
            CardRevisionImportStatus::IdentityConflict => {
                return Err(CardTransferError::IdentityConflict(preview.revision_id).into());
            }
            CardRevisionImportStatus::AlreadyPresent => {}
            CardRevisionImportStatus::NewRevision => {
                transaction.execute(
                    "INSERT INTO card_imports (user_id, core_card_id, revision, revision_json)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        user_id,
                        preview.revision_id.card_id(),
                        preview.revision_id.revision(),
                        serde_json::to_string(transfer.revision())?
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(preview)
    }
}

fn preview_transfer(
    connection: &Connection,
    user_id: i64,
    transfer: &CardRevisionTransfer,
) -> Result<CardTransferPreview, CardWorkshopError> {
    let id = transfer.revision().id();
    let catalogs = load_card_catalogs(connection, user_id, Some(id))?;
    let global_status = transfer.prepare_import(&catalogs.global).status();
    let status = if global_status == CardRevisionImportStatus::IdentityConflict {
        global_status
    } else {
        transfer.prepare_import(&catalogs.account).status()
    };
    Ok(CardTransferPreview {
        revision_id: id.clone(),
        status,
    })
}

pub(super) struct TransferCatalogs {
    pub global: CardCatalog,
    pub account: CardCatalog,
}

pub(super) fn load_card_catalogs(
    connection: &Connection,
    user_id: i64,
    id: Option<&CardRevisionId>,
) -> Result<TransferCatalogs, CardWorkshopError> {
    let mut published: BTreeMap<_, _> = starter_card_catalog()
        .revisions()
        .filter(|revision| id.is_none_or(|id| revision.id() == id))
        .map(|revision| (revision.id().clone(), revision.clone()))
        .collect();
    let mut accessible: BTreeSet<_> = published.keys().cloned().collect();
    let query = if id.is_some() {
        "SELECT user_id, core_card_id, revision, revision_json FROM card_revisions
         WHERE core_card_id = ?1 AND revision = ?2
         UNION ALL
         SELECT user_id, core_card_id, revision, revision_json FROM card_imports
         WHERE core_card_id = ?1 AND revision = ?2"
    } else {
        "SELECT user_id, core_card_id, revision, revision_json FROM card_revisions
         UNION ALL
         SELECT user_id, core_card_id, revision, revision_json FROM card_imports"
    };
    let mut statement = connection.prepare(query)?;
    let mut rows = match id {
        Some(id) => statement.query(params![id.card_id(), id.revision()])?,
        None => statement.query([])?,
    };
    while let Some(row) = rows.next()? {
        let owner: i64 = row.get(0)?;
        let card_id: String = row.get(1)?;
        let number: u32 = row.get(2)?;
        let json: String = row.get(3)?;
        let revision: PublishedCardRevision = serde_json::from_str(&json)?;
        if revision.id().card_id() != card_id || revision.id().revision() != number {
            return Err(CardWorkshopError::IncompatibleStoredRevision(
                revision.id().clone(),
            ));
        }
        if owner == user_id {
            accessible.insert(revision.id().clone());
        }
        match published.entry(revision.id().clone()) {
            Entry::Vacant(entry) => {
                entry.insert(revision);
            }
            Entry::Occupied(entry) if entry.get() != &revision => {
                return Err(CardWorkshopError::IncompatibleStoredRevision(
                    revision.id().clone(),
                ));
            }
            Entry::Occupied(_) => {}
        }
    }
    let account = CardCatalog::new(
        published
            .values()
            .filter(|revision| accessible.contains(revision.id()))
            .cloned(),
    )
    .map_err(CardTransferError::Catalog)?;
    let global = CardCatalog::new(published.into_values()).map_err(CardTransferError::Catalog)?;
    Ok(TransferCatalogs { global, account })
}

pub(super) fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS card_imports (
            user_id INTEGER NOT NULL,
            core_card_id TEXT NOT NULL,
            revision INTEGER NOT NULL,
            revision_json TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (user_id, core_card_id, revision),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS card_imports_identity_idx ON card_imports(core_card_id, revision);",
    )
}

pub(super) fn ensure_publication_compatible(
    connection: &Connection,
    user_id: i64,
    revision: &PublishedCardRevision,
) -> Result<(), CardWorkshopError> {
    let transfer = CardRevisionTransfer::new(revision.clone());
    let preview = preview_transfer(connection, user_id, &transfer)?;
    if preview.status == CardRevisionImportStatus::IdentityConflict {
        return Err(CardTransferError::IdentityConflict(preview.revision_id).into());
    }
    Ok(())
}
