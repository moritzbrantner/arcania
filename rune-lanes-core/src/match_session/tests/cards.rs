use super::*;

#[test]
fn starter_deck_has_the_expected_rarity_counts() {
    let game = MatchState::new_with_seed(7);
    let all_cards: Vec<_> = game
        .player
        .hand
        .iter()
        .chain(game.player.deck.iter())
        .collect();

    assert_eq!(all_cards.len(), 60);
    assert_eq!(
        all_cards
            .iter()
            .filter(|card| card.rarity == Rarity::Basic)
            .count(),
        50
    );
    assert_eq!(
        all_cards
            .iter()
            .filter(|card| card.rarity == Rarity::Advanced)
            .count(),
        9
    );
    assert_eq!(
        all_cards
            .iter()
            .filter(|card| card.rarity == Rarity::Rare)
            .count(),
        1
    );
    assert_eq!(starter_card_templates().len(), 79);
    assert_eq!(game.player.hand.len(), 7);
    assert_eq!(game.player.deck_count, 53);
}

#[test]
fn playing_a_unit_spends_mana_only_and_summons_adjacent() {
    let mut game = MatchState::new_with_seed(7);
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id,
                target: ActionTarget::Hex { coord: hex(0, 2) },
            },
            3,
        )
        .expect("unit should be playable next to hero");

    let unit = game.board.units.first().expect("unit should be on board");
    assert_eq!(game.player.mana, 2);
    assert_eq!(game.player.hero.ap_remaining, 3);
    assert_eq!(unit.position, hex(0, 2));
    assert_eq!(unit.template_id.as_deref(), Some("ember-squire"));
    assert_eq!(unit.ap_remaining, 1);
    assert_eq!(unit.max_ap, 2);
    assert_eq!(
        card_play_event_names(&frames),
        vec!["cardPlayed", "actionQueued", "unitSummoned"]
    );
}

#[test]
fn unit_summons_must_target_empty_adjacent_hexes() {
    let mut game = MatchState::new_with_seed(7);
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);

    let result = game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 1) },
    });

    assert_eq!(result, Err(MatchError::InvalidTarget));
}

#[test]
fn unit_summons_reject_occupied_adjacent_hexes() {
    let mut game = MatchState::new_with_seed(7);
    game.board.units.push(Unit {
        id: "blocking-unit".to_string(),
        side: Side::Player,
        name: "Blocking Unit".to_string(),
        template_id: Some("blocking-unit".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 1,
        max_armor: 1,
        position: hex(0, 2),
        ap_remaining: 1,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let card = player_unit_card(&game, "ember-squire");
    let card_id = put_card_in_hand(&mut game, card);
    let initial_mana = game.player.mana;
    let initial_hero_ap = game.player.hero.ap_remaining;

    let result = game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 2) },
    });

    assert_eq!(result, Err(MatchError::OccupiedHex));
    assert_eq!(game.player.mana, initial_mana);
    assert_eq!(game.player.hero.ap_remaining, initial_hero_ap);
    assert_eq!(game.board.units.len(), 1);
}

#[test]
fn spells_heal_buff_and_damage_with_caps() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
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
    game.board.units.push(Unit {
        id: "enemy".to_string(),
        side: Side::Opponent,
        name: "Swift Familiar".to_string(),
        template_id: Some("swift-familiar".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 4,
        max_armor: 4,
        position: hex(0, 0),
        ap_remaining: 3,
        max_ap: 3,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });

    let heal = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "mending-rune")
        .expect("heal exists");
    let heal_id = put_card_in_hand(&mut game, heal);
    let heal_frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: heal_id,
                target: ActionTarget::Piece {
                    piece_id: "ally".to_string(),
                },
            },
            10,
        )
        .expect("heal should work");
    assert_eq!(
        game.board
            .units
            .iter()
            .find(|unit| unit.id == "ally")
            .map(|unit| unit.armor),
        Some(4)
    );
    assert_eq!(
        card_play_event_names(&heal_frames),
        vec!["cardPlayed", "actionQueued", "pieceHealed"]
    );

    let buff = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "war-chant")
        .expect("buff exists");
    let buff_id = put_card_in_hand(&mut game, buff);
    let buff_frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: buff_id,
                target: ActionTarget::Piece {
                    piece_id: "ally".to_string(),
                },
            },
            11,
        )
        .expect("buff should work");
    let ally = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "ally")
        .expect("ally survives");
    assert_eq!(ally.attack, 2);
    assert_eq!(ally.armor, 5);
    assert_eq!(ally.max_armor, 5);
    assert_eq!(
        card_play_event_names(&buff_frames),
        vec!["cardPlayed", "actionQueued", "pieceBuffed"]
    );

    let bolt = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "starfire-bolt")
        .expect("bolt exists");
    let bolt_id = put_card_in_hand(&mut game, bolt);
    game.player.mana = 5;
    game.player.hero.ap_remaining = 1;
    let damage_frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: bolt_id,
                target: ActionTarget::Piece {
                    piece_id: "enemy".to_string(),
                },
            },
            12,
        )
        .expect("bolt should work");
    assert!(!game.board.units.iter().any(|unit| unit.id == "enemy"));
    assert_eq!(
        card_play_event_names(&damage_frames),
        vec!["cardPlayed", "actionQueued", "pieceDamaged"]
    );
}

