use std::collections::BTreeSet;
use std::fmt;

use crate::card_transfers::card_revision_import_status;
use serde::{Deserialize, Serialize};

use crate::{
    CardCatalog, CardCatalogError, CardPackCatalog, CardPackManifest, CardPackRevisionId,
    CardPackValidationError, CardRevisionId, CardRevisionImportStatus, PublishedCardRevision,
};

pub const CARD_PACK_TRANSFER_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug)]
pub struct CardPackTransfer {
    root: CardPackRevisionId,
    cards: CardCatalog,
    packs: CardPackCatalog,
}

impl Serialize for CardPackTransfer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Document<'a> {
            schema_version: u32,
            root: &'a CardPackRevisionId,
            cards: Vec<&'a PublishedCardRevision>,
            manifests: Vec<&'a CardPackManifest>,
        }
        Document {
            schema_version: CARD_PACK_TRANSFER_SCHEMA_VERSION,
            root: &self.root,
            cards: self.cards.revisions().collect(),
            manifests: self.packs.manifests().collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CardPackTransfer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Document {
            schema_version: u32,
            root: CardPackRevisionId,
            cards: Vec<PublishedCardRevision>,
            manifests: Vec<CardPackManifest>,
        }
        let document = Document::deserialize(deserializer)?;
        if document.schema_version != CARD_PACK_TRANSFER_SCHEMA_VERSION {
            return Err(serde::de::Error::custom(format!(
                "unsupported Card pack transfer schema {}; expected {}",
                document.schema_version, CARD_PACK_TRANSFER_SCHEMA_VERSION
            )));
        }
        Self::from_parts(document.root, document.cards, document.manifests)
            .map_err(serde::de::Error::custom)
    }
}

impl CardPackTransfer {
    fn from_parts(
        root: CardPackRevisionId,
        revisions: Vec<PublishedCardRevision>,
        manifests: Vec<CardPackManifest>,
    ) -> Result<Self, CardPackTransferError> {
        let cards = CardCatalog::new(revisions).map_err(CardPackTransferError::Cards)?;
        let packs =
            CardPackCatalog::new(manifests, &cards).map_err(CardPackTransferError::InvalidPacks)?;
        let closure = dependency_closure(&packs, &root)?;
        if let Some(extra) = packs
            .manifests()
            .find(|pack| !closure.pack_ids.contains(&pack.id))
        {
            return Err(CardPackTransferError::UnreferencedPack(extra.id.clone()));
        }
        if let Some(extra) = cards
            .revisions()
            .find(|card| !closure.card_ids.contains(card.id()))
        {
            return Err(CardPackTransferError::UnreferencedCard(extra.id().clone()));
        }
        Ok(Self { root, cards, packs })
    }

