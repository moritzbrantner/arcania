use super::*;

fn unit_definition_json_with_id(id: &str, name: &str, attack: i32, armor: i32) -> String {
    format!(
        r#"{{
            "id":"{id}",
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

fn unit_definition_json(name: &str, attack: i32, armor: i32) -> String {
    unit_definition_json_with_id("ash-duelist", name, attack, armor)
}

fn draft_request_json(name: &str, attack: i32, armor: i32) -> String {
    format!(
        r#"{{"definition":{}}}"#,
        unit_definition_json(name, attack, armor)
    )
}

fn draft_request_json_with_id(id: &str, name: &str, attack: i32, armor: i32) -> String {
    format!(
        r#"{{"definition":{}}}"#,
        unit_definition_json_with_id(id, name, attack, armor)
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
    let catalog_id = draft["catalogId"]
        .as_str()
        .expect("draft catalog id should exist")
        .to_string();

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
    assert_eq!(first["revision"]["id"]["cardId"], catalog_id);
    assert_eq!(first["revision"]["id"]["revision"], 1);
    assert_eq!(first["revision"]["definition"]["name"], "Ash Duelist");

    let (status, retry) = json_request(
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
    assert_eq!(retry["revision"]["id"]["revision"], 1);

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

    let (status, delayed_retry) = json_request(
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
    assert_eq!(delayed_retry["revision"]["id"]["revision"], 1);
    assert_eq!(delayed_retry["revision"]["id"]["cardId"], catalog_id);

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
    assert_eq!(fork["catalogId"], catalog_id);
    assert_eq!(fork["definition"]["id"], "ash-duelist");
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


#[tokio::test]
async fn published_core_ids_are_unique_across_accounts_and_starter_ids() {
    let path = test_db_path("card-workshop-global-identity");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let first_user = register_test_account(app.clone(), "workshop-global-a@example.com").await;
    let second_user = register_test_account(app.clone(), "workshop-global-b@example.com").await;

    async fn create_and_publish(
        app: Router,
        token: &str,
        request_json: String,
    ) -> serde_json::Value {
        let (status, draft) = json_request(
            app.clone(),
            Request::builder()
                .method("POST")
                .uri("/api/card-drafts")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(request_json))
                .expect("request should build"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let draft_id = draft["id"].as_i64().expect("draft id should exist");

        let (status, published) = json_request(
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
        assert_eq!(status, StatusCode::OK);
        published
    }

    let first = create_and_publish(
        app.clone(),
        &first_user,
        draft_request_json("Ash Duelist A", 2, 3),
    )
    .await;
    let second = create_and_publish(
        app.clone(),
        &second_user,
        draft_request_json("Ash Duelist B", 2, 3),
    )
    .await;
    let starter_named = create_and_publish(
        app,
        &first_user,
        draft_request_json_with_id("ember-squire", "Custom Ember Squire", 2, 3),
    )
    .await;

    let first_id = first["revision"]["id"]["cardId"]
        .as_str()
        .expect("first core card id should exist");
    let second_id = second["revision"]["id"]["cardId"]
        .as_str()
        .expect("second core card id should exist");
    let starter_named_id = starter_named["revision"]["id"]["cardId"]
        .as_str()
        .expect("starter-named core card id should exist");

    assert!(first_id.starts_with("custom-"));
    assert!(second_id.starts_with("custom-"));
    assert_ne!(first_id, second_id);
    assert_ne!(starter_named_id, "ember-squire");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn duplicate_local_card_ids_cannot_create_parallel_custom_identities() {
    let path = test_db_path("card-workshop-local-id-conflict");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "workshop-card-id@example.com").await;

    for expected_status in [StatusCode::OK, StatusCode::CONFLICT] {
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

        let (status, _) = json_request(
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
        assert_eq!(status, expected_status);
    }

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn publish_rechecks_the_draft_version_from_an_independent_connection() {
    let path = test_db_path("card-workshop-cross-connection-version");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "workshop-cross-connection@example.com").await;

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

    {
        let mut concurrent_store =
            SqliteMatchStore::new(&path).expect("second store should open same database");
        concurrent_store
            .connection_mut()
            .execute(
                "UPDATE card_drafts SET version = version + 1 WHERE id = ?1",
                rusqlite::params![draft_id],
            )
            .expect("independent draft update should succeed");
    }

    let (status, body) = json_request(
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
        body["message"]
            .as_str()
            .expect("message should be a string")
            .contains("current version is 2")
    );

    let _ = fs::remove_file(path);
}
