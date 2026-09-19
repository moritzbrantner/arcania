use crate::card_catalog::card_template_by_id;

use super::{
    Card, CarriedItem, HexCoord, ItemPassiveEffect, MatchMode, MatchState, Phase, Side,
    StackAction, StackItem, Unit,
};

#[derive(Clone, Debug)]
pub struct MatchScenarioDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub primary_actions: &'static [&'static str],
}

const SCENARIOS: &[MatchScenarioDefinition] = &[
    MatchScenarioDefinition {
        id: "play-unit-card",
        name: "Play Unit Card",
        description: "Start with an affordable unit card and an adjacent empty hex.",
        primary_actions: &["playCard"],
    },
    MatchScenarioDefinition {
        id: "move-unit",
        name: "Move Unit",
        description: "Start with a ready allied unit and adjacent empty hexes.",
        primary_actions: &["movePiece"],
    },
    MatchScenarioDefinition {
        id: "adjacent-attack",
        name: "Adjacent Attack",
        description: "Start with opposing pieces adjacent for an immediate attack.",
        primary_actions: &["attack"],
    },
    MatchScenarioDefinition {
        id: "draw-spell",
        name: "Draw Spell",
        description: "Start with a draw spell in hand and cards waiting in deck.",
        primary_actions: &["playCard", "draw"],
    },
    MatchScenarioDefinition {
        id: "priority-response",
        name: "Priority Response",
        description: "Start with an opponent stack item and player priority.",
        primary_actions: &["playCard", "passPriority", "advanceAi"],
    },
    MatchScenarioDefinition {
        id: "item-equip-activate",
        name: "Item Equip And Activate",
        description: "Start with an allied unit carrying an activatable item.",
        primary_actions: &["activateItem"],
    },
];

pub fn match_scenarios() -> Vec<MatchScenarioDefinition> {
    SCENARIOS.to_vec()
}

pub fn build_match_scenario(id: &str) -> Option<MatchState> {
    match id {
        "play-unit-card" => Some(play_unit_card()),
        "move-unit" => Some(move_unit()),
        "adjacent-attack" => Some(adjacent_attack()),
        "draw-spell" => Some(draw_spell()),
        "priority-response" => Some(priority_response()),
        "item-equip-activate" => Some(item_equip_activate()),
        _ => None,
    }
}

fn base_match() -> MatchState {
    let mut game = MatchState::new_with_seed(7);
    game.mode = MatchMode::Solo;
    game.round = 1;
    game.phase = Phase::Movement;
    game.active_side = Side::Player;
    game.priority_side = None;
    game.action_stack.clear();
    game.board.units.clear();
    game.board.dropped_items.clear();
    game.log = vec!["Loaded a local match scenario.".to_string()];
    game.winner = None;
    game.next_stack_item_id = 1;
    game.next_unit_id = 1;
    game.next_item_id = 1;

    reset_player(&mut game, Side::Player);
    reset_player(&mut game, Side::Opponent);
    game.player.hero.position = hex(0, 1);
    game.opponent.hero.position = hex(1, -1);
    game
}

fn reset_player(game: &mut MatchState, side: Side) {
    let player = game.player_mut(side);
    player.mana = 8;
    player.max_mana = 8;
    player.hero.ap_remaining = player.hero.max_ap;
    player.hero.has_attacked = false;
    player.hand.clear();
    player.deck.clear();
    player.discard.clear();
    player.deck_count = 0;
    player.discard_count = 0;
    player.has_started_first_turn = true;
    player.summoned_unit_count = 0;
}

fn play_unit_card() -> MatchState {
    let mut game = base_match();
    game.player
        .hand
        .push(scenario_card("ember-squire", "scenario-ember-squire"));
    game
}

fn move_unit() -> MatchState {
    let mut game = base_match();
    game.board.units.push(unit(UnitSpec {
        id: "player-unit",
        side: Side::Player,
        name: "Rune Runner",
        template_id: "rune-runner",
        attack: 1,
        armor: 1,
        max_ap: 4,
        position: hex(0, 0),
    }));
    game
}

