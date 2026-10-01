//! Descriptive paired evidence over owned exact Card inputs, using the existing AI lab.
use rune_lanes_core::{
    AiPolicyConfig, CardDefinition, CardRevisionId, ReplayEvent, Side, SoloAiPolicy,
    workshop::card_scenarios::{CardTestScenario, PreparedCardTestScenario},
};
use serde::{Deserialize, Serialize};

use super::{
    AiLabError,
    self_play::{GameOutcome, run_self_play},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum CardEvidenceSource {
    Draft {
        draft_id: i64,
        version: u64,
        catalog_id: String,
    },
    Published {
        revision_id: CardRevisionId,
    },
}

impl CardEvidenceSource {
    fn identifies_same_version(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Draft {
                    draft_id: left_id,
                    version: left_version,
                    ..
                },
                Self::Draft {
                    draft_id: right_id,
                    version: right_version,
                    ..
                },
            ) => left_id == right_id && left_version == right_version,
            (Self::Published { revision_id: left }, Self::Published { revision_id: right }) => {
                left == right
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardEvidenceSnapshot {
    pub source: CardEvidenceSource,
    pub definition: CardDefinition,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardEvidenceInput {
    pub candidate: CardEvidenceSnapshot,
    pub baseline: CardEvidenceSnapshot,
    pub cases: Vec<CardTestScenario>,
    pub policy_config: AiPolicyConfig,
    pub player_policy_id: String,
    pub opponent_policy_id: String,
    pub tested_side: Side,
    pub max_actions: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEvidenceReport {
    pub input: CardEvidenceInput,
    pub core_default_ruleset_version: String,
    pub built_in_cards: Vec<CardDefinition>,
    pub pairs: Vec<CardEvidencePair>,
    pub limitations: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEvidencePair {
    pub case_index: usize,
    pub candidate: CardEvidenceGame,
    pub baseline: CardEvidenceGame,
    pub delta: CardEvidenceDelta,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEvidenceGame {
    pub outcome: CardEvidenceOutcome,
    pub action_count: u32,
    pub card_plays: u32,
    pub play_action_indices: Vec<Option<u32>>,
    pub surviving_units: u32,
    pub event_totals: CardEvidenceEventTotals,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CardEvidenceOutcome {
    TestedSideWin,
    OtherSideWin,
    Draw,
    Timeout,
    IllegalAction { reason: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEvidenceDelta {
    pub card_plays: i64,
    pub surviving_units: i64,
    pub tested_side_wins: i64,
    pub event_totals: CardEvidenceEventTotals,
}

/// Whole-game recorded event amounts, without source-Card attribution. Damage
/// and healing amounts are the event values, not effective HP/armor changes.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardEvidenceEventTotals {
    pub damage_amount: i64,
    pub healing_amount: i64,
    pub draw_events: i64,
}

pub fn run_comparison(input: CardEvidenceInput) -> Result<CardEvidenceReport, AiLabError> {
    let prepared = PreparedComparison::new(input)?;
    prepared.run()
}

pub(super) fn run_cli(args: &[String]) -> Result<(), AiLabError> {
    let mut input_path = None;
    let mut output_path = None;
    let (pairs, remainder) = args.as_chunks::<2>();
    for pair in pairs {
        let target = match pair[0].as_str() {
            "--input" => &mut input_path,
            "--out" => &mut output_path,
            unknown => {
                return Err(AiLabError::Usage(format!(
                    "Unknown Card comparison option: {unknown}"
                )));
            }
        };
        if target.replace(std::path::PathBuf::from(&pair[1])).is_some() {
            return Err(AiLabError::Usage(format!(
                "Duplicate Card comparison option: {}",
                pair[0]
            )));
        }
    }
    if !remainder.is_empty() {
        return Err(AiLabError::Usage(
            "Card comparison options require a value.".into(),
        ));
    }
    let input_path =
        input_path.ok_or_else(|| AiLabError::Usage("Card comparison requires --input.".into()))?;
    let output_path =
        output_path.ok_or_else(|| AiLabError::Usage("Card comparison requires --out.".into()))?;
    let input = serde_json::from_str(&std::fs::read_to_string(input_path)?)?;
    let report = run_comparison(input)?;
    let encoded = serde_json::to_string_pretty(&report)?;
    std::fs::create_dir_all(&output_path)?;
    std::fs::write(output_path.join("card-comparison.json"), encoded)?;
    println!(
        "Card comparison wrote {} paired cases to {}",
        report.pairs.len(),
        output_path.display()
    );
    Ok(())
}

struct PreparedComparison {
    input: CardEvidenceInput,
    cases: Vec<PreparedCardTestScenario>,
    player_policy: SoloAiPolicy,
    opponent_policy: SoloAiPolicy,
}

impl PreparedComparison {
    fn new(input: CardEvidenceInput) -> Result<Self, AiLabError> {
        for snapshot in [&input.candidate, &input.baseline] {
            rune_lanes_core::workshop::validate_card_for_experiment(&snapshot.definition).map_err(
                |error| AiLabError::Config(format!("invalid Card evidence definition: {error}")),
            )?;
            let identity_matches = match &snapshot.source {
                CardEvidenceSource::Draft {
                    draft_id,
                    version,
                    catalog_id,
                } => {
                    *draft_id > 0
                        && *version > 0
                        && catalog_id.starts_with("custom-")
                        && *catalog_id == snapshot.definition.id
                }
                CardEvidenceSource::Published { revision_id } => {
                    revision_id.card_id() == snapshot.definition.id
                }
            };
            if !identity_matches {
                return Err(AiLabError::Config(
                    "Card evidence source identity must match its exact definition.".into(),
                ));
            }
        }
        if input
            .candidate
            .source
            .identifies_same_version(&input.baseline.source)
            && input.candidate.definition != input.baseline.definition
        {
            return Err(AiLabError::Config(
                "The same source identity cannot contain different Card definitions.".into(),
            ));
        }
        if input.cases.is_empty() {
            return Err(AiLabError::Config(
                "Card evidence requires at least one case.".into(),
            ));
        }
        if input.max_actions == 0 {
            return Err(AiLabError::Config(
                "Card evidence requires a positive action limit.".into(),
            ));
        }
        if !matches!(input.tested_side, Side::Player | Side::Opponent) {
            return Err(AiLabError::Config(
                "Card evidence supports the two Duel sides.".into(),
            ));
        }
        if input
            .cases
            .iter()
            .any(|case| !case.workshop.cards.is_empty())
        {
            return Err(AiLabError::Config(
                "Card evidence cases require an empty authored Card list.".into(),
            ));
        }
        input
            .policy_config
            .validate()
            .map_err(|error| AiLabError::Config(error.to_string()))?;
        let policy = |id: &str| {
            input
                .policy_config
                .policy(id)
                .map(SoloAiPolicy::from_definition)
                .ok_or_else(|| {
                    AiLabError::Config(format!("Card evidence policy was not found: {id}"))
                })
        };
        let player_policy = policy(&input.player_policy_id)?;
        let opponent_policy = policy(&input.opponent_policy_id)?;
        let cases = input
            .cases
            .iter()
            .map(|case| case.prepare().map_err(AiLabError::Config))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            input,
            cases,
            player_policy,
            opponent_policy,
        })
    }

    fn run(self) -> Result<CardEvidenceReport, AiLabError> {
        let mut pairs = Vec::with_capacity(self.cases.len());
        for (case_index, case) in self.cases.iter().enumerate() {
            let candidate = self.run_case(case, &self.input.candidate.definition)?;
            let baseline = self.run_case(case, &self.input.baseline.definition)?;
            let delta = CardEvidenceDelta {
                card_plays: i64::from(candidate.card_plays) - i64::from(baseline.card_plays),
                surviving_units: i64::from(candidate.surviving_units)
                    - i64::from(baseline.surviving_units),
                tested_side_wins: i64::from(matches!(
                    candidate.outcome,
                    CardEvidenceOutcome::TestedSideWin
                )) - i64::from(matches!(
                    baseline.outcome,
                    CardEvidenceOutcome::TestedSideWin
                )),
                event_totals: CardEvidenceEventTotals {
                    damage_amount: candidate.event_totals.damage_amount
                        - baseline.event_totals.damage_amount,
                    healing_amount: candidate.event_totals.healing_amount
                        - baseline.event_totals.healing_amount,
                    draw_events: candidate.event_totals.draw_events
                        - baseline.event_totals.draw_events,
                },
            };
            pairs.push(CardEvidencePair {
                case_index,
                candidate,
                baseline,
                delta,
            });
        }
        Ok(CardEvidenceReport {
            input: self.input,
            core_default_ruleset_version: rune_lanes_core::rules::current_ruleset_version(),
            built_in_cards: rune_lanes_core::starter_card_definitions(),
            pairs,
            limitations: vec![
                "Damage, healing and draw events do not consistently identify their source Card; per-Card generated-effect attribution is unavailable.",
                "Event totals cover both sides, including combat, counterattacks and turn draws. Recorded damage/healing amounts are not effective HP/Unit armor changes, and paired deltas are descriptive rather than causal attribution.",
                "Surviving Units are new Units on the tested side matching the input Card template; initial fixture Units are excluded. Timeouts and illegal actions remain incomplete outcomes, not draws or losses.",
            ],
        })
    }

    fn run_case(
        &self,
        case: &PreparedCardTestScenario,
        definition: &CardDefinition,
    ) -> Result<CardEvidenceGame, AiLabError> {
        let mut initial = case.create_match();
        let tested = initial.player_mut_for_ai_lab(self.input.tested_side);
        // Every existing recipe copy on the tested side uses the same frozen
        // definition. Keep instance IDs and seeded draw order unchanged.
        for card in tested.deck_mut_for_ai_lab() {
            if card.template_id == definition.id {
                *card = definition.instantiate(card.id.clone());
            }
        }
        tested
            .hand
            .push(definition.instantiate("card-evidence-input"));
        let (tested_policy, other_policy) = match self.input.tested_side {
            Side::Player => (&self.player_policy, &self.opponent_policy),
            _ => (&self.opponent_policy, &self.player_policy),
        };
        let result = run_self_play(
            initial,
            self.input.tested_side,
            other_policy,
            tested_policy,
            self.input.max_actions,
        )?;
        let play_action_indices = result
            .frames
            .iter()
            .filter_map(|frame| match &frame.event {
                ReplayEvent::CardPlayed { side, card, .. }
                    if *side == self.input.tested_side && card.template_id == definition.id =>
                {
                    Some(frame.action_index)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let surviving_units = result
            .final_state
            .board
            .units
            .iter()
            .filter(|unit| {
                unit.side == self.input.tested_side
                    && unit.template_id.as_deref() == Some(&definition.id)
                    && !case
                        .initial_state()
                        .board
                        .units
                        .iter()
                        .any(|initial| initial.id == unit.id)
            })
            .count();
        let mut event_totals = CardEvidenceEventTotals::default();
        for frame in &result.frames {
            match &frame.event {
                ReplayEvent::PieceDamaged { amount, .. } => {
                    event_totals.damage_amount += i64::from(*amount)
                }
                ReplayEvent::PieceAttacked {
                    damage_to_target,
                    counter_damage_to_attacker,
                    ..
                } => {
                    event_totals.damage_amount +=
                        i64::from(*damage_to_target) + i64::from(*counter_damage_to_attacker);
                }
                ReplayEvent::PieceHealed { amount, .. } => {
                    event_totals.healing_amount += i64::from(*amount)
                }
                ReplayEvent::CardDrawn { .. } => event_totals.draw_events += 1,
                _ => {}
            }
        }
        Ok(CardEvidenceGame {
            outcome: match result.outcome {
                GameOutcome::CandidateWin => CardEvidenceOutcome::TestedSideWin,
                GameOutcome::BaselineWin => CardEvidenceOutcome::OtherSideWin,
                GameOutcome::Draw => CardEvidenceOutcome::Draw,
                GameOutcome::Timeout => CardEvidenceOutcome::Timeout,
                GameOutcome::IllegalAction(reason) => CardEvidenceOutcome::IllegalAction { reason },
            },
            action_count: result.action_count,
            card_plays: u32::try_from(play_action_indices.len())
                .map_err(|error| AiLabError::Config(error.to_string()))?,
            play_action_indices,
            surviving_units: u32::try_from(surviving_units)
                .map_err(|error| AiLabError::Config(error.to_string()))?,
            event_totals,
        })
    }
}
