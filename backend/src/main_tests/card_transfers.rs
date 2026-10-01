use super::*;

async fn transfer_request(
    app: Router,
    token: &str,
    method: &str,
    uri: &str,
    document: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    json_request(
        app,
        Request::builder()
            .method(method)
            .uri(uri)
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(document.map_or_else(Body::empty, |value| Body::from(value.to_string())))
            .unwrap(),
    )
    .await
}

async fn publish_transfer_fixture(
    app: Router,
    token: &str,
) -> (serde_json::Value, serde_json::Value) {
    let mut definition = rune_lanes_core::starter_card_definitions().remove(0);
    definition.id = "ash-duelist".to_string();
    definition.taxonomy.faction = Some("ember-clan".to_string());
    let (status, draft) = transfer_request(
        app.clone(),
        token,
        "POST",
        "/api/card-drafts",
        Some(serde_json::json!({"definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, published) = transfer_request(
        app,
        token,
        "POST",
        &format!("/api/card-drafts/{}/publish", draft["id"]),
        Some(serde_json::json!({"version": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    (draft, published)
}

fn export_uri(published: &serde_json::Value) -> String {
    let id = &published["revision"]["id"];
    format!(
        "/api/card-transfers/{}/{}",
        id["cardId"].as_str().unwrap(),
        id["revision"]
    )
}

#[tokio::test]
async fn exact_card_exports_are_account_scoped_and_survive_restart() {
    let path = test_db_path("card-transfer-export");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "transfer-owner@example.com").await;
    let other = register_test_account(app.clone(), "transfer-other@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let uri = export_uri(&published);
    let (status, exported) = transfer_request(app.clone(), &owner, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exported["schemaVersion"], 1);
    assert_eq!(exported["revision"], published["revision"]);
    assert_eq!(
        exported["revision"]["definition"]["taxonomy"]["faction"],
        "ember-clan"
    );
    let (status, _) = transfer_request(app.clone(), &other, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = transfer_request(app.clone(), "", "GET", &uri, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) =
        transfer_request(app.clone(), &owner, "GET", &uri.replace("/1", "/2"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = transfer_request(
        app.clone(),
        &other,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, history) =
        transfer_request(app, &other, "GET", "/api/card-revisions/ash-duelist", None).await;
    assert_eq!(history["revisions"], serde_json::json!([]));
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    let (status, reexported) = transfer_request(reopened, &owner, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reexported, exported);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn cross_account_import_previews_without_mutation_and_preserves_exact_content() {
    let path = test_db_path("card-transfer-import");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "import-source@example.com").await;
    let recipient = register_test_account(app.clone(), "import-recipient@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let uri = export_uri(&published);
    let (_, document) = transfer_request(app.clone(), &owner, "GET", &uri, None).await;

    let (status, preview) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-transfers/preview",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["status"], "newRevision");
    assert_eq!(preview["revisionId"], document["revision"]["id"]);
    let (status, _) = transfer_request(app.clone(), &recipient, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, imported) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(imported["status"], "newRevision");
    let (status, reexported) = transfer_request(app.clone(), &recipient, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(reexported, document);
    let (status, repeated) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated["status"], "alreadyPresent");
    let (_, history) = transfer_request(
        app.clone(),
        &recipient,
        "GET",
        "/api/card-revisions/ash-duelist",
        None,
    )
    .await;
    assert_eq!(history["revisions"], serde_json::json!([]));
    let (status, _) = transfer_request(
        app.clone(),
        &recipient,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let mut changed = draft["definition"].clone();
    changed["cost"] = serde_json::json!(4);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &format!("/api/card-drafts/{}", draft["id"]),
        Some(serde_json::json!({"version":1,"definition":changed})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, newer) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        &format!("/api/card-drafts/{}/publish", draft["id"]),
        Some(serde_json::json!({"version":2})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(newer["revision"]["id"]["revision"], 2);
    let (status, _) = transfer_request(app, &recipient, "GET", &export_uri(&newer), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    let (status, persisted) = transfer_request(reopened, &recipient, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(persisted, document);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn publication_cannot_diverge_from_an_imported_exact_revision() {
    let path = test_db_path("card-transfer-publication-conflict");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "publication-owner@example.com").await;
    let recipient = register_test_account(app.clone(), "publication-recipient@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let (_, mut document) =
        transfer_request(app.clone(), &owner, "GET", &export_uri(&published), None).await;
    document["revision"]["id"]["revision"] = serde_json::json!(2);
    document["revision"]["definition"]["cost"] = serde_json::json!(4);
    let imported_uri = format!(
        "/api/card-transfers/{}/2",
        published["revision"]["id"]["cardId"].as_str().unwrap()
    );
    let (status, _) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut changed = draft["definition"].clone();
    changed["cost"] = serde_json::json!(5);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &format!("/api/card-drafts/{}", draft["id"]),
        Some(serde_json::json!({"version":1,"definition":changed})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        &format!("/api/card-drafts/{}/publish", draft["id"]),
        Some(serde_json::json!({"version":2})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, history) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        "/api/card-revisions/ash-duelist",
        None,
    )
    .await;
    assert_eq!(history["revisions"].as_array().unwrap().len(), 1);
    let (status, preserved) =
        transfer_request(app.clone(), &recipient, "GET", &imported_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preserved, document);

    let mut compatible = draft["definition"].clone();
    compatible["cost"] = serde_json::json!(4);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &format!("/api/card-drafts/{}", draft["id"]),
        Some(serde_json::json!({"version":2,"definition":compatible})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, agreed) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        &format!("/api/card-drafts/{}/publish", draft["id"]),
        Some(serde_json::json!({"version":3})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(agreed["revision"], document["revision"]);
    let (status, _) = transfer_request(app, &recipient, "GET", &imported_uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn conflicting_and_malformed_transfers_never_grant_account_access() {
    let path = test_db_path("card-transfer-rejection");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "rejection-owner@example.com").await;
    let recipient = register_test_account(app.clone(), "rejection-recipient@example.com").await;
    let (_, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let uri = export_uri(&published);
    let (_, original) = transfer_request(app.clone(), &owner, "GET", &uri, None).await;
    for endpoint in ["/api/card-transfers/preview", "/api/card-transfers/import"] {
        let (status, _) =
            transfer_request(app.clone(), "", "POST", endpoint, Some(original.clone())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        for (pointer, value) in [
            ("/schemaVersion", serde_json::json!(2)),
            ("/revision/definition/kind/armor", serde_json::json!(-1)),
            ("/revision/id/revision", serde_json::json!(0)),
        ] {
            let mut invalid = original.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            let (status, _) =
                transfer_request(app.clone(), &recipient, "POST", endpoint, Some(invalid)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        }
    }
    for (pointer, value) in [
        ("/revision/definition/cost", serde_json::json!(4)),
        (
            "/revision/definition/taxonomy/faction",
            serde_json::json!("stone-clan"),
        ),
    ] {
        let mut conflict = original.clone();
        *conflict.pointer_mut(pointer).unwrap() = value;
        let (status, preview) = transfer_request(
            app.clone(),
            &recipient,
            "POST",
            "/api/card-transfers/preview",
            Some(conflict.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(preview["status"], "identityConflict");
        let (status, _) = transfer_request(
            app.clone(),
            &recipient,
            "POST",
            "/api/card-transfers/import",
            Some(conflict),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }
    let (status, _) = transfer_request(app.clone(), &recipient, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, unchanged) = transfer_request(app, &owner, "GET", &uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unchanged, original);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn import_rechecks_identity_conflicts_after_an_earlier_preview() {
    let path = test_db_path("card-transfer-stale-preview");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let first = register_test_account(app.clone(), "stale-first@example.com").await;
    let second = register_test_account(app.clone(), "stale-second@example.com").await;
    let (_, published) = publish_transfer_fixture(app.clone(), &first).await;
    let (_, mut original) =
        transfer_request(app.clone(), &first, "GET", &export_uri(&published), None).await;
    original["revision"]["id"]["cardId"] = serde_json::json!("custom-imported-duelist");
    original["revision"]["definition"]["id"] = serde_json::json!("custom-imported-duelist");
    let mut different = original.clone();
    different["revision"]["definition"]["cost"] = serde_json::json!(4);
    for (token, document) in [(&first, &original), (&second, &different)] {
        let (status, preview) = transfer_request(
            app.clone(),
            token,
            "POST",
            "/api/card-transfers/preview",
            Some(document.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(preview["status"], "newRevision");
    }
    let (status, _) = transfer_request(
        app.clone(),
        &first,
        "POST",
        "/api/card-transfers/import",
        Some(original.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = transfer_request(
        app.clone(),
        &second,
        "POST",
        "/api/card-transfers/import",
        Some(different),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let uri = "/api/card-transfers/custom-imported-duelist/1";
    let (status, _) = transfer_request(app.clone(), &second, "GET", uri, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, preserved) = transfer_request(app, &first, "GET", uri, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preserved, original);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn system_revision_identities_cannot_be_overwritten_by_imports() {
    let path = test_db_path("card-transfer-system-identity");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let token = register_test_account(app.clone(), "system-transfer@example.com").await;
    let uri = "/api/card-transfers/ember-squire/1";
    let (status, original) = transfer_request(app.clone(), &token, "GET", uri, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, identical) = transfer_request(
        app.clone(),
        &token,
        "POST",
        "/api/card-transfers/import",
        Some(original.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(identical["status"], "alreadyPresent");
    let mut changed = original.clone();
    changed["revision"]["definition"]["cost"] = serde_json::json!(4);
    let (status, _) = transfer_request(
        app.clone(),
        &token,
        "POST",
        "/api/card-transfers/import",
        Some(changed),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let (_, preserved) = transfer_request(app, &token, "GET", uri, None).await;
    assert_eq!(preserved, original);
    let _ = fs::remove_file(path);
}
