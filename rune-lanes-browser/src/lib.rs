//! Browser application adapter. The core remains the only rules implementation.
use rune_lanes_core::{
    MatchActionRequest, MatchState, Side, SoloAiPolicy,
    commands::{CommandContext, GameCommand},
    deck_library::system_deck_recipes,
    rules::CURRENT_RULESET,
    workshop::WorkshopSetup,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use wasm_bindgen::prelude::*;

const JOURNAL_VERSION: u32 = 1;
const MAX_COMMANDS: usize = 10_000;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Journal {
    version: u32,
    setup: WorkshopSetup,
    commands: Vec<RecordedCommand>,
}

#[derive(Serialize, Deserialize)]
struct RecordedCommand {
    side: Side,
    command: GameCommand,
}

#[wasm_bindgen]
pub struct BrowserMatch {
    state: MatchState,
    journal: Journal,
}

#[wasm_bindgen]
impl BrowserMatch {
    #[wasm_bindgen(constructor)]
    pub fn new(setup_json: &str) -> Result<BrowserMatch, String> {
        let setup: WorkshopSetup = serde_json::from_str(setup_json).map_err(|e| e.to_string())?;
        let setup = setup.normalized()?;
        let state = setup.create_match()?;
        Ok(Self {
            state,
            journal: Journal {
                version: JOURNAL_VERSION,
                setup,
                commands: vec![],
            },
        })
    }

    pub fn restore(journal_json: &str) -> Result<BrowserMatch, String> {
        let journal: Journal = serde_json::from_str(journal_json).map_err(|e| e.to_string())?;
        if journal.version != JOURNAL_VERSION || journal.commands.len() > MAX_COMMANDS {
            return Err(
                "This saved match uses an unsupported format or exceeds the command limit.".into(),
            );
        }
        let mut state = journal.setup.create_match()?;
        for (index, record) in journal.commands.iter().enumerate() {
            record
                .command
                .clone()
                .execute_compatibility(
                    &mut state,
                    CommandContext {
                        side: record.side,
                        action_index: index as u32,
                    },
                )
                .map_err(|e| format!("Saved match command {index} is incompatible: {e}"))?;
        }
        Ok(Self { state, journal })
    }

    pub fn view(&self) -> Result<String, String> {
        let view = self.state.queries().public_match(Side::Player);
        serde_json::to_string(&json!({
            "matchState": view,
            "heroAppearances": [],
        }))
        .map_err(|e| e.to_string())
    }

    pub fn journal(&self) -> Result<String, String> {
        serde_json::to_string(&self.journal).map_err(|e| e.to_string())
    }

    pub fn catalog(&self) -> Result<String, String> {
        catalog_value(&self.journal.setup.cards)
    }

    pub fn act(&mut self, action_json: &str) -> Result<String, String> {
        if self.journal.commands.len() >= MAX_COMMANDS {
            return Err(
                "This match reached its command limit. Start a new match in the workshop.".into(),
            );
        }
        let action: MatchActionRequest =
            serde_json::from_str(action_json).map_err(|e| e.to_string())?;
        let (side, command) = match action {
            MatchActionRequest::AdvanceAi => (
                Side::Opponent,
                self.state
                    .next_solo_ai_game_command_with_policy(
                        Side::Opponent,
                        &SoloAiPolicy::baseline(),
                    )
                    .map_err(|e| e.to_string())?
                    .unwrap_or(GameCommand::EndTurn),
            ),
            action => (
                Side::Player,
                GameCommand::try_from(action).map_err(|e| format!("{e:?}"))?,
            ),
        };
        // A rejected action must leave both the live state and journal unchanged.
        let mut candidate = self.state.clone();
        command
            .clone()
            .execute_compatibility(
                &mut candidate,
                CommandContext {
                    side,
                    action_index: self.journal.commands.len() as u32,
                },
            )
            .map_err(|e| e.to_string())?;
        self.state = candidate;
        self.journal
            .commands
            .push(RecordedCommand { side, command });
        self.view()
    }
}

#[wasm_bindgen]
pub fn workshop_defaults() -> Result<String, String> {
    serde_json::to_string(&WorkshopSetup {
        seed: 42,
        player_hero: Default::default(),
        opponent_hero: Default::default(),
        player_deck_recipe: system_deck_recipes()[0].cards.clone(),
        opponent_deck_recipe: system_deck_recipes()[0].cards.clone(),
        ruleset: CURRENT_RULESET,
        cards: vec![],
    })
    .map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn validate_setup(setup_json: &str) -> Result<String, String> {
    let setup: WorkshopSetup = serde_json::from_str(setup_json).map_err(|e| e.to_string())?;
    let setup = setup.normalized()?;
    serde_json::to_string(&setup).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn catalog() -> Result<String, String> {
    catalog_value(&[])
}

#[wasm_bindgen]
pub fn system_decks() -> Result<String, String> {
    serde_json::to_string(system_deck_recipes()).map_err(|error| error.to_string())
}

fn catalog_value(custom: &[rune_lanes_core::CardDefinition]) -> Result<String, String> {
    let cards: Vec<_> = rune_lanes_core::starter_card_definitions().iter().chain(custom).map(|card| json!({
        "id": card.id, "templateId": card.id, "name": card.name, "rarity": card.rarity,
        "cost": card.cost, "text": card.text, "kind": card.kind, "copyCount": 1,
        "artKey": card.id, "artPath": if card.id.starts_with("custom-") { "workshop/rune-field.svg".to_owned() } else { format!("card-art/{}.svg", card.id) },
    })).collect();
    serde_json::to_string(&json!({"cards": cards})).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bot_turn_and_reload_use_the_same_command_journal() {
        let mut game = BrowserMatch::new(&workshop_defaults().unwrap()).unwrap();
        game.act(r#"{"type":"startAttackPhase"}"#).unwrap();
        game.act(r#"{"type":"startCardPlay"}"#).unwrap();
        game.act(r#"{"type":"endTurn"}"#).unwrap();
        for _ in 0..100 {
            game.act(r#"{"type":"advanceAi"}"#).unwrap();
            if game.state.priority_side == Some(Side::Player) {
                game.act(r#"{"type":"passPriority"}"#).unwrap();
            }
            if game.state.active_side == Side::Player && game.state.action_stack.is_empty() {
                break;
            }
        }
        assert_eq!(game.state.active_side, Side::Player);
        assert!(game.state.round >= 2);
        let restored = BrowserMatch::restore(&game.journal().unwrap()).unwrap();
        assert_eq!(restored.view().unwrap(), game.view().unwrap());
    }

    #[test]
    fn historical_saved_journals_keep_the_original_card_timing() {
        let mut setup: serde_json::Value =
            serde_json::from_str(&workshop_defaults().unwrap()).unwrap();
        setup["ruleset"]["turn"]
            .as_object_mut()
            .unwrap()
            .remove("movementCardPlay");
        setup.as_object_mut().unwrap().remove("playerDeckRecipe");
        setup.as_object_mut().unwrap().remove("opponentDeckRecipe");
        let mut game = BrowserMatch::new(&setup.to_string()).unwrap();
        game.act(r#"{"type":"startCardPlay"}"#).unwrap();
        game.act(r#"{"type":"endTurn"}"#).unwrap();
        let mut old_journal: serde_json::Value =
            serde_json::from_str(&game.journal().unwrap()).unwrap();
        old_journal["setup"]
            .as_object_mut()
            .unwrap()
            .remove("playerDeckRecipe");
        old_journal["setup"]
            .as_object_mut()
            .unwrap()
            .remove("opponentDeckRecipe");
        let saved = old_journal.to_string();
        assert!(!saved.contains("movementCardPlay"));
        let restored = BrowserMatch::restore(&saved).unwrap();
        assert_eq!(restored.view().unwrap(), game.view().unwrap());
        assert!(!restored.state.queries().ruleset().turn.movement_card_play);
    }

    #[test]
    fn invalid_commands_and_corrupt_saves_do_not_repair_state() {
        let mut game = BrowserMatch::new(&workshop_defaults().unwrap()).unwrap();
        let before = game.journal().unwrap();
        assert!(game.act(r#"{"type":"endTurn"}"#).is_err());
        assert_eq!(before, game.journal().unwrap());
        assert!(BrowserMatch::restore(&before.replace("\"version\":1", "\"version\":99")).is_err());
    }

    #[test]
    fn exposes_the_exact_shared_system_recipes() {
        let recipes: Vec<rune_lanes_core::deck_library::SystemDeckRecipe> =
            serde_json::from_str(&system_decks().unwrap()).unwrap();
        assert_eq!(recipes, system_deck_recipes());
        assert_eq!(recipes.len(), 8);
        assert_eq!(recipes[0].id, "balanced-starter");
    }

    #[test]
    fn independent_recipes_are_normalized_frozen_and_restored_in_the_journal() {
        use rune_lanes_core::deck_library::{DeckCardCount, system_deck_by_id};
        let mut setup: WorkshopSetup = serde_json::from_str(&workshop_defaults().unwrap()).unwrap();
        setup.player_deck_recipe = system_deck_by_id("ember-burn").unwrap().cards.clone();
        setup.player_deck_recipe.reverse();
        setup.opponent_deck_recipe = vec![
            DeckCardCount {
                template_id: "spark-jolt".into(),
                count: 10,
            },
            DeckCardCount {
                template_id: "ember-squire".into(),
                count: 10,
            },
        ];
        let valid: WorkshopSetup =
            serde_json::from_str(&validate_setup(&serde_json::to_string(&setup).unwrap()).unwrap())
                .unwrap();
        assert_eq!(
            valid.player_deck_recipe,
            system_deck_by_id("ember-burn").unwrap().cards
        );
        assert_eq!(valid.opponent_deck_recipe[0].template_id, "ember-squire");
        assert_eq!(valid.opponent_deck_recipe[0].count, 10);
        let mut game = BrowserMatch::new(&serde_json::to_string(&setup).unwrap()).unwrap();
        setup.player_deck_recipe.clear();
        game.act(r#"{"type":"startAttackPhase"}"#).unwrap();
        let saved = game.journal().unwrap();
        let journal: Journal = serde_json::from_str(&saved).unwrap();
        assert_eq!(journal.setup.player_deck_recipe, valid.player_deck_recipe);
        assert_eq!(
            journal.setup.opponent_deck_recipe,
            valid.opponent_deck_recipe
        );
        let restored = BrowserMatch::restore(&saved).unwrap();
        assert_eq!(restored.view().unwrap(), game.view().unwrap());
        assert_eq!(
            restored.state.to_snapshot_json().unwrap(),
            game.state.to_snapshot_json().unwrap()
        );
        let mut corrupt: serde_json::Value = serde_json::from_str(&saved).unwrap();
        corrupt["setup"]["opponentDeckRecipe"][0]["templateId"] = json!("missing-card");
        assert!(BrowserMatch::restore(&corrupt.to_string()).is_err());
    }
}
