use rune_lanes_core::{deck_library::starter_recipe_count, starter_card_templates};
use serde::Serialize;

use crate::match_session::{CardKind, Rarity};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_projects_core_templates_with_starter_copy_counts() {
        let cards = starter_catalog();
        let templates = starter_card_templates();
        assert_eq!(cards.len(), templates.len());
        assert_eq!(
            cards
                .iter()
                .map(|card| u16::from(card.copy_count))
                .sum::<u16>(),
            60
        );
        for (card, template) in cards.iter().zip(templates) {
            assert_eq!(card.id, template.template_id);
            assert_eq!(card.template_id, template.template_id);
            assert_eq!(card.name, template.name);
            assert_eq!(card.rarity, template.rarity);
            assert_eq!(card.cost, template.cost);
            assert_eq!(card.text, template.text);
            assert_eq!(
                serde_json::to_value(&card.kind).unwrap(),
                serde_json::to_value(&template.kind).unwrap()
            );
            assert_eq!(
                card.art_path,
                format!("/card-art/{}.svg", template.template_id)
            );
            assert_eq!(card.art_key, template.template_id);
        }
        for (template_id, expected_count) in [
            ("ember-squire", 4),
            ("mana-well", 5),
            ("iron-colossus", 1),
            ("meteor-bloom", 0),
        ] {
            let card = cards
                .iter()
                .find(|card| card.template_id == template_id)
                .unwrap();
            assert_eq!(card.copy_count, expected_count, "{template_id}");
        }
    }
}