#[test]
fn item_cards_equip_passives_activate_and_drop_when_carrier_dies() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.board.units.push(Unit {
        id: "ally".to_string(),
        side: Side::Player,
        name: "Stoneguard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 1,
        max_armor: 4,
        position: hex(0, 2),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let flask = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "ember-flask")
        .expect("item exists");
    let flask_id = put_card_in_hand(&mut game, flask);

    let equip_frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: flask_id,
                target: ActionTarget::Piece {
                    piece_id: "ally".to_string(),
                },
            },
            20,
        )
        .expect("item should equip to an allied unit");

    let item_id = {
        let ally = game
            .board
            .units
            .iter()
            .find(|unit| unit.id == "ally")
            .expect("ally should survive");
        assert_eq!(ally.attack, 2);
        assert_eq!(ally.items.len(), 1);
        ally.items[0].id.clone()
    };
    assert_eq!(
        card_play_event_names(&equip_frames),
        vec!["cardPlayed", "actionQueued", "itemEquipped"]
    );

    let activation_frames = game
        .apply_action_recording(
            MatchActionRequest::ActivateItem {
                carrier_id: "ally".to_string(),
                item_id: item_id.clone(),
                target: None,
            },
            21,
        )
        .expect("active item should be usable by its carrier");

    let ally = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "ally")
        .expect("ally should survive");
    assert_eq!(ally.armor, 3);
    assert_eq!(ally.ap_remaining, 1);
    assert!(ally.items[0].active_used_this_turn);
    assert_eq!(
        card_play_event_names(&activation_frames),
        vec!["actionQueued", "pieceHealed", "itemActivated"]
    );

    let result = game.apply_action(MatchActionRequest::ActivateItem {
        carrier_id: "ally".to_string(),
        item_id,
        target: None,
    });
    assert_eq!(result, Err(MatchError::ItemExhausted));

    let mut frames = Vec::new();
    game.damage_pieces(
        Side::Opponent,
        vec!["ally".to_string()],
        10,
        &mut frames,
        None,
    );

    assert!(!game.board.units.iter().any(|unit| unit.id == "ally"));
    assert_eq!(game.board.dropped_items.len(), 1);
    assert_eq!(game.board.dropped_items[0].item.name, "Ember Flask");
    assert!(
        frames
            .iter()
            .any(|frame| matches!(frame.event, ReplayEvent::ItemDropped { .. }))
    );
}

