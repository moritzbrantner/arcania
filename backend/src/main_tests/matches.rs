use super::*;

#[tokio::test]
async fn signed_out_match_archive_requires_an_account() {
    let path = test_db_path("archive-auth");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, body) = json_request(
        app,
        Request::builder()
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "Sign in to continue.");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn signed_out_players_can_start_with_system_loadouts() {
    let path = test_db_path("player-system-loadout");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, created) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"playerDeck":{"source":"system","systemDeckId":"ember-burn"}}"#,
            ))
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        created["matchState"]["player"]["hero"]["heroType"],
        "pyromancer"
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn create_match_persists_and_can_be_loaded_by_id() {
    let path = test_db_path("create-load");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");
    assert!(match_id.starts_with("arc-"));
    assert_eq!(created["matchState"]["round"], 1);

    let (status, loaded) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{match_id}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["matchId"], match_id);
    assert_eq!(
        loaded["matchState"]["player"]["hand"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    assert!(loaded["matchState"]["opponent"].get("hand").is_none());
    assert!(path.exists());

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn create_match_accepts_player_hero_type() {
    let path = test_db_path("create-hero-type");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, created) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        created["matchState"]["player"]["hero"]["heroType"],
        "chronomancer"
    );
    assert_eq!(created["matchState"]["player"]["hero"]["maxAp"], 4);
    assert_eq!(created["matchState"]["player"]["hero"]["maxHp"], 16);
    assert_eq!(
        created["matchState"]["opponent"]["hero"]["heroType"],
        "runekeeper"
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn create_match_initializes_archive_and_replay_frame() {
    let path = test_db_path("archive-replay");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "archive-replay@example.com").await;

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");

    let (status, archive) = json_request(
        app.clone(),
        Request::builder()
            .uri("/api/matches")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(archive["matches"][0]["matchId"], match_id);
    assert_eq!(archive["matches"][0]["frameCount"], 1);

    let (status, replay) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!("/api/matches/{match_id}/replay"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["visibility"], "public");
    assert_eq!(replay["frames"][0]["frameIndex"], 0);
    assert_eq!(replay["frames"][0]["event"]["type"], "matchCreated");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn completed_match_summary_includes_viewer_reward_unlocks() {
    let path = test_db_path("match-summary-reward");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "summary-reward@example.com").await;

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"pyromancer"}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");
    complete_match_by_forfeit(&path, match_id, Side::Player);

    let (status, summary) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{match_id}/summary"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["matchId"], match_id);
    assert_eq!(summary["summary"]["mode"], "solo");
    assert_eq!(summary["viewer"]["side"], "player");
    assert_eq!(summary["viewer"]["result"], "victory");
    assert_eq!(summary["reward"]["accountXpGained"], 150);
    assert_eq!(summary["reward"]["heroXpGained"], 150);
    assert_eq!(summary["reward"]["winBonusXp"], 50);
    assert_eq!(summary["reward"]["account"]["before"]["level"], 1);
    assert_eq!(summary["reward"]["account"]["after"]["level"], 2);
    assert!(
        summary["reward"]["unlocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unlock| unlock["type"] == "runeUnlocked" && unlock["runeId"] == "vitality")
    );
    assert!(
        summary["reward"]["unlocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|unlock| unlock["type"] == "skillPointUnlocked")
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn match_actions_persist_and_reload_by_match_id() {
    let path = test_db_path("action-persist");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");

    let acted = end_turn_from_card_play(app.clone(), match_id).await;
    assert_eq!(acted["matchId"], match_id);
    assert_eq!(acted["matchState"]["round"], 1);
    assert_eq!(acted["matchState"]["activeSide"], "opponent");

    let acted = advance_solo_match_to_player_turn(app.clone(), match_id).await;
    assert_eq!(acted["matchState"]["round"], 2);
    assert_eq!(
        acted["matchState"]["player"]["hand"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    assert!(acted["matchState"]["player"].get("deck").is_none());
    assert!(acted["matchState"]["player"].get("discard").is_none());
    assert!(acted["matchState"]["opponent"].get("hand").is_none());

    let reopened = create_app(SqliteMatchStore::new(&path).expect("store should reopen"));
    let (status, loaded) = json_request(
        reopened,
        Request::builder()
            .uri(format!("/api/matches/{match_id}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["matchState"], acted["matchState"]);

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn accepted_actions_append_action_record_and_internal_replay_frames() {
    let path = test_db_path("action-replay");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (_, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let match_id = created["matchId"].as_str().expect("match id should exist");

    end_turn_from_card_play(app.clone(), match_id).await;
    advance_solo_match_to_player_turn(app.clone(), match_id).await;

    let (status, replay) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!("/api/matches/{match_id}/replay"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let frames = replay["frames"].as_array().expect("frames should exist");
    assert!(frames.len() > 3);
    assert!(frames.iter().any(|frame| {
        frame["event"]["type"] == "turnStarted" && frame["event"]["side"] == "opponent"
    }));
    let (status, loaded) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{match_id}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        frames.last().expect("last frame exists")["matchState"],
        loaded["matchState"]
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn match_action_response_includes_replay_frames_for_live_playback() {
    let path = test_db_path("action-live-frames");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (_, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let match_id = created["matchId"].as_str().expect("match id should exist");

    end_turn_from_card_play(app.clone(), match_id).await;
    let (status, _) = post_match_action(app.clone(), match_id, r#"{"type":"advanceAi"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let (status, advanced) =
        post_match_action(app.clone(), match_id, r#"{"type":"advanceAi"}"#).await;
    assert_eq!(status, StatusCode::OK);

    let frames = advanced["replayFrames"]
        .as_array()
        .expect("live replay frames should exist");
    assert!(frames.iter().any(|frame| {
        frame["event"]["type"] == "actionQueued" && frame["event"]["side"] == "opponent"
    }));
    assert_eq!(
        frames.last().expect("last frame should exist")["matchState"],
        advanced["matchState"]
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn invalid_actions_do_not_append_replay_frames() {
    let path = test_db_path("invalid-replay");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (_, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let match_id = created["matchId"].as_str().expect("match id should exist");

    let (status, _) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/matches/{match_id}/actions"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"type":"movePiece","pieceId":"player-hero","to":{"q":0,"r":0}}"#,
            ))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, replay) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{match_id}/replay"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["frames"].as_array().unwrap().len(), 1);

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn active_replay_redacts_opponent_hidden_draws() {
    let path = test_db_path("redacted-replay");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (_, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let match_id = created["matchId"].as_str().expect("match id should exist");

    for _ in 0..2 {
        end_turn_from_card_play(app.clone(), match_id).await;
        advance_solo_match_to_player_turn(app.clone(), match_id).await;
    }

    let (status, replay) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{match_id}/replay"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let hidden_draw = replay["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| {
            frame["event"]["type"] == "cardDrawn"
                && frame["event"]["side"] == "opponent"
                && frame["event"]["hidden"] == true
        })
        .expect("opponent hidden draw should be present");
    assert!(hidden_draw["event"]["card"].is_null());

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn missing_match_returns_json_404() {
    let path = test_db_path("missing");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, body) = json_request(
        app,
        Request::builder()
            .uri("/api/matches/arc-unknown")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "Match arc-unknown was not found");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn missing_match_action_returns_json_404() {
    let path = test_db_path("missing-action");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, body) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/matches/arc-unknown/actions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"type":"endTurn"}"#))
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "Match arc-unknown was not found");

    let _ = fs::remove_file(path);
}

#[test]
fn database_path_defaults_to_ignored_data_directory() {
    assert_eq!(
        match_store::database_path_from_environment(),
        PathBuf::from("data/arcania.sqlite3")
    );
}
