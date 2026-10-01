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
        let catalogs = load_transfer_catalogs(self.connection, user_id, id)?;
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
    let catalogs = load_transfer_catalogs(connection, user_id, id)?;
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

struct TransferCatalogs {
    global: CardCatalog,
    account: CardCatalog,
}

fn load_transfer_catalogs(
    connection: &Connection,
    user_id: i64,
    id: &CardRevisionId,
) -> Result<TransferCatalogs, CardWorkshopError> {
    let mut published = starter_card_catalog().resolve(id).cloned();
    let mut account_has_access = published.is_some();
    let mut statement = connection.prepare(
        "SELECT user_id, revision_json FROM card_revisions WHERE core_card_id = ?1 AND revision = ?2
         UNION ALL
         SELECT user_id, revision_json FROM card_imports WHERE core_card_id = ?1 AND revision = ?2",
    )?;
    let rows = statement.query_map(params![id.card_id(), id.revision()], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (owner, json) = row?;
        let revision: PublishedCardRevision = serde_json::from_str(&json)?;
        if revision.id() != id
            || published
                .as_ref()
                .is_some_and(|existing| existing != &revision)
        {
            return Err(CardWorkshopError::IncompatibleStoredRevision(id.clone()));
        }
        account_has_access |= owner == user_id;
        published.get_or_insert(revision);
    }
    let global = CardCatalog::new(published.iter().cloned()).map_err(CardTransferError::Catalog)?;
    let account = CardCatalog::new(published.filter(|_| account_has_access))
        .map_err(CardTransferError::Catalog)?;
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
