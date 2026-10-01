use super::{CardTestBoardSetup, CardTestScenario};
use crate::{
    ActionTarget, CardDefinition, HeroType, HexCoord, Side,
    commands::GameCommand,
    event_sourcing::{
        CommandDecision, CommandId, CommandMetadata, EventEnvelope, EventSourcedMatch,
    },
    rules::CURRENT_RULESET,
    workshop::WorkshopSetup,
};

fn scenario(board_setup: CardTestBoardSetup, mut card: CardDefinition) -> CardTestScenario {
    card.id = "custom-scenario-card".into();
    CardTestScenario {
        workshop: WorkshopSetup {
            seed: 42,
            player_hero: HeroType::Runekeeper,
            opponent_hero: HeroType::Runekeeper,
            player_deck_recipe: crate::deck_library::system_deck_by_id("balanced-starter")
                .unwrap()
                .cards
                .clone(),
            opponent_deck_recipe: crate::deck_library::system_deck_by_id("balanced-starter")
                .unwrap()
                .cards
                .clone(),
            ruleset: CURRENT_RULESET,
            cards: vec![card],
        },
        board_setup,
    }
}

fn execute(aggregate: &mut EventSourcedMatch, side: Side, command: GameCommand) -> EventEnvelope {
    let index = u32::try_from(aggregate.version().0).unwrap();
    let decision = aggregate
        .decide(
            CommandMetadata {
                command_id: CommandId::new(format!("scenario-command-{index}")).unwrap(),
                expected_version: aggregate.version(),
                side,
                action_index: index,
            },
            command,
        )
        .unwrap();
    let CommandDecision::Append(event) = decision else {
        panic!("new scenario command should append");
    };
    aggregate.evolve(&event).unwrap();
    event
}

#[test]
fn prepared_authored_unit_scenario_uses_core_queries_commands_and_domain_events() {
    let definition = crate::starter_card_definitions().remove(0);
    let prepared = scenario(CardTestBoardSetup::Empty, definition)
        .prepare()
        .unwrap();
    let (mut aggregate, _) = EventSourcedMatch::create(prepared.create_match()).unwrap();
    assert_eq!(aggregate.state().player.hand.len(), 1);
    assert_eq!(aggregate.state().player.mana, 3);
    let play = GameCommand::PlayCard {
        card_id: "player-custom-0".into(),
        target: ActionTarget::Hex {
            coord: HexCoord { q: 0, r: 1 },
        },
    };
    assert!(
        aggregate
            .queries()
            .command_availability(Side::Player, &play)
            .allowed
    );
    execute(&mut aggregate, Side::Player, play);
    while let Some(side) = aggregate.state().priority_side {
        execute(&mut aggregate, side, GameCommand::PassPriority);
    }
    assert_eq!(
        aggregate.state().board.units[0].template_id.as_deref(),
        Some("custom-scenario-card")
    );
    assert_eq!(aggregate.state().player.mana, 2);
    assert_eq!(prepared.initial_state().board.units.len(), 0);
}

#[test]
fn serialized_adjacent_enemy_scenario_executes_authored_damage() {
    let definition = crate::starter_card_definitions()
        .into_iter()
        .find(|card| card.id == "spark-jolt")
        .unwrap();
    let mut document =
        serde_json::to_value(scenario(CardTestBoardSetup::Empty, definition)).unwrap();
    document["boardSetup"] = serde_json::json!("adjacentEnemy");
    let decoded: CardTestScenario = serde_json::from_value(document).unwrap();
    let prepared = decoded.prepare().unwrap();
    let (mut aggregate, _) = EventSourcedMatch::create(prepared.create_match()).unwrap();
    assert_eq!(
        aggregate.state().board.units[0].position,
        HexCoord { q: 1, r: 0 }
    );
    let play = GameCommand::PlayCard {
        card_id: "player-custom-0".into(),
        target: ActionTarget::Piece {
            piece_id: "scenario-enemy-0".into(),
        },
    };
    assert!(
        aggregate
            .queries()
            .command_availability(Side::Player, &play)
            .allowed
    );
    execute(&mut aggregate, Side::Player, play);
    while let Some(side) = aggregate.state().priority_side {
        execute(&mut aggregate, side, GameCommand::PassPriority);
    }
    assert_eq!(aggregate.state().board.units[0].armor, 3);
    assert_eq!(prepared.initial_state().board.units[0].armor, 4);
}

