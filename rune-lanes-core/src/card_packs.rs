use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::rules::RULESET_SCHEMA_VERSION;
use crate::{CardCatalog, CardRevisionId};

pub const CARD_PACK_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPackRevisionId {
    pack_id: String,
    version: u32,
}

impl CardPackRevisionId {
    pub fn new(pack_id: impl Into<String>, version: u32) -> Result<Self, CardPackValidationError> {
        let pack_id = pack_id.into();
        if pack_id.is_empty()
            || pack_id.starts_with('-')
            || pack_id.ends_with('-')
            || pack_id.contains("--")
            || !pack_id
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(CardPackValidationError::InvalidPackId { value: pack_id });
        }
        if version == 0 {
            return Err(CardPackValidationError::ZeroPackVersion { pack_id });
        }
        Ok(Self { pack_id, version })
    }

    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }

    pub fn version(&self) -> u32 {
        self.version
    }
}

impl<'de> Deserialize<'de> for CardPackRevisionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct UncheckedId {
            pack_id: String,
            version: u32,
        }
        let unchecked = UncheckedId::deserialize(deserializer)?;
        Self::new(unchecked.pack_id, unchecked.version).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for CardPackRevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@{}", self.pack_id, self.version)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardPackCompatibility {
    pub ruleset_schema_version: u32,
}

/// Serializable metadata; validate against exact published Cards and pack dependencies
/// with `CardPackCatalog::new` before consuming a manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardPackManifest {
    pub schema_version: u32,
    pub id: CardPackRevisionId,
    pub compatibility: CardPackCompatibility,
    pub cards: Vec<CardRevisionId>,
    pub dependencies: Vec<CardPackRevisionId>,
}

/// A validated set of manifests. Construction checks every supplied pack, including
/// transitive dependencies, without recursive loading or latest-version fallback.
#[derive(Clone, Debug)]
pub struct CardPackCatalog {
    manifests: BTreeMap<CardPackRevisionId, CardPackManifest>,
}

impl CardPackCatalog {
    pub fn new(
        manifests: impl IntoIterator<Item = CardPackManifest>,
        cards: &CardCatalog,
    ) -> Result<Self, Vec<CardPackValidationError>> {
        let mut indexed = BTreeMap::new();
        let mut errors = Vec::new();
        for manifest in manifests {
            match indexed.entry(manifest.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(manifest);
                }
                Entry::Occupied(entry) => errors.push(CardPackValidationError::DuplicatePack {
                    pack: entry.key().clone(),
                }),
            }
        }
        if !errors.is_empty() {
            errors.sort();
            errors.dedup();
            return Err(errors);
        }
        for manifest in indexed.values() {
            validate_manifest(manifest, cards, &indexed, &mut errors);
        }
        errors.sort();
        errors.dedup();
        if errors.is_empty() {
            Ok(Self { manifests: indexed })
        } else {
            Err(errors)
        }
    }

    pub fn resolve(&self, id: &CardPackRevisionId) -> Option<&CardPackManifest> {
        self.manifests.get(id)
    }

    /// Every exact manifest, in stable pack identity order.
    pub fn manifests(&self) -> impl Iterator<Item = &CardPackManifest> {
        self.manifests.values()
    }
}

