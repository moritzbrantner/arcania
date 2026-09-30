//! Browser application adapter. The core remains the only rules implementation.
use rune_lanes_core::{
    MatchActionRequest, MatchState, Side, SoloAiPolicy,
    commands::{CommandContext, GameCommand},
    system_deck_recipes,
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
        let mut view = self.state.queries().public_match(Side::Player);
        view["legalCommands"] = json!(self.state.queries().legal_commands(Side::Player));
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
    setup.validate()?;
    serde_json::to_string(&setup).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn catalog() -> Result<String, String> {
    catalog_value(&[])
}

#[wasm_bindgen]
pub fn system_decks() -> Result<String, String> {
    serde_json::to_string(&system_deck_recipes()).map_err(|e| e.to_string())
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
    fn invalid_commands_and_corrupt_saves_do_not_repair_state() {
        let mut game = BrowserMatch::new(&workshop_defaults().unwrap()).unwrap();
        let before = game.journal().unwrap();
        assert!(game.act(r#"{"type":"endTurn"}"#).is_err());
        assert_eq!(before, game.journal().unwrap());
        assert!(BrowserMatch::restore(&before.replace("\"version\":1", "\"version\":99")).is_err());
    }

    #[test]
    fn exposes_the_shared_system_deck_recipes() {
        let decks: Vec<rune_lanes_core::SystemDeckRecipe> =
            serde_json::from_str(&system_decks().unwrap()).unwrap();
        assert_eq!(decks.len(), 8);
        assert_eq!(decks[0].id, "balanced-starter");
        assert_eq!(
            decks[0]
                .cards
                .iter()
                .map(|card| u32::from(card.count))
                .sum::<u32>(),
            60
        );
    }
}
