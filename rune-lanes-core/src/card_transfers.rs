use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{CardCatalog, CardCatalogError, CardRevisionId, PublishedCardRevision};

pub const CARD_REVISION_TRANSFER_SCHEMA_VERSION: u32 = 1;

/// Portable content for one exact published revision, independent of Accounts.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardRevisionTransfer {
    schema_version: u32,
    revision: PublishedCardRevision,
}

impl<'de> Deserialize<'de> for CardRevisionTransfer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct UncheckedTransfer {
            schema_version: u32,
            revision: PublishedCardRevision,
        }
        let unchecked = UncheckedTransfer::deserialize(deserializer)?;
        if unchecked.schema_version != CARD_REVISION_TRANSFER_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported Card revision transfer schema {}; expected {}",
                unchecked.schema_version, CARD_REVISION_TRANSFER_SCHEMA_VERSION
            )));
        }
        Ok(Self {
            schema_version: unchecked.schema_version,
            revision: unchecked.revision,
        })
    }
}

impl CardRevisionTransfer {
    pub fn export(catalog: &CardCatalog, id: &CardRevisionId) -> Result<Self, CardTransferError> {
        let revision = catalog
            .resolve(id)
            .ok_or_else(|| CardTransferError::MissingRevision(id.clone()))?;
        Ok(Self {
            schema_version: CARD_REVISION_TRANSFER_SCHEMA_VERSION,
            revision: revision.clone(),
        })
    }

    pub fn revision(&self) -> &PublishedCardRevision {
        &self.revision
    }

    pub fn prepare_import<'a>(&'a self, destination: &'a CardCatalog) -> CardRevisionImport<'a> {
        let status = match destination.resolve(self.revision.id()) {
            Some(existing) if existing == &self.revision => {
                CardRevisionImportStatus::AlreadyPresent
            }
            Some(_) => CardRevisionImportStatus::IdentityConflict,
            None => CardRevisionImportStatus::NewRevision,
        };
        CardRevisionImport {
            destination,
            revision: &self.revision,
            status,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CardRevisionImportStatus {
    NewRevision,
    AlreadyPresent,
    IdentityConflict,
}

/// Borrows immutable source and destination content so preview cannot go stale.
pub struct CardRevisionImport<'a> {
    destination: &'a CardCatalog,
    revision: &'a PublishedCardRevision,
    status: CardRevisionImportStatus,
}

impl CardRevisionImport<'_> {
    pub fn status(&self) -> CardRevisionImportStatus {
        self.status
    }

    /// Creates an owned catalog; the destination is never changed in place.
    pub fn into_catalog(self) -> Result<CardCatalog, CardTransferError> {
        match self.status {
            CardRevisionImportStatus::AlreadyPresent => return Ok(self.destination.clone()),
            CardRevisionImportStatus::IdentityConflict => {
                return Err(CardTransferError::IdentityConflict(
                    self.revision.id().clone(),
                ));
            }
            CardRevisionImportStatus::NewRevision => {}
        }
        CardCatalog::new(
            self.destination
                .revisions()
                .cloned()
                .chain(std::iter::once(self.revision.clone())),
        )
        .map_err(CardTransferError::Catalog)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardTransferError {
    MissingRevision(CardRevisionId),
    IdentityConflict(CardRevisionId),
    Catalog(CardCatalogError),
}

impl fmt::Display for CardTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRevision(id) => write!(formatter, "missing Card revision {id}"),
            Self::IdentityConflict(id) => {
                write!(
                    formatter,
                    "Card revision {id} already has different content"
                )
            }
            Self::Catalog(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CardTransferError {}

#[cfg(test)]
mod tests;
