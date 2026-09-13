use super::*;

#[test]
fn movement_costs_action_points_and_requires_empty_adjacency() {
    let mut game = MatchState::new_with_seed(7);

    game.apply_action(MatchActionRequest::MovePiece {
        piece_id: "player-hero".to_string(),
        to: hex(0, 2),
    })
    .expect("hero can move one hex");

    assert_eq!(game.player.hero.position, hex(0, 2));
    assert_eq!(game.player.hero.ap_remaining, 2);

    let result = game.apply_action(MatchActionRequest::MovePiece {
        piece_id: "player-hero".to_string(),
        to: hex(0, 0),
    });

    assert_eq!(result, Err(MatchError::NotAdjacent));
}

#[test]
fn adjacent_attacks_apply_counterdamage_once_per_piece() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(Unit {
        id: "player-unit".to_string(),
        side: Side::Player,
        name: "Rune Bruiser".to_string(),
        template_id: Some("rune-bruiser".to_string()),
        attack: 2,
        attack_range: 1,
        armor: 2,
        max_armor: 2,
        position: hex(0, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "opponent-unit".to_string(),
        side: Side::Opponent,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 4,
        max_armor: 4,
        position: hex(1, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });

    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-unit".to_string(),
        target_id: "opponent-unit".to_string(),
    })
    .expect("adjacent attack should work");

    let player = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "player-unit")
        .expect("player unit survives");
    let opponent = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "opponent-unit")
        .expect("opponent unit survives");
    assert_eq!(player.armor, 1);
    assert_eq!(opponent.armor, 2);
    assert!(player.has_attacked);

    enter_attack_phase(&mut game);
    let result = game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-unit".to_string(),
        target_id: "opponent-unit".to_string(),
    });

    assert_eq!(result, Err(MatchError::AlreadyAttacked));
}

#[test]
fn archer_attacks_at_range_two_without_melee_counterdamage() {
    let mut game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Archer);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 1),
        5,
        1,
        4,
    ));

    let frames = game
        .apply_action_recording(
            MatchActionRequest::Attack {
                attacker_id: "player-hero".to_string(),
                target_id: "opponent-unit".to_string(),
            },
            50,
        )
        .expect("archer should attack at range two");

    assert_eq!(game.player.hero.hp, game.player.hero.max_hp);
    assert_eq!(unit_armor(&game, "opponent-unit"), Some(2));
    assert!(frames.iter().any(|frame| matches!(
        frame.event,
        ReplayEvent::PieceAttacked {
            counter_damage_to_attacker: 0,
            ..
        }
    )));
}

#[test]
fn melee_pieces_cannot_attack_at_range_two() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 1),
        1,
        1,
        2,
    ));

    let result = game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-hero".to_string(),
        target_id: "opponent-unit".to_string(),
    });

    assert_eq!(result, Err(MatchError::NotAdjacent));
}

#[test]
fn range_two_targets_counterdamage_at_range_two() {
    let mut game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Archer);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 1),
        3,
        2,
        4,
    ));

    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-hero".to_string(),
        target_id: "opponent-unit".to_string(),
    })
    .expect("range two target should be attackable");

    assert_eq!(game.player.hero.hp, game.player.hero.max_hp - 3);
}

#[test]
fn barbarian_hero_gains_mana_over_cap_when_attack_kills_unit() {
    let mut game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Barbarian);
    enter_attack_phase(&mut game);
    game.player.mana = 8;
    game.player.max_mana = 8;
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 2),
        0,
        1,
        3,
    ));

    let frames = game
        .apply_action_recording(
            MatchActionRequest::Attack {
                attacker_id: "player-hero".to_string(),
                target_id: "opponent-unit".to_string(),
            },
            51,
        )
        .expect("barbarian should kill adjacent unit");

    assert_eq!(game.player.mana, 10);
    assert_eq!(game.player.max_mana, 8);
    let destroyed_index = frames
        .iter()
        .position(|frame| matches!(frame.event, ReplayEvent::UnitDestroyed { .. }))
        .expect("kill should destroy a unit");
    let mana_index = frames
        .iter()
        .position(|frame| matches!(frame.event, ReplayEvent::ManaGained { .. }))
        .expect("barbarian kill should gain mana");
    assert!(destroyed_index < mana_index);
    assert!(matches!(
        &frames[mana_index].event,
        ReplayEvent::ManaGained {
            side: Side::Player,
            amount: 2,
            source: ReplayManaSource::BarbarianKill { hero_id, unit_id },
        } if hero_id == "player-hero" && unit_id == "opponent-unit"
    ));
}