fn adjacent_attack() -> MatchState {
    let mut game = move_unit();
    game.board.units.push(unit(UnitSpec {
        id: "opponent-unit",
        side: Side::Opponent,
        name: "Ember Squire",
        template_id: "ember-squire",
        attack: 1,
        armor: 2,
        max_ap: 2,
        position: hex(1, 0),
    }));
    game
}

fn draw_spell() -> MatchState {
    let mut game = base_match();
    game.player
        .hand
        .push(scenario_card("runic-insight", "scenario-runic-insight"));
    game.player
        .deck
        .push(scenario_card("swift-familiar", "scenario-draw-card"));
    game.player.deck_count = game.player.deck.len();
    game
}

fn priority_response() -> MatchState {
    let mut game = base_match();
    game.player
        .hand
        .push(scenario_card("arcane-parry", "scenario-arcane-parry"));
    game.board.units.push(unit(UnitSpec {
        id: "player-unit",
        side: Side::Player,
        name: "Stoneguard",
        template_id: "stoneguard",
        attack: 1,
        armor: 2,
        max_ap: 2,
        position: hex(0, 0),
    }));
    game.action_stack.push(StackItem {
        id: "scenario-stack-1".to_string(),
        side: Side::Opponent,
        priority: 0,
        action: StackAction::Attack {
            attacker_id: "opponent-hero".to_string(),
            target_id: "player-unit".to_string(),
        },
    });
    game.priority_side = Some(Side::Player);
    game.active_side = Side::Opponent;
    game
}

fn item_equip_activate() -> MatchState {
    let mut game = base_match();
    let mut carrier = unit(UnitSpec {
        id: "player-unit",
        side: Side::Player,
        name: "Rune Runner",
        template_id: "rune-runner",
        attack: 1,
        armor: 1,
        max_ap: 4,
        position: hex(0, 0),
    });
    carrier.armor = 0;
    carrier.items.push(CarriedItem {
        id: "player-item-1".to_string(),
        template_id: "ember-flask".to_string(),
        name: "Ember Flask".to_string(),
        passive: ItemPassiveEffect::StatBonus {
            attack: 1,
            armor: 0,
            max_ap: 0,
        },
        active: Some(super::ItemActiveEffect::HealCarrier {
            amount: 2,
            priority: 0,
        }),
        active_used_this_turn: false,
    });
    carrier.attack += 1;
    game.board.units.push(carrier);
    game
}

fn scenario_card(template_id: &str, id: &str) -> Card {
    let mut card = card_template_by_id(template_id).expect("scenario card template should exist");
    card.id = id.to_string();
    card
}

struct UnitSpec {
    id: &'static str,
    side: Side,
    name: &'static str,
    template_id: &'static str,
    attack: i32,
    armor: i32,
    max_ap: u8,
    position: HexCoord,
}

fn unit(spec: UnitSpec) -> Unit {
    Unit {
        id: spec.id.to_string(),
        side: spec.side,
        name: spec.name.to_string(),
        template_id: Some(spec.template_id.to_string()),
        attack: spec.attack,
        attack_range: 1,
        armor: spec.armor,
        max_armor: spec.armor,
        position: spec.position,
        ap_remaining: spec.max_ap,
        max_ap: spec.max_ap,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    }
}

fn hex(q: i32, r: i32) -> HexCoord {
    HexCoord { q, r }
}

#[cfg(test)]
mod tests {
    use super::super::{ActionTarget, MatchActionRequest};
    use super::*;

    #[test]
    fn lists_core_match_scenarios() {
        let scenarios = match_scenarios();

        assert_eq!(scenarios.len(), 6);
        assert!(
            scenarios
                .iter()
                .any(|scenario| scenario.id == "play-unit-card")
        );
        assert!(
            scenarios
                .iter()
                .any(|scenario| scenario.id == "priority-response")
        );
    }

