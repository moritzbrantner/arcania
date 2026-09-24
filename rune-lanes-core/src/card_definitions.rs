use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::match_session::{
    BuffTargetPolicy, BuildingEffect, Card, CardKind, ItemActiveEffect, ItemPassiveEffect, Rarity,
    SpellEffect,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardDefinition {
    pub id: String,
    pub name: String,
    pub rarity: Rarity,
    pub cost: u8,
    pub text: String,
    pub kind: CardKind,
}

impl CardDefinition {
    pub fn instantiate(&self, instance_id: impl Into<String>) -> Card {
        Card {
            id: instance_id.into(),
            template_id: self.id.clone(),
            name: self.name.clone(),
            rarity: self.rarity,
            cost: self.cost,
            text: self.text.clone(),
            kind: self.kind.clone(),
        }
    }

    pub fn validation_errors(&self) -> Vec<CardDefinitionValidationError> {
        let mut errors = Vec::new();

        if !valid_card_id(&self.id) {
            errors.push(CardDefinitionValidationError::InvalidId {
                value: self.id.clone(),
            });
        }
        if self.name.trim().is_empty() {
            errors.push(CardDefinitionValidationError::BlankName);
        }

        match &self.kind {
            CardKind::Unit {
                attack,
                armor,
                max_ap,
            } => {
                if *attack < 0 {
                    errors.push(CardDefinitionValidationError::NegativeValue {
                        field: "kind.attack".to_string(),
                        value: *attack,
                    });
                }
                if *armor <= 0 {
                    errors.push(CardDefinitionValidationError::NonPositiveValue {
                        field: "kind.armor".to_string(),
                        value: *armor,
                    });
                }
                if *max_ap == 0 {
                    errors.push(CardDefinitionValidationError::ZeroValue {
                        field: "kind.maxAp".to_string(),
                    });
                }
            }
            CardKind::Spell { range, effect, .. } => {
                validate_spell_effect(*range, effect, &mut errors);
            }
            CardKind::Item {
                range,
                targets,
                passive,
                active,
            } => {
                if *range == 0 && matches!(targets, BuffTargetPolicy::UnitsOnly) {
                    errors.push(CardDefinitionValidationError::ZeroValue {
                        field: "kind.range".to_string(),
                    });
                }
                let passive_is_noop = item_passive_is_noop(passive);
                if passive_is_noop && active.is_none() {
                    errors.push(CardDefinitionValidationError::EmptyStatChange {
                        field: "kind.passive".to_string(),
                    });
                }
                if let Some(active) = active {
                    validate_item_active_effect(active, &mut errors);
                }
            }
            CardKind::Building { effect } => validate_building_effect(effect, &mut errors),
            CardKind::ManaSource => {}
        }

        errors
    }

    pub fn validate(&self) -> Result<(), Vec<CardDefinitionValidationError>> {
        let errors = self.validation_errors();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl From<Card> for CardDefinition {
    fn from(card: Card) -> Self {
        Self {
            id: card.template_id,
            name: card.name,
            rarity: card.rarity,
            cost: card.cost,
            text: card.text,
            kind: card.kind,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardRevisionId {
    card_id: String,
    revision: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UncheckedCardRevisionId {
    card_id: String,
    revision: u32,
}

impl<'de> Deserialize<'de> for CardRevisionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let unchecked = UncheckedCardRevisionId::deserialize(deserializer)?;
        if unchecked.revision == 0 {
            return Err(serde::de::Error::custom("card revisions start at 1"));
        }
        if !valid_card_id(&unchecked.card_id) {
            return Err(serde::de::Error::custom(
                "card revision has an invalid card id",
            ));
        }

        Ok(Self {
            card_id: unchecked.card_id,
            revision: unchecked.revision,
        })
    }
}

impl CardRevisionId {
    pub fn card_id(&self) -> &str {
        &self.card_id
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }
}

impl fmt::Display for CardRevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.card_id, self.revision)
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedCardRevision {
    id: CardRevisionId,
    definition: CardDefinition,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UncheckedPublishedCardRevision {
    id: CardRevisionId,
    definition: CardDefinition,
}

impl<'de> Deserialize<'de> for PublishedCardRevision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let unchecked = UncheckedPublishedCardRevision::deserialize(deserializer)?;
        let published = Self {
            id: unchecked.id,
            definition: unchecked.definition,
        };
        published
            .validate()
            .map_err(|error| serde::de::Error::custom(error.to_string()))?;
        Ok(published)
    }
}

impl PublishedCardRevision {
    pub fn new(
        definition: CardDefinition,
        revision: u32,
    ) -> Result<Self, PublishedCardRevisionError> {
        let id = CardRevisionId {
            card_id: definition.id.clone(),
            revision,
        };
        let published = Self { id, definition };
        published.validate()?;
        Ok(published)
    }

    pub fn id(&self) -> &CardRevisionId {
        &self.id
    }

    pub fn definition(&self) -> &CardDefinition {
        &self.definition
    }

    pub fn validate(&self) -> Result<(), PublishedCardRevisionError> {
        if self.id.revision == 0 {
            return Err(PublishedCardRevisionError::ZeroRevision);
        }
        if self.id.card_id != self.definition.id {
            return Err(PublishedCardRevisionError::CardIdMismatch {
                revision_card_id: self.id.card_id.clone(),
                definition_card_id: self.definition.id.clone(),
            });
        }

        let errors = self.definition.validation_errors();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(PublishedCardRevisionError::InvalidDefinition {
                card_id: self.definition.id.clone(),
                errors,
            })
        }
    }
}

#[derive(Clone, Debug)]
pub struct CardCatalog {
    revisions: BTreeMap<CardRevisionId, PublishedCardRevision>,
    latest: BTreeMap<String, CardRevisionId>,
}

impl CardCatalog {
    pub fn new(
        revisions: impl IntoIterator<Item = PublishedCardRevision>,
    ) -> Result<Self, CardCatalogError> {
        let mut catalog = Self {
            revisions: BTreeMap::new(),
            latest: BTreeMap::new(),
        };

        for revision in revisions {
            revision
                .validate()
                .map_err(CardCatalogError::InvalidRevision)?;
            let id = revision.id().clone();
            if catalog.revisions.contains_key(&id) {
                return Err(CardCatalogError::DuplicateRevision(id));
            }

            let latest = catalog
                .latest
                .entry(id.card_id.clone())
                .or_insert_with(|| id.clone());
            if id.revision > latest.revision {
                *latest = id.clone();
            }
            catalog.revisions.insert(id, revision);
        }

        Ok(catalog)
    }

    pub fn resolve(&self, id: &CardRevisionId) -> Option<&PublishedCardRevision> {
        self.revisions.get(id)
    }

    pub fn latest(&self, card_id: &str) -> Option<&PublishedCardRevision> {
        self.latest
            .get(card_id)
            .and_then(|revision_id| self.revisions.get(revision_id))
    }

    pub fn instantiate_latest(
        &self,
        card_id: &str,
        instance_id: impl Into<String>,
    ) -> Option<Card> {
        self.latest(card_id)
            .map(|revision| revision.definition().instantiate(instance_id))
    }

    pub fn len(&self) -> usize {
        self.revisions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.revisions.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CardDefinitionValidationError {
    InvalidId { value: String },
    BlankName,
    NegativeValue { field: String, value: i32 },
    NonPositiveValue { field: String, value: i32 },
    ZeroValue { field: String },
    EmptyStatChange { field: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PublishedCardRevisionError {
    ZeroRevision,
    CardIdMismatch {
        revision_card_id: String,
        definition_card_id: String,
    },
    InvalidDefinition {
        card_id: String,
        errors: Vec<CardDefinitionValidationError>,
    },
}

impl fmt::Display for PublishedCardRevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroRevision => formatter.write_str("card revisions start at 1"),
            Self::CardIdMismatch {
                revision_card_id,
                definition_card_id,
            } => write!(
                formatter,
                "revision card id {revision_card_id} does not match definition id {definition_card_id}"
            ),
            Self::InvalidDefinition { card_id, errors } => {
                write!(
                    formatter,
                    "card {card_id} has {} validation error(s)",
                    errors.len()
                )
            }
        }
    }
}

impl std::error::Error for PublishedCardRevisionError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CardCatalogError {
    InvalidRevision(PublishedCardRevisionError),
    DuplicateRevision(CardRevisionId),
}

impl fmt::Display for CardCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRevision(error) => error.fmt(formatter),
            Self::DuplicateRevision(id) => write!(formatter, "duplicate card revision {id}"),
        }
    }
}

