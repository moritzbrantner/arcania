use super::*;

fn paced_solo_ai_step(
    state: &mut MatchState,
    policy: &SoloAiPolicy,
    action_index: u32,
) -> Option<crate::commands::GameCommand> {
    use crate::commands::CommandContext;
    let side = state.active_side;
    let before = state.to_snapshot_json().unwrap();
    let command = state
        .next_solo_ai_game_command_with_policy(side, policy)
        .unwrap();
    assert_eq!(state.to_snapshot_json().unwrap(), before);
    let mut compatibility = state.clone();
    let mut frames = Vec::new();
    let outcome = compatibility
        .advance_ai_for_side_with_policy_strict(side, policy, &mut frames, Some(action_index))
        .unwrap();
    if let Some(command) = &command {
        assert!(state.queries().command_availability(side, command).allowed);
        command
            .clone()
            .execute_compatibility(state, CommandContext { side, action_index })
            .unwrap();
        assert_eq!(outcome, AiAdvanceOutcome::ActionApplied);
    } else {
        state
            .finish_solo_ai_turn_recording(side, action_index)
            .unwrap();
        assert_eq!(outcome, AiAdvanceOutcome::FinishedTurn);
    }
    assert_eq!(
        state.to_snapshot_json().unwrap(),
        compatibility.to_snapshot_json().unwrap()
    );
    command
}

fn pass_solo_response_window(state: &mut MatchState, action_index: u32) {
    use crate::commands::{CommandContext, GameCommand};
    assert_eq!(state.priority_side, Some(Side::Player));
    GameCommand::PassPriority
        .execute_compatibility(
            state,
            CommandContext {
                side: Side::Player,
                action_index,
            },
        )
        .unwrap();
    assert!(state.action_stack.is_empty());
}

#[test]
fn solo_ai_repositions_then_summons_and_moves_with_partial_entry_ap() {
    use crate::commands::GameCommand;
    let mut state = MatchState::new_with_seed(7);
    state.active_side = Side::Opponent;
    state.opponent.hero.ap_remaining = 1;
    state.opponent.mana = 2;
    let card = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "rune-runner")
        .unwrap();
    let card_id = card.id.clone();
    state.opponent.hand = vec![card];
    state.player.hand.clear();
    let policy = SoloAiPolicy::baseline();

    assert_eq!(
        paced_solo_ai_step(&mut state, &policy, 0),
        Some(GameCommand::MovePiece {
            piece_id: state.opponent.hero.id.clone(),
            to: hex(0, -2),
        })
    );
    assert_eq!(state.opponent.hero.position, hex(0, -3));
    assert_eq!(state.phase, Phase::Movement);
    pass_solo_response_window(&mut state, 1);
    assert_eq!(state.opponent.hero.ap_remaining, 0);
    assert_eq!(
        paced_solo_ai_step(&mut state, &policy, 2),
        Some(GameCommand::PlayCard {
            card_id,
            target: ActionTarget::Hex { coord: hex(0, -1) },
        })
    );
    assert!(state.board.units.is_empty());
    assert_eq!(state.phase, Phase::Movement);
    pass_solo_response_window(&mut state, 3);
    let unit_id = state.board.units[0].id.clone();
    assert_eq!(
        (
            state.board.units[0].ap_remaining,
            state.board.units[0].max_ap
        ),
        (2, 4)
    );
    assert_eq!(state.opponent.mana, 0);
    for (index, to, ap) in [(4, hex(0, 0), 1), (6, hex(0, 1), 0)] {
        assert_eq!(
            paced_solo_ai_step(&mut state, &policy, index),
            Some(GameCommand::MovePiece {
                piece_id: unit_id.clone(),
                to,
            })
        );
        pass_solo_response_window(&mut state, index + 1);
        assert_eq!(state.board.units[0].ap_remaining, ap);
        assert_eq!(state.phase, Phase::Movement);
    }
    assert_eq!(
        paced_solo_ai_step(&mut state, &policy, 8),
        Some(GameCommand::StartAttackPhase)
    );
    assert_eq!(state.phase, Phase::Attack);
}

