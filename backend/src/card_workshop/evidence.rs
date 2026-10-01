//! Resolve Account-accessible exact sources into owned inputs for the common AI lab.
use super::{CardWorkshop, CardWorkshopError};
use crate::ai_lab::AiLabError;
use crate::ai_lab::card_evidence::{CardEvidenceInput, CardEvidenceSnapshot, CardEvidenceSource};
use rune_lanes_core::CardRevisionId;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CardEvidenceReference {
    Draft { draft_id: i64, version: u64 },
    Published { revision_id: CardRevisionId },
}

pub type CardEvidenceRequest = CardEvidenceInput<CardEvidenceReference>;

impl CardWorkshop<'_> {
    /// Freeze both sources while storage is owned; self-play needs no storage access.
    pub fn resolve_evidence_for_user(
        &self,
        user_id: i64,
        request: CardEvidenceRequest,
    ) -> Result<CardEvidenceInput, CardWorkshopError> {
        if request.cases.len() > 8 || request.max_actions > 300 {
            return Err(CardWorkshopError::Evidence(AiLabError::Config(
                "Account Card evidence requests allow at most 8 cases and 300 actions per game."
                    .into(),
            )));
        }
        Ok(CardEvidenceInput {
            candidate: self.resolve_evidence_source(user_id, request.candidate)?,
            baseline: self.resolve_evidence_source(user_id, request.baseline)?,
            cases: request.cases,
            policy_config: request.policy_config,
            player_policy_id: request.player_policy_id,
            opponent_policy_id: request.opponent_policy_id,
            tested_side: request.tested_side,
            max_actions: request.max_actions,
        })
    }

    fn resolve_evidence_source(
        &self,
        user_id: i64,
        reference: CardEvidenceReference,
    ) -> Result<CardEvidenceSnapshot, CardWorkshopError> {
        match reference {
            CardEvidenceReference::Draft { draft_id, version } => {
                let draft = self
                    .load_draft_for_user(user_id, draft_id)?
                    .ok_or(CardWorkshopError::NotFound)?;
                if draft.version != version {
                    return Err(CardWorkshopError::VersionConflict {
                        expected: version,
                        current: draft.version,
                    });
                }
                if !draft.validation_errors.is_empty() {
                    return Err(CardWorkshopError::InvalidDefinition {
                        errors: draft.validation_errors,
                    });
                }
                let mut definition = draft.definition;
                definition.id = draft.catalog_id.clone();
                Ok(CardEvidenceSnapshot {
                    source: CardEvidenceSource::Draft {
                        draft_id: draft.id,
                        version: draft.version,
                        catalog_id: draft.catalog_id,
                    },
                    definition,
                })
            }
            CardEvidenceReference::Published { revision_id } => {
                let transfer = self.export_revision_for_user(user_id, &revision_id)?;
                Ok(CardEvidenceSnapshot {
                    source: CardEvidenceSource::Published { revision_id },
                    definition: transfer.revision().definition().clone(),
                })
            }
        }
    }
}
