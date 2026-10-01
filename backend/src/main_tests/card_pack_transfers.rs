use super::card_transfers::{publish_transfer_fixture, transfer_request};
use super::*;

fn pack_document(revision: &serde_json::Value) -> serde_json::Value {
    let companion = rune_lanes_core::PublishedCardRevision::new(
        rune_lanes_core::starter_card_definitions().remove(1),
        1,
    )
    .unwrap();
    serde_json::json!({
        "schemaVersion": 1,
        "root": {"packId": "duelists", "version": 1},
        "cards": [revision, companion],
        "manifests": [
            {
                "schemaVersion": 1, "id": {"packId": "duelists", "version": 1},
                "compatibility": {"rulesetSchemaVersion": 1},
                "cards": [revision["id"]],
                "dependencies": [{"packId": "kit", "version": 1}]
            },
            {
                "schemaVersion": 1, "id": {"packId": "kit", "version": 1},
                "compatibility": {"rulesetSchemaVersion": 1},
                "cards": [companion.id()], "dependencies": []
            }
        ]
    })
}

#[tokio::test]
async fn installed_pack_exports_its_exact_complete_closure_after_restart() {
    let path = test_db_path("pack-transfer-roundtrip");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-owner@example.com").await;
    let (_, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let document = pack_document(&published["revision"]);
    let (status, installed) = transfer_request(
        app,
        &owner,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(installed["packs"][0]["status"], "newRevision");
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    let (status, exported) = transfer_request(
        reopened,
        &owner,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exported, document);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn cross_account_pack_preview_is_read_only_and_installation_is_idempotent() {
    let path = test_db_path("pack-transfer-accounts");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-source@example.com").await;
    let recipient = register_test_account(app.clone(), "pack-recipient@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let document = pack_document(&published["revision"]);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, preview) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/preview",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["cards"][0]["status"], "newRevision");
    assert_eq!(preview["cards"][1]["status"], "alreadyPresent");
    for pack in preview["packs"].as_array().unwrap() {
        assert_eq!(pack["status"], "newRevision");
    }
    for uri in [
        "/api/card-pack-transfers/duelists/1".to_string(),
        format!(
            "/api/card-transfers/{}/1",
            published["revision"]["id"]["cardId"].as_str().unwrap()
        ),
    ] {
        let (status, _) = transfer_request(app.clone(), &recipient, "GET", &uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, installed) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(installed, preview);
    let (status, repeated) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for entry in repeated["cards"]
        .as_array()
        .unwrap()
        .iter()
        .chain(repeated["packs"].as_array().unwrap())
    {
        assert_eq!(entry["status"], "alreadyPresent");
    }
    let (status, _) = transfer_request(
        app.clone(),
        &recipient,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (_, history) = transfer_request(
        app.clone(),
        &recipient,
        "GET",
        "/api/card-revisions/ash-duelist",
        None,
    )
    .await;
    assert_eq!(history["revisions"], serde_json::json!([]));
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    for token in [&owner, &recipient] {
        let (status, exported) = transfer_request(
            reopened.clone(),
            token,
            "GET",
            "/api/card-pack-transfers/duelists/1",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(exported, document);
    }
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn older_pack_and_card_revisions_remain_exact_when_newer_content_is_installed() {
    let path = test_db_path("pack-transfer-history");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-history@example.com").await;
    let (draft, older) = publish_transfer_fixture(app.clone(), &owner).await;
    let old_document = pack_document(&older["revision"]);
    let mut definition = draft["definition"].clone();
    definition["cost"] = serde_json::json!(4);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &format!("/api/card-drafts/{}", draft["id"]),
        Some(serde_json::json!({"version": 1, "definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, newer) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        &format!("/api/card-drafts/{}/publish", draft["id"]),
        Some(serde_json::json!({"version": 2})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut new_document = pack_document(&newer["revision"]);
    new_document["root"]["version"] = serde_json::json!(2);
    new_document["manifests"][0]["id"]["version"] = serde_json::json!(2);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        "/api/card-pack-transfers/import",
        Some(new_document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = transfer_request(
        app,
        &owner,
        "POST",
        "/api/card-pack-transfers/import",
        Some(old_document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    for (version, expected) in [(1, old_document), (2, new_document)] {
        let (status, actual) = transfer_request(
            reopened.clone(),
            &owner,
            "GET",
            &format!("/api/card-pack-transfers/duelists/{version}"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(actual, expected);
    }
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn card_and_manifest_conflicts_reject_every_content_and_access_change() {
    let path = test_db_path("pack-transfer-conflicts");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-conflict-owner@example.com").await;
    let recipient = register_test_account(app.clone(), "pack-conflict-recipient@example.com").await;
    let (_, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let original = pack_document(&published["revision"]);
    let (status, _) = transfer_request(
        app.clone(),
        &owner,
        "POST",
        "/api/card-pack-transfers/import",
        Some(original.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let mut incoming = original.clone();
    let mut extra = incoming["cards"][1].clone();
    extra["id"]["cardId"] = serde_json::json!("custom-extra");
    extra["definition"]["id"] = serde_json::json!("custom-extra");
    incoming["cards"]
        .as_array_mut()
        .unwrap()
        .push(extra.clone());
    incoming["manifests"][0]["cards"]
        .as_array_mut()
        .unwrap()
        .push(extra["id"].clone());
    incoming["cards"][0]["definition"]["taxonomy"]["faction"] = serde_json::json!("different-clan");
    let (status, preview) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/preview",
        Some(incoming.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let cards = preview["cards"].as_array().unwrap();
    assert_eq!(
        cards
            .iter()
            .find(|card| card["id"] == published["revision"]["id"])
            .unwrap()["status"],
        "identityConflict"
    );
    assert_eq!(
        cards.iter().find(|card| card["id"] == extra["id"]).unwrap()["status"],
        "newRevision"
    );
    assert_eq!(preview["packs"][0]["status"], "identityConflict");
    for document in [incoming.clone(), {
        incoming["cards"][0] = original["cards"][0].clone();
        incoming
    }] {
        let (status, _) = transfer_request(
            app.clone(),
            &recipient,
            "POST",
            "/api/card-pack-transfers/import",
            Some(document),
        )
        .await;
        assert_eq!(status, StatusCode::CONFLICT);
    }
    for uri in [
        "/api/card-pack-transfers/duelists/1",
        "/api/card-pack-transfers/kit/1",
        "/api/card-transfers/custom-extra/1",
    ] {
        let (status, _) = transfer_request(app.clone(), &recipient, "GET", uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, source) = transfer_request(
        app,
        &owner,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(source, original);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn pack_transfer_routes_require_auth_and_reject_invalid_complete_documents() {
    let path = test_db_path("pack-transfer-invalid");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-validation@example.com").await;
    let (_, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let document = pack_document(&published["revision"]);
    for (method, uri) in [
        ("GET", "/api/card-pack-transfers/duelists/1"),
        ("POST", "/api/card-pack-transfers/preview"),
        ("POST", "/api/card-pack-transfers/import"),
    ] {
        let (status, _) = transfer_request(
            app.clone(),
            "",
            method,
            uri,
            (method == "POST").then(|| document.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    let mut invalid = Vec::new();
    let mut unsupported = document.clone();
    unsupported["schemaVersion"] = serde_json::json!(2);
    invalid.push(unsupported);
    let mut missing_card = document.clone();
    missing_card["cards"].as_array_mut().unwrap().remove(0);
    invalid.push(missing_card);
    let mut missing_pack = document.clone();
    missing_pack["manifests"].as_array_mut().unwrap().pop();
    invalid.push(missing_pack);
    let mut wrong_dependency = document.clone();
    wrong_dependency["manifests"][0]["dependencies"][0]["version"] = serde_json::json!(2);
    invalid.push(wrong_dependency);
    let mut invalid_mechanics = document.clone();
    invalid_mechanics["cards"][0]["definition"]["kind"]["armor"] = serde_json::json!(-1);
    invalid.push(invalid_mechanics);
    let mut unknown_field = document.clone();
    unknown_field["futureMechanics"] = serde_json::json!(true);
    invalid.push(unknown_field);
    for invalid in invalid {
        for uri in [
            "/api/card-pack-transfers/preview",
            "/api/card-pack-transfers/import",
        ] {
            let (status, _) =
                transfer_request(app.clone(), &owner, "POST", uri, Some(invalid.clone())).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        }
    }
    let (status, _) = transfer_request(
        app,
        &owner,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn stale_pack_previews_recheck_global_card_conflicts_before_any_access_is_granted() {
    let path = test_db_path("pack-transfer-stale");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let first = register_test_account(app.clone(), "pack-first@example.com").await;
    let second = register_test_account(app.clone(), "pack-second@example.com").await;
    let revision = rune_lanes_core::PublishedCardRevision::new(
        {
            let mut definition = rune_lanes_core::starter_card_definitions().remove(0);
            definition.id = "custom-contested".to_string();
            definition
        },
        2,
    )
    .unwrap();
    let first_document = pack_document(&serde_json::to_value(revision).unwrap());
    let mut second_document = first_document.clone();
    second_document["root"]["packId"] = serde_json::json!("contenders");
    second_document["manifests"][0]["id"]["packId"] = serde_json::json!("contenders");
    second_document["cards"][0]["definition"]["cost"] = serde_json::json!(5);
    for (token, document) in [(&first, &first_document), (&second, &second_document)] {
        let (status, preview) = transfer_request(
            app.clone(),
            token,
            "POST",
            "/api/card-pack-transfers/preview",
            Some(document.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(preview["cards"][0]["status"], "newRevision");
    }
    let (status, _) = transfer_request(
        app.clone(),
        &first,
        "POST",
        "/api/card-pack-transfers/import",
        Some(first_document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = transfer_request(
        app.clone(),
        &second,
        "POST",
        "/api/card-pack-transfers/import",
        Some(second_document),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    for uri in [
        "/api/card-transfers/custom-contested/2",
        "/api/card-pack-transfers/contenders/1",
        "/api/card-pack-transfers/kit/1",
    ] {
        let (status, _) = transfer_request(app.clone(), &second, "GET", uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    let (status, unchanged) = transfer_request(
        app,
        &first,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(unchanged, first_document);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn manifest_storage_failure_rolls_back_earlier_card_and_pack_installations() {
    let path = test_db_path("pack-transfer-rollback");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-rollback-source@example.com").await;
    let recipient = register_test_account(app.clone(), "pack-rollback-target@example.com").await;
    let (_, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let document = pack_document(&published["revision"]);
    // Fail the final manifest write; observe rollback through the HTTP facade.
    let database = rusqlite::Connection::open(&path).unwrap();
    database.execute_batch("CREATE TRIGGER reject_kit_install BEFORE INSERT ON card_pack_imports WHEN NEW.pack_id = 'kit' BEGIN SELECT RAISE(ABORT, 'injected storage failure'); END;").unwrap();
    let (status, _) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    for uri in [
        "/api/card-pack-transfers/duelists/1".to_string(),
        "/api/card-pack-transfers/kit/1".to_string(),
        format!(
            "/api/card-transfers/{}/1",
            published["revision"]["id"]["cardId"].as_str().unwrap()
        ),
    ] {
        let (status, _) = transfer_request(app.clone(), &recipient, "GET", &uri, None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
    database
        .execute_batch("DROP TRIGGER reject_kit_install;")
        .unwrap();
    let (status, preview) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["cards"][0]["status"], "newRevision");
    for pack in preview["packs"].as_array().unwrap() {
        assert_eq!(pack["status"], "newRevision");
    }
    let (status, actual) = transfer_request(
        app,
        &recipient,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(actual, document);
    drop(database);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn pack_installed_cards_obey_single_card_import_and_publication_identity_rules() {
    let path = test_db_path("pack-transfer-publication");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "pack-publisher@example.com").await;
    let recipient = register_test_account(app.clone(), "pack-importer@example.com").await;
    let (draft, published) = publish_transfer_fixture(app.clone(), &owner).await;
    let mut future = published["revision"].clone();
    future["id"]["revision"] = serde_json::json!(2);
    future["definition"]["cost"] = serde_json::json!(4);
    let document = pack_document(&future);
    let (status, _) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-pack-transfers/import",
        Some(document.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, repeated) = transfer_request(
        app.clone(),
        &recipient,
        "POST",
        "/api/card-transfers/import",
        Some(serde_json::json!({"schemaVersion": 1, "revision": future})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated["status"], "alreadyPresent");
    for (version, cost, expected_status) in [(1, 5, StatusCode::CONFLICT), (2, 4, StatusCode::OK)] {
        let mut definition = draft["definition"].clone();
        definition["cost"] = serde_json::json!(cost);
        let (status, _) = transfer_request(
            app.clone(),
            &owner,
            "PATCH",
            &format!("/api/card-drafts/{}", draft["id"]),
            Some(serde_json::json!({"version": version, "definition": definition})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = transfer_request(
            app.clone(),
            &owner,
            "POST",
            &format!("/api/card-drafts/{}/publish", draft["id"]),
            Some(serde_json::json!({"version": version + 1})),
        )
        .await;
        assert_eq!(status, expected_status);
    }
    let (status, actual) = transfer_request(
        app,
        &recipient,
        "GET",
        "/api/card-pack-transfers/duelists/1",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(actual, document);
    let _ = fs::remove_file(path);
}
