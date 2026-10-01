use serde::Serialize;

use crate::deck_library::starter_recipe_count;
use crate::match_session::{CardKind, Rarity};
pub use rune_lanes_core::{card_template_by_id, starter_card_templates};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogCard {
    pub id: String,
    pub template_id: String,
    pub name: String,
    pub rarity: Rarity,
    pub cost: u8,
    pub text: String,
    pub kind: CardKind,
    pub copy_count: u8,
    pub art_key: String,
    pub art_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogResponse {
    pub cards: Vec<CatalogCard>,
}

pub fn starter_catalog() -> Vec<CatalogCard> {
    starter_card_templates()
        .into_iter()
        .map(|card| {
            let copy_count = starter_recipe_count(&card.template_id)
                .try_into()
                .expect("starter recipe counts should fit in u8");
            let art_key = card.template_id.clone();
            CatalogCard {
                id: card.template_id.clone(),
                template_id: card.template_id,
                name: card.name,
                rarity: card.rarity,
                cost: card.cost,
                text: card.text,
                kind: card.kind,
                copy_count,
                art_path: format!("/card-art/{art_key}.svg"),
                art_key,
            }
        })
        .collect()
}
