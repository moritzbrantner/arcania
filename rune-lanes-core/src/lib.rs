mod ai_lab_support;
mod card_catalog;
mod card_definitions;
mod card_pack_transfers;
mod card_packs;
mod card_transfers;
pub mod deck_library;
mod match_session;

pub mod commands;
pub mod cqrs;
pub mod event_sourcing;
pub mod queries;
pub mod rules;
#[cfg(feature = "test-support")]
pub mod test_support;

pub use card_catalog::{
    card_template_by_id, starter_card_catalog, starter_card_definitions, starter_card_templates,
};
pub use card_definitions::{
    CardCatalog, CardCatalogError, CardDefinition, CardDefinitionValidationError, CardRevisionId,
    CardTaxonomy, PublishedCardRevision, PublishedCardRevisionError,
};
pub use card_pack_transfers::{
    CARD_PACK_TRANSFER_SCHEMA_VERSION, CardPackImport, CardPackImportPreview, CardPackImportResult,
    CardPackRevisionImportPreview, CardPackRevisionImportStatus, CardPackTransfer,
    CardPackTransferError, CardRevisionImportPreview,
};
pub use card_packs::{
    CARD_PACK_MANIFEST_SCHEMA_VERSION, CardPackCatalog, CardPackCompatibility, CardPackManifest,
    CardPackRevisionId, CardPackValidationError,
};
pub use card_transfers::{
    CARD_REVISION_TRANSFER_SCHEMA_VERSION, CardRevisionImport, CardRevisionImportStatus,
    CardRevisionTransfer, CardTransferError,
};
pub use match_session::*;

pub mod workshop;
