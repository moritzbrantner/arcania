use super::*;
use crate::{CardDefinition, CardKind, PublishedCardRevision, Rarity};

#[test]
fn manifests_round_trip_and_resolve_exact_pack_and_card_versions() {
    let first = revision(1);
    let second = revision(2);
    let cards = CardCatalog::new([first.clone(), second]).unwrap();
    let base = manifest("base", 1, vec![first.id().clone()]);
    let mut addon = manifest("addon", 2, vec![first.id().clone()]);
    addon.dependencies.push(base.id.clone());
    let encoded = serde_json::to_value(&addon).unwrap();
    assert_eq!(encoded["schemaVersion"], 1);
    assert_eq!(
        encoded["id"],
        serde_json::json!({ "packId": "addon", "version": 2 })
    );
    assert_eq!(
        encoded["cards"][0],
        serde_json::json!({ "cardId": "custom-squire", "revision": 1 })
    );
    assert_eq!(encoded["compatibility"]["rulesetSchemaVersion"], 1);
    let decoded: CardPackManifest = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded, addon);
    let packs = CardPackCatalog::new([addon.clone(), base], &cards).unwrap();
    assert_eq!(packs.resolve(&addon.id), Some(&addon));
    assert!(packs.resolve(&pack_id("addon", 1)).is_none());
    assert_eq!(cards.latest("custom-squire").unwrap().id().revision(), 2);
    let pinned = &packs.resolve(&addon.id).unwrap().cards[0];
    assert_eq!(cards.resolve(pinned).unwrap().definition().cost, 1);
}

#[test]
fn unknown_manifest_versions_are_rejected_with_actionable_diagnostics() {
    let cards = CardCatalog::new([revision(1)]).unwrap();
    let mut unsupported = manifest("future", 1, vec![]);
    unsupported.schema_version = 99;
    let errors = CardPackCatalog::new([unsupported], &cards).unwrap_err();
    assert_eq!(
        errors,
        vec![CardPackValidationError::UnknownManifestVersion {
            pack: pack_id("future", 1),
            version: 99,
            supported: 1,
        }]
    );
    let message = errors[0].to_string();
    assert!(message.contains("future@1"));
    assert!(message.contains("schema 99"));
    assert!(message.contains("supported schema is 1"));
}

#[test]
fn incompatible_ruleset_schemas_fail_without_changing_card_catalogs() {
    let first = revision(1);
    let cards = CardCatalog::new([first.clone()]).unwrap();
    let mut incompatible = manifest("future", 1, vec![first.id().clone()]);
    incompatible.compatibility.ruleset_schema_version = RULESET_SCHEMA_VERSION + 1;
    assert_eq!(
        CardPackCatalog::new([incompatible], &cards).unwrap_err(),
        vec![CardPackValidationError::IncompatibleRulesetSchema {
            pack: pack_id("future", 1),
            required: RULESET_SCHEMA_VERSION + 1,
            supported: RULESET_SCHEMA_VERSION,
        },]
    );
    assert_eq!(cards.resolve(first.id()).unwrap().definition().cost, 1);
}

#[test]
fn missing_card_revisions_never_fall_back_to_latest() {
    let missing = revision(1).id().clone();
    let cards = CardCatalog::new([revision(2)]).unwrap();
    assert_eq!(
        CardPackCatalog::new([manifest("base", 1, vec![missing.clone()])], &cards).unwrap_err(),
        vec![CardPackValidationError::MissingCardRevision {
            pack: pack_id("base", 1),
            card: missing
        },]
    );
}

#[test]
fn missing_pack_dependencies_identify_the_requiring_pack_and_exact_dependency() {
    let cards = CardCatalog::new([]).unwrap();
    let mut addon = manifest("addon", 1, vec![]);
    addon.dependencies.push(pack_id("missing", 4));
    let errors = CardPackCatalog::new([addon], &cards).unwrap_err();
    assert_eq!(
        errors,
        vec![CardPackValidationError::MissingDependency {
            pack: pack_id("addon", 1),
            dependency: pack_id("missing", 4),
        }]
    );
    assert!(errors[0].to_string().contains("addon@1"));
    assert!(errors[0].to_string().contains("missing@4"));
}

#[test]
fn wrong_dependency_versions_report_sorted_available_versions_without_substitution() {
    let cards = CardCatalog::new([]).unwrap();
    let mut addon = manifest("addon", 1, vec![]);
    addon.dependencies.push(pack_id("base", 2));
    let packs = [
        addon,
        manifest("base", 3, vec![]),
        manifest("base", 1, vec![]),
    ];
    let errors = CardPackCatalog::new(packs, &cards).unwrap_err();
    assert_eq!(
        errors,
        vec![CardPackValidationError::IncompatibleDependencyVersion {
            pack: pack_id("addon", 1),
            dependency: pack_id("base", 2),
            available_versions: vec![1, 3],
        }]
    );
    assert!(errors[0].to_string().contains("base@2"));
    assert!(errors[0].to_string().contains("[1, 3]"));
}