impl std::error::Error for CardCatalogError {}

fn valid_card_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_spell_effect(
    range: u8,
    effect: &SpellEffect,
    errors: &mut Vec<CardDefinitionValidationError>,
) {
    if spell_requires_positive_range(effect) {
        require_positive_u8(errors, "kind.range", range);
    }

    match effect {
        SpellEffect::Heal { amount } | SpellEffect::Damage { amount } => {
            require_positive_i32(errors, "kind.effect.amount", *amount);
        }
        SpellEffect::Buff { attack, armor } => {
            require_nonzero_stats(errors, "kind.effect", *attack, *armor, 0);
        }
        SpellEffect::StatBuff {
            attack,
            armor,
            max_ap,
            ..
        } => {
            require_nonzero_stats(errors, "kind.effect", *attack, *armor, *max_ap);
        }
        SpellEffect::Draw { amount } => {
            require_positive_u8(errors, "kind.effect.amount", *amount);
        }
        SpellEffect::AreaDamage { amount, radius } => {
            require_positive_i32(errors, "kind.effect.amount", *amount);
            require_positive_u8(errors, "kind.effect.radius", *radius);
        }
        SpellEffect::LineDamage { amount } => {
            require_positive_i32(errors, "kind.effect.amount", *amount);
        }
    }
}