    #[test]
    fn unknown_match_scenario_returns_none() {
        assert!(build_match_scenario("missing").is_none());
    }

    #[test]
    fn each_match_scenario_builds_a_serializable_match() {
        for scenario in match_scenarios() {
            let game = build_match_scenario(scenario.id).expect("scenario should build");
            let snapshot = game.to_snapshot_json().expect("scenario should serialize");
            MatchState::from_snapshot_json(&snapshot).expect("scenario should deserialize");
        }
    }

    #[test]
    fn play_unit_card_scenario_accepts_unit_play() {
        let mut game = build_match_scenario("play-unit-card").expect("scenario should build");
        game.phase = Phase::CardPlay;

        game.apply_action(MatchActionRequest::PlayCard {
            card_id: "scenario-ember-squire".to_string(),
            target: ActionTarget::Hex { coord: hex(0, 0) },
        })
        .expect("unit should be playable");

        assert!(
            game.board
                .units
                .iter()
                .any(|unit| unit.template_id.as_deref() == Some("ember-squire"))
        );
    }

    #[test]
    fn move_unit_scenario_accepts_unit_move() {
        let mut game = build_match_scenario("move-unit").expect("scenario should build");

        game.apply_action(MatchActionRequest::MovePiece {
            piece_id: "player-unit".to_string(),
            to: hex(-1, 0),
        })
        .expect("unit should move");

        assert_eq!(
            game.board
                .units
                .iter()
                .find(|unit| unit.id == "player-unit")
                .unwrap()
                .position,
            hex(-1, 0)
        );
    }

    #[test]
    fn adjacent_attack_scenario_accepts_attack() {
        let mut game = build_match_scenario("adjacent-attack").expect("scenario should build");
        game.phase = Phase::Attack;

        game.apply_action(MatchActionRequest::Attack {
            attacker_id: "player-unit".to_string(),
            target_id: "opponent-unit".to_string(),
        })
        .expect("attack should work");

        assert!(
            game.board
                .units
                .iter()
                .any(|unit| unit.id == "opponent-unit" && unit.armor < 2)
        );
    }

    #[test]
    fn draw_spell_scenario_accepts_draw_spell() {
        let mut game = build_match_scenario("draw-spell").expect("scenario should build");
        game.phase = Phase::CardPlay;

        game.apply_action(MatchActionRequest::PlayCard {
            card_id: "scenario-runic-insight".to_string(),
            target: ActionTarget::Piece {
                piece_id: "player-hero".to_string(),
            },
        })
        .expect("draw spell should work");

        assert!(
            game.player
                .hand
                .iter()
                .any(|card| card.id == "scenario-draw-card")
        );
        assert_eq!(game.player.deck_count, 0);
    }

    #[test]
    fn priority_response_scenario_accepts_response_spell() {
        let mut game = build_match_scenario("priority-response").expect("scenario should build");

        game.apply_action(MatchActionRequest::PlayCard {
            card_id: "scenario-arcane-parry".to_string(),
            target: ActionTarget::Piece {
                piece_id: "player-unit".to_string(),
            },
        })
        .expect("higher-priority response should work");

        assert_eq!(game.priority_side, Some(Side::Opponent));
        assert_eq!(game.action_stack.len(), 2);
    }

    #[test]
    fn item_equip_activate_scenario_accepts_item_activation() {
        let mut game = build_match_scenario("item-equip-activate").expect("scenario should build");

        game.apply_action(MatchActionRequest::ActivateItem {
            carrier_id: "player-unit".to_string(),
            item_id: "player-item-1".to_string(),
            target: None,
        })
        .expect("item should activate");

        let unit = game
            .board
            .units
            .iter()
            .find(|unit| unit.id == "player-unit")
            .unwrap();
        assert_eq!(unit.armor, 1);
        assert!(unit.items[0].active_used_this_turn);
    }
}
