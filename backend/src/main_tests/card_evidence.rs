use super::card_transfers::{publish_transfer_fixture, transfer_request};
use super::*;
use serde_json::{Value, json};

const RUN_URI: &str = "/api/card-evidence/run";

fn comparison_request(draft: &Value, published: &Value) -> Value {
    let mut request: Value =
        serde_json::from_str(include_str!("../../config/card-comparison.example.json")).unwrap();
    request["candidate"] =
        json!({"type": "draft", "draftId": draft["id"], "version": draft["version"]});
    request["baseline"] = json!({"type": "published", "revisionId": published["revision"]["id"]});
    request
}

#[tokio::test]
async fn card_evidence_bounds_application_work_and_rejects_client_replacement_definitions() {
    let path = test_db_path("card-evidence-request-validation");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "evidence-validation@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let request = comparison_request(&draft, &published);
    let mut oversized = request.clone();
    oversized["cases"] = json!(vec![request["cases"][0].clone(); 9]);
    let mut too_many_actions = request.clone();
    too_many_actions["maxActions"] = json!(301);
    for invalid in [oversized, too_many_actions] {
        let (status, error) =
            transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(invalid)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{error}");
    }
    for source in ["candidate", "baseline"] {
        let mut replacement = request.clone();
        replacement[source]["definition"] = draft["definition"].clone();
        let (status, _) =
            transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(replacement)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    for (pointer, value) in [
        ("/maxActions", json!(0)),
        ("/cases", json!([])),
        ("/playerPolicyId", json!("missing-policy")),
        ("/cases/0/workshop/ruleset/turn/baseHeroMana", json!(0)),
        ("/cases/0/workshop/cards", json!([draft["definition"]])),
    ] {
        let mut invalid = request.clone();
        *invalid.pointer_mut(pointer).unwrap() = value;
        let (status, _) =
            transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(invalid)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (_, unchanged) = transfer_request(
        app,
        &owner,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(unchanged, draft);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn owned_card_evidence_preserves_exact_sources_and_repeats_after_restart() {
    let path = test_db_path("card-evidence-repeat");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "evidence-owner@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let draft_uri = format!("/api/card-drafts/{}", draft["id"]);
    let (_, history_before) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        "/api/card-revisions/ash-duelist",
        None,
    )
    .await;
    let (_, matches_before) =
        transfer_request(app.clone(), &owner, "GET", "/api/profile/matches", None).await;
    let request = comparison_request(&draft, &published);
    let (status, report) =
        transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(request.clone())).await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(
        report["input"]["candidate"]["source"],
        json!({"type": "draft", "draftId": draft["id"], "version": 1, "catalogId": draft["catalogId"]})
    );
    let mut runtime_definition = draft["definition"].clone();
    runtime_definition["id"] = draft["catalogId"].clone();
    assert_eq!(
        report["input"]["candidate"]["definition"],
        runtime_definition
    );
    assert_eq!(report["input"]["baseline"]["source"], request["baseline"]);
    assert_eq!(
        report["input"]["baseline"]["definition"],
        published["revision"]["definition"]
    );
    let pairs = report["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 2);
    for pair in pairs {
        assert_eq!(pair["candidate"]["cardPlays"], 1);
        assert_eq!(pair["candidate"]["survivingUnits"], 1);
        assert_eq!(pair["candidate"], pair["baseline"]);
        assert_eq!(pair["delta"]["cardPlays"], 0);
    }
    let (status, repeated) =
        transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(request.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated, report);
    for (uri, before) in [
        (draft_uri.as_str(), &draft),
        ("/api/card-revisions/ash-duelist", &history_before),
        ("/api/profile/matches", &matches_before),
    ] {
        let (status, after) = transfer_request(app.clone(), &owner, "GET", uri, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&after, before);
    }
    drop(app);
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    let (status, repeated) =
        transfer_request(reopened, &owner, "POST", RUN_URI, Some(request)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated, report);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn card_evidence_enforces_account_access_for_both_sources_and_imported_copies() {
    let path = test_db_path("card-evidence-access");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "evidence-source@example.com").await;
    let recipient = register_test_account(app.clone(), "evidence-recipient@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let request = comparison_request(&draft, &published);
    let (status, _) =
        transfer_request(app.clone(), "", "POST", RUN_URI, Some(request.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    for source in ["candidate", "baseline"] {
        for reference in [request["candidate"].clone(), request["baseline"].clone()] {
            let mut private_request = request.clone();
            private_request["candidate"] =
                json!({"type": "published", "revisionId": {"cardId": "stoneguard", "revision": 1}});
            private_request["baseline"] = private_request["candidate"].clone();
            private_request[source] = reference;
            let (status, _) = transfer_request(
                app.clone(),
                &recipient,
                "POST",
                RUN_URI,
                Some(private_request),
            )
            .await;
            assert_eq!(status, StatusCode::NOT_FOUND);
        }
        let mut missing = request.clone();
        missing[source] = json!({"type": "draft", "draftId": 999999, "version": 1});
        let (status, _) =
            transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(missing)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let mut missing = request.clone();
        missing[source] = request["baseline"].clone();
        missing[source]["revisionId"]["revision"] = json!(2);
        let (status, _) =
            transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(missing)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let id = &published["revision"]["id"];
    let (_, document) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        &format!("/api/card-transfers/{}/1", id["cardId"].as_str().unwrap()),
        None,
    )
    .await;
    let (status, _) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-transfers/import",
        Some(document),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut imported = request.clone();
    imported["candidate"] = request["baseline"].clone();
    let (status, report) =
        transfer_request(app.clone(), &recipient, "POST", RUN_URI, Some(imported)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        report["input"]["candidate"]["definition"],
        published["revision"]["definition"]
    );
    let (status, _) =
        transfer_request(app.clone(), &recipient, "POST", RUN_URI, Some(request)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, history) = transfer_request(
        app,
        &recipient,
        "GET",
        "/api/card-revisions/ash-duelist",
        None,
    )
    .await;
    assert_eq!(history["revisions"], json!([]));
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn card_evidence_rejects_stale_drafts_and_never_substitutes_latest_revisions() {
    let path = test_db_path("card-evidence-versions");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "evidence-versions@example.com").await;
    let (draft, first) = publish_transfer_fixture(app.clone(), &owner).await;
    let draft_uri = format!("/api/card-drafts/{}", draft["id"]);
    let mut definition = draft["definition"].clone();
    definition["cost"] = json!(4);
    let (status, updated) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &draft_uri,
        Some(json!({"version": 1, "definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let request = comparison_request(&draft, &first);
    for source in ["candidate", "baseline"] {
        let mut stale = request.clone();
        stale[source] = request["candidate"].clone();
        let (status, error) =
            transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(stale)).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("current version is 2")
        );
    }
    let (status, second) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        &format!("{draft_uri}/publish"),
        Some(json!({"version": 2})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, report) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        RUN_URI,
        Some(comparison_request(&updated, &first)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["input"]["candidate"]["source"]["version"], 2);
    for pair in report["pairs"].as_array().unwrap() {
        assert_eq!(pair["candidate"]["cardPlays"], 0);
        assert_eq!(pair["baseline"]["cardPlays"], 1);
        assert_eq!(pair["delta"]["cardPlays"], -1);
    }
    let mut definition = updated["definition"].clone();
    definition["cost"] = json!(2);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &draft_uri,
        Some(json!({"version": 2, "definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut exact = comparison_request(&updated, &first);
    exact["candidate"] = json!({"type": "published", "revisionId": second["revision"]["id"]});
    let (status, frozen) =
        transfer_request(app.clone(), &owner, "POST", RUN_URI, Some(exact)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        frozen["input"]["candidate"]["definition"],
        second["revision"]["definition"]
    );
    assert_eq!(
        frozen["input"]["baseline"]["definition"],
        first["revision"]["definition"]
    );
    assert_eq!(frozen["pairs"], report["pairs"]);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn card_evidence_rejects_invalid_persisted_definitions_before_running() {
    let path = test_db_path("card-evidence-invalid-draft");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "evidence-invalid@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let draft_uri = format!("/api/card-drafts/{}", draft["id"]);
    let mut definition = draft["definition"].clone();
    definition["kind"]["armor"] = json!(-1);
    let (status, invalid) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &draft_uri,
        Some(json!({"version": 1, "definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!invalid["validationErrors"].as_array().unwrap().is_empty());
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        RUN_URI,
        Some(comparison_request(&invalid, &published)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut definition = draft["definition"].clone();
    definition["cost"] = json!(31);
    let (status, out_of_bounds) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &draft_uri,
        Some(json!({"version": 2, "definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        out_of_bounds["validationErrors"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        RUN_URI,
        Some(comparison_request(&out_of_bounds, &published)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, unchanged) = transfer_request(app.clone(), &owner, "GET", &draft_uri, None).await;
    assert_eq!(unchanged, out_of_bounds);
    let mut definition = draft["definition"].clone();
    definition["id"] = json!("invalid draft id!");
    let (status, invalid_id) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        "/api/card-drafts",
        Some(json!({"definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        RUN_URI,
        Some(comparison_request(&invalid_id, &published)),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, history) =
        transfer_request(app, &owner, "GET", "/api/card-revisions/ash-duelist", None).await;
    assert_eq!(history["revisions"], json!([published]));
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn card_evidence_fails_closed_on_incompatible_stored_revision_identity() {
    let path = test_db_path("card-evidence-incompatible-revision");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "evidence-corrupt@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    // Fault injection in this test's database; observe rejection through the real HTTP boundary.
    let mut incompatible = published["revision"].clone();
    incompatible["id"]["cardId"] = json!("custom-incompatible-evidence");
    incompatible["definition"]["id"] = json!("custom-incompatible-evidence");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE card_revisions SET revision_json = ?1 WHERE core_card_id = ?2 AND revision = 1",
            rusqlite::params![
                incompatible.to_string(),
                published["revision"]["id"]["cardId"].as_str().unwrap()
            ],
        )
        .unwrap();
    drop(connection);
    let (status, error) = transfer_request(
        app,
        &owner,
        "POST",
        RUN_URI,
        Some(comparison_request(&draft, &published)),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("incompatible immutable content")
    );
    let _ = fs::remove_file(path);
}
