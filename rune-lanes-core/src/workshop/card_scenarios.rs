//! Prepared authored-Card starting states; play uses the existing command/query facade.
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::WorkshopSetup;
use crate::{CardKind, HexCoord, MatchState, Side, Unit, starter_card_catalog};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CardTestBoardSetup {
    Empty,
    AdjacentEnemy,
    ClusteredEnemies,
    DamagedAlly,
    LowHpHero,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CardTestScenario {
    pub workshop: WorkshopSetup,
    pub board_setup: CardTestBoardSetup,
}

impl CardTestScenario {
    pub fn prepare(&self) -> Result<PreparedCardTestScenario, String> {
        let mut state = self.workshop.create_match()?;
        let custom_ids: HashSet<_> = self
            .workshop
            .cards
            .iter()
            .map(|card| card.id.as_str())
            .collect();
        state
            .player
            .hand
            .retain(|card| custom_ids.contains(card.template_id.as_str()));
        state
            .opponent
            .hand
            .retain(|card| custom_ids.contains(card.template_id.as_str()));
        state.player.hero.position = HexCoord { q: 0, r: 0 };
        state.opponent.hero.position = HexCoord { q: 0, r: -2 };
        state.board.mana_sources.clear();
        state.board.buildings.clear();
        match self.board_setup {
            CardTestBoardSetup::Empty => {}
            CardTestBoardSetup::AdjacentEnemy => state.board.units.push(scenario_unit(
                "scenario-enemy-0",
                Side::Opponent,
                HexCoord { q: 1, r: 0 },
                self.workshop.ruleset.turn.default_attack_range,
            )),
            CardTestBoardSetup::ClusteredEnemies => {
                for (index, position) in [
                    HexCoord { q: 1, r: 0 },
                    HexCoord { q: 1, r: -1 },
                    HexCoord { q: 2, r: -1 },
                ]
                .into_iter()
                .enumerate()
                {
                    state.board.units.push(scenario_unit(
                        &format!("scenario-enemy-{index}"),
                        Side::Opponent,
                        position,
                        self.workshop.ruleset.turn.default_attack_range,
                    ));
                }
            }
            CardTestBoardSetup::DamagedAlly => {
                let mut ally = scenario_unit(
                    "scenario-ally-0",
                    Side::Player,
                    HexCoord { q: 1, r: 0 },
                    self.workshop.ruleset.turn.default_attack_range,
                );
                ally.armor /= 2;
                state.board.units.push(ally);
            }
            CardTestBoardSetup::LowHpHero => {
                state.player.hero.hp = 1;
                state.opponent.hero.hp = 1;
            }
        }
        state.log = vec!["Prepared an authored Card test scenario.".into()];
        Ok(PreparedCardTestScenario {
            initial_state: state,
        })
    }
}

fn scenario_unit(id: &str, side: Side, position: HexCoord, attack_range: u8) -> Unit {
    let definition = starter_card_catalog()
        .latest("stoneguard")
        .expect("the compiled scenario Stoneguard template exists")
        .definition();
    let CardKind::Unit {
        attack,
        armor,
        max_ap,
    } = definition.kind
    else {
        unreachable!("the compiled scenario Stoneguard template is a Unit");
    };
    Unit {
        id: id.into(),
        side,
        name: definition.name.clone(),
        template_id: Some(definition.id.clone()),
        attack,
        attack_range,
        armor,
        max_armor: armor,
        position,
        ap_remaining: max_ap,
        max_ap,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    }
}

#[derive(Debug)]
pub struct PreparedCardTestScenario {
    initial_state: MatchState,
}

impl PreparedCardTestScenario {
    pub fn initial_state(&self) -> &MatchState {
        &self.initial_state
    }

    /// Start or reset an independent run from the same validated initial state.
    pub fn create_match(&self) -> MatchState {
        self.initial_state.clone()
    }
}

#[cfg(test)]
mod tests;
