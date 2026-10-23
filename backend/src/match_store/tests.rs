use super::*;
use crate::deck_library;
use rusqlite::params;
use std::time::{SystemTime, UNIX_EPOCH};

fn test_db_path(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "rune-lanes-{name}-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    path
}

#[test]
fn created_matches_can_be_loaded_from_sqlite() {
    let path = test_db_path("create-load");
    let created = {
        let mut store = SqliteMatchStore::new(&path).expect("store should open");
        store
            .create_match_for_user(HeroType::default(), None)
            .expect("match should be created")
    };

    let reopened = SqliteMatchStore::new(&path)
        .expect("store should reopen")
        .load_match(&created.id)
        .expect("match should load")
        .expect("match should exist");

    assert_eq!(reopened.id, created.id);
    assert_eq!(reopened.state.round, created.state.round);
    assert_eq!(
        reopened.state.player.hand.len(),
        created.state.player.hand.len()
    );
    assert_eq!(
        reopened.state.opponent.deck_count,
        created.state.opponent.deck_count
    );

    let _ = fs::remove_file(path);
}

#[test]
fn match_start_records_frame_zero_and_genesis_then_appends_dense_frames() {
    let path = test_db_path("match-start");
    let mut store = SqliteMatchStore::new(&path).expect("store should open");
    let created = store
        .create_match_for_user(HeroType::default(), None)
        .expect("match should be created");

    let replay = store
        .load_replay(&created.id)
        .expect("replay should load")
        .expect("replay should exist");
    assert_eq!(replay.frames.len(), 1);
    assert_eq!(replay.frames[0].frame_index, 0);
    assert_eq!(replay.frames[0].action_index, None);
    let (aggregate, next_action_index) = store
        .load_event_sourced_match(&created.id)
        .expect("event stream should load")
        .expect("genesis should exist");
    assert_eq!(next_action_index, 0);

    let mut candidate = aggregate.clone();
    let decision = aggregate
        .decide_forfeit(
            rune_lanes_core::event_sourcing::CommandMetadata {
                command_id: rune_lanes_core::event_sourcing::CommandId::new("store-test:0")
                    .expect("command id should be valid"),
                expected_version: aggregate.version(),
                side: Side::Player,
                action_index: 0,
            },
            Side::Player,
        )
        .expect("forfeit should decide");
    let rune_lanes_core::event_sourcing::CommandDecision::Append(event) = decision else {
        panic!("forfeit should append an event");
    };
    let outcome = candidate.evolve(&event).expect("forfeit should evolve");
    store
        .append_event_and_projection(
            &created.id,
            &event,
            &candidate.snapshot().expect("snapshot should build"),
            r#"{"type":"testForfeit"}"#,
            candidate.state(),
            &outcome.replay_frames,
            None,
        )
        .expect("event should append");

    let replay = store
        .load_replay(&created.id)
        .expect("replay should load")
        .expect("replay should exist");
    assert_eq!(replay.frames.len(), 1 + outcome.replay_frames.len());
    for (index, frame) in replay.frames.iter().enumerate() {
        assert_eq!(frame.frame_index as usize, index);
    }
    assert_eq!(replay.summary.state.winner, Some(Side::Player));
    assert_eq!(
        store
            .load_event_sourced_match(&created.id)
            .expect("event stream should load")
            .expect("match should exist")
            .1,
        1
    );

    let _ = fs::remove_file(path);
}

#[test]
fn missing_matches_load_as_none() {
    let path = test_db_path("missing");
    let store = SqliteMatchStore::new(&path).expect("store should open");

    let missing = store
        .load_match("rl-missing")
        .expect("lookup should succeed");

    assert!(missing.is_none());
    let _ = fs::remove_file(path);
}

#[test]
fn legacy_matches_without_replay_metadata_are_excluded_from_replays() {
    let path = test_db_path("legacy");
    let store = SqliteMatchStore::new(&path).expect("store should open");
    let legacy_state = MatchState::new();
    let snapshot = legacy_state
        .to_snapshot_json()
        .expect("snapshot should serialize");
    store
        .connection
        .execute(
            "
                INSERT INTO matches (id, snapshot_json, created_at, updated_at)
                VALUES (?1, ?2, unixepoch(), unixepoch())
                ",
            params!["rl-legacy", snapshot],
        )
        .expect("legacy row should insert");

    assert!(
        store
            .list_replayable_matches_for_user(1)
            .expect("archive should load")
            .is_empty()
    );
    assert!(
        store
            .load_replay("rl-legacy")
            .expect("replay lookup should succeed")
            .is_none()
    );

    let _ = fs::remove_file(path);
}

