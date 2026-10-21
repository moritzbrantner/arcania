use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::*;
use crate::app_state::AppState;
use crate::match_store::{SharedMatchStatus, SqliteMatchStore};

fn lifecycle(name: &str) -> SharedMatchLifecycle {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!(
        "rune-lanes-shared-lifecycle-{name}-{suffix}.sqlite3"
    ));
    let store = SqliteMatchStore::new(path).expect("store should open");
    SharedMatchLifecycle::new(Arc::new(AppState::new(store)))
}

fn join<'a>(match_id: &'a str, seat_token: &'a str, hero_type: HeroType) -> SharedSeatJoin<'a> {
    SharedSeatJoin {
        match_id,
        seat_token,
        user_id: None,
        hero_type,
        deck_choice: None,
        legacy_deck_id: None,
        requested_rune_ids: None,
    }
}

/// A Duel with both seats joined, so the match is active.
fn active_duel(lifecycle: &SharedMatchLifecycle) -> CreatedSharedMatch {
    let created = lifecycle
        .create(None, SharedMatchFormat::Duel)
        .expect("shared match should be created");
    lifecycle
        .join(join(
            &created.match_id,
            &created.player_token,
            HeroType::Pyromancer,
        ))
        .expect("player seat should join");
    lifecycle
        .join(join(
            &created.match_id,
            &created.opponent_token,
            HeroType::Warden,
        ))
        .expect("opponent seat should join");
    created
}

fn accepted(outcome: &SharedClientOutcome) -> bool {
    matches!(
        outcome.reply,
        Some(SharedServerMessage::ActionAccepted { .. })
    )
}

fn rejected_message(outcome: &SharedClientOutcome) -> Option<&str> {
    match &outcome.reply {
        Some(SharedServerMessage::ActionRejected { message, .. }) => Some(message),
        _ => None,
    }
}

#[test]
fn setup_creates_private_seat_links_for_each_side() {
    let lifecycle = lifecycle("setup");
    let created = lifecycle
        .create(None, SharedMatchFormat::Duel)
        .expect("shared match should be created");
    assert_ne!(created.player_token, created.opponent_token);

    let player = lifecycle
        .open_seat(&created.match_id, &created.player_token)
        .expect("player seat should open");
    let opponent = lifecycle
        .open_seat(&created.match_id, &created.opponent_token)
        .expect("opponent seat should open");
    assert_eq!(player.status, SharedMatchStatus::Setup);
    // Each seat link is a capability for exactly one side.
    assert_ne!(player.viewer_seat.side, opponent.viewer_seat.side);
    assert!(matches!(
        lifecycle.open_seat(&created.match_id, "not-a-seat"),
        Err(SharedLifecycleError::NotFound)
    ));
    assert!(
        !lifecycle
            .seat_exists(&created.match_id, "not-a-seat")
            .unwrap()
    );
}

#[test]
fn joining_every_seat_activates_the_match_and_notifies_the_other_seats() {
    let lifecycle = lifecycle("join");
    let created = lifecycle
        .create(None, SharedMatchFormat::Duel)
        .expect("shared match should be created");
    let mut notifications = lifecycle.state.match_sender(&created.match_id).subscribe();

    let first = lifecycle
        .join(join(
            &created.match_id,
            &created.player_token,
            HeroType::Pyromancer,
        ))
        .expect("player seat should join");
    assert_eq!(first.status, SharedMatchStatus::Setup);
    assert!(notifications.try_recv().is_ok());

    let second = lifecycle
        .join(join(
            &created.match_id,
            &created.opponent_token,
            HeroType::Warden,
        ))
        .expect("opponent seat should join");
    assert_eq!(second.status, SharedMatchStatus::Active);
    assert!(second.state.is_some());
    assert!(notifications.try_recv().is_ok());
}