    pub fn export(
        packs: &CardPackCatalog,
        cards: &CardCatalog,
        root: &CardPackRevisionId,
    ) -> Result<Self, CardPackTransferError> {
        let closure = dependency_closure(packs, root)?;
        let manifests = packs
            .manifests()
            .filter(|pack| closure.pack_ids.contains(&pack.id))
            .cloned()
            .collect();
        let revisions = closure
            .card_ids
            .into_iter()
            .map(|id| {
                cards
                    .resolve(&id)
                    .cloned()
                    .ok_or(CardPackTransferError::MissingCard(id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_parts(root.clone(), revisions, manifests)
    }

    pub fn root(&self) -> &CardPackRevisionId {
        &self.root
    }
    pub fn cards(&self) -> &CardCatalog {
        &self.cards
    }
    pub fn packs(&self) -> &CardPackCatalog {
        &self.packs
    }

    pub fn prepare_import<'a>(
        &'a self,
        destination_cards: &'a CardCatalog,
        destination_packs: &'a CardPackCatalog,
    ) -> CardPackImport<'a> {
        let cards = self
            .cards
            .revisions()
            .map(|revision| CardRevisionImportPreview {
                id: revision.id().clone(),
                status: card_revision_import_status(revision, destination_cards),
            })
            .collect();
        let packs = self
            .packs
            .manifests()
            .map(|manifest| {
                let status = match destination_packs.resolve(&manifest.id) {
                    Some(existing) if existing == manifest => {
                        CardPackRevisionImportStatus::AlreadyPresent
                    }
                    Some(_) => CardPackRevisionImportStatus::IdentityConflict,
                    None => CardPackRevisionImportStatus::NewRevision,
                };
                CardPackRevisionImportPreview {
                    id: manifest.id.clone(),
                    status,
                }
            })
            .collect();
        CardPackImport {
            transfer: self,
            destination_cards,
            destination_packs,
            preview: CardPackImportPreview { cards, packs },
        }
    }
}

struct PackDependencyClosure {
    pack_ids: BTreeSet<CardPackRevisionId>,
    card_ids: BTreeSet<CardRevisionId>,
}

fn dependency_closure(
    packs: &CardPackCatalog,
    root: &CardPackRevisionId,
) -> Result<PackDependencyClosure, CardPackTransferError> {
    let mut pending = vec![root.clone()];
    let mut pack_ids = BTreeSet::new();
    let mut card_ids = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !pack_ids.insert(id.clone()) {
            continue;
        }
        let manifest = packs
            .resolve(&id)
            .ok_or_else(|| CardPackTransferError::MissingPack(id.clone()))?;
        pending.extend(manifest.dependencies.iter().cloned());
        card_ids.extend(manifest.cards.iter().cloned());
    }
    Ok(PackDependencyClosure { pack_ids, card_ids })
}

pub struct CardPackImport<'a> {
    transfer: &'a CardPackTransfer,
    destination_cards: &'a CardCatalog,
    destination_packs: &'a CardPackCatalog,
    preview: CardPackImportPreview,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CardRevisionImportPreview {
    pub id: CardRevisionId,
    pub status: CardRevisionImportStatus,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CardPackRevisionImportStatus {
    NewRevision,
    AlreadyPresent,
    IdentityConflict,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CardPackRevisionImportPreview {
    pub id: CardPackRevisionId,
    pub status: CardPackRevisionImportStatus,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CardPackImportPreview {
    pub cards: Vec<CardRevisionImportPreview>,
    pub packs: Vec<CardPackRevisionImportPreview>,
}

#[derive(Debug)]
pub struct CardPackImportResult {
    pub cards: CardCatalog,
    pub packs: CardPackCatalog,
}

impl CardPackImport<'_> {
    pub fn preview(&self) -> &CardPackImportPreview {
        &self.preview
    }

    /// Check the cached identity dispositions without constructing result catalogs.
    pub fn check_conflicts(&self) -> Result<(), CardPackTransferError> {
        if let Some(conflict) = self
            .preview
            .cards
            .iter()
            .find(|card| card.status == CardRevisionImportStatus::IdentityConflict)
        {
            return Err(CardPackTransferError::CardIdentityConflict(
                conflict.id.clone(),
            ));
        }
        if let Some(conflict) = self
            .preview
            .packs
            .iter()
            .find(|pack| pack.status == CardPackRevisionImportStatus::IdentityConflict)
        {
            return Err(CardPackTransferError::PackIdentityConflict(
                conflict.id.clone(),
            ));
        }
        Ok(())
    }

    pub fn into_catalogs(self) -> Result<CardPackImportResult, CardPackTransferError> {
        self.check_conflicts()?;
        let new_cards: BTreeSet<_> = self
            .preview
            .cards
            .iter()
            .filter(|card| card.status == CardRevisionImportStatus::NewRevision)
            .map(|card| card.id.clone())
            .collect();
        let new_packs: BTreeSet<_> = self
            .preview
            .packs
            .iter()
            .filter(|pack| pack.status == CardPackRevisionImportStatus::NewRevision)
            .map(|pack| pack.id.clone())
            .collect();
        let cards = CardCatalog::new(
            self.destination_cards
                .revisions()
                .chain(
                    self.transfer
                        .cards
                        .revisions()
                        .filter(|card| new_cards.contains(card.id())),
                )
                .cloned(),
        )
        .map_err(CardPackTransferError::Cards)?;
        let packs = CardPackCatalog::new(
            self.destination_packs
                .manifests()
                .chain(
                    self.transfer
                        .packs
                        .manifests()
                        .filter(|pack| new_packs.contains(&pack.id)),
                )
                .cloned(),
            &cards,
        )
        .map_err(CardPackTransferError::InvalidPacks)?;
        Ok(CardPackImportResult { cards, packs })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardPackTransferError {
    Cards(CardCatalogError),
    InvalidPacks(Vec<CardPackValidationError>),
    MissingPack(CardPackRevisionId),
    MissingCard(CardRevisionId),
    UnreferencedPack(CardPackRevisionId),
    UnreferencedCard(CardRevisionId),
    CardIdentityConflict(CardRevisionId),
    PackIdentityConflict(CardPackRevisionId),
}

impl fmt::Display for CardPackTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cards(error) => error.fmt(formatter),
            Self::InvalidPacks(errors) => {
                for (index, error) in errors.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str("; ")?;
                    }
                    error.fmt(formatter)?;
                }
                Ok(())
            }
            Self::MissingPack(id) => write!(formatter, "missing Card pack {id}"),
            Self::MissingCard(id) => write!(formatter, "missing Card revision {id}"),
            Self::UnreferencedPack(id) => write!(
                formatter,
                "Card pack {id} is outside the root dependency closure"
            ),
            Self::UnreferencedCard(id) => write!(
                formatter,
                "Card revision {id} is not referenced by the root dependency closure"
            ),
            Self::CardIdentityConflict(id) => write!(
                formatter,
                "Card revision {id} already has different content"
            ),
            Self::PackIdentityConflict(id) => {
                write!(formatter, "Card pack {id} already has different content")
            }
        }
    }
}
impl std::error::Error for CardPackTransferError {}

#[cfg(test)]
mod tests;
