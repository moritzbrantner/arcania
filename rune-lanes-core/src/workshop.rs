//! Validated experimental setup. Card effects still use the ordinary command path.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub mod card_scenarios;

use crate::{
    CardDefinition, HeroType, MatchState, Side,
    deck_library::{DeckCardCount, deck_from_counts, system_deck_recipes},
    rules::RuneLanesRuleset,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkshopSetup {
    pub seed: u32,
    pub player_hero: HeroType,
    pub opponent_hero: HeroType,
    #[serde(default = "starter_deck_recipe")]
    pub player_deck_recipe: Vec<DeckCardCount>,
    #[serde(default = "starter_deck_recipe")]
    pub opponent_deck_recipe: Vec<DeckCardCount>,
    pub ruleset: RuneLanesRuleset,
    pub cards: Vec<CardDefinition>,
}

impl WorkshopSetup {
    pub fn validate(&self) -> Result<(), String> {
        self.ruleset.validate().map_err(|error| error.to_string())?;
        // Bound experiments to values the board and arithmetic can safely support.
        let rules = &self.ruleset;
        if rules.arena != crate::rules::CURRENT_RULESET.arena
            || rules.turn.base_hero_mana > 30
            || rules.turn.opening_hand_size > 12
            || rules.turn.max_carried_items > 6
            || rules.turn.default_attack_range > 6
        {
            return Err("Use the standard arena, 1–30 Mana, 1–12 opening cards, 1–6 Items and 1–6 attack range.".into());
        }
        for hero in [
            HeroType::Runekeeper,
            HeroType::Pyromancer,
            HeroType::Chronomancer,
            HeroType::Warden,
            HeroType::Battlemage,
            HeroType::Barbarian,
            HeroType::Archer,
            HeroType::Builder,
        ] {
            let hero = rules.hero(hero);
            if hero.max_hp > 100 || hero.attack > 20 || hero.max_ap > 10 || hero.attack_range > 6 {
                return Err("Hero limits: 100 HP, 20 attack, 10 AP, 6 attack range.".into());
            }
        }
        validate_deck_recipe(
            "Player",
            &self.player_deck_recipe,
            rules.turn.opening_hand_size,
        )?;
        validate_deck_recipe(
            "Opponent",
            &self.opponent_deck_recipe,
            rules.turn.opening_hand_size,
        )?;
        if self.cards.len() > 8 {
            return Err("A Rule preset can include up to 8 custom cards.".into());
        }
        let mut ids = HashSet::new();
        for card in &self.cards {
            if !card.id.starts_with("custom-") || !ids.insert(&card.id) {
                return Err("Custom cards need unique IDs starting with custom-.".into());
            }
            validate_card_for_experiment(card)?;
        }
        Ok(())
    }

    pub fn normalized(mut self) -> Result<Self, String> {
        self.validate()?;
        self.player_deck_recipe = normalized_deck_recipe(&self.player_deck_recipe);
        self.opponent_deck_recipe = normalized_deck_recipe(&self.opponent_deck_recipe);
        Ok(self)
    }

    pub fn create_match(&self) -> Result<MatchState, String> {
        self.validate()?;
        let player_deck = deck_from_counts(
            Side::Player,
            &normalized_deck_recipe(&self.player_deck_recipe),
        )
        .map_err(|error| error.to_string())?;
        let opponent_deck = deck_from_counts(
            Side::Opponent,
            &normalized_deck_recipe(&self.opponent_deck_recipe),
        )
        .map_err(|error| error.to_string())?;
        let mut state = MatchState::new_with_rule_preset(
            u64::from(self.seed),
            self.player_hero,
            self.opponent_hero,
            player_deck,
            opponent_deck,
            self.ruleset,
        )
        .map_err(|error| error.to_string())?;
        // Extra opening copies make every authored card immediately available to try.
        // Played cards enter the ordinary discard/reshuffle cycle.
        for (index, card) in self.cards.iter().enumerate() {
            state
                .player
                .hand
                .push(card.instantiate(format!("player-custom-{index}")));
            state
                .opponent
                .hand
                .push(card.instantiate(format!("opponent-custom-{index}")));
        }
        Ok(state)
    }
}

/// Validate an exact Card definition for bounded experimental play, including
/// built-in published identities. Collection identity rules stay in WorkshopSetup.
pub fn validate_card_for_experiment(card: &CardDefinition) -> Result<(), String> {
    card.validate()
        .map_err(|errors| format!("{}: {errors:?}", card.name))?;
    if card.name.len() > 80 || card.text.len() > 500 || card.cost > 30 {
        return Err(
            "Card limits: 80 name characters, 500 description characters and 30 Mana.".into(),
        );
    }
    // Recursively bound numeric effect values before any arithmetic in the engine.
    fn bounded(value: &serde_json::Value) -> bool {
        match value {
            serde_json::Value::Number(n) => n.as_i64().is_some_and(|v| (-100..=100).contains(&v)),
            serde_json::Value::Array(values) => values.iter().all(bounded),
            serde_json::Value::Object(fields) => fields.values().all(bounded),
            _ => true,
        }
    }
    if !bounded(&serde_json::to_value(&card.kind).map_err(|error| error.to_string())?) {
        return Err("Card effect values must be between -100 and 100.".into());
    }
    Ok(())
}

fn starter_deck_recipe() -> Vec<DeckCardCount> {
    crate::deck_library::system_deck_by_id("balanced-starter")
        .expect("the configured starter recipe exists")
        .cards
        .clone()
}

fn validate_deck_recipe(
    label: &str,
    cards: &[DeckCardCount],
    opening_hand_size: u8,
) -> Result<(), String> {
    let mut template_ids = HashSet::new();
    for card in cards {
        if card.count == 0 || card.count > 30 {
            return Err(format!(
                "{label} deck recipe card counts must be between 1 and 30."
            ));
        }
        if !template_ids.insert(&card.template_id) {
            return Err(format!(
                "{label} deck recipe contains duplicate template {}.",
                card.template_id
            ));
        }
        if crate::card_template_by_id(&card.template_id).is_none() {
            return Err(format!(
                "{label} deck recipe references unknown card template {}.",
                card.template_id
            ));
        }
    }
    let total: u32 = cards.iter().map(|card| u32::from(card.count)).sum();
    if total < u32::from(opening_hand_size) || total > 120 {
        return Err(format!(
            "{label} deck recipe must contain between {opening_hand_size} and 120 cards."
        ));
    }
    Ok(())
}

fn normalized_deck_recipe(cards: &[DeckCardCount]) -> Vec<DeckCardCount> {
    // Existing configured order is part of historical seeded openings.
    if let Some(recipe) = system_deck_recipes().iter().find(|recipe| {
        recipe.cards.len() == cards.len() && cards.iter().all(|card| recipe.cards.contains(card))
    }) {
        return recipe.cards.clone();
    }
    let mut ordered = cards.to_vec();
    ordered.sort_by(|left, right| left.template_id.cmp(&right.template_id));
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionTarget, HexCoord,
        commands::{CommandContext, GameCommand},
        rules::CURRENT_RULESET,
    };

    fn setup() -> WorkshopSetup {
        WorkshopSetup {
            seed: 42,
            player_hero: HeroType::Runekeeper,
            opponent_hero: HeroType::Runekeeper,
            player_deck_recipe: starter_deck_recipe(),
            opponent_deck_recipe: starter_deck_recipe(),
            ruleset: CURRENT_RULESET,
            cards: vec![],
        }
    }

    #[test]
    fn exact_experimental_cards_share_bounds_without_relaxing_workshop_identity_rules() {
        let mut card = crate::starter_card_definitions()
            .into_iter()
            .find(|card| card.id == "stoneguard")
            .unwrap();
        assert!(validate_card_for_experiment(&card).is_ok());
        let mut workshop = setup();
        workshop.cards.push(card.clone());
        assert!(
            workshop
                .validate()
                .unwrap_err()
                .contains("starting with custom-")
        );
        card.cost = 31;
        assert!(
            validate_card_for_experiment(&card)
                .unwrap_err()
                .contains("30 Mana")
        );
        card.cost = 1;
        card.kind = crate::CardKind::Unit {
            attack: 101,
            armor: 4,
            max_ap: 2,
        };
        assert!(
            validate_card_for_experiment(&card)
                .unwrap_err()
                .contains("between -100 and 100")
        );
    }

    #[test]
    fn card_effects_use_client_field_names_and_read_legacy_snapshots() {
        let legacy = r#"{"type":"statBonus","attack":0,"armor":1,"max_ap":2}"#;
        let passive: crate::ItemPassiveEffect = serde_json::from_str(legacy).unwrap();
        let value = serde_json::to_value(passive).unwrap();
        assert_eq!(value["maxAp"], 2);
        assert!(value.get("max_ap").is_none());
        for definition in crate::starter_card_definitions() {
            let encoded = serde_json::to_string(&definition).unwrap();
            assert!(!encoded.contains("max_ap"));
            let decoded: crate::CardDefinition = serde_json::from_str(&encoded).unwrap();
            assert!(decoded.validate().is_ok());
        }
    }

    #[test]
    fn seed_42_has_the_portable_opening_hand() {
        let state = setup().create_match().unwrap();
        let ids: Vec<_> = state
            .player
            .hand
            .iter()
            .map(|card| card.template_id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "rune-bruiser",
                "rune-charm",
                "rune-bruiser",
                "prism-initiate",
                "iron-colossus",
                "quick-salve",
                "prism-initiate"
            ]
        );
    }

    #[test]
    fn rule_preset_survives_turns_and_snapshot_roundtrip() {
        let mut setup = setup();
        setup.ruleset.turn.base_hero_mana = 9;
        setup.ruleset.turn.opening_hand_size = 4;
        setup.ruleset.heroes.runekeeper.max_hp = 42;
        let mut state = setup.create_match().unwrap();
        assert_eq!(state.player.hand.len(), 4);
        assert_eq!(state.player.hero.hp, 42);
        assert_eq!(state.player.mana, 9);
        for command in [
            GameCommand::StartAttackPhase,
            GameCommand::StartCardPlay,
            GameCommand::EndTurn,
        ] {
            command
                .execute_compatibility(
                    &mut state,
                    CommandContext {
                        side: Side::Player,
                        action_index: 0,
                    },
                )
                .unwrap();
        }
        assert_eq!(state.opponent.mana, 9);
        let restored = MatchState::from_snapshot_json(&state.to_snapshot_json().unwrap()).unwrap();
        assert_eq!(restored.queries().ruleset(), setup.ruleset);
        assert_eq!(MatchState::new_with_seed(42).player.mana, 3);
    }

    #[test]
    fn custom_unit_uses_the_real_command_and_configured_attack_range() {
        let mut setup = setup();
        setup.ruleset.turn.default_attack_range = 3;
        let mut card = crate::starter_card_definitions().remove(0);
        card.id = "custom-guardian".into();
        setup.cards.push(card);
        let mut state = setup.create_match().unwrap();
        let context = CommandContext {
            side: Side::Player,
            action_index: 0,
        };
        GameCommand::StartAttackPhase
            .execute_compatibility(&mut state, context)
            .unwrap();
        GameCommand::StartCardPlay
            .execute_compatibility(&mut state, context)
            .unwrap();
        let play = GameCommand::PlayCard {
            card_id: "player-custom-0".into(),
            target: ActionTarget::Hex {
                coord: HexCoord { q: 0, r: 2 },
            },
        };
        assert!(
            state
                .queries()
                .command_availability(Side::Player, &play)
                .allowed
        );
        play.execute_compatibility(&mut state, context).unwrap();
        while let Some(side) = state.priority_side {
            GameCommand::PassPriority
                .execute_compatibility(
                    &mut state,
                    CommandContext {
                        side,
                        action_index: 1,
                    },
                )
                .unwrap();
        }
        assert_eq!(
            state.board.units[0].template_id.as_deref(),
            Some("custom-guardian")
        );
        assert_eq!(state.board.units[0].attack_range, 3);
    }

    #[test]
    fn invalid_and_duplicate_custom_cards_fail_before_match_creation() {
        let mut setup = setup();
        let mut card = crate::starter_card_definitions().remove(0);
        card.id = "custom-guardian".into();
        setup.cards = vec![card.clone(), card];
        assert!(setup.create_match().is_err());
        setup.cards.clear();
        setup.ruleset.turn.base_hero_mana = 0;
        assert!(setup.create_match().is_err());
    }

    #[test]
    fn selected_recipes_are_independent_and_survive_normalization() {
        let mut setup = setup();
        setup.player_deck_recipe = crate::deck_library::system_deck_by_id("ember-burn")
            .unwrap()
            .cards
            .clone();
        let state = setup.create_match().unwrap();
        let templates = |side: &crate::PlayerState| {
            side.hand
                .iter()
                .chain(side.deck.iter())
                .map(|card| card.template_id.clone())
                .collect::<Vec<_>>()
        };
        assert!(templates(&state.player).contains(&"meteor-bloom".to_string()));
        assert!(!templates(&state.player).contains(&"runekeeper-lens".to_string()));
        assert!(templates(&state.opponent).contains(&"runekeeper-lens".to_string()));
        assert_eq!(
            setup.clone().normalized().unwrap().player_deck_recipe,
            setup.player_deck_recipe
        );
    }

    #[test]
    fn reordered_system_and_custom_recipes_keep_identical_seeded_states() {
        for recipe in system_deck_recipes() {
            let mut system = setup();
            system.player_deck_recipe = recipe.cards.clone();
            system.opponent_deck_recipe = recipe.cards.clone();
            let expected = system.create_match().unwrap().to_snapshot_json().unwrap();
            system.player_deck_recipe.reverse();
            let removed = system.opponent_deck_recipe.remove(0);
            system.opponent_deck_recipe.push(removed);
            assert_eq!(
                system.create_match().unwrap().to_snapshot_json().unwrap(),
                expected,
                "{}",
                recipe.id
            );
            let normalized = system.normalized().unwrap();
            assert_eq!(normalized.player_deck_recipe, recipe.cards);
            assert_eq!(normalized.opponent_deck_recipe, recipe.cards);
        }

        let mut custom = setup();
        custom.player_deck_recipe = vec![
            DeckCardCount {
                template_id: "spark-jolt".into(),
                count: 10,
            },
            DeckCardCount {
                template_id: "ember-squire".into(),
                count: 10,
            },
        ];
        custom.opponent_deck_recipe = custom.player_deck_recipe.clone();
        let expected = custom.create_match().unwrap().to_snapshot_json().unwrap();
        custom.player_deck_recipe.reverse();
        let removed = custom.opponent_deck_recipe.remove(0);
        custom.opponent_deck_recipe.push(removed);
        assert_eq!(
            custom.create_match().unwrap().to_snapshot_json().unwrap(),
            expected
        );
        let normalized = custom.normalized().unwrap();
        assert_eq!(normalized.player_deck_recipe[0].template_id, "ember-squire");
        assert_eq!(normalized.player_deck_recipe[1].count, 10);
    }

    #[test]
    fn historical_presets_without_recipe_fields_keep_the_original_opening() {
        let expected = setup().create_match().unwrap().to_snapshot_json().unwrap();
        let mut old = serde_json::to_value(setup()).unwrap();
        old.as_object_mut().unwrap().remove("playerDeckRecipe");
        old.as_object_mut().unwrap().remove("opponentDeckRecipe");
        let restored: WorkshopSetup = serde_json::from_value(old).unwrap();
        assert_eq!(restored.player_deck_recipe, starter_deck_recipe());
        assert_eq!(restored.opponent_deck_recipe, starter_deck_recipe());
        assert_eq!(
            restored.create_match().unwrap().to_snapshot_json().unwrap(),
            expected
        );
    }

    #[test]
    fn invalid_recipe_counts_templates_duplicates_and_totals_are_rejected() {
        for (recipe, message) in [
            (vec![], "between 7 and 120"),
            (
                vec![DeckCardCount {
                    template_id: "ember-squire".into(),
                    count: 6,
                }],
                "between 7 and 120",
            ),
            (
                vec![DeckCardCount {
                    template_id: "ember-squire".into(),
                    count: 0,
                }],
                "between 1 and 30",
            ),
            (
                vec![DeckCardCount {
                    template_id: "ember-squire".into(),
                    count: 31,
                }],
                "between 1 and 30",
            ),
            (
                vec![DeckCardCount {
                    template_id: "missing-card".into(),
                    count: 7,
                }],
                "unknown card template",
            ),
            (
                vec![
                    DeckCardCount {
                        template_id: "ember-squire".into(),
                        count: 7
                    };
                    2
                ],
                "duplicate template",
            ),
            (
                [
                    "ember-squire",
                    "swift-familiar",
                    "stoneguard",
                    "rune-bruiser",
                    "spark-jolt",
                ]
                .into_iter()
                .map(|template_id| DeckCardCount {
                    template_id: template_id.into(),
                    count: 30,
                })
                .collect(),
                "between 7 and 120",
            ),
        ] {
            let mut player = setup();
            player.player_deck_recipe = recipe.clone();
            let error = player.normalized().unwrap_err();
            assert!(
                error.contains("Player") && error.contains(message),
                "{error}"
            );
            let mut opponent = setup();
            opponent.opponent_deck_recipe = recipe;
            let error = opponent.create_match().unwrap_err();
            assert!(
                error.contains("Opponent") && error.contains(message),
                "{error}"
            );
        }
        for count in [7, 30] {
            let mut valid = setup();
            valid.player_deck_recipe = vec![DeckCardCount {
                template_id: "ember-squire".into(),
                count,
            }];
            assert!(valid.create_match().is_ok());
        }
        let mut upper = setup();
        upper.player_deck_recipe = [
            "ember-squire",
            "swift-familiar",
            "stoneguard",
            "rune-bruiser",
        ]
        .into_iter()
        .map(|template_id| DeckCardCount {
            template_id: template_id.into(),
            count: 30,
        })
        .collect();
        assert!(upper.create_match().is_ok());
        let mut smaller = setup();
        smaller.ruleset.turn.opening_hand_size = 3;
        smaller.player_deck_recipe = vec![DeckCardCount {
            template_id: "ember-squire".into(),
            count: 3,
        }];
        assert!(smaller.create_match().is_ok());
    }

    #[test]
    fn recipe_wire_counts_reject_negative_fractional_and_out_of_range_numbers() {
        for count in [
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!(65_536),
        ] {
            let mut value = serde_json::to_value(setup()).unwrap();
            value["playerDeckRecipe"][0]["count"] = count;
            assert!(serde_json::from_value::<WorkshopSetup>(value).is_err());
        }
    }
}