#[test]
fn solo_ai_finishes_useful_movement_before_attacking_and_spends_final_card_mana() {
    use crate::commands::GameCommand;
    let mut state = MatchState::new_with_seed(7);
    state.active_side = Side::Opponent;
    state.opponent.hero.ap_remaining = 0;
    state.opponent.mana = 0;
    state.opponent.hand.clear();
    state.player.hand.clear();
    state.board.units = vec![
        board_unit("ready-attacker", Side::Opponent, hex(0, 2), 1, 1, 4),
        board_unit("moving-unit", Side::Opponent, hex(1, -2), 1, 1, 4),
    ];
    state.board.units[1].ap_remaining = 1;
    let policy = SoloAiPolicy::baseline();
    assert!(matches!(
        paced_solo_ai_step(&mut state, &policy, 0),
        Some(GameCommand::MovePiece { piece_id, .. }) if piece_id == "moving-unit"
    ));
    pass_solo_response_window(&mut state, 1);
    assert_eq!(
        paced_solo_ai_step(&mut state, &policy, 2),
        Some(GameCommand::StartAttackPhase)
    );

    let card = player_unit_card(&state, "runic-insight");
    let card_id = card.id.clone();
    let card_cost = card.cost;
    state.opponent.hand = vec![card];
    state.opponent.mana = card_cost;
    assert_eq!(
        paced_solo_ai_step(&mut state, &policy, 3),
        Some(GameCommand::Attack {
            attacker_id: "ready-attacker".into(),
            target_id: state.player.hero.id.clone(),
        })
    );
    assert_eq!(state.phase, Phase::Attack);
    assert_eq!(state.opponent.mana, card_cost);
    assert_eq!(state.opponent.hand[0].id, card_id);
    pass_solo_response_window(&mut state, 4);
    assert_eq!(state.phase, Phase::CardPlay);
    assert_eq!(
        paced_solo_ai_step(&mut state, &policy, 5),
        Some(GameCommand::PlayCard {
            card_id,
            target: ActionTarget::Piece {
                piece_id: state.opponent.hero.id.clone(),
            },
        })
    );
    pass_solo_response_window(&mut state, 6);
    assert_eq!(state.opponent.mana, 0);
    state.opponent.hand.clear();
    assert_eq!(paced_solo_ai_step(&mut state, &policy, 7), None);
    assert_eq!(state.active_side, Side::Player);
}

#[test]
fn solo_ai_uses_eligible_item_activation_during_attack_without_proactive_cards() {
    use crate::commands::GameCommand;
    let mut state = MatchState::new_with_seed(7);
    state.active_side = Side::Opponent;
    state.phase = Phase::Attack;
    state.opponent.hero.position = hex(0, 1);
    state.opponent.hero.ap_remaining = 1;
    state.opponent.mana = 2;
    state.opponent.hand = vec![player_unit_card(&state, "ember-squire")];
    state.player.hand.clear();
    let card = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "spark-needle")
        .unwrap();
    let CardKind::Item {
        passive, active, ..
    } = card.kind
    else {
        panic!("Spark Needle must be an Item card");
    };
    let item_id = card.id.clone();
    state.opponent.hero.items.push(CarriedItem {
        id: card.id,
        template_id: card.template_id,
        name: card.name,
        passive,
        active,
        active_used_this_turn: false,
    });
    assert_eq!(
        paced_solo_ai_step(&mut state, &SoloAiPolicy::baseline(), 0),
        Some(GameCommand::ActivateItem {
            carrier_id: state.opponent.hero.id.clone(),
            item_id,
            target: Some(ActionTarget::Piece {
                piece_id: state.player.hero.id.clone(),
            }),
        })
    );
    assert_eq!(state.opponent.hero.ap_remaining, 0);
    assert_eq!(state.opponent.mana, 2);
    assert_eq!(state.opponent.hand.len(), 1);
    assert_eq!(state.phase, Phase::Attack);
    pass_solo_response_window(&mut state, 1);
}

#[test]
fn solo_ai_policies_keep_distinct_legal_movement_preferences() {
    use crate::commands::GameCommand;
    let config =
        AiPolicyConfig::from_json(include_str!("../../../../backend/config/ai-policies.json"))
            .unwrap();
    let mut state = MatchState::new_with_seed(7);
    state.active_side = Side::Opponent;
    state.opponent.mana = 2;
    state.opponent.hand = vec![player_unit_card(&state, "ember-squire")];
    state.player.hand.clear();
    for (policy_id, summons_first) in [("baseline-v1", false), ("candidate-aggressive-v1", true)] {
        let policy = SoloAiPolicy::from_definition(config.policy(policy_id).unwrap());
        let mut game = state.clone();
        let command = paced_solo_ai_step(&mut game, &policy, 0).unwrap();
        assert_eq!(
            matches!(command, GameCommand::PlayCard { .. }),
            summons_first
        );
        assert_eq!(
            matches!(command, GameCommand::MovePiece { .. }),
            !summons_first
        );
        assert_eq!(game.phase, Phase::Movement);
    }
}