#[test]
fn presence_snapshot_heartbeat_and_disconnect() {
    let lifecycle = lifecycle("presence");
    let created = active_duel(&lifecycle);
    let mut notifications = lifecycle.state.match_sender(&created.match_id).subscribe();

    let connection = lifecycle
        .connect(&created.match_id, &created.player_token)
        .expect("seat should connect");
    assert!(connection.is_current());
    assert!(matches!(
        connection.snapshot(),
        Some(SharedServerMessage::Snapshot { .. })
    ));
    connection.announce();
    assert!(notifications.try_recv().is_ok());

    let heartbeat = connection.handle_text(r#"{"type":"heartbeat"}"#, unix_timestamp());
    assert!(heartbeat.keep_open && heartbeat.reply.is_none() && !heartbeat.notify_others);
    assert!(!connection.should_close(Duration::from_secs(10)));
    assert!(connection.should_close(SHARED_SEAT_SILENCE_LIMIT + Duration::from_secs(1)));

    let invalid = connection.handle_text("not json", unix_timestamp());
    assert!(invalid.keep_open);
    assert!(matches!(
        invalid.reply,
        Some(SharedServerMessage::Error { .. })
    ));

    connection.finish();
    assert!(notifications.try_recv().is_ok());
    let opponent_view = lifecycle
        .open_seat(&created.match_id, &created.opponent_token)
        .expect("opponent seat should open");
    let player_seat = opponent_view
        .seats
        .iter()
        .find(|seat| seat.seat_token == created.player_token)
        .expect("player seat should be listed");
    assert!(player_seat.disconnected_at.is_some());
}

#[test]
fn a_newer_connection_replaces_the_previous_one() {
    let lifecycle = lifecycle("replace");
    let created = active_duel(&lifecycle);
    let first = lifecycle
        .connect(&created.match_id, &created.player_token)
        .expect("seat should connect");
    let second = lifecycle
        .connect(&created.match_id, &created.player_token)
        .expect("seat should reconnect");
    assert!(!first.is_current());
    assert!(second.is_current());

    let stale = first.handle_text(r#"{"type":"heartbeat"}"#, unix_timestamp());
    assert!(!stale.keep_open);
    // Finishing a replaced connection does not mark the seat disconnected.
    first.finish();
    let view = lifecycle
        .open_seat(&created.match_id, &created.opponent_token)
        .expect("opponent seat should open");
    assert!(view.seats.iter().all(|seat| seat.disconnected_at.is_none()));
}

#[test]
fn actions_are_accepted_and_notify_or_rejected_without_notifying() {
    let lifecycle = lifecycle("actions");
    let created = active_duel(&lifecycle);
    let player = lifecycle
        .connect(&created.match_id, &created.player_token)
        .expect("player seat should connect");
    let opponent = lifecycle
        .connect(&created.match_id, &created.opponent_token)
        .expect("opponent seat should connect");
    let start_attack = r#"{"type":"action","requestId":"r1","action":{"type":"startAttackPhase"}}"#;

    // The opponent may not act on the player's turn.
    let rejected = opponent.handle_text(start_attack, unix_timestamp());
    assert!(rejected_message(&rejected).is_some());
    assert!(!rejected.notify_others && rejected.keep_open);

    let applied = player.handle_text(start_attack, unix_timestamp());
    assert!(accepted(&applied), "the active seat may act");
    assert!(applied.notify_others);
}

#[test]
fn forfeit_is_blocked_until_the_opponent_has_been_disconnected_long_enough() {
    let lifecycle = lifecycle("forfeit");
    let created = active_duel(&lifecycle);
    let player = lifecycle
        .connect(&created.match_id, &created.player_token)
        .expect("player seat should connect");
    let opponent = lifecycle
        .connect(&created.match_id, &created.opponent_token)
        .expect("opponent seat should connect");
    let claim = r#"{"type":"claimForfeit","requestId":"f1"}"#;

    let still_connected = player.handle_text(claim, unix_timestamp());
    assert!(rejected_message(&still_connected).is_some());

    opponent.finish();
    let too_early = player.handle_text(claim, unix_timestamp());
    assert!(rejected_message(&too_early).is_some());
    assert!(!too_early.notify_others);

    let after_timeout = player.handle_text(claim, unix_timestamp() + 600);
    assert!(accepted(&after_timeout));
    assert!(after_timeout.notify_others);
    let view = lifecycle
        .open_seat(&created.match_id, &created.player_token)
        .expect("player seat should open");
    assert_eq!(view.status, SharedMatchStatus::Forfeited);
}
