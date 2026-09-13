use super::*;

#[test]
fn radius_three_board_has_thirty_seven_tiles() {
    let game = MatchState::new_with_seed(7);

    assert_eq!(game.board.radius, 3);
    assert_eq!(game.board.tiles.len(), 37);
    assert_eq!(
        game.board
            .buildings
            .iter()
            .map(|building| building.position)
            .collect::<Vec<_>>(),
        vec![hex(0, 0)]
    );
    assert!(game.board.is_valid(hex(0, 0)));
    assert!(!game.board.is_valid(hex(4, 0)));
}

#[test]
fn old_snapshots_drop_outer_natural_mana_wells_but_keep_built_wells() {
    let game = MatchState::new_with_seed(7);
    let mut snapshot = serde_json::from_str::<serde_json::Value>(
        &game.to_snapshot_json().expect("snapshot should serialize"),
    )
    .expect("snapshot should parse");
    snapshot["board"]["buildings"] = json!([
        {
            "id": "natural-mana-1",
            "templateId": "mana-well",
            "name": "Mana Well",
            "position": { "q": -2, "r": 0 },
            "effect": { "type": "turnStartMana", "amount": 1 },
            "activatedThisTurn": false
        },
        {
            "id": "natural-mana-2",
            "templateId": "mana-well",
            "name": "Mana Well",
            "position": { "q": 0, "r": 0 },
            "effect": { "type": "turnStartMana", "amount": 1 },
            "activatedThisTurn": false
        },
        {
            "id": "natural-mana-3",
            "templateId": "mana-well",
            "name": "Mana Well",
            "position": { "q": 2, "r": 0 },
            "effect": { "type": "turnStartMana", "amount": 1 },
            "activatedThisTurn": false
        },
        {
            "id": "player-built-mana",
            "templateId": "mana-well",
            "name": "Mana Well",
            "position": { "q": 2, "r": 0 },
            "effect": { "type": "turnStartMana", "amount": 1 },
            "activatedThisTurn": false
        }
    ]);

    let restored = MatchState::from_snapshot_json(&snapshot.to_string())
        .expect("old snapshot should deserialize");

    assert_eq!(
        restored
            .board
            .buildings
            .iter()
            .map(|building| (building.id.clone(), building.position))
            .collect::<Vec<_>>(),
        vec![
            ("natural-mana-2".to_string(), hex(0, 0)),
            ("player-built-mana".to_string(), hex(2, 0)),
        ]
    );
}

#[test]
fn match_action_payloads_accept_camel_case_api_fields() {
    let action: MatchActionRequest = serde_json::from_str(
        r#"{
                "type": "playCard",
                "cardId": "p-0-ember-squire",
                "target": {
                    "type": "hex",
                    "coord": { "q": 0, "r": 2 }
                }
            }"#,
    )
    .expect("frontend play-card payload should deserialize");

    match action {
        MatchActionRequest::PlayCard { card_id, target } => {
            assert_eq!(card_id, "p-0-ember-squire");
            assert!(matches!(target, ActionTarget::Hex { coord } if coord == hex(0, 2)));
        }
        _ => panic!("expected play-card action"),
    }

    let action: MatchActionRequest = serde_json::from_str(
        r#"{
                "type": "attack",
                "attackerId": "player-hero",
                "targetId": "opponent-hero"
            }"#,
    )
    .expect("frontend attack payload should deserialize");

    assert!(matches!(
        action,
        MatchActionRequest::Attack {
            attacker_id,
            target_id
        } if attacker_id == "player-hero" && target_id == "opponent-hero"
    ));

    let action: MatchActionRequest = serde_json::from_str(r#"{"type":"startAttackPhase"}"#)
        .expect("frontend start-attack payload should deserialize");
    assert!(matches!(action, MatchActionRequest::StartAttackPhase));

    let action: MatchActionRequest = serde_json::from_str(r#"{"type":"startCardPlay"}"#)
        .expect("frontend start-card-play payload should deserialize");
    assert!(matches!(action, MatchActionRequest::StartCardPlay));
}

#[test]
fn card_payloads_emit_camel_case_kind_fields() {
    let game = MatchState::new_with_seed(7);
    let card = game
        .player
        .hand
        .iter()
        .chain(game.player.deck.iter())
        .find(|card| matches!(card.kind, CardKind::Unit { .. }))
        .expect("starter deck should contain a unit");

    let value = serde_json::to_value(card).expect("card should serialize");

    assert!(value["kind"].get("maxAp").is_some());
    assert!(value["kind"].get("max_ap").is_none());

    let spell = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "arcane-parry")
        .expect("starter deck should contain a spell");
    let value = serde_json::to_value(spell).expect("spell should serialize");
    assert_eq!(value["kind"]["priority"], 4);
}