#[test]
fn solo_ai_preserves_historical_card_windows() {
    use crate::commands::GameCommand;
    let config =
        AiPolicyConfig::from_json(include_str!("../../../../backend/config/ai-policies.json"))
            .unwrap();
    for policy_id in ["baseline-v1", "candidate-aggressive-v1"] {
        let policy = SoloAiPolicy::from_definition(config.policy(policy_id).unwrap());
        let mut state = MatchState::new_with_seed(7);
        state.ruleset = crate::rules::LEGACY_RULESET;
        state.active_side = Side::Opponent;
        state.opponent.hero.ap_remaining = 0;
        state.opponent.mana = 2;
        state.opponent.hand = vec![player_unit_card(&state, "ember-squire")];
        assert_eq!(
            paced_solo_ai_step(&mut state, &policy, 0),
            Some(GameCommand::StartCardPlay)
        );
        assert!(matches!(
            paced_solo_ai_step(&mut state, &policy, 1),
            Some(GameCommand::PlayCard { .. })
        ));
        assert_eq!(state.phase, Phase::CardPlay);
    }
}

fn pending_solo_response_game(phase: Phase) -> MatchState {
    use crate::commands::{CommandContext, GameCommand};
    let mut state = MatchState::new_with_seed(7);
    state.active_side = Side::Opponent;
    state.phase = phase.clone();
    state.player.hero.position = hex(0, 1);
    state.opponent.hero.position = hex(0, -1);
    state.opponent.mana = 8;
    let command = match phase {
        Phase::Movement => GameCommand::MovePiece {
            piece_id: state.opponent.hero.id.clone(),
            to: hex(1, -1),
        },
        Phase::Attack => {
            state.board.units.push(board_unit(
                "attack-target",
                Side::Player,
                hex(0, 0),
                1,
                1,
                4,
            ));
            GameCommand::Attack {
                attacker_id: state.opponent.hero.id.clone(),
                target_id: "attack-target".into(),
            }
        }
        Phase::CardPlay => {
            let card = player_unit_card(&state, "runic-insight");
            let card_id = card.id.clone();
            state.opponent.hand = vec![card];
            GameCommand::PlayCard {
                card_id,
                target: ActionTarget::Piece {
                    piece_id: state.opponent.hero.id.clone(),
                },
            }
        }
        Phase::MatchOver => panic!("response fixture needs a live phase"),
    };
    command
        .execute_compatibility(
            &mut state,
            CommandContext {
                side: Side::Opponent,
                action_index: 0,
            },
        )
        .unwrap();
    assert_eq!(state.priority_side, Some(Side::Player));
    let card = player_unit_card(&state, "spark-jolt");
    let card_id = card.id.clone();
    state.player.hand = vec![card];
    GameCommand::PlayCard {
        card_id,
        target: ActionTarget::Piece {
            piece_id: state.opponent.hero.id.clone(),
        },
    }
    .execute_compatibility(
        &mut state,
        CommandContext {
            side: Side::Player,
            action_index: 1,
        },
    )
    .unwrap();
    assert_eq!(state.priority_side, Some(Side::Opponent));
    state.opponent.hero.ap_remaining = 0;
    state.opponent.mana = 2;
    state.opponent.hand.clear();
    state
}

