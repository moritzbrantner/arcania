use super::*;

#[test]
fn shared_turn_start_refreshes_only_the_active_side_mana() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    game.player.mana = 1;
    game.player.max_mana = 4;
    game.opponent.mana = 2;
    game.opponent.max_mana = 4;
    enter_card_play(&mut game);

    game.apply_action_recording_for_side(Side::Player, MatchActionRequest::EndTurn, 0)
        .expect("player can end their active turn");

    assert_eq!(game.round, 1);
    assert_eq!(game.active_side, Side::Opponent);
    assert_eq!(game.player.mana, 1);
    assert_eq!(game.player.max_mana, 4);
    assert_eq!(game.opponent.mana, 3);
    assert_eq!(game.opponent.max_mana, 3);
}

#[test]
fn shared_matches_reject_inactive_side_actions() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );

    let result =
        game.apply_action_recording_for_side(Side::Opponent, MatchActionRequest::EndTurn, 0);

    assert_eq!(result.err(), Some(MatchError::NotActiveSide));
}

#[test]
fn shared_turns_pass_between_humans_without_running_ai() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    enter_card_play(&mut game);

    game.apply_action_recording_for_side(Side::Player, MatchActionRequest::EndTurn, 0)
        .expect("player can end their active turn");

    assert_eq!(game.active_side, Side::Opponent);
    assert_eq!(game.round, 1);
    assert_eq!(game.opponent.hero.hero_type, HeroType::Pyromancer);

    enter_card_play(&mut game);
    game.apply_action_recording_for_side(Side::Opponent, MatchActionRequest::EndTurn, 1)
        .expect("opponent can end their active turn");

    assert_eq!(game.active_side, Side::Player);
    assert_eq!(game.round, 2);
    assert_eq!(game.player.hand.len(), 8);
}

#[test]
fn shared_turn_start_refreshes_only_active_side_unit_armor() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    game.board.units.push(damaged_board_unit(
        "player-guard",
        Side::Player,
        hex(0, 2),
        1,
        5,
    ));
    game.board.units.push(damaged_board_unit(
        "opponent-guard",
        Side::Opponent,
        hex(0, -2),
        2,
        6,
    ));
    game.board.units.push(damaged_board_unit(
        "opponent-full-guard",
        Side::Opponent,
        hex(1, -2),
        4,
        4,
    ));
    game.board.units.push(damaged_board_unit(
        "opponent-dead-guard",
        Side::Opponent,
        hex(-1, -2),
        0,
        4,
    ));

    enter_card_play(&mut game);
    let opponent_turn_frames = game
        .apply_action_recording_for_side(Side::Player, MatchActionRequest::EndTurn, 0)
        .expect("player can end their active turn");

    assert_eq!(unit_armor(&game, "player-guard"), Some(1));
    assert_eq!(unit_armor(&game, "opponent-guard"), Some(6));
    assert_eq!(unit_armor(&game, "opponent-full-guard"), Some(4));
    assert_eq!(unit_armor(&game, "opponent-dead-guard"), Some(0));
    assert!(matches!(
        opponent_turn_frames.as_slice(),
        [
            RecordedReplayFrame {
                event: ReplayEvent::TurnEnded {
                    side: Side::Player,
                    ..
                },
                ..
            },
            RecordedReplayFrame {
                event: ReplayEvent::TurnStarted {
                    side: Side::Opponent,
                    ..
                },
                ..
            },
            RecordedReplayFrame {
                event: ReplayEvent::UnitArmorRefreshed {
                    side: Side::Opponent,
                    unit_id,
                    amount: 4,
                },
                ..
            },
            ..
        ] if unit_id == "opponent-guard"
    ));
    assert_eq!(
        opponent_turn_frames
            .iter()
            .filter(|frame| matches!(frame.event, ReplayEvent::UnitArmorRefreshed { .. }))
            .count(),
        1
    );

    enter_card_play(&mut game);
    let player_turn_frames = game
        .apply_action_recording_for_side(Side::Opponent, MatchActionRequest::EndTurn, 1)
        .expect("opponent can end their active turn");

    assert_eq!(unit_armor(&game, "player-guard"), Some(5));
    assert_eq!(
        card_play_event_names(&player_turn_frames),
        vec!["unitArmorRefreshed", "cardDrawn"]
    );
    assert!(matches!(
        player_turn_frames.as_slice(),
        [
            RecordedReplayFrame {
                event: ReplayEvent::TurnEnded {
                    side: Side::Opponent,
                    ..
                },
                ..
            },
            RecordedReplayFrame {
                event: ReplayEvent::RoundStarted { .. },
                ..
            },
            RecordedReplayFrame {
                event: ReplayEvent::TurnStarted {
                    side: Side::Player,
                    ..
                },
                ..
            },
            RecordedReplayFrame {
                event: ReplayEvent::UnitArmorRefreshed {
                    side: Side::Player,
                    unit_id,
                    amount: 4,
                },
                ..
            },
            RecordedReplayFrame {
                event: ReplayEvent::CardDrawn {
                    side: Side::Player,
                    ..
                },
                ..
            },
        ] if unit_id == "player-guard"
    ));
}

