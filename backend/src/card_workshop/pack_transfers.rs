use super::{CardWorkshop, CardWorkshopError, transfers};
use rune_lanes_core::{
    CardPackCatalog, CardPackImport, CardPackImportPreview, CardPackManifest, CardPackRevisionId,
    CardPackRevisionImportStatus, CardPackTransfer, CardPackTransferError,
    CardRevisionImportStatus,
};
use rusqlite::{Connection, TransactionBehavior, params};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

impl CardWorkshop<'_> {
    pub fn preview_pack_for_user(
        &self,
        user_id: i64,
        transfer: &CardPackTransfer,
    ) -> Result<CardPackImportPreview, CardWorkshopError> {
        let catalogs = load_catalogs(self.connection, user_id)?;
        Ok(catalogs.prepare_import(transfer).preview)
    }
    pub fn export_pack_for_user(
        &self,
        user_id: i64,
        id: &CardPackRevisionId,
    ) -> Result<CardPackTransfer, CardWorkshopError> {
        let catalogs = load_catalogs(self.connection, user_id)?;
        Ok(CardPackTransfer::export(
            &catalogs.account,
            &catalogs.cards.account,
            id,
        )?)
    }

    pub fn import_pack_for_user(
        &mut self,
        user_id: i64,
        transfer: CardPackTransfer,
    ) -> Result<CardPackImportPreview, CardWorkshopError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let catalogs = load_catalogs(&transaction, user_id)?;
        let prepared = catalogs.prepare_import(&transfer);
        prepared.global.check_conflicts()?;
        let preview = prepared.preview;
        for (card, disposition) in transfer.cards().revisions().zip(&preview.cards) {
            if disposition.status == CardRevisionImportStatus::NewRevision {
                transaction.execute(
                    "INSERT INTO card_imports (user_id, core_card_id, revision, revision_json) VALUES (?1, ?2, ?3, ?4)",
                    params![user_id, card.id().card_id(), card.id().revision(), serde_json::to_string(card)?],
                )?;
            }
        }
        for (pack, disposition) in transfer.packs().manifests().zip(&preview.packs) {
            if disposition.status == CardPackRevisionImportStatus::NewRevision {
                transaction.execute(
                    "INSERT INTO card_pack_imports (user_id, pack_id, version, manifest_json) VALUES (?1, ?2, ?3, ?4)",
                    params![user_id, pack.id.pack_id(), pack.id.version(), serde_json::to_string(pack)?],
                )?;
            }
        }
        transaction.commit()?;
        Ok(preview)
    }
}

struct PackTransferCatalogs {
    cards: transfers::TransferCatalogs,
    global: CardPackCatalog,
    account: CardPackCatalog,
}

struct PreparedPackImport<'a> {
    global: CardPackImport<'a>,
    preview: CardPackImportPreview,
}

impl PackTransferCatalogs {
    fn prepare_import<'a>(&'a self, transfer: &'a CardPackTransfer) -> PreparedPackImport<'a> {
        let global = transfer.prepare_import(&self.cards.global, &self.global);
        let mut preview = transfer
            .prepare_import(&self.cards.account, &self.account)
            .preview()
            .clone();
        for (global, account) in global.preview().cards.iter().zip(&mut preview.cards) {
            if global.status == CardRevisionImportStatus::IdentityConflict {
                account.status = global.status;
            }
        }
        for (global, account) in global.preview().packs.iter().zip(&mut preview.packs) {
            if global.status == CardPackRevisionImportStatus::IdentityConflict {
                account.status = global.status;
            }
        }
        PreparedPackImport { global, preview }
    }
}

fn load_catalogs(
    connection: &Connection,
    user_id: i64,
) -> Result<PackTransferCatalogs, CardWorkshopError> {
    let cards = transfers::load_card_catalogs(connection, user_id, None)?;
    let mut global: BTreeMap<CardPackRevisionId, CardPackManifest> = BTreeMap::new();
    let mut accessible = BTreeSet::new();
    let mut statement = connection
        .prepare("SELECT user_id, pack_id, version, manifest_json FROM card_pack_imports")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, u32>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (owner, pack_id, version, json) = row?;
        let manifest: CardPackManifest = serde_json::from_str(&json)?;
        if manifest.id.pack_id() != pack_id || manifest.id.version() != version {
            return Err(CardWorkshopError::IncompatibleStoredPack(manifest.id));
        }
        if owner == user_id {
            accessible.insert(manifest.id.clone());
        }
        match global.entry(manifest.id.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(manifest);
            }
            Entry::Occupied(entry) if entry.get() != &manifest => {
                return Err(CardWorkshopError::IncompatibleStoredPack(manifest.id));
            }
            Entry::Occupied(_) => {}
        }
    }
    let account = CardPackCatalog::new(
        global
            .values()
            .filter(|manifest| accessible.contains(&manifest.id))
            .cloned(),
        &cards.account,
    )
    .map_err(CardPackTransferError::InvalidPacks)?;
    let global = CardPackCatalog::new(global.into_values(), &cards.global)
        .map_err(CardPackTransferError::InvalidPacks)?;
    Ok(PackTransferCatalogs {
        cards,
        global,
        account,
    })
}

pub(super) fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS card_pack_imports (
            user_id INTEGER NOT NULL,
            pack_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            manifest_json TEXT NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (user_id, pack_id, version),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS card_pack_imports_identity_idx ON card_pack_imports(pack_id, version);",
    )
}