#[test]
fn terminal_match_phase_serializes_as_match_over() {
    let mut game = MatchState::new_with_seed(7);
    game.phase = Phase::MatchOver;

    let value = serde_json::to_value(&game).expect("match should serialize");

    assert_eq!(value["phase"], "matchOver");
    assert_ne!(value["phase"], "gameOver");
}

#[test]
fn new_match_phase_serializes_as_movement() {
    let game = MatchState::new_with_seed(7);

    let value = serde_json::to_value(&game).expect("match should serialize");

    assert_eq!(value["phase"], "movement");
}

#[test]
fn legacy_planning_snapshots_restore_as_movement_phase() {
    let mut snapshot = serde_json::from_str::<serde_json::Value>(
        &MatchState::new_with_seed(7)
            .to_snapshot_json()
            .expect("snapshot should serialize"),
    )
    .expect("snapshot should parse");
    snapshot["phase"] = serde_json::Value::String("planning".to_string());

    let restored = MatchState::from_snapshot_json(&snapshot.to_string())
        .expect("legacy snapshot should deserialize");

    assert_eq!(restored.phase, Phase::Movement);
}

#[test]
fn public_match_state_hides_opponent_hand_and_private_piles() {
    let game = MatchState::new_with_seed(7);

    let value = serde_json::to_value(&game).expect("match should serialize");

    assert!(value["player"].get("hand").is_some());
    assert!(value["player"].get("deck").is_none());
    assert!(value["player"].get("discard").is_none());
    assert!(value["opponent"].get("hand").is_none());
    assert!(value["opponent"].get("deck").is_none());
    assert!(value["opponent"].get("discard").is_none());
    assert_eq!(value["player"]["handCount"], 7);
    assert_eq!(value["opponent"]["handCount"], 7);
    assert_eq!(value["opponent"]["deckCount"], 53);
}

#[test]
fn public_match_state_serializes_player_discard_count_after_unit_play() {
    let mut game = MatchState::new_with_seed(7);
    let value = serde_json::to_value(&game).expect("match should serialize");
    assert_eq!(value["player"]["discardCount"], 0);

    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);

    game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 2) },
    })
    .expect("unit should be playable next to hero");

    let value = serde_json::to_value(&game).expect("match should serialize");
    assert_eq!(value["player"]["discardCount"], 1);
}

#[test]
fn public_match_state_serializes_player_discard_count_after_spell_play() {
    let mut game = MatchState::new_with_seed(7);
    game.board.units.push(Unit {
        id: "ally".to_string(),
        side: Side::Player,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 2,
        max_armor: 4,
        position: hex(0, 2),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let card = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "mending-rune")
        .expect("heal exists");
    let card_id = put_card_in_hand(&mut game, card);

    game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Piece {
            piece_id: "ally".to_string(),
        },
    })
    .expect("spell should be playable on damaged ally");

    let value = serde_json::to_value(&game).expect("match should serialize");
    assert_eq!(value["player"]["discardCount"], 1);
}

