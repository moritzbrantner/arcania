use super::*;

#[tokio::test]
async fn debug_match_scenarios_can_be_listed() {
    let path = test_db_path("dev-scenarios-list");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, body) = json_request(
        app,
        Request::builder()
            .uri("/api/dev/match-scenarios")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let scenarios = body["scenarios"]
        .as_array()
        .expect("scenarios should exist");
    assert!(
        scenarios
            .iter()
            .any(|scenario| scenario["id"] == "play-unit-card")
    );
    assert!(
        scenarios
            .iter()
            .any(|scenario| scenario["id"] == "priority-response")
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn debug_match_scenario_creation_returns_a_playable_match() {
    let path = test_db_path("dev-scenario-create");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/dev/match-scenarios/play-unit-card/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let match_id = created["matchId"].as_str().expect("match id should exist");
    assert!(match_id.starts_with("dev-play-unit-card-"));
    assert_eq!(
        created["matchState"]["player"]["hand"][0]["templateId"],
        "ember-squire"
    );

    let (status, loaded) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!("/api/matches/{match_id}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["matchId"], match_id);

    let (status, acted) = post_match_action(
            app,
            match_id,
            r#"{"type":"playCard","cardId":"scenario-ember-squire","target":{"type":"hex","coord":{"q":0,"r":0}}}"#,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        acted["matchState"]["board"]["units"]
            .as_array()
            .expect("units should exist")
            .iter()
            .any(|unit| unit["templateId"] == "ember-squire")
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn unknown_debug_match_scenario_returns_not_found() {
    let path = test_db_path("dev-scenario-missing");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, body) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/dev/match-scenarios/missing/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["message"], "Match scenario missing was not found");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn debug_match_scenarios_do_not_appear_in_account_archives() {
    let path = test_db_path("dev-scenario-archive");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, _created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/dev/match-scenarios/play-unit-card/matches")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let token = register_test_account(app.clone(), "scenario-archive@example.com").await;
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
    assert_eq!(
        archive["matches"]
            .as_array()
            .expect("matches should exist")
            .len(),
        0
    );

    let _ = fs::remove_file(path);
}