#[test]
fn item_cards_only_target_allied_units_in_range() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.board.units.push(Unit {
        id: "ally-out-of-range".to_string(),
        side: Side::Player,
        name: "Far Guard".to_string(),
        template_id: Some("stoneguard".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 1,
        max_armor: 4,
        position: hex(0, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "enemy-in-range".to_string(),
        side: Side::Opponent,
        name: "Ash Hound".to_string(),
        template_id: Some("ash-hound".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 3,
        max_armor: 3,
        position: hex(0, 2),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    let flask = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "ember-flask")
        .expect("item exists");
    let flask_id = put_card_in_hand(&mut game, flask);
    let initial_mana = game.player.mana;
    let initial_hero_ap = game.player.hero.ap_remaining;

    let enemy_result = game.apply_action(MatchActionRequest::PlayCard {
        card_id: flask_id.clone(),
        target: ActionTarget::Piece {
            piece_id: "enemy-in-range".to_string(),
        },
    });
    let range_result = game.apply_action(MatchActionRequest::PlayCard {
        card_id: flask_id,
        target: ActionTarget::Piece {
            piece_id: "ally-out-of-range".to_string(),
        },
    });

    assert_eq!(enemy_result, Err(MatchError::InvalidTarget));
    assert_eq!(range_result, Err(MatchError::InvalidTarget));
    assert_eq!(game.player.mana, initial_mana);
    assert_eq!(game.player.hero.ap_remaining, initial_hero_ap);
    assert!(game.board.units.iter().all(|unit| unit.items.is_empty()));
}

#[test]
fn units_pick_up_dropped_items_by_moving_onto_their_hex() {
    let mut game = MatchState::new_with_seed(7);
    game.board.units.push(Unit {
        id: "attacker".to_string(),
        side: Side::Player,
        name: "Rune Bruiser".to_string(),
        template_id: Some("rune-bruiser".to_string()),
        attack: 3,
        attack_range: 2,
        armor: 3,
        max_armor: 3,
        position: hex(2, 0),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "looter".to_string(),
        side: Side::Player,
        name: "Swift Familiar".to_string(),
        template_id: Some("swift-familiar".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 1,
        max_armor: 1,
        position: hex(-1, 2),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    });
    game.board.units.push(Unit {
        id: "carrier".to_string(),
        side: Side::Opponent,
        name: "Ash Hound".to_string(),
        template_id: Some("ash-hound".to_string()),
        attack: 1,
        attack_range: 1,
        armor: 3,
        max_armor: 3,
        position: hex(0, 2),
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: vec![CarriedItem {
            id: "OI9".to_string(),
            template_id: "ember-flask".to_string(),
            name: "Ember Flask".to_string(),
            passive: ItemPassiveEffect::StatBonus {
                attack: 1,
                armor: 0,
                max_ap: 0,
            },
            active: Some(ItemActiveEffect::HealCarrier {
                amount: 2,
                priority: 0,
            }),
            active_used_this_turn: false,
        }],
        stat_markers: Vec::new(),
    });

    enter_attack_phase(&mut game);
    game.apply_action(MatchActionRequest::Attack {
        attacker_id: "attacker".to_string(),
        target_id: "carrier".to_string(),
    })
    .expect("adjacent attack should destroy the carrier");
    assert!(!game.board.units.iter().any(|unit| unit.id == "carrier"));
    assert_eq!(game.board.dropped_items.len(), 1);

    game.phase = Phase::Movement;
    game.apply_action(MatchActionRequest::MovePiece {
        piece_id: "looter".to_string(),
        to: hex(0, 2),
    })
    .expect("unit should move onto the dropped item");

    let looter = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "looter")
        .expect("looter survives");
    assert_eq!(looter.position, hex(0, 2));
    assert_eq!(looter.attack, 2);
    assert_eq!(looter.items.len(), 1);
    assert_eq!(looter.items[0].name, "Ember Flask");
    assert!(game.board.dropped_items.is_empty());

    game.apply_action(MatchActionRequest::ActivateItem {
        carrier_id: "looter".to_string(),
        item_id: "OI9".to_string(),
        target: None,
    })
    .expect("picked up item should be usable by its new carrier");
}

#[test]
fn draw_spells_target_the_caster_and_draw_cards() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    let initial_hand = game.player.hand.len();
    let initial_deck = game.player.deck_count;
    let insight = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "runic-insight")
        .expect("draw spell exists");
    let insight_id = put_card_in_hand(&mut game, insight);

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id: insight_id,
                target: ActionTarget::Piece {
                    piece_id: game.player.hero.id.clone(),
                },
            },
            30,
        )
        .expect("draw spell should target the caster");

    assert_eq!(game.player.hand.len(), initial_hand + 1);
    assert_eq!(game.player.deck_count, initial_deck - 1);
    assert_eq!(game.player.discard_count, 1);
    assert_eq!(
        card_play_event_names(&frames),
        vec!["cardPlayed", "actionQueued", "cardDrawn"]
    );
    assert!(frames.iter().any(|frame| matches!(
        &frame.event,
        ReplayEvent::CardDrawn {
            side: Side::Player,
            card: Some(_),
            hidden: false,
        }
    )));
}

#[test]
fn mana_well_builds_a_permanent_mana_source() {
    let mut game = MatchState::new_with_seed(7);
    let card = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "mana-well")
        .expect("mana well exists");
    let card_id = put_card_in_hand(&mut game, card);

    let frames = game
        .apply_action_recording(
            MatchActionRequest::PlayCard {
                card_id,
                target: ActionTarget::Hex { coord: hex(0, 2) },
            },
            9,
        )
        .expect("mana well should be buildable next to hero");

    assert!(
        game.board
            .buildings
            .iter()
            .any(|building| building.position == hex(0, 2) && building.template_id == "mana-well")
    );
    assert_eq!(game.player.mana, 1);
    assert_eq!(game.player.hero.ap_remaining, 3);
    assert_eq!(
        card_play_event_names(&frames),
        vec![
            "cardPlayed",
            "actionQueued",
            "buildingBuilt",
            "manaSourceBuilt"
        ]
    );
}