#[test]
fn shared_spell_responses_require_higher_priority_and_resolve_lifo() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    game.player.mana = 8;
    game.opponent.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.opponent.hero.ap_remaining = 3;
    game.opponent.hero.position = hex(0, -2);
    game.board.units.push(Unit {
        id: "guard".to_string(),
        side: Side::Opponent,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 4,
        max_armor: 4,
        position: hex(0, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });

    let bolt = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "starfire-bolt")
        .expect("damage spell exists");
    let parry = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "arcane-parry")
        .expect("priority response spell exists");
    let bolt_id = put_card_in_side_hand(&mut game, Side::Player, bolt);
    let parry_id = put_card_in_side_hand(&mut game, Side::Opponent, parry);

    enter_card_play(&mut game);
    game.apply_action_recording_for_side(
        Side::Player,
        MatchActionRequest::PlayCard {
            card_id: bolt_id,
            target: ActionTarget::Piece {
                piece_id: "guard".to_string(),
            },
        },
        0,
    )
    .expect("active player can put a spell on the stack");

    assert_eq!(game.action_stack.len(), 1);
    assert_eq!(game.priority_side, Some(Side::Opponent));
    assert_eq!(
        game.board
            .units
            .iter()
            .find(|unit| unit.id == "guard")
            .map(|unit| unit.armor),
        Some(4)
    );

    game.apply_action_recording_for_side(
        Side::Opponent,
        MatchActionRequest::PlayCard {
            card_id: parry_id,
            target: ActionTarget::Piece {
                piece_id: "guard".to_string(),
            },
        },
        1,
    )
    .expect("opponent can answer with higher priority");

    assert_eq!(game.action_stack.len(), 2);
    assert_eq!(game.priority_side, Some(Side::Player));

    game.apply_action_recording_for_side(Side::Player, MatchActionRequest::PassPriority, 2)
        .expect("player can pass to resolve the response");
    assert_eq!(
        game.board
            .units
            .iter()
            .find(|unit| unit.id == "guard")
            .map(|unit| (unit.armor, unit.max_armor)),
        Some((6, 6))
    );
    assert_eq!(game.priority_side, Some(Side::Opponent));

    game.apply_action_recording_for_side(Side::Opponent, MatchActionRequest::PassPriority, 3)
        .expect("opponent can pass to resolve the original spell");
    assert_eq!(game.action_stack.len(), 0);
    assert_eq!(game.priority_side, None);
    assert_eq!(
        game.board
            .units
            .iter()
            .find(|unit| unit.id == "guard")
            .map(|unit| unit.armor),
        Some(2)
    );
}

#[test]
fn lower_priority_spells_cannot_answer_pending_actions() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        MatchMode::Shared,
    );
    game.player.mana = 8;
    game.opponent.mana = 8;
    game.opponent.hero.position = hex(0, -2);
    game.board.units.push(Unit {
        id: "guard".to_string(),
        side: Side::Opponent,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 4,
        max_armor: 4,
        position: hex(0, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });

    let bolt = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "starfire-bolt")
        .expect("higher priority damage spell exists");
    let chant = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "war-chant")
        .expect("lower priority spell exists");
    let bolt_id = put_card_in_side_hand(&mut game, Side::Player, bolt);
    let chant_id = put_card_in_side_hand(&mut game, Side::Opponent, chant);

    enter_card_play(&mut game);
    game.apply_action_recording_for_side(
        Side::Player,
        MatchActionRequest::PlayCard {
            card_id: bolt_id,
            target: ActionTarget::Piece {
                piece_id: "guard".to_string(),
            },
        },
        0,
    )
    .expect("active player can put a spell on the stack");

    let result = game.apply_action_recording_for_side(
        Side::Opponent,
        MatchActionRequest::PlayCard {
            card_id: chant_id,
            target: ActionTarget::Piece {
                piece_id: "guard".to_string(),
            },
        },
        1,
    );

    assert_eq!(result.err(), Some(MatchError::PriorityTooLow));
}
