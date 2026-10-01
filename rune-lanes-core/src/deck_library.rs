use serde::{Deserialize, Serialize};

use crate::card_catalog::starter_card_catalog;
use crate::match_session::{Card, Side};

mod system_decks;

pub use system_decks::{SystemDeckRecipe, system_deck_by_id, system_deck_recipes};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeckCardCount {
    pub template_id: String,
    pub count: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct DeckRecipeSnapshot {
    cards: Vec<DeckCardCount>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeckLibraryError {
    UnknownTemplate(String),
}

impl std::fmt::Display for DeckLibraryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTemplate(template_id) => {
                write!(f, "unknown card template {template_id}")
            }
        }
    }
}

impl std::error::Error for DeckLibraryError {}

pub(crate) fn starter_deck_snapshot() -> DeckRecipeSnapshot {
    DeckRecipeSnapshot {
        cards: system_deck_by_id("balanced-starter")
            .expect("compiled starter system recipe should exist")
            .cards
            .clone(),
    }
}

pub fn starter_recipe_count(template_id: &str) -> u16 {
    system_deck_by_id("balanced-starter")
        .expect("compiled starter system recipe should exist")
        .cards
        .iter()
        .find(|card| card.template_id == template_id)
        .map_or(0, |card| card.count)
}

pub(crate) fn deck_from_snapshot(
    side: Side,
    snapshot: &DeckRecipeSnapshot,
) -> Result<Vec<Card>, DeckLibraryError> {
    deck_from_counts(side, &snapshot.cards)
}

/// Expand counts in their supplied order so frozen recipes and seeded openings
/// retain their historical behavior. Recipe normalization belongs at authoring input.
pub fn deck_from_counts(
    side: Side,
    counts: &[DeckCardCount],
) -> Result<Vec<Card>, DeckLibraryError> {
    let mut cards = Vec::new();
    for count in counts {
        let Some(revision) = starter_card_catalog().latest(&count.template_id) else {
            return Err(DeckLibraryError::UnknownTemplate(count.template_id.clone()));
        };
        for copy in 0..count.count {
            cards.push(revision.definition().instantiate(format!(
                "{}-{copy}-{}",
                side.card_prefix(),
                count.template_id
            )));
        }
    }
    Ok(cards)
}

#[cfg(test)]
mod tests;