#[test]
fn clustered_enemy_scenario_executes_authored_area_damage() {
    let mut definition = crate::starter_card_definitions()
        .into_iter()
        .find(|card| card.id == "spark-jolt")
        .unwrap();
    definition.kind = crate::CardKind::Spell {
        range: 2,
        priority: 0,
        effect: crate::SpellEffect::AreaDamage {
            amount: 1,
            radius: 1,
        },
    };
    let mut document =
        serde_json::to_value(scenario(CardTestBoardSetup::Empty, definition)).unwrap();
    document["boardSetup"] = serde_json::json!("clusteredEnemies");
    let decoded: CardTestScenario = serde_json::from_value(document).unwrap();
    let (mut aggregate, _) =
        EventSourcedMatch::create(decoded.prepare().unwrap().create_match()).unwrap();
    let play = GameCommand::PlayCard {
        card_id: "player-custom-0".into(),
        target: ActionTarget::Piece {
            piece_id: "scenario-enemy-0".into(),
        },
    };
    assert!(
        aggregate
            .queries()
            .command_availability(Side::Player, &play)
            .allowed
    );
    execute(&mut aggregate, Side::Player, play);
    while let Some(side) = aggregate.state().priority_side {
        execute(&mut aggregate, side, GameCommand::PassPriority);
    }
    let armor: Vec<_> = aggregate
        .state()
        .board
        .units
        .iter()
        .map(|unit| unit.armor)
        .collect();
    assert_eq!(armor, [3, 3, 3]);
}

#[test]
fn injured_carrier_scenarios_execute_authored_healing() {
    let definition = crate::starter_card_definitions()
        .into_iter()
        .find(|card| card.id == "quick-salve")
        .unwrap();
    for (layout, piece_id, expected) in [
        ("damagedAlly", "scenario-ally-0", 4),
        ("lowHpHero", "player-hero", 3),
    ] {
        let mut document =
            serde_json::to_value(scenario(CardTestBoardSetup::Empty, definition.clone())).unwrap();
        document["boardSetup"] = serde_json::json!(layout);
        let decoded: CardTestScenario = serde_json::from_value(document).unwrap();
        let (mut aggregate, _) =
            EventSourcedMatch::create(decoded.prepare().unwrap().create_match()).unwrap();
        let play = GameCommand::PlayCard {
            card_id: "player-custom-0".into(),
            target: ActionTarget::Piece {
                piece_id: piece_id.into(),
            },
        };
        assert!(
            aggregate
                .queries()
                .command_availability(Side::Player, &play)
                .allowed
        );
        execute(&mut aggregate, Side::Player, play);
        while let Some(side) = aggregate.state().priority_side {
            execute(&mut aggregate, side, GameCommand::PassPriority);
        }
        let restored = aggregate
            .state()
            .board
            .units
            .iter()
            .find(|unit| unit.id == piece_id)
            .map_or(aggregate.state().player.hero.hp, |unit| unit.armor);
        assert_eq!(restored, expected);
    }
}