#[test]
fn barbarian_hero_gains_mana_when_counterdamage_kills_unit() {
    let mut game = MatchState::new_with_seed_hero_types_and_mode(
        7,
        HeroType::Runekeeper,
        HeroType::Barbarian,
        MatchMode::Solo,
    );
    enter_attack_phase(&mut game);
    game.board
        .units
        .push(board_unit("player-unit", Side::Player, hex(0, -2), 0, 1, 3));
    let starting_mana = game.opponent.mana;

    let frames = game
        .apply_action_recording(
            MatchActionRequest::Attack {
                attacker_id: "player-unit".to_string(),
                target_id: "opponent-hero".to_string(),
            },
            52,
        )
        .expect("unit should attack adjacent barbarian");

    assert_eq!(game.opponent.mana, starting_mana + 2);
    assert!(frames.iter().any(|frame| matches!(
        &frame.event,
        ReplayEvent::ManaGained {
            side: Side::Opponent,
            source: ReplayManaSource::BarbarianKill { hero_id, unit_id },
            ..
        } if hero_id == "opponent-hero" && unit_id == "player-unit"
    )));
}

#[test]
fn barbarian_controlled_unit_kill_does_not_gain_mana() {
    let mut game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Barbarian);
    enter_attack_phase(&mut game);
    game.player.mana = 4;
    game.board
        .units
        .push(board_unit("player-unit", Side::Player, hex(0, 2), 3, 1, 4));
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 1),
        0,
        1,
        3,
    ));

    let frames = game
        .apply_action_recording(
            MatchActionRequest::Attack {
                attacker_id: "player-unit".to_string(),
                target_id: "opponent-unit".to_string(),
            },
            53,
        )
        .expect("barbarian-controlled unit should kill enemy unit");

    assert_eq!(game.player.mana, 4);
    assert!(
        !frames
            .iter()
            .any(|frame| matches!(frame.event, ReplayEvent::ManaGained { .. }))
    );
}

#[test]
fn barbarian_spell_kill_does_not_gain_mana() {
    let mut game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Barbarian);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 2),
        0,
        1,
        2,
    ));
    let bolt = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "starfire-bolt")
        .expect("damage spell exists");
    let bolt_id = put_card_in_hand(&mut game, bolt);

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: bolt_id,
                target: ActionTarget::Piece {
                    piece_id: "opponent-unit".to_string(),
                },
            },
            54,
        )
        .expect("spell should kill enemy unit");

    assert_eq!(game.player.mana, 3);
    assert!(
        !frames
            .iter()
            .any(|frame| matches!(frame.event, ReplayEvent::ManaGained { .. }))
    );
}

#[test]
fn area_damage_hits_enemies_near_the_target_only() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.board.units.push(Unit {
        id: "enemy-center".to_string(),
        side: Side::Opponent,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 4,
        max_armor: 4,
        position: hex(0, 1),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "enemy-neighbor".to_string(),
        side: Side::Opponent,
        name: "Rune Bruiser".to_string(),
        template_id: Some("rune-bruiser".to_string()),
        attack: 2,
        attack_range: 1,
        armor: 3,
        max_armor: 3,
        position: hex(1, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "ally-neighbor".to_string(),
        side: Side::Player,
        name: "Ember Squire".to_string(),
        template_id: Some("ember-squire".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 2,
        max_armor: 2,
        position: hex(-1, 2),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let cinder = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "cinder-ring")
        .expect("area spell exists");
    let cinder_id = put_card_in_hand(&mut game, cinder);

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: cinder_id,
                target: ActionTarget::Piece {
                    piece_id: "enemy-center".to_string(),
                },
            },
            50,
        )
        .expect("area damage should be playable on an enemy");

    assert_eq!(unit_armor(&game, "enemy-center"), Some(3));
    assert_eq!(unit_armor(&game, "enemy-neighbor"), Some(2));
    assert_eq!(unit_armor(&game, "ally-neighbor"), Some(2));
    assert_eq!(
        card_play_event_names(&frames),
        vec!["cardPlayed", "actionQueued", "pieceDamaged", "pieceDamaged",]
    );
}

#[test]
fn line_damage_hits_enemies_in_a_straight_line() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.board.units.push(Unit {
        id: "enemy-front".to_string(),
        side: Side::Opponent,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 4,
        max_armor: 4,
        position: hex(0, 1),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "enemy-back".to_string(),
        side: Side::Opponent,
        name: "Rune Bruiser".to_string(),
        template_id: Some("rune-bruiser".to_string()),
        attack: 2,
        attack_range: 1,
        armor: 3,
        max_armor: 3,
        position: hex(0, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "enemy-offline".to_string(),
        side: Side::Opponent,
        name: "Swift Familiar".to_string(),
        template_id: Some("swift-familiar".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 3,
        max_armor: 3,
        position: hex(1, 0),
        ap_remaining: 3,
        max_ap: 3,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let ray = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "prism-ray")
        .expect("line spell exists");
    let ray_id = put_card_in_hand(&mut game, ray);

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: ray_id,
                target: ActionTarget::Piece {
                    piece_id: "enemy-front".to_string(),
                },
            },
            60,
        )
        .expect("line damage should be playable on a straight-line enemy");

    assert_eq!(unit_armor(&game, "enemy-front"), Some(2));
    assert_eq!(unit_armor(&game, "enemy-back"), Some(1));
    assert_eq!(unit_armor(&game, "enemy-offline"), Some(3));
    assert_eq!(
        card_play_event_names(&frames),
        vec!["cardPlayed", "actionQueued", "pieceDamaged", "pieceDamaged",]
    );
}