fn validate_manifest(
    manifest: &CardPackManifest,
    cards: &CardCatalog,
    packs: &BTreeMap<CardPackRevisionId, CardPackManifest>,
    errors: &mut Vec<CardPackValidationError>,
) {
    let pack = &manifest.id;
    if manifest.schema_version != CARD_PACK_MANIFEST_SCHEMA_VERSION {
        errors.push(CardPackValidationError::UnknownManifestVersion {
            pack: pack.clone(),
            version: manifest.schema_version,
            supported: CARD_PACK_MANIFEST_SCHEMA_VERSION,
        });
    }
    if manifest.compatibility.ruleset_schema_version != RULESET_SCHEMA_VERSION {
        errors.push(CardPackValidationError::IncompatibleRulesetSchema {
            pack: pack.clone(),
            required: manifest.compatibility.ruleset_schema_version,
            supported: RULESET_SCHEMA_VERSION,
        });
    }
    let mut seen_cards = BTreeSet::new();
    for card in &manifest.cards {
        if !seen_cards.insert(card) {
            errors.push(CardPackValidationError::DuplicateCard {
                pack: pack.clone(),
                card: card.clone(),
            });
        }
        if cards.resolve(card).is_none() {
            errors.push(CardPackValidationError::MissingCardRevision {
                pack: pack.clone(),
                card: card.clone(),
            });
        }
    }
    let mut seen_dependencies = BTreeSet::new();
    for dependency in &manifest.dependencies {
        if !seen_dependencies.insert(dependency) {
            errors.push(CardPackValidationError::DuplicateDependency {
                pack: pack.clone(),
                dependency: dependency.clone(),
            });
        }
        if packs.contains_key(dependency) {
            continue;
        }
        let available_versions: Vec<_> = packs
            .keys()
            .filter(|candidate| candidate.pack_id == dependency.pack_id)
            .map(|candidate| candidate.version)
            .collect();
        if available_versions.is_empty() {
            errors.push(CardPackValidationError::MissingDependency {
                pack: pack.clone(),
                dependency: dependency.clone(),
            });
        } else {
            errors.push(CardPackValidationError::IncompatibleDependencyVersion {
                pack: pack.clone(),
                dependency: dependency.clone(),
                available_versions,
            });
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "code",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CardPackValidationError {
    InvalidPackId {
        value: String,
    },
    ZeroPackVersion {
        pack_id: String,
    },
    DuplicatePack {
        pack: CardPackRevisionId,
    },
    UnknownManifestVersion {
        pack: CardPackRevisionId,
        version: u32,
        supported: u32,
    },
    IncompatibleRulesetSchema {
        pack: CardPackRevisionId,
        required: u32,
        supported: u32,
    },
    DuplicateCard {
        pack: CardPackRevisionId,
        card: CardRevisionId,
    },
    MissingCardRevision {
        pack: CardPackRevisionId,
        card: CardRevisionId,
    },
    DuplicateDependency {
        pack: CardPackRevisionId,
        dependency: CardPackRevisionId,
    },
    MissingDependency {
        pack: CardPackRevisionId,
        dependency: CardPackRevisionId,
    },
    IncompatibleDependencyVersion {
        pack: CardPackRevisionId,
        dependency: CardPackRevisionId,
        available_versions: Vec<u32>,
    },
}

impl fmt::Display for CardPackValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPackId { value } => write!(
                formatter,
                "invalid Card pack id {value:?}; use lowercase letters, digits and single interior hyphens"
            ),
            Self::ZeroPackVersion { pack_id } => {
                write!(formatter, "Card pack {pack_id} versions start at 1")
            }
            Self::DuplicatePack { pack } => write!(
                formatter,
                "duplicate Card pack {pack}; each identity/version must be unique"
            ),
            Self::UnknownManifestVersion {
                pack,
                version,
                supported,
            } => write!(
                formatter,
                "Card pack {pack} uses manifest schema {version}; supported schema is {supported}"
            ),
            Self::IncompatibleRulesetSchema {
                pack,
                required,
                supported,
            } => write!(
                formatter,
                "Card pack {pack} requires ruleset schema {required}; supported schema is {supported}"
            ),
            Self::DuplicateCard { pack, card } => write!(
                formatter,
                "Card pack {pack} lists Card revision {card} more than once"
            ),
            Self::MissingCardRevision { pack, card } => write!(
                formatter,
                "Card pack {pack} requires missing published Card revision {card}"
            ),
            Self::DuplicateDependency { pack, dependency } => write!(
                formatter,
                "Card pack {pack} lists dependency {dependency} more than once"
            ),
            Self::MissingDependency { pack, dependency } => write!(
                formatter,
                "Card pack {pack} requires missing dependency {dependency}"
            ),
            Self::IncompatibleDependencyVersion {
                pack,
                dependency,
                available_versions,
            } => write!(
                formatter,
                "Card pack {pack} requires dependency {dependency}; available versions are {available_versions:?}"
            ),
        }
    }
}

impl std::error::Error for CardPackValidationError {}

#[cfg(test)]
mod tests;
