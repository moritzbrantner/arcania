use std::collections::BTreeMap;

use rune_lanes_core::card_template_by_id;
use serde::{Deserialize, Serialize};

use crate::match_session::Rarity;

pub const MIN_DECK_CARDS: u16 = 60;
pub const BASIC_COPY_LIMIT: u16 = 5;
pub const ADVANCED_COPY_LIMIT: u16 = 4;
pub const RARE_COPY_LIMIT: u16 = 3;
pub const ADVANCED_TOTAL_LIMIT: u16 = 24;
pub const RARE_TOTAL_LIMIT: u16 = 12;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckRules {
    pub max_decks_per_account: usize,
    pub min_cards: u16,
    pub basic_copy_limit: u16,
    pub advanced_copy_limit: u16,
    pub rare_copy_limit: u16,
    pub advanced_total_limit: u16,
    pub rare_total_limit: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeckCardCount {
    pub template_id: String,
    pub count: u16,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckLegality {
    pub legal: bool,
    pub total_cards: u16,
    pub basic_cards: u16,
    pub advanced_cards: u16,
    pub rare_cards: u16,
    pub messages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckCardCountRequest {
    pub template_id: String,
    pub count: i32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeckLegalityPreviewRequest {
    pub cards: Vec<DeckCardCountRequest>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DeckRecipeLegalityError {
    UnknownTemplate(String),
    NegativeCount(String),
}

pub fn deck_rules(max_decks_per_account: usize) -> DeckRules {
    DeckRules {
        max_decks_per_account,
        min_cards: MIN_DECK_CARDS,
        basic_copy_limit: BASIC_COPY_LIMIT,
        advanced_copy_limit: ADVANCED_COPY_LIMIT,
        rare_copy_limit: RARE_COPY_LIMIT,
        advanced_total_limit: ADVANCED_TOTAL_LIMIT,
        rare_total_limit: RARE_TOTAL_LIMIT,
    }
}

pub fn normalize_requested_cards(
    cards: Vec<DeckCardCountRequest>,
) -> Result<Vec<DeckCardCount>, DeckRecipeLegalityError> {
    let mut counts = BTreeMap::<String, u16>::new();
    for card in cards {
        let template_id = card.template_id.trim().to_string();
        if card.count < 0 {
            return Err(DeckRecipeLegalityError::NegativeCount(template_id));
        }
        if card.count == 0 {
            continue;
        }
        if card_template_by_id(&template_id).is_none() {
            return Err(DeckRecipeLegalityError::UnknownTemplate(template_id));
        }
        let entry = counts.entry(template_id).or_default();
        *entry = entry.saturating_add(card.count as u16);
    }
    Ok(deck_card_counts_from_map(counts))
}

pub fn preview_requested_cards(cards: Vec<DeckCardCountRequest>) -> DeckLegality {
    let mut counts = BTreeMap::<String, u16>::new();
    let mut messages = Vec::new();

    for card in cards {
        let template_id = card.template_id.trim().to_string();
        if card.count < 0 {
            messages.push(format!("Card count must not be negative: {template_id}"));
            continue;
        }
        if card.count == 0 {
            continue;
        }
        let entry = counts.entry(template_id).or_default();
        *entry = entry.saturating_add(card.count as u16);
    }

    validate_recipe_with_messages(&deck_card_counts_from_map(counts), messages)
}

pub fn validate_recipe(cards: &[DeckCardCount]) -> DeckLegality {
    validate_recipe_with_messages(cards, Vec::new())
}

fn validate_recipe_with_messages(
    cards: &[DeckCardCount],
    mut messages: Vec<String>,
) -> DeckLegality {
    let mut total_cards = 0_u16;
    let mut basic_cards = 0_u16;
    let mut advanced_cards = 0_u16;
    let mut rare_cards = 0_u16;

    for card_count in cards {
        let Some(template) = card_template_by_id(&card_count.template_id) else {
            messages.push(format!(
                "{} is not in the card catalog.",
                card_count.template_id
            ));
            total_cards = total_cards.saturating_add(card_count.count);
            continue;
        };
        total_cards = total_cards.saturating_add(card_count.count);
        match template.rarity {
            Rarity::Basic => {
                basic_cards = basic_cards.saturating_add(card_count.count);
                if card_count.count > BASIC_COPY_LIMIT {
                    messages.push(format!(
                        "{} has {} copies; Basic cards allow at most {}.",
                        template.name, card_count.count, BASIC_COPY_LIMIT
                    ));
                }
            }
            Rarity::Advanced => {
                advanced_cards = advanced_cards.saturating_add(card_count.count);
                if card_count.count > ADVANCED_COPY_LIMIT {
                    messages.push(format!(
                        "{} has {} copies; Advanced cards allow at most {}.",
                        template.name, card_count.count, ADVANCED_COPY_LIMIT
                    ));
                }
            }
            Rarity::Rare => {
                rare_cards = rare_cards.saturating_add(card_count.count);
                if card_count.count > RARE_COPY_LIMIT {
                    messages.push(format!(
                        "{} has {} copies; Rare cards allow at most {}.",
                        template.name, card_count.count, RARE_COPY_LIMIT
                    ));
                }
            }
        }
    }

    if total_cards < MIN_DECK_CARDS {
        messages.push(format!(
            "Deck has {total_cards} cards; at least {MIN_DECK_CARDS} are required."
        ));
    }
    if advanced_cards > ADVANCED_TOTAL_LIMIT {
        messages.push(format!(
            "Deck has {advanced_cards} Advanced cards; at most {ADVANCED_TOTAL_LIMIT} are allowed."
        ));
    }
    if rare_cards > RARE_TOTAL_LIMIT {
        messages.push(format!(
            "Deck has {rare_cards} Rare cards; at most {RARE_TOTAL_LIMIT} are allowed."
        ));
    }

    DeckLegality {
        legal: messages.is_empty(),
        total_cards,
        basic_cards,
        advanced_cards,
        rare_cards,
        messages,
    }
}

fn deck_card_counts_from_map(counts: BTreeMap<String, u16>) -> Vec<DeckCardCount> {
    counts
        .into_iter()
        .map(|(template_id, count)| DeckCardCount { template_id, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_rejects_too_few_cards() {
        let legality = validate_recipe(&[DeckCardCount {
            template_id: "ember-squire".to_string(),
            count: 5,
        }]);
        assert!(!legality.legal);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("at least 60"))
        );
    }

    #[test]
    fn validator_rejects_copy_and_rarity_caps() {
        let legality = validate_recipe(&[
            DeckCardCount {
                template_id: "ember-squire".to_string(),
                count: 6,
            },
            DeckCardCount {
                template_id: "blade-dancer".to_string(),
                count: 5,
            },
            DeckCardCount {
                template_id: "iron-colossus".to_string(),
                count: 4,
            },
            DeckCardCount {
                template_id: "shield-adept".to_string(),
                count: 20,
            },
            DeckCardCount {
                template_id: "starfire-bolt".to_string(),
                count: 10,
            },
        ]);

        assert!(!legality.legal);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("Basic cards"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("Advanced cards"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("Rare cards"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("at most 24"))
        );
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("at most 12"))
        );
    }

    #[test]
    fn validator_rejects_unknown_templates() {
        let legality = validate_recipe(&[DeckCardCount {
            template_id: "missing-card".to_string(),
            count: 60,
        }]);
        assert!(!legality.legal);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("not in the card catalog"))
        );
        assert_eq!(legality.total_cards, 60);
    }

    #[test]
    fn preview_reports_negative_counts_as_draft_legality() {
        let legality = preview_requested_cards(vec![DeckCardCountRequest {
            template_id: "ember-squire".to_string(),
            count: -1,
        }]);

        assert!(!legality.legal);
        assert_eq!(legality.total_cards, 0);
        assert!(
            legality
                .messages
                .iter()
                .any(|message| message.contains("must not be negative"))
        );
    }

    #[test]
    fn strict_normalization_rejects_negative_counts() {
        let error = normalize_requested_cards(vec![DeckCardCountRequest {
            template_id: "ember-squire".to_string(),
            count: -1,
        }])
        .expect_err("negative count should fail");

        assert_eq!(
            error,
            DeckRecipeLegalityError::NegativeCount("ember-squire".to_string())
        );
    }

    #[test]
    fn strict_normalization_rejects_unknown_templates() {
        let error = normalize_requested_cards(vec![DeckCardCountRequest {
            template_id: "missing-card".to_string(),
            count: 1,
        }])
        .expect_err("unknown template should fail");

        assert_eq!(
            error,
            DeckRecipeLegalityError::UnknownTemplate("missing-card".to_string())
        );
    }

    #[test]
    fn duplicate_counts_aggregate_before_validation() {
        let cards = normalize_requested_cards(vec![
            DeckCardCountRequest {
                template_id: "ember-squire".to_string(),
                count: 3,
            },
            DeckCardCountRequest {
                template_id: " ember-squire ".to_string(),
                count: 3,
            },
        ])
        .expect("duplicates should normalize");

        assert_eq!(
            cards,
            vec![DeckCardCount {
                template_id: "ember-squire".to_string(),
                count: 6,
            }]
        );
        assert!(
            validate_recipe(&cards)
                .messages
                .iter()
                .any(|message| message.contains("Basic cards"))
        );
    }
}
