pub mod ai_lab;
pub mod match_store;
pub mod routes;

pub(crate) mod app_state;
pub(crate) mod auth_context;
pub(crate) mod card_catalog;
pub(crate) mod deck_library;
pub(crate) mod deck_recipe_legality;
pub(crate) mod http_errors;
pub(crate) mod http_types;
pub(crate) mod identity;
pub(crate) mod loadout_resolution;
pub(crate) mod match_access;
pub(crate) mod match_commands;
pub(crate) mod match_session;
pub(crate) mod preferences;
pub(crate) mod progression;
pub(crate) mod rules_kernel;

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
#[path = "main_tests/mod.rs"]
mod main_tests;
