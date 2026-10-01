use crate::{
    CardCatalog, CardDefinition, CardRevisionImportStatus, CardRevisionTransfer, CardTaxonomy,
    CardTransferError, PublishedCardRevision, starter_card_catalog, starter_card_definitions,
};

fn custom_definition() -> CardDefinition {
    let mut definition = starter_card_definitions().remove(0);
    definition.id = "custom-ash-duelist".to_string();
    definition.name = "Ash Duelist".to_string();
    definition.taxonomy = CardTaxonomy {
        faction: Some("ember-clan".to_string()),
        traits: vec!["duelist".to_string()],
        ..CardTaxonomy::default()
    };
    definition
}

#[test]
fn exact_revision_round_trip_preserves_definition_and_destination_history() {
    let older = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let mut newer_definition = custom_definition();
    newer_definition.cost = 4;
    let newer = PublishedCardRevision::new(newer_definition, 2).unwrap();
    let unrelated = PublishedCardRevision::new(starter_card_definitions().remove(1), 1).unwrap();
    let source = CardCatalog::new([older.clone(), newer.clone()]).unwrap();
    let destination = CardCatalog::new([newer.clone(), unrelated.clone()]).unwrap();

    let exported = CardRevisionTransfer::export(&source, older.id()).unwrap();
    let json = serde_json::to_value(&exported).unwrap();
    assert_eq!(json["schemaVersion"], 1);
    assert_eq!(json["revision"]["id"]["revision"], 1);
    assert_eq!(json["revision"]["definition"]["cost"], 1);
    assert_eq!(
        json["revision"]["definition"]["taxonomy"]["faction"],
        "ember-clan"
    );
    let imported: CardRevisionTransfer = serde_json::from_value(json).unwrap();
    let prepared = imported.prepare_import(&destination);
    assert_eq!(prepared.status(), CardRevisionImportStatus::NewRevision);
    assert!(destination.resolve(older.id()).is_none());

    let result = prepared.into_catalog().unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(
        result.resolve(older.id()).unwrap().definition(),
        older.definition()
    );
    assert_eq!(
        result.resolve(unrelated.id()).unwrap().definition(),
        unrelated.definition()
    );
    assert_eq!(
        result.latest("custom-ash-duelist").unwrap().id(),
        newer.id()
    );
    assert_eq!(
        result.resolve(newer.id()).unwrap().definition(),
        newer.definition()
    );
    assert_eq!(source.len(), 2);
    assert_eq!(destination.len(), 2);
    assert!(destination.resolve(older.id()).is_none());
}

#[test]
fn unsupported_transfer_version_is_rejected_before_preparation() {
    let revision = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let source = CardCatalog::new([revision.clone()]).unwrap();
    let transfer = CardRevisionTransfer::export(&source, revision.id()).unwrap();
    let mut json = serde_json::to_value(transfer).unwrap();
    json["schemaVersion"] = serde_json::json!(2);
    assert!(serde_json::from_value::<CardRevisionTransfer>(json).is_err());
}

#[test]
fn identical_import_is_previewed_and_applied_idempotently() {
    let revision = PublishedCardRevision::new(custom_definition(), 3).unwrap();
    let catalog = CardCatalog::new([revision.clone()]).unwrap();
    let transfer = CardRevisionTransfer::export(&catalog, revision.id()).unwrap();
    let prepared = transfer.prepare_import(&catalog);
    assert_eq!(
        serde_json::to_value(prepared.status()).unwrap(),
        "alreadyPresent"
    );
    let result = prepared.into_catalog().unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result.resolve(revision.id()), Some(&revision));
    assert_eq!(catalog.resolve(revision.id()), Some(&revision));
}

#[test]
fn differing_content_at_the_same_exact_identity_is_previewed_without_overwriting() {
    let existing = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let mut changed = custom_definition();
    changed.cost = 4;
    let incoming = PublishedCardRevision::new(changed, 1).unwrap();
    let source = CardCatalog::new([incoming.clone()]).unwrap();
    let destination = CardCatalog::new([existing.clone()]).unwrap();
    let transfer = CardRevisionTransfer::export(&source, incoming.id()).unwrap();
    let prepared = transfer.prepare_import(&destination);
    assert_eq!(
        serde_json::to_value(prepared.status()).unwrap(),
        "identityConflict"
    );
    assert_eq!(
        prepared.into_catalog().unwrap_err(),
        CardTransferError::IdentityConflict(existing.id().clone())
    );
    assert_eq!(destination.len(), 1);
    assert_eq!(destination.resolve(existing.id()), Some(&existing));
    assert_eq!(source.resolve(incoming.id()), Some(&incoming));
}

