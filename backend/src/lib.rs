#[allow(
    clippy::single_match,
    reason = "AI lab rule-preset overrides predate this core extraction and are behaviorally unrelated to it"
)]
pub mod ai_lab;
pub mod match_store;
pub mod routes;

pub(crate) mod app_state;
pub(crate) mod auth_context;
pub(crate) mod card_catalog;
pub(crate) mod card_workshop;
pub(crate) mod deck_library;
pub(crate) mod deck_recipe_legality;
pub(crate) mod http_errors;
pub(crate) mod http_types;
#[allow(
    clippy::format_collect,
    reason = "identity token formatting is existing backend behavior outside the Rune Lanes extraction"
)]
pub(crate) mod identity;
pub(crate) mod loadout_resolution;
pub(crate) mod match_access;
pub(crate) mod match_commands;
pub(crate) mod match_session;
pub(crate) mod multiplayer_sessions;
pub(crate) mod preferences;
#[allow(
    dead_code,
    reason = "progression contains a tested staged hero-appearance API that is not yet wired to production routes"
)]
pub(crate) mod progression;

pub(crate) use http_types::*;

#[cfg(test)]
use axum::Router;
#[cfg(test)]
use match_session::Side;
#[cfg(test)]
use match_store::SqliteMatchStore;
#[cfg(test)]
use routes::create_app;

#[cfg(test)]
mod event_store_migration_tests;

#[cfg(test)]
#[path = "main_tests/mod.rs"]
mod main_tests;