#[test]
fn a_present_dependency_must_itself_be_compatible_and_have_complete_dependencies() {
    let cards = CardCatalog::new([]).unwrap();
    let mut addon = manifest("addon", 1, vec![]);
    addon.dependencies.push(pack_id("base", 1));
    let mut base = manifest("base", 1, vec![]);
    base.dependencies.push(pack_id("missing", 1));
    base.compatibility.ruleset_schema_version = 9;
    let errors = CardPackCatalog::new([addon.clone(), base.clone()], &cards).unwrap_err();
    assert_eq!(
        errors,
        vec![
            CardPackValidationError::IncompatibleRulesetSchema {
                pack: base.id.clone(),
                required: 9,
                supported: RULESET_SCHEMA_VERSION
            },
            CardPackValidationError::MissingDependency {
                pack: base.id.clone(),
                dependency: pack_id("missing", 1)
            },
        ]
    );
    assert_eq!(
        CardPackCatalog::new([base, addon], &cards).unwrap_err(),
        errors
    );
}

#[test]
fn conflicting_pack_identities_are_rejected_instead_of_overwritten() {
    let cards = CardCatalog::new([]).unwrap();
    let first = manifest("base", 1, vec![]);
    let mut conflicting = first.clone();
    conflicting.dependencies.push(pack_id("other", 1));
    let errors = CardPackCatalog::new([first.clone(), conflicting.clone()], &cards).unwrap_err();
    assert_eq!(
        CardPackCatalog::new([conflicting, first], &cards).unwrap_err(),
        errors
    );
    assert_eq!(
        errors,
        vec![CardPackValidationError::DuplicatePack {
            pack: pack_id("base", 1)
        }]
    );
}

#[test]
fn repeated_card_and_dependency_references_are_rejected() {
    let first = revision(1);
    let cards = CardCatalog::new([first.clone()]).unwrap();
    let base = manifest("base", 1, vec![]);
    let mut addon = manifest("addon", 1, vec![first.id().clone(), first.id().clone()]);
    addon.dependencies = vec![base.id.clone(), base.id.clone()];
    assert_eq!(
        CardPackCatalog::new([addon.clone(), base.clone()], &cards).unwrap_err(),
        vec![
            CardPackValidationError::DuplicateCard {
                pack: addon.id.clone(),
                card: first.id().clone()
            },
            CardPackValidationError::DuplicateDependency {
                pack: addon.id,
                dependency: base.id
            },
        ]
    );
}

#[test]
fn pack_identity_invariants_apply_to_construction_and_deserialization() {
    for invalid in [
        "",
        "Uppercase",
        "-leading",
        "trailing-",
        "double--hyphen",
        "with space",
        "a/b",
    ] {
        assert!(matches!(
            CardPackRevisionId::new(invalid, 1),
            Err(CardPackValidationError::InvalidPackId { .. })
        ));
        assert!(
            serde_json::from_value::<CardPackRevisionId>(
                serde_json::json!({ "packId": invalid, "version": 1 })
            )
            .is_err()
        );
    }
    assert_eq!(
        CardPackRevisionId::new("base", 0).unwrap_err(),
        CardPackValidationError::ZeroPackVersion {
            pack_id: "base".to_string()
        }
    );
    assert!(
        serde_json::from_value::<CardPackRevisionId>(
            serde_json::json!({ "packId": "base", "version": 0 })
        )
        .is_err()
    );
    let id = pack_id("base-v2", 3);
    assert_eq!(id.pack_id(), "base-v2");
    assert_eq!(id.version(), 3);
    assert_eq!(id.to_string(), "base-v2@3");
}

#[test]
fn malformed_manifest_fields_and_card_reference_identities_fail_to_deserialize() {
    let valid = serde_json::to_value(manifest("base", 1, vec![revision(1).id().clone()])).unwrap();
    let mut unknown_field = valid.clone();
    unknown_field["unrecognized"] = serde_json::json!(true);
    assert!(serde_json::from_value::<CardPackManifest>(unknown_field).is_err());
    let mut invalid_card = valid;
    invalid_card["cards"][0]["revision"] = serde_json::json!(0);
    assert!(serde_json::from_value::<CardPackManifest>(invalid_card).is_err());
}

#[test]
fn validation_errors_are_serializable_without_losing_reference_identity() {
    let error = CardPackValidationError::MissingDependency {
        pack: pack_id("addon", 1),
        dependency: pack_id("base", 7),
    };
    let value = serde_json::to_value(&error).unwrap();
    assert_eq!(value["code"], "missingDependency");
    assert_eq!(
        serde_json::from_value::<CardPackValidationError>(value).unwrap(),
        error
    );
}

fn revision(number: u32) -> PublishedCardRevision {
    PublishedCardRevision::new(
        CardDefinition {
            id: "custom-squire".to_string(),
            name: "Custom Squire".to_string(),
            rarity: Rarity::Basic,
            cost: u8::try_from(number).unwrap(),
            text: "A test Unit.".to_string(),
            kind: CardKind::Unit {
                attack: 1,
                armor: 2,
                max_ap: 2,
            },
        },
        number,
    )
    .unwrap()
}

fn pack_id(id: &str, version: u32) -> CardPackRevisionId {
    CardPackRevisionId::new(id, version).unwrap()
}

fn manifest(id: &str, version: u32, cards: Vec<CardRevisionId>) -> CardPackManifest {
    CardPackManifest {
        schema_version: CARD_PACK_MANIFEST_SCHEMA_VERSION,
        id: pack_id(id, version),
        compatibility: CardPackCompatibility {
            ruleset_schema_version: RULESET_SCHEMA_VERSION,
        },
        cards,
        dependencies: vec![],
    }
}
