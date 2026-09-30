use super::*;

#[tokio::test]
async fn completed_two_v_two_summaries_report_results_for_each_viewer_team() {
    for (winner, results) in [
        (Side::Player, ["victory", "defeat", "victory", "defeat"]),
        (Side::Opponent, ["defeat", "victory", "defeat", "victory"]),
    ] {
        let path = test_db_path("shared-two-v-two-summary-results");
        let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
        let (status, created) = json_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/shared-matches")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"format":"twoVTwo"}"#))
                .expect("request should build"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let match_id = created["matchId"].as_str().expect("match id should exist");
        let seats = created["seatUrls"]
            .as_array()
            .expect("seat URLs should exist");
        assert_eq!(seats.len(), 4);

        for (seat, hero_type) in
            seats
                .iter()
                .zip(["runekeeper", "pyromancer", "warden", "barbarian"])
        {
            let token = seat_token_from_url(seat["url"].as_str().expect("seat URL should exist"));
            let (status, _) = json_request(
                app.clone(),
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/shared-matches/{match_id}/seats/{token}/join"))
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"heroType":"{hero_type}"}}"#)))
                    .expect("request should build"),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }

        complete_match_by_forfeit(&path, match_id, winner);

        for (seat, result) in seats.iter().zip(results) {
            let token = seat_token_from_url(seat["url"].as_str().expect("seat URL should exist"));
            let (status, summary) = json_request(
                app.clone(),
                Request::builder()
                    .uri(format!(
                        "/api/shared-matches/{match_id}/seats/{token}/summary"
                    ))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(summary["viewer"]["side"], seat["side"]);
            assert_eq!(summary["viewer"]["result"], result, "seat {}", seat["side"]);
            assert_eq!(summary["summary"]["viewerResult"], result);
            assert!(summary["reward"].is_null());
        }
        let _ = fs::remove_file(path);
    }
}

#[tokio::test]
async fn completed_shared_seat_links_can_load_summary_and_replay() {
    let path = test_db_path("shared-summary-replay");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/shared-matches")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");
    let player_token = seat_token_from_url(
        created["playerSeatUrl"]
            .as_str()
            .expect("player URL exists"),
    );
    let opponent_token = seat_token_from_url(
        created["inviteSeatUrl"]
            .as_str()
            .expect("invite URL exists"),
    );

    for (token, hero_type) in [(player_token, "pyromancer"), (opponent_token, "warden")] {
        let (status, _) = json_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri(format!("/api/shared-matches/{match_id}/seats/{token}/join"))
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"heroType":"{hero_type}"}}"#)))
                .expect("request should build"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    complete_match_by_forfeit(&path, match_id, Side::Opponent);

    let (status, player_summary) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{player_token}/summary"
            ))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(player_summary["summary"]["mode"], "shared");
    assert_eq!(player_summary["viewer"]["side"], "player");
    assert_eq!(player_summary["viewer"]["result"], "defeat");
    assert!(player_summary["reward"].is_null());

    let (status, opponent_summary) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{opponent_token}/summary"
            ))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(opponent_summary["viewer"]["side"], "opponent");
    assert_eq!(opponent_summary["viewer"]["result"], "victory");

    let (status, replay) = json_request(
        app,
        Request::builder()
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{opponent_token}/replay"
            ))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["visibility"], "revealed");
    assert_eq!(replay["summary"]["winner"], "opponent");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn completed_shared_account_participant_can_load_summary_from_match_route() {
    let path = test_db_path("shared-account-summary");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let player_auth = register_test_account(app.clone(), "shared-player@example.com").await;
    let opponent_auth = register_test_account(app.clone(), "shared-opponent@example.com").await;

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/shared-matches")
            .header("authorization", format!("Bearer {player_auth}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");
    let player_token = seat_token_from_url(
        created["playerSeatUrl"]
            .as_str()
            .expect("player URL exists"),
    );
    let opponent_token = seat_token_from_url(
        created["inviteSeatUrl"]
            .as_str()
            .expect("invite URL exists"),
    );

    for (token, auth, hero_type) in [
        (player_token, &player_auth, "pyromancer"),
        (opponent_token, &opponent_auth, "warden"),
    ] {
        let (status, _) = json_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri(format!("/api/shared-matches/{match_id}/seats/{token}/join"))
                .header("authorization", format!("Bearer {auth}"))
                .header("content-type", "application/json")
                .body(Body::from(format!(r#"{{"heroType":"{hero_type}"}}"#)))
                .expect("request should build"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }

    complete_match_by_forfeit(&path, match_id, Side::Opponent);

    let (status, summary) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!("/api/matches/{match_id}/summary"))
            .header("authorization", format!("Bearer {player_auth}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["summary"]["mode"], "shared");
    assert_eq!(summary["viewer"]["side"], "player");
    assert_eq!(summary["viewer"]["result"], "defeat");
    assert_eq!(summary["reward"]["accountXpGained"], 100);

    let (status, replay) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{match_id}/replay"))
            .header("authorization", format!("Bearer {opponent_auth}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replay["summary"]["mode"], "shared");
    assert_eq!(replay["visibility"], "revealed");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn shared_match_creation_returns_private_seat_links_and_hides_setup_from_archive() {
    let path = test_db_path("shared-create");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "shared-create@example.com").await;

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/shared-matches")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(created["mode"], "shared");
    assert_eq!(created["status"], "setup");
    assert_eq!(created["viewerSide"], "player");
    assert!(created.get("viewerHeroType").is_none());
    assert_ne!(created["playerSeatUrl"], created["inviteSeatUrl"]);
    assert!(
        created["playerSeatUrl"]
            .as_str()
            .unwrap()
            .contains("/match/")
    );
    assert!(
        created["inviteSeatUrl"]
            .as_str()
            .unwrap()
            .contains("/match/")
    );

    let (status, archive) = json_request(
        app,
        Request::builder()
            .uri("/api/matches")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(archive["matches"].as_array().unwrap().is_empty());

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn shared_match_join_exposes_only_the_viewer_seat_hand() {
    let path = test_db_path("shared-join");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "shared-join@example.com").await;

    let (_, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/shared-matches")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;
    let match_id = created["matchId"].as_str().expect("match id should exist");
    let player_token = seat_token_from_url(
        created["playerSeatUrl"]
            .as_str()
            .expect("player URL exists"),
    );
    let opponent_token = seat_token_from_url(
        created["inviteSeatUrl"]
            .as_str()
            .expect("invite URL exists"),
    );

    let (status, setup) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{player_token}"
            ))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(setup["status"], "setup");
    assert_eq!(setup["viewerReady"], false);
    assert_eq!(setup["opponentReady"], false);
    assert!(setup["viewerHeroType"].is_null());
    assert!(setup["opponentHeroType"].is_null());
    assert!(setup["matchState"].is_null());

    let (status, joined) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{opponent_token}/join"
            ))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"pyromancer"}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(joined["status"], "setup");
    assert_eq!(joined["viewerSide"], "opponent");
    assert_eq!(joined["viewerReady"], true);
    assert_eq!(joined["opponentReady"], false);
    assert_eq!(joined["viewerHeroType"], "pyromancer");
    assert!(joined["matchState"].is_null());

    let (status, activated) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{player_token}/join"
            ))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(activated["status"], "active");
    assert_eq!(activated["viewerSide"], "player");
    assert_eq!(activated["viewerReady"], true);
    assert_eq!(activated["opponentReady"], true);
    assert_eq!(activated["viewerHeroType"], "chronomancer");
    assert_eq!(activated["opponentHeroType"], "pyromancer");

    let (status, joined) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{opponent_token}"
            ))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(joined["status"], "active");
    assert_eq!(joined["viewerSide"], "opponent");
    assert_eq!(
        joined["matchState"]["opponent"]["hero"]["heroType"],
        "pyromancer"
    );
    assert!(joined["matchState"]["opponent"].get("hand").is_some());
    assert!(joined["matchState"]["player"].get("hand").is_none());
    assert_eq!(joined["matchState"]["opponent"]["handCount"], 7);
    assert_eq!(joined["matchState"]["player"]["handCount"], 7);

    let (status, player_view) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{player_token}"
            ))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        player_view["matchState"]["player"]["hero"]["heroType"],
        "chronomancer"
    );
    assert!(player_view["matchState"]["player"].get("hand").is_some());
    assert!(player_view["matchState"]["opponent"].get("hand").is_none());
    assert_eq!(player_view["matchState"]["player"]["handCount"], 7);
    assert_eq!(player_view["matchState"]["opponent"]["handCount"], 7);

    let (status, _) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!("/api/matches/{match_id}/replay"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, archive) = json_request(
        app,
        Request::builder()
            .uri("/api/matches")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(archive["matches"].as_array().unwrap().is_empty());

    let _ = fs::remove_file(path);
}