fn spell_requires_positive_range(effect: &SpellEffect) -> bool {
    match effect {
        SpellEffect::Damage { .. }
        | SpellEffect::AreaDamage { .. }
        | SpellEffect::LineDamage { .. }
        | SpellEffect::Buff { .. } => true,
        SpellEffect::StatBuff {
            targets: BuffTargetPolicy::UnitsOnly,
            ..
        } => true,
        SpellEffect::Heal { .. } | SpellEffect::Draw { .. } | SpellEffect::StatBuff { .. } => false,
    }
}

fn item_passive_is_noop(passive: &ItemPassiveEffect) -> bool {
    match passive {
        ItemPassiveEffect::StatBonus {
            attack,
            armor,
            max_ap,
        } => *attack == 0 && *armor == 0 && *max_ap == 0,
    }
}

fn validate_item_active_effect(
    active: &ItemActiveEffect,
    errors: &mut Vec<CardDefinitionValidationError>,
) {
    match active {
        ItemActiveEffect::HealCarrier { amount, .. } => {
            require_positive_i32(errors, "kind.active.amount", *amount);
        }
        ItemActiveEffect::DamageTarget { amount, range, .. } => {
            require_positive_i32(errors, "kind.active.amount", *amount);
            require_positive_u8(errors, "kind.active.range", *range);
        }
        ItemActiveEffect::Draw { amount, .. } => {
            require_positive_u8(errors, "kind.active.amount", *amount);
        }
        ItemActiveEffect::StatMarker {
            attack,
            armor,
            max_ap,
            ..
        } => {
            require_nonzero_stats(errors, "kind.active", *attack, *armor, *max_ap);
        }
    }
}