#[test]
fn match_snapshot_restores_hidden_piles_rng_unit_ids_and_first_turn_flags() {
    let mut game = MatchState::new_with_seed(7);
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);

    game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 2) },
    })
    .expect("unit should be playable next to hero");

    let snapshot = game.to_snapshot_json().expect("snapshot should serialize");
    let mut restored =
        MatchState::from_snapshot_json(&snapshot).expect("snapshot should deserialize");

    assert_eq!(restored.player.deck.len(), game.player.deck.len());
    assert_eq!(restored.player.discard.len(), game.player.discard.len());
    assert_eq!(restored.player.rng_seed, game.player.rng_seed);
    assert_eq!(
        restored.player.has_started_first_turn,
        game.player.has_started_first_turn
    );
    assert_eq!(restored.next_unit_id, game.next_unit_id);

    game.apply_action(MatchActionRequest::EndTurn)
        .expect("end turn should apply");
    restored
        .apply_action(MatchActionRequest::EndTurn)
        .expect("end turn should apply after restore");

    assert_eq!(
        serde_json::to_value(&restored).expect("restored match should serialize"),
        serde_json::to_value(&game).expect("match should serialize")
    );
}

#[test]
fn heroes_start_on_opposite_centered_edges() {
    let game = MatchState::new_with_seed(7);

    assert_eq!(game.player.hero.position, hex(0, 3));
    assert_eq!(game.opponent.hero.position, hex(0, -3));
    assert_eq!(game.player.hero.hp, 20);
    assert_eq!(game.player.hero.attack, 1);
    assert_eq!(game.player.hero.ap_remaining, 3);
}

#[test]
fn selected_hero_type_sets_player_starting_stats() {
    let game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Pyromancer);

    assert_eq!(game.player.hero.hero_type, HeroType::Pyromancer);
    assert_eq!(game.player.hero.hp, 18);
    assert_eq!(game.player.hero.attack, 2);
    assert_eq!(game.player.hero.ap_remaining, 3);
    assert_eq!(game.opponent.hero.hero_type, HeroType::Runekeeper);
}

#[test]
fn new_hero_types_set_player_starting_stats() {
    let cases = [
        (HeroType::Barbarian, 22, 3, 2),
        (HeroType::Archer, 16, 2, 4),
        (HeroType::Builder, 24, 1, 2),
    ];

    for (hero_type, hp, attack, ap) in cases {
        let game = MatchState::new_with_seed_and_player_hero_type(7, hero_type);

        assert_eq!(game.player.hero.hero_type, hero_type);
        assert_eq!(game.player.hero.hp, hp);
        assert_eq!(game.player.hero.attack, attack);
        assert_eq!(game.player.hero.ap_remaining, ap);
    }
}

#[test]
fn attack_range_serializes_for_heroes_and_units() {
    let mut game = MatchState::new_with_seed_and_player_hero_type(7, HeroType::Archer);
    game.board
        .units
        .push(board_unit("player-unit", Side::Player, hex(0, 2), 1, 1, 2));

    let value = serde_json::to_value(&game).expect("match should serialize");

    assert_eq!(value["player"]["hero"]["attackRange"], 2);
    assert_eq!(value["opponent"]["hero"]["attackRange"], 1);
    assert_eq!(value["board"]["units"][0]["attackRange"], 1);
}

#[test]
fn attack_range_defaults_when_old_snapshots_do_not_include_it() {
    let hero: Hero = serde_json::from_value(json!({
        "id": "player-hero",
        "side": "player",
        "heroType": "runekeeper",
        "hp": 20,
        "maxHp": 20,
        "attack": 1,
        "position": { "q": 0, "r": 3 },
        "apRemaining": 3,
        "maxAp": 3,
        "hasAttacked": false
    }))
    .expect("old hero snapshot should deserialize");
    let unit: Unit = serde_json::from_value(json!({
        "id": "unit",
        "side": "player",
        "name": "Unit",
        "templateId": "unit",
        "attack": 1,
        "armor": 2,
        "maxArmor": 2,
        "position": { "q": 0, "r": 2 },
        "apRemaining": 2,
        "maxAp": 2,
        "hasAttacked": false,
        "items": []
    }))
    .expect("old unit snapshot should deserialize");

    assert_eq!(hero.attack_range, 1);
    assert_eq!(unit.attack_range, 1);
}
