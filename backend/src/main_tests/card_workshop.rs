use super::*;

fn unit_definition_json(name: &str, attack: i32, armor: i32) -> String {
    format!(
        r#"{{
            "id":"ash-duelist",
            "name":"{name}",
            "rarity":"advanced",
            "cost":2,
            "text":"2 attack / 3 armor / 2 AP.",
            "kind":{{
                "type":"unit",
                "attack":{attack},
                "armor":{armor},
                "maxAp":2
            }}
        }}"#
    )
}

fn draft_request_json(name: &str, attack: i32, armor: i32) -> String {
    format!(
        r#"{{"definition":{}}}"#,
        unit_definition_json(name, attack, armor)
    )
}

fn update_request_json(version: u64, name: &str, attack: i32, armor: i32) -> String {
    format!(
        r#"{{"version":{version},"definition":{}}}"#,
        unit_definition_json(name, attack, armor)
    )
}

#[tokio::test]
async fn card_workshop_requires_an_account() {
    let path = test_db_path("card-workshop-auth");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let (status, body) = json_request(
        app,
        Request::builder()
            .uri("/api/card-drafts")
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["message"], "Sign in to continue.");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn invalid_drafts_are_editable_but_cannot_be_published() {
    let path = test_db_path("card-workshop-invalid-draft");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "workshop-invalid@example.com").await;

    let (status, draft) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/card-drafts")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(draft_request_json("Broken Duelist", 2, 0)))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(draft["version"], 1);
    assert!(
        !draft["validationErrors"]
            .as_array()
            .expect("validation errors should be an array")
            .is_empty()
    );

    let draft_id = draft["id"].as_i64().expect("draft id should exist");
    let (status, rejected) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/card-drafts/{draft_id}/publish"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"version":1}"#))
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        rejected["message"]
            .as_str()
            .expect("message should be a string")
            .contains("validation error")
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn stale_draft_updates_and_publication_are_rejected() {
    let path = test_db_path("card-workshop-version-conflict");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "workshop-conflict@example.com").await;

    let (_, draft) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/card-drafts")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(draft_request_json("Ash Duelist", 2, 3)))
            .expect("request should build"),
    )
    .await;
    let draft_id = draft["id"].as_i64().expect("draft id should exist");

    let (status, updated) = json_request(
        app.clone(),
        Request::builder()
            .method("PATCH")
            .uri(format!("/api/card-drafts/{draft_id}"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(update_request_json(
                1,
                "Ash Duelist Revised",
                3,
                3,
            )))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["version"], 2);

    let (status, stale_update) = json_request(
        app.clone(),
        Request::builder()
            .method("PATCH")
            .uri(format!("/api/card-drafts/{draft_id}"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(update_request_json(1, "Stale Edit", 9, 9)))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        stale_update["message"]
            .as_str()
            .expect("message should be a string")
            .contains("current version is 2")
    );

    let (status, stale_publish) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/card-drafts/{draft_id}/publish"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"version":1}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        stale_publish["message"]
            .as_str()
            .expect("message should be a string")
            .contains("current version is 2")
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn published_revisions_are_immutable_and_forkable() {
    let path = test_db_path("card-workshop-publish-history");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "workshop-history@example.com").await;

    let (_, draft) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/card-drafts")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(draft_request_json("Ash Duelist", 2, 3)))
            .expect("request should build"),
    )
    .await;
    let draft_id = draft["id"].as_i64().expect("draft id should exist");

    let (status, first) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/card-drafts/{draft_id}/publish"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"version":1}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["revision"]["id"]["cardId"], "ash-duelist");
    assert_eq!(first["revision"]["id"]["revision"], 1);
    assert_eq!(first["revision"]["definition"]["name"], "Ash Duelist");

    let (_, updated) = json_request(
        app.clone(),
        Request::builder()
            .method("PATCH")
            .uri(format!("/api/card-drafts/{draft_id}"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(update_request_json(
                1,
                "Ash Duelist Revised",
                3,
                3,
            )))
            .expect("request should build"),
    )
    .await;
    assert_eq!(updated["version"], 2);

    let (status, second) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/card-drafts/{draft_id}/publish"))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"version":2}"#))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["revision"]["id"]["revision"], 2);
    assert_eq!(
        second["revision"]["definition"]["name"],
        "Ash Duelist Revised"
    );

    let (status, history) = json_request(
        app.clone(),
        Request::builder()
            .uri("/api/card-revisions/ash-duelist")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let revisions = history["revisions"]
        .as_array()
        .expect("history should be an array");
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0]["revision"]["id"]["revision"], 2);
    assert_eq!(revisions[1]["revision"]["id"]["revision"], 1);
    assert_eq!(
        revisions[1]["revision"]["definition"]["name"],
        "Ash Duelist"
    );
    assert_eq!(revisions[1]["revision"]["definition"]["kind"]["attack"], 2);

    let (status, fork) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/card-revisions/ash-duelist/1/fork")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fork["version"], 1);
    assert_eq!(fork["sourceRevision"], 1);
    assert_eq!(fork["definition"]["name"], "Ash Duelist");
    assert_eq!(fork["definition"]["kind"]["attack"], 2);

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn workshop_content_is_scoped_to_the_owning_account() {
    let path = test_db_path("card-workshop-ownership");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let owner = register_test_account(app.clone(), "workshop-owner@example.com").await;
    let other = register_test_account(app.clone(), "workshop-other@example.com").await;

    let (_, draft) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/card-drafts")
            .header("authorization", format!("Bearer {owner}"))
            .header("content-type", "application/json")
            .body(Body::from(draft_request_json("Ash Duelist", 2, 3)))
            .expect("request should build"),
    )
    .await;
    let draft_id = draft["id"].as_i64().expect("draft id should exist");

    let (_, _) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!("/api/card-drafts/{draft_id}/publish"))
            .header("authorization", format!("Bearer {owner}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"version":1}"#))
            .expect("request should build"),
    )
    .await;

    let (status, _) = json_request(
        app.clone(),
        Request::builder()
            .uri(format!("/api/card-drafts/{draft_id}"))
            .header("authorization", format!("Bearer {other}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, history) = json_request(
        app.clone(),
        Request::builder()
            .uri("/api/card-revisions/ash-duelist")
            .header("authorization", format!("Bearer {other}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        history["revisions"]
            .as_array()
            .expect("history should be an array")
            .is_empty()
    );

    let (status, _) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/card-revisions/ash-duelist/1/fork")
            .header("authorization", format!("Bearer {other}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let _ = fs::remove_file(path);
}