#[test]
fn solo_ai_responds_with_legal_spells_at_zero_hero_ap_in_every_phase() {
    use crate::commands::{CommandContext, GameCommand};
    let config =
        AiPolicyConfig::from_json(include_str!("../../../../backend/config/ai-policies.json"))
            .unwrap();
    for policy_id in ["baseline-v1", "candidate-aggressive-v1"] {
        let policy = SoloAiPolicy::from_definition(config.policy(policy_id).unwrap());
        for phase in [Phase::Movement, Phase::Attack, Phase::CardPlay] {
            let mut state = pending_solo_response_game(phase.clone());
            let card = starter_card_templates()
                .into_iter()
                .find(|card| card.template_id == "overload-spark")
                .unwrap();
            let card_id = card.id.clone();
            let blocked_spell = starter_card_templates()
                .into_iter()
                .find(|card| card.template_id == "spark-jolt")
                .unwrap();
            let blocked_unit = starter_card_templates()
                .into_iter()
                .find(|card| card.template_id == "ember-squire")
                .unwrap();
            state.opponent.hand = vec![blocked_spell, blocked_unit, card];
            let before = state.to_snapshot_json().unwrap();
            let command = state
                .next_solo_ai_game_command_with_policy(Side::Opponent, &policy)
                .unwrap()
                .unwrap();
            assert!(
                matches!(&command, GameCommand::PlayCard { card_id: chosen, .. } if chosen == &card_id)
            );
            assert!(
                state
                    .queries()
                    .command_availability(Side::Opponent, &command)
                    .allowed
            );
            assert_eq!(state.to_snapshot_json().unwrap(), before);
            let mut paced = state.clone();
            command
                .execute_compatibility(
                    &mut state,
                    CommandContext {
                        side: Side::Opponent,
                        action_index: 2,
                    },
                )
                .unwrap();
            let mut frames = Vec::new();
            assert!(matches!(
                paced
                    .advance_ai_for_side_with_policy_strict(
                        Side::Opponent,
                        &policy,
                        &mut frames,
                        Some(2)
                    )
                    .unwrap(),
                AiAdvanceOutcome::ActionApplied
            ));
            assert_eq!(
                state.to_snapshot_json().unwrap(),
                paced.to_snapshot_json().unwrap()
            );
            assert_eq!(state.opponent.hero.ap_remaining, 0);
            assert_eq!(state.opponent.mana, 0);
            assert_eq!(state.phase, phase);
            assert_eq!(state.priority_side, Some(Side::Player));
            assert_eq!(state.action_stack.len(), 3);
        }
    }
}

#[test]
fn solo_ai_passes_without_useful_eligible_responses_and_rejects_wrong_priority() {
    use crate::commands::GameCommand;
    let config =
        AiPolicyConfig::from_json(include_str!("../../../../backend/config/ai-policies.json"))
            .unwrap();
    for policy_id in ["baseline-v1", "candidate-aggressive-v1"] {
        let policy = SoloAiPolicy::from_definition(config.policy(policy_id).unwrap());
        for unavailable in [
            "nonSpell",
            "lowPriority",
            "unaffordable",
            "noTarget",
            "notUseful",
        ] {
            let mut state = pending_solo_response_game(Phase::Movement);
            let template = match unavailable {
                "nonSpell" => "ember-squire",
                "notUseful" => "quick-salve",
                _ => "overload-spark",
            };
            let mut card = starter_card_templates()
                .into_iter()
                .find(|card| card.template_id == template)
                .unwrap();
            match unavailable {
                "lowPriority" => {
                    if let CardKind::Spell { priority, .. } = &mut card.kind {
                        *priority = 1;
                    }
                }
                "notUseful" => {
                    if let CardKind::Spell { priority, .. } = &mut card.kind {
                        *priority = 4;
                    }
                }
                "unaffordable" => state.opponent.mana = 0,
                "noTarget" => {
                    if let CardKind::Spell { range, .. } = &mut card.kind {
                        *range = 0;
                    }
                }
                _ => {}
            }
            state.opponent.hand = vec![card];
            assert_eq!(
                state
                    .next_solo_ai_game_command_with_policy(Side::Opponent, &policy)
                    .unwrap(),
                Some(GameCommand::PassPriority),
                "{policy_id}: {unavailable}"
            );
            let before = state.to_snapshot_json().unwrap();
            assert_eq!(
                state.next_solo_ai_game_command_with_policy(Side::Player, &policy),
                Err(MatchError::NotPrioritySide)
            );
            assert_eq!(
                state.advance_ai_for_side_with_policy_strict(
                    Side::Player,
                    &policy,
                    &mut Vec::new(),
                    Some(2)
                ),
                Err(MatchError::NotPrioritySide)
            );
            assert_eq!(state.to_snapshot_json().unwrap(), before);
            assert!(matches!(
                state
                    .advance_ai_for_side_with_policy_strict(
                        Side::Opponent,
                        &policy,
                        &mut Vec::new(),
                        Some(2)
                    )
                    .unwrap(),
                AiAdvanceOutcome::PriorityPassed
            ));
        }
    }
}

