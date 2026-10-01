use crate::{
    CardCatalog, CardPackCatalog, CardPackCompatibility, CardPackManifest, CardPackRevisionId,
    CardPackTransfer, PublishedCardRevision, starter_card_definitions,
};

fn manifest(
    id: &str,
    version: u32,
    cards: &[PublishedCardRevision],
    dependencies: Vec<CardPackRevisionId>,
) -> CardPackManifest {
    CardPackManifest {
        schema_version: 1,
        id: CardPackRevisionId::new(id, version).unwrap(),
        compatibility: CardPackCompatibility {
            ruleset_schema_version: crate::rules::RULESET_SCHEMA_VERSION,
        },
        cards: cards.iter().map(|card| card.id().clone()).collect(),
        dependencies,
    }
}

fn fixture() -> (CardCatalog, CardPackCatalog, CardPackRevisionId) {
    let mut definition = starter_card_definitions().remove(0);
    definition.id = "custom-ash-duelist".to_string();
    let older = PublishedCardRevision::new(definition.clone(), 1).unwrap();
    definition.cost = 4;
    let newer = PublishedCardRevision::new(definition, 2).unwrap();
    let companion = PublishedCardRevision::new(starter_card_definitions().remove(1), 1).unwrap();
    let dependency = manifest("kit", 1, std::slice::from_ref(&companion), vec![]);
    let root = manifest(
        "duelists",
        1,
        std::slice::from_ref(&older),
        vec![dependency.id.clone()],
    );
    let later = manifest(
        "duelists",
        2,
        std::slice::from_ref(&newer),
        vec![dependency.id.clone()],
    );
    let cards = CardCatalog::new([older, newer, companion]).unwrap();
    let packs = CardPackCatalog::new([root.clone(), dependency, later], &cards).unwrap();
    (cards, packs, root.id)
}

#[test]
fn exact_pack_round_trip_includes_dependency_closure_and_preserves_destination_history() {
    let (source_cards, source_packs, root) = fixture();
    let transfer = CardPackTransfer::export(&source_packs, &source_cards, &root).unwrap();
    let json = serde_json::to_value(&transfer).unwrap();
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["root"]["version"], 1);
    assert_eq!(json["cards"].as_array().unwrap().len(), 2);
    assert_eq!(json["manifests"].as_array().unwrap().len(), 2);
    assert_eq!(json["cards"][0]["id"]["revision"], 1);
    assert_eq!(json["cards"][0]["definition"]["cost"], 1);
    let restored: CardPackTransfer = serde_json::from_value(json).unwrap();
    let newer = source_cards.latest("custom-ash-duelist").unwrap().clone();
    let unrelated = PublishedCardRevision::new(starter_card_definitions().remove(2), 1).unwrap();
    let destination_cards = CardCatalog::new([newer.clone(), unrelated.clone()]).unwrap();
    let unrelated_pack = manifest("stoneguards", 3, std::slice::from_ref(&unrelated), vec![]);
    let destination_packs =
        CardPackCatalog::new([unrelated_pack.clone()], &destination_cards).unwrap();
    let imported = restored
        .prepare_import(&destination_cards, &destination_packs)
        .into_catalogs()
        .unwrap();
    assert_eq!(imported.cards.len(), 4);
    assert_eq!(imported.cards.latest("custom-ash-duelist").unwrap(), &newer);
    assert_eq!(imported.cards.resolve(unrelated.id()), Some(&unrelated));
    assert_eq!(imported.packs.resolve(&root), source_packs.resolve(&root));
    assert_eq!(
        imported.packs.resolve(&unrelated_pack.id),
        Some(&unrelated_pack)
    );
    assert!(
        imported
            .packs
            .resolve(&CardPackRevisionId::new("kit", 1).unwrap())
            .is_some()
    );
    assert_eq!(destination_cards.len(), 2);
    assert!(destination_packs.resolve(&root).is_none());
    assert_eq!(source_cards.len(), 3);
}