#[test]
fn line_damage_rejects_non_straight_targets() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.board.units.push(Unit {
        id: "enemy-offline".to_string(),
        side: Side::Opponent,
        name: "Swift Familiar".to_string(),
        template_id: Some("swift-familiar".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 3,
        max_armor: 3,
        position: hex(1, 1),
        ap_remaining: 3,
        max_ap: 3,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let ray = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "prism-ray")
        .expect("line spell exists");
    let ray_id = put_card_in_hand(&mut game, ray);

    let result = game.apply_action(MatchActionRequest::PlayCard {
        card_id: ray_id,
        target: ActionTarget::Piece {
            piece_id: "enemy-offline".to_string(),
        },
    });

    assert_eq!(result, Err(MatchError::InvalidTarget));
}

#[test]
fn hero_death_ends_the_match() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(Unit {
        id: "player-unit".to_string(),
        side: Side::Player,
        name: "Iron Colossus".to_string(),
        template_id: Some("iron-colossus".to_string()),
        attack: 20,
        attack_range: 1,
        armor: 6,
        max_armor: 6,
        position: hex(0, -2),
        ap_remaining: 1,
        max_ap: 1,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });

    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-unit".to_string(),
        target_id: "opponent-hero".to_string(),
    })
    .expect("hero can be attacked when adjacent");

    assert_eq!(game.phase, Phase::MatchOver);
    assert_eq!(game.winner, Some(Side::Player));
}

#[test]
fn surviving_melee_attacker_advances_after_destroying_unit() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "player-unit",
        Side::Player,
        hex(0, 1),
        3,
        1,
        4,
    ));
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 0),
        1,
        1,
        2,
    ));

    let frames = game
        .apply_action_recording(
            MatchActionRequest::Attack {
                attacker_id: "player-unit".to_string(),
                target_id: "opponent-unit".to_string(),
            },
            90,
        )
        .expect("melee kill should resolve");

    let attacker = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "player-unit")
        .expect("attacker should survive");
    assert_eq!(attacker.position, hex(0, 0));
    assert!(frames.iter().any(|frame| matches!(
        &frame.event,
        ReplayEvent::PieceMoved {
            piece_id,
            from,
            to,
            ..
        } if piece_id == "player-unit" && *from == hex(0, 1) && *to == hex(0, 0)
    )));
}

#[test]
fn ranged_kill_does_not_advance_attacker() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "player-unit",
        Side::Player,
        hex(0, 2),
        3,
        2,
        4,
    ));
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 0),
        0,
        1,
        2,
    ));

    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-unit".to_string(),
        target_id: "opponent-unit".to_string(),
    })
    .expect("ranged kill should resolve");

    let attacker = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "player-unit")
        .expect("attacker should survive");
    assert_eq!(attacker.position, hex(0, 2));
}

#[test]
fn simultaneous_melee_death_does_not_advance_attacker() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.board.units.push(board_unit(
        "player-unit",
        Side::Player,
        hex(0, 1),
        2,
        1,
        2,
    ));
    game.board.units.push(board_unit(
        "opponent-unit",
        Side::Opponent,
        hex(0, 0),
        2,
        1,
        2,
    ));

    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-unit".to_string(),
        target_id: "opponent-unit".to_string(),
    })
    .expect("simultaneous death should resolve");

    assert!(!game
        .board
        .units
        .iter()
        .any(|unit| unit.id == "player-unit"));
    assert!(!game
        .board
        .units
        .iter()
        .any(|unit| unit.position == hex(0, 0)));
}

#[test]
fn hero_knockout_does_not_advance_attacker() {
    let mut game = MatchState::new_with_seed(7);
    enter_attack_phase(&mut game);
    game.opponent.hero.hp = 1;
    game.opponent.hero.position = hex(0, 2);

    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "player-hero".to_string(),
        target_id: "opponent-hero".to_string(),
    })
    .expect("hero knockout should resolve");

    assert_eq!(game.player.hero.position, hex(0, 3));
    assert_eq!(game.phase, Phase::MatchOver);
}