#[test]
fn solo_ai_card_evaluators_use_mana_with_zero_hero_ap_for_both_policies() {
    use crate::commands::{CommandContext, GameCommand};

    let config =
        AiPolicyConfig::from_json(include_str!("../../../../backend/config/ai-policies.json"))
            .unwrap();
    for policy_id in ["baseline-v1", "candidate-aggressive-v1"] {
        let policy = SoloAiPolicy::from_definition(config.policy(policy_id).unwrap());
        for template in [
            "ember-squire",
            "runic-insight",
            "rune-charm",
            "mana-well",
            "legacy-mana-source",
        ] {
            let mut state = MatchState::new_with_seed(7);
            state.active_side = Side::Opponent;
            state.phase = Phase::CardPlay;
            state.opponent.hero.ap_remaining = 0;
            state.opponent.mana = 8;
            if template == "rune-charm" {
                state.board.units.push(board_unit(
                    "ai-carrier",
                    Side::Opponent,
                    hex(0, -2),
                    1,
                    1,
                    2,
                ));
            }
            let catalog_template = if template == "legacy-mana-source" {
                "mana-well"
            } else {
                template
            };
            let mut card = starter_card_templates()
                .into_iter()
                .find(|card| card.template_id == catalog_template)
                .unwrap();
            if template == "legacy-mana-source" {
                card.kind = CardKind::ManaSource;
            }
            let cost = card.cost;
            let card_id = card.id.clone();
            state.opponent.hand = vec![card];
            let before = state.to_snapshot_json().unwrap();
            let command = state
                .next_solo_ai_game_command_with_policy(Side::Opponent, &policy)
                .unwrap()
                .unwrap();
            assert!(
                matches!(&command, GameCommand::PlayCard { card_id: chosen, .. } if chosen == &card_id),
                "{policy_id} must consider {template} with zero Hero AP"
            );
            let context = CommandContext {
                side: Side::Opponent,
                action_index: 0,
            };
            assert_eq!(state.to_snapshot_json().unwrap(), before);
            assert!(
                state
                    .queries()
                    .command_availability(Side::Opponent, &command)
                    .allowed
            );
            let mut paced = state.clone();
            command.execute_compatibility(&mut state, context).unwrap();
            assert_eq!(state.opponent.hero.ap_remaining, 0);
            assert_eq!(state.opponent.mana, 8 - cost);
            assert!(matches!(
                paced
                    .advance_ai_for_side_with_policy_strict(
                        Side::Opponent,
                        &policy,
                        &mut Vec::new(),
                        Some(0)
                    )
                    .unwrap(),
                AiAdvanceOutcome::ActionApplied
            ));
            assert_eq!(
                paced.to_snapshot_json().unwrap(),
                state.to_snapshot_json().unwrap()
            );

            let mut unaffordable = MatchState::from_snapshot_json(&before).unwrap();
            unaffordable.opponent.mana = cost - 1;
            assert_eq!(
                unaffordable
                    .next_solo_ai_game_command_with_policy(Side::Opponent, &policy)
                    .unwrap(),
                None,
                "{policy_id} must still reject unaffordable {template}"
            );
        }
    }
}

#[test]
fn ending_turn_starts_paced_ai_turn() {
    let mut game = MatchState::new_with_seed(7);
    enter_card_play(&mut game);

    game.apply_action(MatchActionRequest::EndTurn)
        .expect("ending turn should work");

    assert_eq!(game.round, 1);
    assert_eq!(game.active_side, Side::Opponent);
    assert_eq!(game.action_stack.len(), 0);
}

#[test]
fn advancing_ai_eventually_advances_round() {
    let mut game = MatchState::new_with_seed(7);
    enter_card_play(&mut game);

    game.apply_action(MatchActionRequest::EndTurn)
        .expect("ending turn should work");

    advance_solo_ai_until_player_turn(&mut game);

    assert_eq!(game.round, 2);
    assert_eq!(game.active_side, Side::Player);
    assert_eq!(game.player.max_mana, 3);
    assert_eq!(game.player.hand.len(), 8);
}

#[test]
fn solo_ai_actions_wait_for_player_priority_response() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.player.hero.hp = 18;
    game.opponent.hero.position = hex(0, -1);
    game.opponent.hero.ap_remaining = 1;
    let salve = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "quick-salve")
        .expect("priority response spell exists");
    let salve_id = put_card_in_side_hand(&mut game, Side::Player, salve);

    enter_card_play(&mut game);
    game.apply_action(MatchActionRequest::EndTurn)
        .expect("ending turn should start AI turn");
    for _ in 0..10 {
        game.apply_action(MatchActionRequest::AdvanceAi)
            .expect("AI can advance through the required phases");
        if !game.action_stack.is_empty() {
            break;
        }
    }

    assert_eq!(game.action_stack.len(), 1);
    assert_eq!(game.priority_side, Some(Side::Player));

    game.apply_action(MatchActionRequest::PlayCard {
        card_id: salve_id,
        target: ActionTarget::Piece {
            piece_id: game.player.hero.id.clone(),
        },
    })
    .expect("player can answer AI action with higher priority spell");

    assert_eq!(game.action_stack.len(), 2);
    assert_eq!(game.priority_side, Some(Side::Opponent));
    assert_eq!(game.player.hero.hp, 18);

    game.apply_action(MatchActionRequest::AdvanceAi)
        .expect("AI should pass priority to resolve the response");
    assert_eq!(game.player.hero.hp, 20);
    assert_eq!(game.priority_side, Some(Side::Player));
}

