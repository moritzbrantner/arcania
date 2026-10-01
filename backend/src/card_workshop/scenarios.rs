use super::{CardWorkshop, CardWorkshopError};
use rune_lanes_core::{
    RecordedReplayFrame, Side,
    commands::GameCommand,
    event_sourcing::{
        CommandDecision, CommandId, CommandMetadata, EventEnvelope, EventSourcedMatch,
    },
    workshop::card_scenarios::{CardTestScenario, PreparedCardTestScenario},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardDraftScenarioRequest {
    pub version: u64,
    /// Supply an empty Card list; the owned persisted draft supplies the Card.
    pub scenario: CardTestScenario,
    pub commands: Vec<CardScenarioCommand>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardScenarioCommand {
    pub side: Side,
    pub command: GameCommand,
}

#[derive(Debug)]
pub struct PreparedDraftScenario {
    draft_id: i64,
    draft_version: u64,
    initial: PreparedCardTestScenario,
    commands: Vec<CardScenarioCommand>,
}

#[derive(Debug)]
pub struct CardDraftScenarioResult {
    pub draft_id: i64,
    pub draft_version: u64,
    pub initial_state: serde_json::Value,
    pub final_state: serde_json::Value,
    pub events: Vec<EventEnvelope>,
    pub replay_frames: Vec<RecordedReplayFrame>,
}

impl CardWorkshop<'_> {
    pub fn prepare_draft_scenario_for_user(
        &self,
        user_id: i64,
        draft_id: i64,
        mut request: CardDraftScenarioRequest,
    ) -> Result<PreparedDraftScenario, CardWorkshopError> {
        let draft = self
            .load_draft_for_user(user_id, draft_id)?
            .ok_or(CardWorkshopError::NotFound)?;
        if draft.version != request.version {
            return Err(CardWorkshopError::VersionConflict {
                expected: request.version,
                current: draft.version,
            });
        }
        if !draft.validation_errors.is_empty() {
            return Err(CardWorkshopError::InvalidDefinition {
                errors: draft.validation_errors,
            });
        }
        if !request.scenario.workshop.cards.is_empty() {
            return Err(CardWorkshopError::InvalidScenario("A draft scenario takes its Card from the owned persisted draft; supply an empty scenario Card list.".into()));
        }
        let mut definition = draft.definition;
        definition.id = draft.catalog_id;
        request.scenario.workshop.cards.push(definition);
        let initial = request
            .scenario
            .prepare()
            .map_err(CardWorkshopError::InvalidScenario)?;
        Ok(PreparedDraftScenario {
            draft_id,
            draft_version: draft.version,
            initial,
            commands: request.commands,
        })
    }
}

impl PreparedDraftScenario {
    /// Execute the frozen draft copy without holding a database connection or lock.
    pub fn run(self) -> Result<CardDraftScenarioResult, CardWorkshopError> {
        let initial_state = self
            .initial
            .initial_state()
            .queries()
            .public_match(Side::Player);
        let mut replay_frames = vec![self.initial.initial_state().initial_replay_frame()];
        let (mut aggregate, genesis) = EventSourcedMatch::create(self.initial.create_match())?;
        let mut events = vec![genesis];
        for (index, step) in self.commands.into_iter().enumerate() {
            let index = u32::try_from(index).map_err(|_| {
                CardWorkshopError::InvalidScenario("Too many Card test commands.".into())
            })?;
            let decision = aggregate
                .decide(
                    CommandMetadata {
                        command_id: CommandId::new(format!("draft-scenario-command-{index}"))?,
                        expected_version: aggregate.version(),
                        side: step.side,
                        action_index: index,
                    },
                    step.command,
                )
                .map_err(|error| CardWorkshopError::Scenario {
                    command_index: Some(index),
                    error,
                })?;
            let CommandDecision::Append(event) = decision else {
                unreachable!("fresh scenario command IDs are unique within the new aggregate");
            };
            let outcome =
                aggregate
                    .evolve(&event)
                    .map_err(|error| CardWorkshopError::Scenario {
                        command_index: Some(index),
                        error,
                    })?;
            replay_frames.extend(outcome.replay_frames);
            events.push(event);
        }
        Ok(CardDraftScenarioResult {
            draft_id: self.draft_id,
            draft_version: self.draft_version,
            initial_state,
            final_state: aggregate.queries().public_match(Side::Player),
            events,
            replay_frames,
        })
    }
}