#[test]
fn mana_well_requires_adjacent_empty_non_source_hex() {
    let mut game = MatchState::new_with_seed(7);
    let card = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "mana-well")
        .expect("mana well exists");
    let card_id = put_card_in_hand(&mut game, card);

    let occupied = game.apply_action(MatchActionRequest::PlayCard {
        card_id: card_id.clone(),
        target: ActionTarget::Hex { coord: hex(0, 3) },
    });
    assert_eq!(occupied, Err(MatchError::OccupiedHex));

    let non_adjacent = game.apply_action(MatchActionRequest::PlayCard {
        card_id: card_id.clone(),
        target: ActionTarget::Hex { coord: hex(0, 1) },
    });
    assert_eq!(non_adjacent, Err(MatchError::InvalidTarget));

    game.board.mana_sources.push(hex(0, 2));
    let duplicate_source = game.apply_action(MatchActionRequest::PlayCard {
        card_id,
        target: ActionTarget::Hex { coord: hex(0, 2) },
    });
    assert_eq!(duplicate_source, Err(MatchError::InvalidTarget));
    assert_eq!(game.player.mana, 3);
    assert_eq!(game.player.hero.ap_remaining, 3);
}

#[test]
fn hero_can_carry_eligible_items_and_unit_only_items_reject_heroes() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;

    let lens = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "runekeeper-lens")
        .expect("hero item exists");
    let lens_id = put_card_in_hand(&mut game, lens);

    game.apply_action(MatchActionRequest::PlayCard {
        card_id: lens_id,
        target: ActionTarget::Piece {
            piece_id: "player-hero".to_string(),
        },
    })
    .expect("hero-eligible item should equip to hero");

    assert_eq!(game.player.hero.items.len(), 1);
    assert_eq!(game.player.hero.items[0].template_id, "runekeeper-lens");

    let buckle = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "warded-buckle")
        .expect("unit-only item exists");
    let buckle_id = put_card_in_hand(&mut game, buckle);
    let result = game.apply_action(MatchActionRequest::PlayCard {
        card_id: buckle_id,
        target: ActionTarget::Piece {
            piece_id: "player-hero".to_string(),
        },
    });

    assert_eq!(result, Err(MatchError::InvalidTarget));
}

#[test]
fn carriers_have_a_three_item_limit() {
    let mut game = MatchState::new_with_seed(7);
    game.player.mana = 8;
    game.player.hero.ap_remaining = 3;
    game.player.hero.items = vec![
        carried_item("one"),
        carried_item("two"),
        carried_item("three"),
    ];

    let lens = starter_card_templates()
        .into_iter()
        .find(|card| card.template_id == "runekeeper-lens")
        .expect("hero item exists");
    let lens_id = put_card_in_hand(&mut game, lens);

    let result = game.apply_action(MatchActionRequest::PlayCard {
        card_id: lens_id,
        target: ActionTarget::Piece {
            piece_id: "player-hero".to_string(),
        },
    });

    assert_eq!(result, Err(MatchError::InvalidTarget));
    assert_eq!(game.player.hero.items.len(), 3);
}

#[test]
fn priority_item_activation_can_target_an_enemy_off_turn() {
    let mut game = MatchState::new_with_seed(7);
    game.mode = MatchMode::Shared;
    game.priority_side = Some(Side::Player);
    game.action_stack.push(StackItem {
        id: "stack-existing".to_string(),
        side: Side::Opponent,
        priority: 1,
        action: StackAction::Attack {
            attacker_id: "opponent-hero".to_string(),
            target_id: "player-hero".to_string(),
        },
    });
    game.player.hero.items.push(CarriedItem {
        id: "spark-item".to_string(),
        template_id: "spark-needle".to_string(),
        name: "Spark Needle".to_string(),
        passive: ItemPassiveEffect::StatBonus {
            attack: 0,
            armor: 0,
            max_ap: 0,
        },
        active: Some(ItemActiveEffect::DamageTarget {
            amount: 1,
            range: 2,
            priority: 3,
        }),
        active_used_this_turn: false,
    });
    game.opponent.hero.position = hex(0, 2);

    game.apply_action(MatchActionRequest::ActivateItem {
        carrier_id: "player-hero".to_string(),
        item_id: "spark-item".to_string(),
        target: Some(ActionTarget::Piece {
            piece_id: "opponent-hero".to_string(),
        }),
    })
    .expect("priority item should queue");

    assert_eq!(game.player.hero.ap_remaining, 2);
    assert_eq!(game.action_stack.last().map(|item| item.priority), Some(3));
    assert!(game.player.hero.items[0].active_used_this_turn);
}