#[test]
fn turn_phase_actions_gate_movement_attack_card_play_and_end_turn() {
    let mut game = MatchState::new_with_seed(7);
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);
    game.phase = Phase::Movement;

    assert_eq!(
        game.apply_action(MatchActionRequest::Attack {
            attacker_id: "player-hero".to_string(),
            target_id: "opponent-hero".to_string(),
        }),
        Err(MatchError::WrongPhase)
    );
    assert_eq!(
        game.apply_action(MatchActionRequest::StartCardPlay),
        Err(MatchError::WrongPhase)
    );
    assert_eq!(
        game.apply_action(MatchActionRequest::EndTurn),
        Err(MatchError::WrongPhase)
    );

    game.apply_action(MatchActionRequest::StartAttackPhase)
        .expect("movement phase can advance to attack");
    assert_eq!(game.phase, Phase::Attack);
    assert_eq!(
        game.apply_action(MatchActionRequest::MovePiece {
            piece_id: "player-hero".to_string(),
            to: hex(0, 2),
        }),
        Err(MatchError::WrongPhase)
    );
    assert_eq!(
        game.apply_action(MatchActionRequest::PlayCard {
            card_id: card_id.clone(),
            target: ActionTarget::Hex { coord: hex(0, 2) },
        }),
        Err(MatchError::WrongPhase)
    );

    game.apply_action(MatchActionRequest::StartCardPlay)
        .expect("attack phase can finish into card play");
    assert_eq!(game.phase, Phase::CardPlay);
    game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 2) },
    })
    .expect("card play phase can play cards");
}

#[test]
fn movement_phase_cannot_skip_attack_phase() {
    let mut game = MatchState::new_with_seed(7);

    assert_eq!(
        game.apply_action(MatchActionRequest::StartCardPlay),
        Err(MatchError::WrongPhase)
    );
    assert_eq!(game.phase, Phase::Movement);
}

#[test]
fn movement_phase_card_play_spends_only_mana_and_resumes_movement() {
    let mut game = MatchState::new_with_seed(7);
    let card = player_unit_card(&game, "ember-squire");
    let card_cost = card.cost;
    let card_id = put_card_in_hand(&mut game, card);
    game.phase = Phase::Movement;
    let hero_ap_before = game.player.hero.ap_remaining;
    let mana_before = game.player.mana;

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id,
                target: ActionTarget::Hex { coord: hex(0, 2) },
            },
            20,
        )
        .expect("movement phase can initiate a proactive card");

    assert_eq!(game.phase, Phase::Movement);
    assert_eq!(game.player.hero.ap_remaining, hero_ap_before);
    assert_eq!(game.player.mana, mana_before - card_cost);
    assert!(game.action_stack.is_empty());
    assert!(
        game.board
            .units
            .iter()
            .any(|unit| unit.template_id.as_deref() == Some("ember-squire"))
    );
    assert!(!frames.iter().any(|frame| matches!(
        frame.event,
        ReplayEvent::PhaseChanged {
            phase: Phase::CardPlay,
            ..
        }
    )));
}

#[test]
fn shared_movement_card_resolves_without_changing_the_underlying_phase() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_side_hand(&mut game, Side::Player, card);
    game.phase = Phase::Movement;

    game.apply_action_recording_for_side(
        Side::Player,
        MatchActionRequest::PlayCard {
            card_id,
            target: ActionTarget::Hex { coord: hex(0, 2) },
        },
        21,
    )
    .expect("shared active side can initiate a movement-phase card");

    assert_eq!(game.phase, Phase::Movement);
    assert_eq!(game.action_stack.len(), 1);
    assert_eq!(game.priority_side, Some(Side::Opponent));

    game.apply_action_recording_for_side(Side::Opponent, MatchActionRequest::PassPriority, 22)
        .expect("opponent can pass priority to resolve the movement-phase card");

    assert_eq!(game.phase, Phase::Movement);
    assert!(game.action_stack.is_empty());
    assert!(
        game.board
            .units
            .iter()
            .any(|unit| unit.template_id.as_deref() == Some("ember-squire"))
    );
}

