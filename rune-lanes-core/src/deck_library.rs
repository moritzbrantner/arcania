use crate::card_catalog::card_template_by_id;
use crate::match_session::{Card, Side};

const BALANCED_STARTER_COUNTS: &[(&str, u16)] = &[
    ("ember-squire", 4),
    ("swift-familiar", 4),
    ("stoneguard", 4),
    ("rune-bruiser", 4),
    ("quick-salve", 4),
    ("spark-jolt", 4),
    ("rune-charm", 2),
    ("runekeeper-lens", 2),
    ("rune-runner", 4),
    ("ash-hound", 4),
    ("prism-initiate", 4),
    ("mana-well", 5),
    ("runic-insight", 5),
    ("blade-dancer", 1),
    ("shield-adept", 1),
    ("mending-rune", 1),
    ("war-chant", 1),
    ("ember-lance", 1),
    ("arcane-parry", 1),
    ("ember-flask", 1),
    ("cinder-ring", 1),
    ("prism-ray", 1),
    ("iron-colossus", 1),
];

#[derive(Clone, Debug)]
pub(crate) struct DeckCardCount {
    template_id: String,
    count: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct DeckRecipeSnapshot {
    cards: Vec<DeckCardCount>,
}

#[derive(Debug)]
pub(crate) enum DeckLibraryError {
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
        cards: BALANCED_STARTER_COUNTS
            .iter()
            .map(|(template_id, count)| DeckCardCount {
                template_id: (*template_id).to_string(),
                count: *count,
            })
            .collect(),
    }
}

pub(crate) fn deck_from_snapshot(
    side: Side,
    snapshot: &DeckRecipeSnapshot,
) -> Result<Vec<Card>, DeckLibraryError> {
    let mut cards = Vec::new();
    for count in &snapshot.cards {
        let Some(template) = card_template_by_id(&count.template_id) else {
            return Err(DeckLibraryError::UnknownTemplate(count.template_id.clone()));
        };
        for copy in 0..count.count {
            let mut card = template.clone();
            card.id = format!("{}-{copy}-{}", side.card_prefix(), card.template_id);
            cards.push(card);
        }
    }
    Ok(cards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balanced_starter_has_sixty_cards() {
        assert_eq!(
            BALANCED_STARTER_COUNTS
                .iter()
                .map(|(_, count)| u32::from(*count))
                .sum::<u32>(),
            60
        );
    }

    #[test]
    fn balanced_starter_references_known_templates() {
        for (template_id, _) in BALANCED_STARTER_COUNTS {
            assert!(
                card_template_by_id(template_id).is_some(),
                "starter deck references unknown template {template_id}"
            );
        }
    }
}
