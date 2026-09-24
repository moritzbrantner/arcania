mod ai_lab_support;
mod card_catalog;
mod card_definitions;
mod deck_library;
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
    PublishedCardRevision, PublishedCardRevisionError,
};
pub use match_session::*;