#[test]
fn attack_phase_auto_enters_card_play_when_no_legal_attacks_remain() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 2),
        0,
        1,
        1,
    ));

    let frames = game
        .apply_action_recording(
            MatchActionRequest::Attack {
                attacker_id: "player-hero".to_string(),
                target_id: "opponent-unit".to_string(),
            },
            10,
        )
        .expect("only legal attack should resolve");

    assert_eq!(game.phase, Phase::CardPlay);
    assert!(frames.iter().any(|frame| matches!(
        frame.event,
        ReplayEvent::PhaseChanged {
            side: Side::Player,
            phase: Phase::CardPlay,
        }
    )));
}

#[test]
fn active_turn_card_play_ignores_hero_action_points() {
    let mut game = MatchState::new_with_seed(7);
    game.player.hero.ap_remaining = 0;
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);
    game.player.hero.ap_remaining = 0;

    game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 2) },
    })
    .expect("card play should spend mana only");

    assert_eq!(game.player.hero.ap_remaining, 0);
    assert_eq!(game.player.mana, 2);
}

#[test]
fn priority_response_spell_ignores_hero_action_points() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 0;
    game.priority_side = Some(Side::Player);
    game.action_stack.push(StackItem {
        id: "pending-attack".to_string(),
        side: Side::Opponent,
        priority: 1,
        action: StackAction::Attack {
            attacker_id: "opponent-hero".to_string(),
            target_id: "player-hero".to_string(),
        },
    });
    let salve = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "quick-salve")
        .expect("priority response spell exists");
    let salve_id = put_card_in_side_hand(&mut game, Side::Player, salve);

    game.apply_action(MatchActionRequest::PlayCard {
        card_id: salve_id,
        target: ActionTarget::Piece {
            piece_id: "player-hero".to_string(),
        },
    })
    .expect("priority spell should not require hero action points");

    assert_eq!(game.player.hero.ap_remaining, 0);
    assert_eq!(game.action_stack.len(), 2);
}

#[test]
fn turn_start_mana_comes_from_hero_and_occupied_mana_sources() {
    let mut game = MatchState::new_with_seed(7);
    game.player.hero.position = hex(0, 0);
    game.player.mana = 0;
    game.player.max_mana = 0;
    game.opponent.mana = 0;
    game.opponent.max_mana = 0;

    game.start_turn(Side::Player, &mut Vec::new(), None);

    assert_eq!(game.player.max_mana, 4);
    assert_eq!(game.player.mana, 4);
    assert_eq!(game.opponent.max_mana, 0);
    assert_eq!(game.opponent.mana, 0);
}

#[test]
fn adjacent_unoccupied_sources_do_not_generate_mana() {
    let mut game = MatchState::new_with_seed(7);
    game.player.hero.position = hex(0, 1);
    game.player.mana = 0;
    game.player.max_mana = 0;

    game.start_turn(Side::Player, &mut Vec::new(), None);

    assert_eq!(game.player.max_mana, 3);
    assert_eq!(game.player.mana, 3);
}

#[test]
fn any_side_can_draw_from_an_occupied_mana_source() {
    let mut game = MatchState::new_with_seed(7);
    game.board.mana_sources.push(hex(0, -2));
    game.opponent.hero.position = hex(0, -2);
    game.opponent.mana = 0;
    game.opponent.max_mana = 0;

    game.start_turn(Side::Opponent, &mut Vec::new(), None);

    assert_eq!(game.opponent.max_mana, 4);
    assert_eq!(game.opponent.mana, 4);
}

#[test]
fn unspent_mana_remains_for_reactions_until_next_own_turn_refresh() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    game.player.mana = 5;
    game.player.max_mana = 5;
    enter_card_play(&mut game);

    game.apply_action_recording_for_side(Side::Player, MatchActionRequest::EndTurn, 0)
        .expect("player can end their active turn");

    assert_eq!(game.active_side, Side::Opponent);
    assert_eq!(game.player.mana, 5);
    assert_eq!(game.player.max_mana, 5);

    enter_card_play(&mut game);
    game.apply_action_recording_for_side(Side::Opponent, MatchActionRequest::EndTurn, 1)
        .expect("opponent can end their active turn");

    assert_eq!(game.active_side, Side::Player);
    assert_eq!(game.player.mana, 3);
    assert_eq!(game.player.max_mana, 3);
}