#[test]
fn every_seeded_setup_resets_and_replays_the_same_core_events_and_private_state() {
    for seed in [1, 7, 42] {
        for layout in [
            CardTestBoardSetup::Empty,
            CardTestBoardSetup::AdjacentEnemy,
            CardTestBoardSetup::ClusteredEnemies,
            CardTestBoardSetup::DamagedAlly,
            CardTestBoardSetup::LowHpHero,
        ] {
            let mut input = scenario(layout, crate::starter_card_definitions().remove(0));
            input.workshop.seed = seed;
            let encoded = serde_json::to_value(&input).unwrap();
            let restored: CardTestScenario = serde_json::from_value(encoded).unwrap();
            let prepared = restored.prepare().unwrap();
            let initial = prepared.initial_state().to_snapshot_json().unwrap();
            let mut runs = Vec::new();
            for _ in 0..2 {
                let (mut aggregate, genesis) =
                    EventSourcedMatch::create(prepared.create_match()).unwrap();
                assert_eq!(aggregate.state().to_snapshot_json().unwrap(), initial);
                let mut events = vec![genesis];
                events.push(execute(
                    &mut aggregate,
                    Side::Player,
                    GameCommand::PlayCard {
                        card_id: "player-custom-0".into(),
                        target: ActionTarget::Hex {
                            coord: HexCoord { q: 0, r: 1 },
                        },
                    },
                ));
                while let Some(side) = aggregate.state().priority_side {
                    events.push(execute(&mut aggregate, side, GameCommand::PassPriority));
                }
                for command in [
                    GameCommand::StartAttackPhase,
                    GameCommand::StartCardPlay,
                    GameCommand::EndTurn,
                ] {
                    events.push(execute(&mut aggregate, Side::Player, command));
                }
                let final_state = aggregate.state().to_snapshot_json().unwrap();
                assert_ne!(final_state, initial);
                let replayed = EventSourcedMatch::rehydrate(&events).unwrap();
                assert_eq!(replayed.state().to_snapshot_json().unwrap(), final_state);
                runs.push((serde_json::to_value(events).unwrap(), final_state));
            }
            assert_eq!(runs[0], runs[1]);
            assert_eq!(
                prepared.initial_state().to_snapshot_json().unwrap(),
                initial
            );
            assert_eq!(
                input
                    .prepare()
                    .unwrap()
                    .initial_state()
                    .to_snapshot_json()
                    .unwrap(),
                initial
            );
        }
    }
}

#[test]
fn invalid_authored_content_and_commands_use_existing_core_errors_without_mutation() {
    let mut input = scenario(
        CardTestBoardSetup::AdjacentEnemy,
        crate::starter_card_definitions().remove(0),
    );
    let prepared = input.prepare().unwrap();
    let initial = prepared.initial_state().to_snapshot_json().unwrap();
    input.workshop.cards[0].cost = 31;
    assert!(input.prepare().unwrap_err().contains("Card limits"));
    assert_eq!(
        prepared.initial_state().to_snapshot_json().unwrap(),
        initial
    );
    let (aggregate, _) = EventSourcedMatch::create(prepared.create_match()).unwrap();
    let command = GameCommand::PlayCard {
        card_id: "player-custom-0".into(),
        target: ActionTarget::Hex {
            coord: HexCoord { q: 1, r: 0 },
        },
    };
    let availability = aggregate
        .queries()
        .command_availability(Side::Player, &command);
    assert!(!availability.allowed);
    let rejection = aggregate
        .decide(
            CommandMetadata {
                command_id: CommandId::new("invalid-scenario-command").unwrap(),
                expected_version: aggregate.version(),
                side: Side::Player,
                action_index: 0,
            },
            command,
        )
        .unwrap_err();
    let crate::event_sourcing::EventSourcingError::Rule(error) = rejection else {
        panic!("scenario action should report the real rule error");
    };
    assert_eq!(Some(error), availability.rejection);
    assert_eq!(aggregate.state().to_snapshot_json().unwrap(), initial);
}

#[test]
fn low_hp_hero_scenario_can_reach_the_real_match_defeat_condition() {
    let definition = crate::starter_card_definitions()
        .into_iter()
        .find(|card| card.id == "spark-jolt")
        .unwrap();
    let (mut aggregate, _) = EventSourcedMatch::create(
        scenario(CardTestBoardSetup::LowHpHero, definition)
            .prepare()
            .unwrap()
            .create_match(),
    )
    .unwrap();
    execute(
        &mut aggregate,
        Side::Player,
        GameCommand::PlayCard {
            card_id: "player-custom-0".into(),
            target: ActionTarget::Piece {
                piece_id: "opponent-hero".into(),
            },
        },
    );
    while let Some(side) = aggregate.state().priority_side {
        execute(&mut aggregate, side, GameCommand::PassPriority);
    }
    assert_eq!(aggregate.queries().winner(), Some(Side::Player));
    assert_eq!(aggregate.queries().phase(), crate::Phase::MatchOver);
}