fn validate_building_effect(
    effect: &BuildingEffect,
    errors: &mut Vec<CardDefinitionValidationError>,
) {
    match effect {
        BuildingEffect::TurnStartMana { amount } => {
            require_positive_u8(errors, "kind.effect.amount", *amount);
        }
        BuildingEffect::AuraStatBonus {
            attack,
            armor,
            max_ap,
            ..
        }
        | BuildingEffect::ActivatedStatBonus {
            attack,
            armor,
            max_ap,
            ..
        } => {
            require_nonzero_stats(errors, "kind.effect", *attack, *armor, *max_ap);
        }
        BuildingEffect::ActivatedDamageLine { range, amount } => {
            require_positive_u8(errors, "kind.effect.range", *range);
            require_positive_i32(errors, "kind.effect.amount", *amount);
        }
        BuildingEffect::ActivatedHeal { amount, .. } => {
            require_positive_i32(errors, "kind.effect.amount", *amount);
        }
    }
}

fn require_positive_i32(errors: &mut Vec<CardDefinitionValidationError>, field: &str, value: i32) {
    if value <= 0 {
        errors.push(CardDefinitionValidationError::NonPositiveValue {
            field: field.to_string(),
            value,
        });
    }
}

fn require_positive_u8(errors: &mut Vec<CardDefinitionValidationError>, field: &str, value: u8) {
    if value == 0 {
        errors.push(CardDefinitionValidationError::ZeroValue {
            field: field.to_string(),
        });
    }
}