#[test]
fn solo_turn_start_refreshes_only_active_side_unit_armor() {
    let mut game = MatchState::new_with_seed(7);
    game.opponent.hand.clear();
    game.board.units.push(damaged_board_unit(
        "player-guard",
        Side::Player,
        hex(0, 2),
        1,
        4,
    ));
    game.board.units.push(damaged_board_unit(
        "opponent-guard",
        Side::Opponent,
        hex(0, -2),
        2,
        6,
    ));

    enter_card_play(&mut game);
    game.apply_action(MatchActionRequest::EndTurn)
        .expect("ending turn should start the opponent turn");

    assert_eq!(unit_armor(&game, "player-guard"), Some(1));
    assert_eq!(unit_armor(&game, "opponent-guard"), Some(6));

    advance_solo_ai_until_player_turn(&mut game);

    assert_eq!(unit_armor(&game, "player-guard"), Some(4));
    assert_eq!(unit_armor(&game, "opponent-guard"), Some(6));
}

#[test]
fn every_card_kind_uses_mana_only_in_both_proactive_windows() {
    for phase in [Phase::Movement, Phase::CardPlay, Phase::Attack] {
        for template in [
            "swift-familiar",
            "runic-insight",
            "runekeeper-lens",
            "mana-well",
            "legacy-mana-source",
        ] {
            let mut game = MatchState::new_with_seed(7);
            game.phase = phase.clone();
            game.player.hero.position = hex(0, 1);
            game.player.mana = 8;
            game.board
                .units
                .push(board_unit("ally", Side::Player, hex(0, 2), 4, 1, 2));
            let mut card = starter_card_templates()
                .into_iter()
                .find(|card| {
                    card.template_id
                        == if template == "legacy-mana-source" {
                            "mana-well"
                        } else {
                            template
                        }
                })
                .unwrap();
            if template == "legacy-mana-source" {
                card.kind = CardKind::ManaSource;
            }
            let cost = card.cost;
            let target = match card.kind {
                CardKind::Unit { .. } | CardKind::Building { .. } | CardKind::ManaSource => {
                    ActionTarget::Hex { coord: hex(1, 1) }
                }
                CardKind::Spell { .. } => ActionTarget::Piece {
                    piece_id: game.player.hero.id.clone(),
                },
                CardKind::Item { .. } => ActionTarget::Piece {
                    piece_id: "ally".into(),
                },
            };
            let card_id = put_card_in_side_hand(&mut game, Side::Player, card);
            let hero_ap = game.player.hero.ap_remaining;
            let unit_ap = game.board.units[0].ap_remaining;
            let action = MatchActionRequest::PlayCard { card_id, target };
            let before = game.to_snapshot_json().unwrap();
            let result = game.apply_action(action);
            if phase == Phase::Attack {
                assert_eq!(result, Err(MatchError::WrongPhase), "{template}");
                assert_eq!(game.to_snapshot_json().unwrap(), before);
            } else {
                result.unwrap();
                assert_eq!(game.player.mana, 8 - cost, "{template}");
                assert_eq!(game.player.hero.ap_remaining, hero_ap, "{template}");
                assert_eq!(game.board.units[0].ap_remaining, unit_ap, "{template}");
                assert_eq!(game.phase, phase);
                assert!(game.action_stack.is_empty());
            }
        }
    }
}

#[test]
fn movement_summoned_unit_can_move_then_attack_with_its_entry_budget() {
    let mut game = MatchState::new_with_seed(7);
    game.player.hero.position = hex(0, 1);
    let card = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "rune-runner")
        .unwrap();
    let card_id = put_card_in_side_hand(&mut game, Side::Player, card);
    game.board
        .units
        .push(board_unit("enemy", Side::Opponent, hex(2, 0), 0, 1, 4));
    game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(1, 1) },
    })
    .unwrap();
    let unit = game
        .board
        .units
        .iter()
        .find(|unit| unit.template_id.as_deref() == Some("rune-runner"))
        .unwrap();
    let unit_id = unit.id.clone();
    assert_eq!((unit.ap_remaining, unit.max_ap), (2, 4));
    game.apply_action(MatchActionRequest::MovePiece {
        piece_id: unit_id.clone(),
        to: hex(1, 0),
    })
    .unwrap();
    assert_eq!(
        game.board
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
            .unwrap()
            .ap_remaining,
        1
    );
    game.apply_action(MatchActionRequest::StartAttackPhase)
        .unwrap();
    game.apply_action(MatchActionRequest::Attack {
        attacker_id: unit_id.clone(),
        target_id: "enemy".into(),
    })
    .unwrap();
    let unit = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == unit_id)
        .unwrap();
    assert_eq!(unit.ap_remaining, 0);
    assert!(unit.has_attacked);
    assert_eq!(unit_armor(&game, "enemy"), Some(3));
}
