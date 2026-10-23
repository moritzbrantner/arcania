use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::deck_library;
use crate::match_session::{HeroType, MatchProgressionLoadout};
use crate::match_store::{CreatedSharedMatch, SharedMatchFormat};

struct TestStore {
    path: PathBuf,
    store: SqliteMatchStore,
}

impl Drop for TestStore {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn test_store(name: &str) -> TestStore {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path =
        std::env::temp_dir().join(format!("rune-lanes-replay-reads-{name}-{suffix}.sqlite3"));
    let mut store = SqliteMatchStore::new(&path).expect("store should open");
    for user_id in [1, 2, 3, 7, 8] {
        store
            .connection_mut()
            .execute(
                "
                INSERT INTO users (id, email, email_normalized, password_hash, display_name)
                VALUES (?1, ?2, ?2, 'hash', 'Test User')
                ",
                rusqlite::params![user_id, format!("user{user_id}@example.test")],
            )
            .expect("user should insert");
    }
    TestStore { path, store }
}

fn complete(store: &mut SqliteMatchStore, match_id: &str, winner: Side) {
    let mut stored = store
        .load_match(match_id)
        .expect("match lookup should succeed")
        .expect("match should exist");
    let frames = rune_lanes_core::test_support::forfeit_match(&mut stored.state, winner, 0);
    store
        .save_custom_action_and_replay_frames(
            match_id,
            0,
            r#"{"type":"testForfeit"}"#,
            &stored.state,
            &frames,
        )
        .expect("completed match should save");
}

/// A Duel joined by account 1 (player seat) and account 2 (opponent seat).
fn active_duel(store: &mut SqliteMatchStore) -> CreatedSharedMatch {
    let created = store
        .create_shared_match(None, SharedMatchFormat::Duel)
        .expect("shared match should be created");
    for (token, user_id) in [(&created.player_token, 1), (&created.opponent_token, 2)] {
        store
            .join_shared_match(
                &created.match_id,
                token,
                HeroType::Chronomancer,
                deck_library::starter_deck_snapshot(),
                MatchProgressionLoadout::default(),
                Some(user_id),
            )
            .expect("seat should join");
    }
    created
}

fn completed_duel(store: &mut SqliteMatchStore) -> CreatedSharedMatch {
    let created = active_duel(store);
    complete(store, &created.match_id, Side::Player);
    store
        .mark_shared_match_forfeited(&created.match_id, Side::Player)
        .expect("shared match should be marked finished");
    created
}

fn seat(match_id: &str, seat_token: &str) -> Actor {
    Actor::Seat {
        match_id: match_id.to_string(),
        seat_token: seat_token.to_string(),
    }
}

fn account(user_id: i64) -> Actor {
    Actor::Account { user_id }
}

fn is_not_found<T>(result: Result<T, ReplayReadError>) -> bool {
    matches!(result, Err(ReplayReadError::NotFound))
}

#[test]
fn missing_matches_are_not_found_for_every_actor() {
    let mut test = test_store("missing");
    let mut reads = ReplayReads::new(&mut test.store);
    for actor in [Actor::Anonymous, account(1), seat("missing", "token")] {
        assert!(is_not_found(reads.replay(&actor, "missing")));
        assert!(is_not_found(reads.summary(&actor, "missing")));
    }
}

#[test]
fn account_owned_matches_are_private_to_their_account() {
    let mut test = test_store("owned");
    let created = test
        .store
        .create_match_for_user(HeroType::default(), Some(7))
        .expect("match should be created");
    complete(&mut test.store, &created.id, Side::Player);
    let mut reads = ReplayReads::new(&mut test.store);

    let read = reads.replay(&account(7), &created.id).expect("owner reads");
    assert_eq!(read.visibility, ReplayVisibility::Revealed);
    let summary = reads
        .summary(&account(7), &created.id)
        .expect("owner reads");
    assert_eq!(summary.viewer_side, Some(Side::Player));

    for actor in [Actor::Anonymous, account(8), seat(&created.id, "token")] {
        assert!(is_not_found(reads.replay(&actor, &created.id)));
        assert!(is_not_found(reads.summary(&actor, &created.id)));
    }
}

#[test]
fn anonymous_matches_replay_publicly_until_revealed_on_completion() {
    let mut test = test_store("anonymous");
    let created = test
        .store
        .create_match_for_user(HeroType::default(), None)
        .expect("match should be created");

    {
        let mut reads = ReplayReads::new(&mut test.store);
        let read = reads
            .replay(&Actor::Anonymous, &created.id)
            .expect("public");
        assert_eq!(read.visibility, ReplayVisibility::Public);
        assert!(is_not_found(reads.summary(&Actor::Anonymous, &created.id)));
    }

    complete(&mut test.store, &created.id, Side::Opponent);
    let mut reads = ReplayReads::new(&mut test.store);
    let read = reads
        .replay(&Actor::Anonymous, &created.id)
        .expect("public");
    assert_eq!(read.visibility, ReplayVisibility::Revealed);
    let summary = reads
        .summary(&Actor::Anonymous, &created.id)
        .expect("summary");
    assert_eq!(summary.viewer_side, Some(Side::Player));
    assert!(summary.reward.is_none());
}

#[test]
fn active_shared_matches_are_hidden_from_every_actor() {
    let mut test = test_store("active-shared");
    let created = active_duel(&mut test.store);
    let mut reads = ReplayReads::new(&mut test.store);
    for actor in [
        Actor::Anonymous,
        account(1),
        account(2),
        seat(&created.match_id, &created.player_token),
    ] {
        assert!(is_not_found(reads.replay(&actor, &created.match_id)));
        assert!(is_not_found(reads.summary(&actor, &created.match_id)));
    }
}

#[test]
fn completed_shared_matches_are_read_from_the_participant_side() {
    let mut test = test_store("completed-shared");
    let created = completed_duel(&mut test.store);
    let mut reads = ReplayReads::new(&mut test.store);

    let read = reads
        .replay(&account(2), &created.match_id)
        .expect("participant reads replay");
    assert_eq!(read.visibility, ReplayVisibility::Revealed);
    let summary = reads
        .summary(&account(2), &created.match_id)
        .expect("participant reads summary");
    assert_eq!(summary.viewer_side, Some(Side::Opponent));

    let summary = reads
        .summary(
            &seat(&created.match_id, &created.player_token),
            &created.match_id,
        )
        .expect("seat link reads summary");
    assert_eq!(summary.viewer_side, Some(Side::Player));
    let read = reads
        .replay(
            &seat(&created.match_id, &created.opponent_token),
            &created.match_id,
        )
        .expect("seat link reads replay");
    assert_eq!(read.visibility, ReplayVisibility::Revealed);

    assert!(is_not_found(reads.summary(&account(3), &created.match_id)));
    assert!(is_not_found(
        reads.summary(&Actor::Anonymous, &created.match_id)
    ));
    assert!(is_not_found(reads.replay(
        &seat(&created.match_id, "wrong-token"),
        &created.match_id
    )));
    assert!(is_not_found(reads.replay(
        &seat("other-match", &created.player_token),
        &created.match_id
    )));
}
