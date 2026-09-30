//! Validated experimental setup. Card effects still use the ordinary command path.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    CardDefinition, HeroType, MatchState, Side,
    deck_library::DeckCardCount,
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
        self.validate_deck_recipe("Player", &self.player_deck_recipe)?;
        self.validate_deck_recipe("Opponent", &self.opponent_deck_recipe)?;
        if self.cards.len() > 8 {
            return Err("A Rule preset can include up to 8 custom cards.".into());
        }
        let mut ids = HashSet::new();
        for card in &self.cards {
            if !card.id.starts_with("custom-") || !ids.insert(&card.id) {
                return Err("Custom cards need unique IDs starting with custom-.".into());
            }
            card.validate()
                .map_err(|errors| format!("{}: {errors:?}", card.name))?;
            if card.name.len() > 80 || card.text.len() > 500 || card.cost > 30 {
                return Err(
                    "Card limits: 80 name characters, 500 description characters and 30 Mana."
                        .into(),
                );
            }
            // Recursively bound numeric effect values before any arithmetic in the engine.
            fn bounded(value: &serde_json::Value) -> bool {
                match value {
                    serde_json::Value::Number(n) => {
                        n.as_i64().is_some_and(|v| (-100..=100).contains(&v))
                    }
                    serde_json::Value::Array(values) => values.iter().all(bounded),
                    serde_json::Value::Object(fields) => fields.values().all(bounded),
                    _ => true,
                }
            }
            if !bounded(&serde_json::to_value(&card.kind).map_err(|error| error.to_string())?) {
                return Err("Card effect values must be between -100 and 100.".into());
            }
        }
        Ok(())
    }

    fn validate_deck_recipe(&self, label: &str, cards: &[DeckCardCount]) -> Result<(), String> {
        let total = cards
            .iter()
            .map(|card| u32::from(card.count))
            .sum::<u32>();
        if total < u32::from(self.ruleset.turn.opening_hand_size) || total > 120 {
            return Err(format!(
                "{label} deck recipe must contain between {} and 120 cards.",
                self.ruleset.turn.opening_hand_size
            ));
        }

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
            if crate::starter_card_catalog().latest(&card.template_id).is_none() {
                return Err(format!(
                    "{label} deck recipe references unknown card template {}.",
                    card.template_id
                ));
            }
        }
        Ok(())
    }

    pub fn create_match(&self) -> Result<MatchState, String> {
        self.validate()?;
        let player_deck =
            crate::deck_library::deck_from_counts(Side::Player, &self.player_deck_recipe)
                .map_err(|error| error.to_string())?;
        let opponent_deck =
            crate::deck_library::deck_from_counts(Side::Opponent, &self.opponent_deck_recipe)
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

fn starter_deck_recipe() -> Vec<DeckCardCount> {
    crate::deck_library::starter_deck_snapshot().cards
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
        for command in [GameCommand::StartCardPlay, GameCommand::EndTurn] {
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
    fn selected_deck_recipe_drives_match_cards() {
        let mut setup = setup();
        setup.player_deck_recipe = crate::deck_library::system_deck_by_id("ember-burn")
            .unwrap()
            .cards;
        let state = setup.create_match().unwrap();
        let player_templates = state
            .player
            .hand
            .iter()
            .chain(state.player.deck.iter())
            .map(|card| card.template_id.as_str())
            .collect::<Vec<_>>();
        let opponent_templates = state
            .opponent
            .hand
            .iter()
            .chain(state.opponent.deck.iter())
            .map(|card| card.template_id.as_str())
            .collect::<Vec<_>>();

        assert!(player_templates.contains(&"meteor-bloom"));
        assert!(!player_templates.contains(&"runekeeper-lens"));
        assert!(opponent_templates.contains(&"runekeeper-lens"));
    }

    #[test]
    fn old_workshop_presets_default_to_the_starter_deck_recipe() {
        let value = serde_json::to_value(setup()).unwrap();
        let mut object = value.as_object().unwrap().clone();
        object.remove("playerDeckRecipe");
        object.remove("opponentDeckRecipe");

        let restored: WorkshopSetup =
            serde_json::from_value(serde_json::Value::Object(object)).unwrap();

        assert_eq!(restored.player_deck_recipe, starter_deck_recipe());
        assert_eq!(restored.opponent_deck_recipe, starter_deck_recipe());
    }

    #[test]
    fn invalid_workshop_deck_recipe_is_rejected() {
        let mut setup = setup();
        setup.player_deck_recipe = vec![DeckCardCount {
            template_id: "missing-card".into(),
            count: 60,
        }];

        assert!(setup.validate().is_err());
    }
}
