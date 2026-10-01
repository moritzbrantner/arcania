use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::HeroType;

use super::DeckCardCount;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SystemDeckRecipe {
    pub id: String,
    pub name: String,
    pub hero_type: HeroType,
    pub cards: Vec<DeckCardCount>,
}

pub fn system_deck_recipes() -> &'static [SystemDeckRecipe] {
    &SYSTEM_DECKS
}

pub fn system_deck_by_id(deck_id: &str) -> Option<&'static SystemDeckRecipe> {
    system_deck_recipes().iter().find(|deck| deck.id == deck_id)
}

static SYSTEM_DECKS: LazyLock<Vec<SystemDeckRecipe>> = LazyLock::new(|| {
    vec![
        system_deck(
            "balanced-starter",
            "Balanced Starter",
            HeroType::Runekeeper,
            &[
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
            ],
        ),
        system_deck(
            "ember-burn",
            "Ember Burn",
            HeroType::Pyromancer,
            &[
                ("ember-squire", 4),
                ("ash-hound", 4),
                ("spark-jolt", 4),
                ("quick-salve", 4),
                ("rune-runner", 4),
                ("mana-well", 5),
                ("swift-familiar", 5),
                ("rune-bruiser", 5),
                ("prism-initiate", 5),
                ("runic-insight", 4),
                ("stoneguard", 3),
                ("ember-lance", 4),
                ("cinder-ring", 4),
                ("pyre-brand", 2),
                ("meteor-bloom", 3),
            ],
        ),
        system_deck(
            "tempo-lines",
            "Tempo Lines",
            HeroType::Chronomancer,
            &[
                ("swift-familiar", 4),
                ("rune-runner", 4),
                ("spark-jolt", 4),
                ("quick-salve", 4),
                ("prism-initiate", 4),
                ("mana-well", 5),
                ("ember-squire", 5),
                ("ash-hound", 5),
                ("rune-bruiser", 5),
                ("warding-sigil", 2),
                ("stoneguard", 2),
                ("watchtower", 1),
                ("overclock-bracers", 1),
                ("surge-protocol", 1),
                ("starlit-study", 4),
                ("prism-ray", 4),
                ("mending-rune", 2),
                ("chrono-cog", 2),
                ("thunder-rail", 1),
            ],
        ),
        system_deck(
            "rune-fortress",
            "Rune Fortress",
            HeroType::Warden,
            &[
                ("stoneguard", 4),
                ("prism-initiate", 4),
                ("warding-sigil", 4),
                ("quick-salve", 4),
                ("ember-squire", 4),
                ("mana-well", 5),
                ("rune-bruiser", 2),
                ("swift-familiar", 5),
                ("rune-runner", 5),
                ("runic-insight", 3),
                ("spark-jolt", 2),
                ("stone-bastion", 1),
                ("titan-aegis", 1),
                ("colossus-oath", 1),
                ("shield-adept", 4),
                ("bastion-rune", 3),
                ("warden-plate", 1),
                ("mending-rune", 4),
                ("vanguard-golem", 3),
            ],
        ),
        system_deck(
            "unit-pressure",
            "Unit Pressure",
            HeroType::Battlemage,
            &[
                ("rune-bruiser", 4),
                ("ash-hound", 4),
                ("rune-runner", 4),
                ("ember-squire", 4),
                ("swift-familiar", 4),
                ("mana-well", 5),
                ("stoneguard", 2),
                ("prism-initiate", 5),
                ("warding-sigil", 4),
                ("emberbrand-charm", 1),
                ("spark-jolt", 4),
                ("quick-salve", 4),
                ("war-foundry", 1),
                ("battle-standard", 1),
                ("surge-protocol", 1),
                ("glass-duelist", 4),
                ("prism-ray", 4),
                ("war-chant", 2),
                ("phoenix-adept", 2),
            ],
        ),
        system_deck(
            "barbarian-fury-line",
            "Barbarian Fury Line",
            HeroType::Barbarian,
            &[
                ("ridge-berserker", 4),
                ("rune-bruiser", 4),
                ("ash-hound", 4),
                ("ember-squire", 4),
                ("swift-familiar", 4),
                ("mana-well", 5),
                ("rune-runner", 5),
                ("prism-initiate", 5),
                ("spark-jolt", 2),
                ("crushing-roar", 4),
                ("war-chant", 4),
                ("ember-lance", 4),
                ("glass-duelist", 4),
                ("rift-crown", 2),
                ("rift-gauntlet", 1),
                ("watchtower", 1),
                ("colossus-oath", 1),
                ("phoenix-adept", 1),
                ("eclipse-strike", 1),
            ],
        ),
        system_deck(
            "archer-volley-line",
            "Archer Volley Line",
            HeroType::Archer,
            &[
                ("pathfinder", 4),
                ("swift-familiar", 4),
                ("rune-runner", 4),
                ("ash-hound", 4),
                ("spark-jolt", 4),
                ("mana-well", 5),
                ("runic-insight", 5),
                ("ember-squire", 5),
                ("prism-initiate", 5),
                ("piercing-volley", 4),
                ("prism-ray", 4),
                ("temporal-bolt", 2),
                ("hunter-scope", 2),
                ("starlit-study", 4),
                ("deadeye-mark", 2),
                ("thunder-rail", 2),
            ],
        ),
        system_deck(
            "builder-worksite",
            "Builder Worksite",
            HeroType::Builder,
            &[
                ("field-mason", 4),
                ("stoneguard", 4),
                ("prism-initiate", 4),
                ("warding-sigil", 4),
                ("rune-charm", 4),
                ("mana-well", 5),
                ("ember-squire", 1),
                ("rune-bruiser", 5),
                ("quick-salve", 5),
                ("fortify-position", 4),
                ("arcane-parry", 4),
                ("bastion-rune", 4),
                ("shield-adept", 4),
                ("clockwork-rig", 1),
                ("starcore-engine", 1),
                ("stone-bastion", 1),
                ("healing-font", 1),
                ("titan-plate", 1),
                ("builder-toolkit", 1),
                ("vanguard-golem", 2),
            ],
        ),
    ]
});

fn system_deck(
    id: &str,
    name: &str,
    hero_type: HeroType,
    counts: &[(&str, u16)],
) -> SystemDeckRecipe {
    let cards = counts
        .iter()
        .map(|(template_id, count)| DeckCardCount {
            template_id: (*template_id).to_string(),
            count: *count,
        })
        .collect::<Vec<_>>();
    SystemDeckRecipe {
        id: id.to_string(),
        name: name.to_string(),
        hero_type,
        cards,
    }
}