#[test]
fn stat_marker_item_activation_persists_visible_marker() {
    let mut game = MatchState::new_with_seed(7);
    game.player.hero.items.push(CarriedItem {
        id: "standard".to_string(),
        template_id: "battle-standard".to_string(),
        name: "Battle Standard".to_string(),
        passive: ItemPassiveEffect::StatBonus {
            attack: 0,
            armor: 0,
            max_ap: 0,
        },
        active: Some(ItemActiveEffect::StatMarker {
            attack: 1,
            armor: 0,
            max_ap: 0,
            priority: 1,
        }),
        active_used_this_turn: false,
    });

    game.apply_action(MatchActionRequest::ActivateItem {
        carrier_id: "player-hero".to_string(),
        item_id: "standard".to_string(),
        target: None,
    })
    .expect("stat marker item should activate");

    assert_eq!(game.player.hero.attack, 2);
    assert_eq!(game.player.hero.stat_markers.len(), 1);
    assert_eq!(game.player.hero.stat_markers[0].source_item_id, "standard");
}

#[test]
fn full_carrier_leaves_dropped_items_on_the_hex() {
    let mut game = MatchState::new_with_seed(7);
    game.board
        .units
        .push(board_unit("looter", Side::Player, hex(-1, 2), 1, 1, 2));
    let looter = game
        .board
        .units
        .iter_mut()
        .find(|unit| unit.id == "looter")
        .expect("looter exists");
    looter.items = vec![
        carried_item("one"),
        carried_item("two"),
        carried_item("three"),
    ];
    game.board.dropped_items.push(DroppedItem {
        id: "drop-four".to_string(),
        position: hex(0, 2),
        item: carried_item("four"),
    });

    game.apply_action(MatchActionRequest::MovePiece {
        piece_id: "looter".to_string(),
        to: hex(0, 2),
    })
    .expect("movement should not be blocked by full inventory");

    let looter = game
        .board
        .units
        .iter()
        .find(|unit| unit.id == "looter")
        .expect("looter survives");
    assert_eq!(looter.items.len(), 3);
    assert_eq!(game.board.dropped_items.len(), 1);
}

#[test]
fn knocked_out_heroes_drop_carried_items() {
    let mut game = MatchState::new_with_seed(7);
    game.player.hero.items.push(carried_item("hero-relic"));
    let mut frames = Vec::new();

    game.damage_pieces(
        Side::Opponent,
        vec!["player-hero".to_string()],
        99,
        &mut frames,
        None,
    );
    game.check_winner(&mut frames, None);

    assert!(game.player.knocked_out);
    assert_eq!(game.board.dropped_items.len(), 1);
    assert_eq!(game.board.dropped_items[0].item.id, "hero-relic");
    assert!(frames.iter().any(|frame| {
        matches!(
            &frame.event,
            ReplayEvent::ItemDropped { carrier_id, .. } if carrier_id == "player-hero"
        )
    }));
}

#[test]
fn legacy_unit_id_item_activation_payload_deserializes() {
    let action: MatchActionRequest = serde_json::from_value(serde_json::json!({
        "type": "activateItem",
        "unitId": "legacy-unit",
        "itemId": "legacy-item"
    }))
    .expect("legacy payload should deserialize");

    match action {
        MatchActionRequest::ActivateItem {
            carrier_id,
            item_id,
            target,
        } => {
            assert_eq!(carrier_id, "legacy-unit");
            assert_eq!(item_id, "legacy-item");
            assert!(target.is_none());
        }
        _ => panic!("expected activate item"),
    }
}

fn carried_item(id: &str) -> CarriedItem {
    CarriedItem {
        id: id.to_string(),
        template_id: "rune-charm".to_string(),
        name: id.to_string(),
        passive: ItemPassiveEffect::StatBonus {
            attack: 0,
            armor: 0,
            max_ap: 0,
        },
        active: None,
        active_used_this_turn: false,
    }
}