#[test]
fn metadata_only_changes_conflict_with_the_existing_immutable_revision() {
    let existing = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let destination = CardCatalog::new([existing.clone()]).unwrap();
    let mut taxonomy_change = custom_definition();
    taxonomy_change.taxonomy.faction = Some("stone-clan".to_string());
    let mut text_change = custom_definition();
    text_change.text = "A different published description.".to_string();
    for definition in [taxonomy_change, text_change] {
        let incoming = PublishedCardRevision::new(definition, 1).unwrap();
        let source = CardCatalog::new([incoming.clone()]).unwrap();
        let transfer = CardRevisionTransfer::export(&source, incoming.id()).unwrap();
        let prepared = transfer.prepare_import(&destination);
        assert_eq!(
            prepared.status(),
            CardRevisionImportStatus::IdentityConflict
        );
        assert_eq!(
            prepared.into_catalog().unwrap_err(),
            CardTransferError::IdentityConflict(existing.id().clone())
        );
    }
    assert_eq!(destination.resolve(existing.id()), Some(&existing));
}

#[test]
fn unknown_transfer_content_is_rejected_instead_of_silently_discarded() {
    let revision = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let source = CardCatalog::new([revision.clone()]).unwrap();
    let transfer = CardRevisionTransfer::export(&source, revision.id()).unwrap();
    let original = serde_json::to_value(transfer).unwrap();
    for pointer in [
        "",
        "/revision",
        "/revision/id",
        "/revision/definition",
        "/revision/definition/kind",
    ] {
        let mut json = original.clone();
        json.pointer_mut(pointer).unwrap()["futureMechanic"] = serde_json::json!(1);
        assert!(
            serde_json::from_value::<CardRevisionTransfer>(json).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn malformed_transfer_shapes_and_invalid_published_content_fail_before_preparation() {
    let revision = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let catalog = CardCatalog::new([revision.clone()]).unwrap();
    let transfer = CardRevisionTransfer::export(&catalog, revision.id()).unwrap();
    let original = serde_json::to_value(transfer).unwrap();
    for (pointer, value) in [
        ("/schemaVersion", serde_json::json!("1")),
        ("/revision/id/revision", serde_json::json!(0)),
        ("/revision/id/cardId", serde_json::json!("another-card")),
        ("/revision/definition/name", serde_json::json!(" ")),
        ("/revision/definition/kind/armor", serde_json::json!(-1)),
        (
            "/revision/definition/kind/type",
            serde_json::json!("unknown"),
        ),
    ] {
        let mut json = original.clone();
        *json.pointer_mut(pointer).unwrap() = value;
        assert!(
            serde_json::from_value::<CardRevisionTransfer>(json).is_err(),
            "{pointer}"
        );
    }
    for json in [
        serde_json::json!([]),
        serde_json::json!({}),
        serde_json::json!({"schemaVersion":1}),
    ] {
        assert!(serde_json::from_value::<CardRevisionTransfer>(json).is_err());
    }
    assert_eq!(catalog.resolve(revision.id()), Some(&revision));
}

#[test]
fn missing_exact_export_revision_never_substitutes_the_latest_revision() {
    let missing = PublishedCardRevision::new(custom_definition(), 1).unwrap();
    let available = PublishedCardRevision::new(custom_definition(), 2).unwrap();
    let catalog = CardCatalog::new([available]).unwrap();
    assert!(matches!(
        CardRevisionTransfer::export(&catalog, missing.id()),
        Err(CardTransferError::MissingRevision(id)) if &id == missing.id()
    ));
}

#[test]
fn existing_card_variants_round_trip_and_unknown_nested_effect_fields_fail() {
    let source = starter_card_catalog();
    let empty = CardCatalog::new([]).unwrap();
    for revision in source.revisions() {
        let transfer = CardRevisionTransfer::export(source, revision.id()).unwrap();
        let json = serde_json::to_value(&transfer).unwrap();
        let imported: CardRevisionTransfer = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(imported.revision(), revision);
        let catalog = imported.prepare_import(&empty).into_catalog().unwrap();
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.resolve(revision.id()), Some(revision));
        for field in ["effect", "passive", "active"] {
            let mut unknown = json.clone();
            if let Some(effect) = unknown["revision"]["definition"]["kind"][field].as_object_mut() {
                effect.insert("futureMechanic".to_string(), serde_json::json!(1));
                assert!(serde_json::from_value::<CardRevisionTransfer>(unknown).is_err());
            }
        }
    }
    assert!(empty.is_empty());
}

#[test]
fn historical_item_defaults_and_stat_aliases_remain_compatible() {
    let source = starter_card_catalog();
    let revision = source.latest("rune-charm").unwrap();
    let transfer = CardRevisionTransfer::export(source, revision.id()).unwrap();
    let mut json = serde_json::to_value(transfer).unwrap();
    let kind = json["revision"]["definition"]["kind"]
        .as_object_mut()
        .unwrap();
    kind.remove("targets");
    let passive = kind["passive"].as_object_mut().unwrap();
    let max_ap = passive.remove("maxAp").unwrap();
    passive.insert("max_ap".to_string(), max_ap);
    let imported: CardRevisionTransfer = serde_json::from_value(json).unwrap();
    assert_eq!(imported.revision(), revision);
}