#[test]
fn unsupported_pack_transfer_versions_fail_before_import_preparation() {
    let (cards, packs, root) = fixture();
    let transfer = CardPackTransfer::export(&packs, &cards, &root).unwrap();
    let mut json = serde_json::to_value(transfer).unwrap();
    json["schemaVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<CardPackTransfer>(json).is_err());
}

#[test]
fn identical_complete_pack_import_is_idempotent_and_keeps_every_existing_revision() {
    let (cards, packs, root) = fixture();
    let transfer = CardPackTransfer::export(&packs, &cards, &root).unwrap();
    let prepared = transfer.prepare_import(&cards, &packs);
    assert!(
        prepared
            .preview()
            .cards
            .iter()
            .all(|card| card.status == crate::CardRevisionImportStatus::AlreadyPresent)
    );
    assert!(
        prepared
            .preview()
            .packs
            .iter()
            .all(|pack| pack.status == crate::CardPackRevisionImportStatus::AlreadyPresent)
    );
    let imported = prepared.into_catalogs().unwrap();
    assert_eq!(imported.cards.len(), 3);
    for revision in cards.revisions() {
        assert_eq!(imported.cards.resolve(revision.id()), Some(revision));
    }
    assert_eq!(imported.packs.manifests().count(), 3);
    for manifest in packs.manifests() {
        assert_eq!(imported.packs.resolve(&manifest.id), Some(manifest));
    }
}

#[test]
fn transfer_payload_cannot_install_content_outside_its_root_dependency_closure() {
    let (cards, packs, root) = fixture();
    let transfer = CardPackTransfer::export(&packs, &cards, &root).unwrap();
    let original = serde_json::to_value(transfer).unwrap();
    let mut extra_card = original.clone();
    let unrelated = PublishedCardRevision::new(starter_card_definitions().remove(2), 1).unwrap();
    extra_card["cards"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(unrelated).unwrap());
    assert!(serde_json::from_value::<CardPackTransfer>(extra_card).is_err());
    let mut extra_pack = original;
    extra_pack["manifests"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(manifest("unrelated", 1, &[], vec![])).unwrap());
    assert!(serde_json::from_value::<CardPackTransfer>(extra_pack).is_err());
}

#[test]
fn malformed_incomplete_and_duplicate_documents_fail_before_preparation() {
    let (cards, packs, root) = fixture();
    let original =
        serde_json::to_value(CardPackTransfer::export(&packs, &cards, &root).unwrap()).unwrap();
    for (pointer, value) in [
        ("/root/version", serde_json::json!(3)),
        ("/manifests/0/schemaVersion", serde_json::json!(2)),
        ("/manifests/0/dependencies/0/version", serde_json::json!(2)),
        (
            "/manifests/0/compatibility/rulesetSchemaVersion",
            serde_json::json!(0),
        ),
        ("/cards/0/id/revision", serde_json::json!(0)),
        ("/cards/0/definition/kind/armor", serde_json::json!(-1)),
    ] {
        let mut json = original.clone();
        *json.pointer_mut(pointer).unwrap() = value;
        assert!(
            serde_json::from_value::<CardPackTransfer>(json).is_err(),
            "{pointer}"
        );
    }
    for (field, duplicate) in [
        ("cards", false),
        ("manifests", false),
        ("cards", true),
        ("manifests", true),
    ] {
        let mut json = original.clone();
        let items = json[field].as_array_mut().unwrap();
        if duplicate {
            items.push(items[0].clone());
        } else {
            items.pop();
        }
        assert!(
            serde_json::from_value::<CardPackTransfer>(json).is_err(),
            "{field}"
        );
    }
    for pointer in [
        "",
        "/root",
        "/manifests/0",
        "/manifests/0/compatibility",
        "/cards/0/definition",
    ] {
        let mut json = original.clone();
        json.pointer_mut(pointer).unwrap()["unsupportedContent"] = serde_json::json!(1);
        assert!(
            serde_json::from_value::<CardPackTransfer>(json).is_err(),
            "{pointer}"
        );
    }
    assert!(serde_json::from_value::<CardPackTransfer>(serde_json::json!({})).is_err());
}

#[test]
fn export_requires_exact_root_and_card_references_without_latest_substitution() {
    let (cards, packs, root) = fixture();
    let missing_root = CardPackRevisionId::new("duelists", 3).unwrap();
    assert_eq!(
        CardPackTransfer::export(&packs, &cards, &missing_root).unwrap_err(),
        crate::CardPackTransferError::MissingPack(missing_root)
    );
    let latest_only =
        CardCatalog::new([cards.latest("custom-ash-duelist").unwrap().clone()]).unwrap();
    assert!(
        matches!(CardPackTransfer::export(&packs, &latest_only, &root), Err(crate::CardPackTransferError::MissingCard(id)) if id.card_id() == "custom-ash-duelist" && id.revision() == 1)
    );
}

#[test]
fn preview_reports_card_and_pack_conflicts_and_rejects_the_complete_transfer() {
    let (cards, packs, root) = fixture();
    let mut document =
        serde_json::to_value(CardPackTransfer::export(&packs, &cards, &root).unwrap()).unwrap();
    document["cards"][0]["definition"]["cost"] = serde_json::json!(4);
    let companion_id = document["cards"][1]["id"].clone();
    document["manifests"][0]["cards"]
        .as_array_mut()
        .unwrap()
        .push(companion_id);
    let incoming: CardPackTransfer = serde_json::from_value(document).unwrap();
    let prepared = incoming.prepare_import(&cards, &packs);
    assert_eq!(
        prepared.preview().cards[0].status,
        crate::CardRevisionImportStatus::IdentityConflict
    );
    assert_eq!(
        prepared.preview().packs[0].status,
        crate::CardPackRevisionImportStatus::IdentityConflict
    );
    assert!(
        matches!(prepared.into_catalogs(), Err(crate::CardPackTransferError::CardIdentityConflict(id)) if id.card_id() == "custom-ash-duelist" && id.revision() == 1)
    );
    assert_eq!(cards.len(), 3);
    assert_eq!(cards.revisions().next().unwrap().definition().cost, 1);
    assert_eq!(packs.resolve(&root).unwrap().cards.len(), 1);
}

#[test]
fn pack_identity_conflict_blocks_every_new_card_from_the_transfer() {
    let (source_cards, source_packs, root) = fixture();
    let existing = CardPackTransfer::export(&source_packs, &source_cards, &root).unwrap();
    let newer = source_cards.latest("custom-ash-duelist").unwrap();
    let mut changed = source_packs.resolve(&root).unwrap().clone();
    changed.cards = vec![newer.id().clone()];
    let dependency = source_packs
        .resolve(&CardPackRevisionId::new("kit", 1).unwrap())
        .unwrap()
        .clone();
    let changed_packs = CardPackCatalog::new([changed, dependency], &source_cards).unwrap();
    let incoming = CardPackTransfer::export(&changed_packs, &source_cards, &root).unwrap();
    let prepared = incoming.prepare_import(existing.cards(), existing.packs());
    assert_eq!(
        prepared.preview().cards[0].status,
        crate::CardRevisionImportStatus::NewRevision
    );
    assert_eq!(
        prepared.preview().packs[0].status,
        crate::CardPackRevisionImportStatus::IdentityConflict
    );
    assert_eq!(
        prepared.into_catalogs().unwrap_err(),
        crate::CardPackTransferError::PackIdentityConflict(root.clone())
    );
    assert!(existing.cards().resolve(newer.id()).is_none());
    assert_eq!(existing.packs().resolve(&root), source_packs.resolve(&root));
}

#[test]
fn cyclic_supplied_dependencies_are_exported_once_without_recursive_loading() {
    let cards =
        CardCatalog::new([
            PublishedCardRevision::new(starter_card_definitions().remove(0), 1).unwrap(),
        ])
        .unwrap();
    let first_id = CardPackRevisionId::new("first", 1).unwrap();
    let second_id = CardPackRevisionId::new("second", 1).unwrap();
    let first = manifest(
        "first",
        1,
        &[cards.revisions().next().unwrap().clone()],
        vec![second_id.clone()],
    );
    let second = manifest("second", 1, &[], vec![first_id.clone()]);
    let packs = CardPackCatalog::new([first, second], &cards).unwrap();
    let transfer = CardPackTransfer::export(&packs, &cards, &first_id).unwrap();
    assert_eq!(transfer.packs().manifests().count(), 2);
    assert_eq!(transfer.cards().len(), 1);
    let encoded = serde_json::to_value(&transfer).unwrap();
    let restored: CardPackTransfer = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), encoded);
}

#[test]
fn missing_dependency_diagnostic_identifies_the_exact_required_version() {
    let (cards, packs, root) = fixture();
    let mut document =
        serde_json::to_value(CardPackTransfer::export(&packs, &cards, &root).unwrap()).unwrap();
    document["manifests"][0]["dependencies"][0]["version"] = serde_json::json!(2);
    let error = serde_json::from_value::<CardPackTransfer>(document)
        .unwrap_err()
        .to_string();
    assert!(error.contains("kit@2"), "{error}");
    assert!(error.contains("available versions"), "{error}");
}