fn require_nonzero_stats(
    errors: &mut Vec<CardDefinitionValidationError>,
    field: &str,
    attack: i32,
    armor: i32,
    max_ap: i8,
) {
    if attack == 0 && armor == 0 && max_ap == 0 {
        errors.push(CardDefinitionValidationError::EmptyStatChange {
            field: field.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_definition() -> CardDefinition {
        CardDefinition {
            id: "ash-duelist".to_string(),
            name: "Ash Duelist".to_string(),
            rarity: Rarity::Advanced,
            cost: 2,
            text: "2 attack / 3 armor / 2 AP.".to_string(),
            kind: CardKind::Unit {
                attack: 2,
                armor: 3,
                max_ap: 2,
            },
        }
    }

    #[test]
    fn card_definition_instantiates_match_card_without_losing_stable_identity() {
        let definition = unit_definition();
        let card = definition.instantiate("player-17-ash-duelist");

        assert_eq!(card.id, "player-17-ash-duelist");
        assert_eq!(card.template_id, "ash-duelist");
        assert_eq!(card.name, "Ash Duelist");
        assert_eq!(card.cost, 2);
    }

    #[test]
    fn validation_reports_multiple_editor_friendly_errors() {
        let definition = CardDefinition {
            id: " Bad ID ".to_string(),
            name: "   ".to_string(),
            rarity: Rarity::Basic,
            cost: 0,
            text: String::new(),
            kind: CardKind::Unit {
                attack: -1,
                armor: 0,
                max_ap: 0,
            },
        };

        assert_eq!(
            definition.validation_errors(),
            vec![
                CardDefinitionValidationError::InvalidId {
                    value: " Bad ID ".to_string(),
                },
                CardDefinitionValidationError::BlankName,
                CardDefinitionValidationError::NegativeValue {
                    field: "kind.attack".to_string(),
                    value: -1,
                },
                CardDefinitionValidationError::NonPositiveValue {
                    field: "kind.armor".to_string(),
                    value: 0,
                },
                CardDefinitionValidationError::ZeroValue {
                    field: "kind.maxAp".to_string(),
                },
            ]
        );
    }

    #[test]
    fn validation_rejects_noop_and_unbounded_effect_parameters() {
        let mut definition = unit_definition();
        definition.kind = CardKind::Spell {
            range: 3,
            priority: 2,
            effect: SpellEffect::AreaDamage {
                amount: 0,
                radius: 0,
            },
        };
        assert_eq!(definition.validation_errors().len(), 2);

        definition.kind = CardKind::Item {
            range: 2,
            targets: BuffTargetPolicy::UnitsAndHeroes,
            passive: ItemPassiveEffect::StatBonus {
                attack: 0,
                armor: 0,
                max_ap: 0,
            },
            active: None,
        };
        assert_eq!(
            definition.validation_errors(),
            vec![CardDefinitionValidationError::EmptyStatChange {
                field: "kind.passive".to_string(),
            }]
        );
    }

    #[test]
    fn validation_rejects_zero_range_for_unit_only_items() {
        let mut definition = unit_definition();
        definition.kind = CardKind::Item {
            range: 0,
            targets: BuffTargetPolicy::UnitsOnly,
            passive: ItemPassiveEffect::StatBonus {
                attack: 1,
                armor: 0,
                max_ap: 0,
            },
            active: None,
        };

        assert!(definition.validation_errors().iter().any(|error| {
            error
                == &CardDefinitionValidationError::ZeroValue {
                    field: "kind.range".to_string(),
                }
        }));

        definition.kind = CardKind::Item {
            range: 0,
            targets: BuffTargetPolicy::UnitsAndHeroes,
            passive: ItemPassiveEffect::StatBonus {
                attack: 1,
                armor: 0,
                max_ap: 0,
            },
            active: None,
        };
        assert!(
            !definition.validation_errors().iter().any(|error| {
                error
                    == &CardDefinitionValidationError::ZeroValue {
                        field: "kind.range".to_string(),
                    }
            }),
            "hero-targetable items may target the caster at range zero"
        );
    }

    #[test]
    fn validation_rejects_zero_range_for_spells_that_need_another_piece() {
        let effects = [
            SpellEffect::Damage { amount: 1 },
            SpellEffect::AreaDamage {
                amount: 1,
                radius: 1,
            },
            SpellEffect::LineDamage { amount: 1 },
            SpellEffect::Buff {
                attack: 1,
                armor: 0,
            },
            SpellEffect::StatBuff {
                attack: 1,
                armor: 0,
                max_ap: 0,
                targets: BuffTargetPolicy::UnitsOnly,
            },
        ];

        for effect in effects {
            let mut definition = unit_definition();
            definition.kind = CardKind::Spell {
                range: 0,
                priority: 2,
                effect,
            };

            assert!(
                definition.validation_errors().iter().any(|error| {
                    error
                        == &CardDefinitionValidationError::ZeroValue {
                            field: "kind.range".to_string(),
                        }
                }),
                "{} should require positive range",
                definition.id
            );
        }
    }

    #[test]
    fn validation_allows_zero_range_for_self_targetable_spells() {
        let effects = [
            SpellEffect::Heal { amount: 1 },
            SpellEffect::Draw { amount: 1 },
            SpellEffect::StatBuff {
                attack: 1,
                armor: 0,
                max_ap: 0,
                targets: BuffTargetPolicy::HeroesOnly,
            },
            SpellEffect::StatBuff {
                attack: 1,
                armor: 0,
                max_ap: 0,
                targets: BuffTargetPolicy::UnitsAndHeroes,
            },
        ];

        for effect in effects {
            let mut definition = unit_definition();
            definition.kind = CardKind::Spell {
                range: 0,
                priority: 2,
                effect,
            };

            assert!(
                !definition.validation_errors().iter().any(|error| {
                    error
                        == &CardDefinitionValidationError::ZeroValue {
                            field: "kind.range".to_string(),
                        }
                }),
                "{} should allow self-targeting at range zero",
                definition.id
            );
        }
    }

    #[test]
    fn revision_id_deserialization_rejects_invalid_identity() {
        let zero_revision = serde_json::json!({
            "cardId": "ash-duelist",
            "revision": 0
        });
        assert!(
            serde_json::from_value::<CardRevisionId>(zero_revision)
                .expect_err("zero revision id must fail")
                .to_string()
                .contains("start at 1")
        );

        let invalid_card_id = serde_json::json!({
            "cardId": "Ash Duelist",
            "revision": 1
        });
        assert!(
            serde_json::from_value::<CardRevisionId>(invalid_card_id)
                .expect_err("invalid card id must fail")
                .to_string()
                .contains("invalid card id")
        );
    }

    #[test]
    fn published_revision_identity_is_stable_and_serializable() {
        let revision = PublishedCardRevision::new(unit_definition(), 3).expect("valid revision");
        let json = serde_json::to_string(revision.id()).expect("revision id should serialize");
        let restored: CardRevisionId =
            serde_json::from_str(&json).expect("revision id should deserialize");

        assert_eq!(revision.id(), &restored);
        assert_eq!(revision.id().to_string(), "ash-duelist@3");
    }

    #[test]
    fn published_revision_rejects_zero_revision() {
        let error = PublishedCardRevision::new(unit_definition(), 0)
            .expect_err("published revisions must start at one");

        assert_eq!(error, PublishedCardRevisionError::ZeroRevision);
    }

    #[test]
    fn published_revision_deserialization_preserves_invariants() {
        let revision = PublishedCardRevision::new(unit_definition(), 2).expect("valid revision");
        let mut json =
            serde_json::to_value(&revision).expect("published revision should serialize");

        json["id"]["revision"] = serde_json::json!(0);
        let error = serde_json::from_value::<PublishedCardRevision>(json)
            .expect_err("zero revision must fail while deserializing");
        assert!(error.to_string().contains("card revisions start at 1"));

        let revision = PublishedCardRevision::new(unit_definition(), 2).expect("valid revision");
        let mut json =
            serde_json::to_value(&revision).expect("published revision should serialize");
        json["id"]["cardId"] = serde_json::json!("different-card");
        let error = serde_json::from_value::<PublishedCardRevision>(json)
            .expect_err("mismatched card ids must fail while deserializing");
        assert!(error.to_string().contains("does not match definition id"));

        let revision = PublishedCardRevision::new(unit_definition(), 2).expect("valid revision");
        let mut json =
            serde_json::to_value(&revision).expect("published revision should serialize");
        json["definition"]["name"] = serde_json::json!(" ");
        let error = serde_json::from_value::<PublishedCardRevision>(json)
            .expect_err("invalid definitions must fail while deserializing");
        assert!(error.to_string().contains("validation error"));
    }

    #[test]
    fn valid_published_revision_round_trips_through_json() {
        let revision = PublishedCardRevision::new(unit_definition(), 4).expect("valid revision");
        let json = serde_json::to_string(&revision).expect("published revision should serialize");
        let restored: PublishedCardRevision =
            serde_json::from_str(&json).expect("valid published revision should deserialize");

        assert_eq!(restored.id().to_string(), "ash-duelist@4");
        assert_eq!(restored.definition().name, "Ash Duelist");
    }

    #[test]
    fn catalog_resolves_exact_and_latest_revision() {
        let first = PublishedCardRevision::new(unit_definition(), 1).expect("valid first revision");
        let mut second_definition = unit_definition();
        second_definition.cost = 3;
        let second =
            PublishedCardRevision::new(second_definition, 2).expect("valid second revision");
        let first_id = first.id().clone();
        let catalog = CardCatalog::new([second, first]).expect("catalog should be valid");

        assert_eq!(
            catalog
                .resolve(&first_id)
                .expect("exact revision should resolve")
                .definition()
                .cost,
            2
        );
        assert_eq!(
            catalog
                .latest("ash-duelist")
                .expect("latest revision should resolve")
                .definition()
                .cost,
            3
        );
        assert_eq!(catalog.len(), 2);
    }

    #[test]
    fn catalog_rejects_duplicate_revision_identity() {
        let first = PublishedCardRevision::new(unit_definition(), 1).expect("valid revision");
        let duplicate = first.clone();
        let expected_id = first.id().clone();

        assert_eq!(
            CardCatalog::new([first, duplicate]).expect_err("duplicate should fail"),
            CardCatalogError::DuplicateRevision(expected_id)
        );
    }
}
