use super::*;

#[test]
fn public_replay_redacts_hidden_opponent_draws() {
    let mut game = MatchState::new_with_seed(7);
    enter_card_play(&mut game);
    game.apply_action_recording(MatchActionRequest::EndTurn, 40)
        .expect("ending turn should start the opponent turn");
    advance_solo_ai_until_player_turn(&mut game);
    let initial_hand = game.opponent.hand.len();
    let initial_deck = game.opponent.deck_count;

    enter_card_play(&mut game);
    let frames = game
        .apply_action_recording(MatchActionRequest::EndTurn, 41)
        .expect("ending turn should start the opponent turn again");

    let draw_frame = frames
        .iter()
        .find(|frame| matches!(frame.event, ReplayEvent::CardDrawn { .. }))
        .expect("opponent turn start should draw a card");
    assert!(matches!(
        &draw_frame.event,
        ReplayEvent::CardDrawn {
            side: Side::Opponent,
            card: Some(_),
            hidden: true,
        }
    ));
    assert!(matches!(
        draw_frame.event.for_visibility(ReplayVisibility::Public),
        ReplayEvent::CardDrawn {
            side: Side::Opponent,
            card: None,
            hidden: true,
        }
    ));

    let replay_state = MatchState::from_snapshot_json(&draw_frame.snapshot_json)
        .expect("replay snapshot should deserialize");
    let public_value = replay_state.replay_value(ReplayVisibility::Public);
    assert!(public_value["opponent"].get("hand").is_none());
    assert_eq!(public_value["opponent"]["handCount"], initial_hand + 1);
    assert_eq!(public_value["opponent"]["deckCount"], initial_deck - 1);

    let revealed_value = replay_state.replay_value(ReplayVisibility::Revealed);
    assert!(revealed_value["opponent"].get("hand").is_some());
}