#[test]
fn replayable_shared_matches_are_filtered_to_joined_accounts() {
    let path = test_db_path("shared-participants");
    let mut store = SqliteMatchStore::new(&path).expect("store should open");

    let created = store
        .create_shared_match(Some(99), SharedMatchFormat::Duel)
        .expect("shared match should be created");
    store
        .join_shared_match(
            &created.match_id,
            &created.player_token,
            HeroType::Chronomancer,
            deck_library::starter_deck_snapshot(),
            MatchProgressionLoadout::default(),
            Some(1),
        )
        .expect("player should join");
    store
        .join_shared_match(
            &created.match_id,
            &created.opponent_token,
            HeroType::Pyromancer,
            deck_library::starter_deck_snapshot(),
            MatchProgressionLoadout::default(),
            Some(2),
        )
        .expect("opponent should join");
    store
        .connection
        .execute(
            "UPDATE shared_matches SET status = 'completed' WHERE match_id = ?1",
            params![&created.match_id],
        )
        .expect("shared match should be markable complete");

    let player_matches = store
        .list_replayable_matches_for_user(1)
        .expect("archive should load");
    let unrelated_matches = store
        .list_replayable_matches_for_user(3)
        .expect("archive should load");
    let creator_matches = store
        .list_replayable_matches_for_user(99)
        .expect("archive should load");

    assert_eq!(player_matches.len(), 1);
    assert_eq!(player_matches[0].id, created.match_id);
    assert!(unrelated_matches.is_empty());
    assert!(creator_matches.is_empty());

    let _ = fs::remove_file(path);
}

#[test]
fn two_v_two_shared_match_starts_after_all_four_seats_join() {
    let path = test_db_path("shared-two-v-two");
    let mut store = SqliteMatchStore::new(&path).expect("store should open");

    let created = store
        .create_shared_match(Some(99), SharedMatchFormat::TwoVTwo)
        .expect("2v2 shared match should be created");
    let player_two_token = created
        .player_two_token
        .clone()
        .expect("2v2 should include player two token");
    let opponent_two_token = created
        .opponent_two_token
        .clone()
        .expect("2v2 should include opponent two token");
    let starter = deck_library::starter_deck_snapshot();

    for (token, hero_type) in [
        (&created.player_token, HeroType::Runekeeper),
        (&created.opponent_token, HeroType::Pyromancer),
        (&player_two_token, HeroType::Warden),
    ] {
        let shared = store
            .join_shared_match(
                &created.match_id,
                token,
                hero_type,
                starter.clone(),
                MatchProgressionLoadout::default(),
                None,
            )
            .expect("seat should join")
            .expect("shared match should load");
        assert_eq!(shared.status, SharedMatchStatus::Setup);
        assert!(shared.state.is_none());
    }

    let shared = store
        .join_shared_match(
            &created.match_id,
            &opponent_two_token,
            HeroType::Barbarian,
            starter,
            MatchProgressionLoadout::default(),
            None,
        )
        .expect("final seat should join")
        .expect("shared match should load");

    assert_eq!(shared.status, SharedMatchStatus::Active);
    assert_eq!(shared.format, SharedMatchFormat::TwoVTwo);
    assert_eq!(shared.seats.len(), 4);
    let state = shared.state.expect("2v2 state should be created");
    assert_eq!(state.format(), "twoVTwo");
    assert_eq!(state.board.radius, 4);
    assert!(state.player_two.is_some());
    assert!(state.opponent_two.is_some());
    assert_eq!(state.active_side, Side::Player);

    let _ = fs::remove_file(path);
}

#[test]
fn database_path_override_uses_given_value() {
    assert_eq!(
        database_path_from_override(Some(OsString::from("/tmp/rune-lanes-test.sqlite3"))),
        PathBuf::from("/tmp/rune-lanes-test.sqlite3")
    );
}
