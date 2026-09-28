//! Script setups: the dev match scenarios from ADR 0010, or a fresh seeded
//! Solo match. Both come from `rune-lanes-core` constructors, so the runner
//! never authors match state of its own.

use rune_lanes_core::MatchState;

const SEED_PREFIX: &str = "seed:";

/// `(id, description)` for every accepted setup form, in a stable order.
pub fn setup_catalog() -> Vec<(String, String)> {
    let mut catalog = scenario_catalog();
    catalog.push((
        format!("{SEED_PREFIX}<n>"),
        "Fresh seeded Solo match (MatchState::new_with_seed).".to_string(),
    ));
    catalog
}

pub fn build_setup(setup: &str) -> Result<MatchState, String> {
    if let Some(seed) = setup.strip_prefix(SEED_PREFIX) {
        let seed: u64 = seed
            .parse()
            .map_err(|_| format!("invalid seed in `{setup}`; expected seed:<u64>"))?;
        return Ok(MatchState::new_with_seed(seed));
    }
    build_scenario(setup).ok_or_else(|| {
        let known: Vec<String> = setup_catalog().into_iter().map(|(id, _)| id).collect();
        format!("unknown setup `{setup}`; known: {}", known.join(", "))
    })
}

// Dev match scenarios are compiled into `rune-lanes-core` only for debug
// builds (ADR 0010), so release builds of the runner offer `seed:<n>` only.
#[cfg(debug_assertions)]
fn scenario_catalog() -> Vec<(String, String)> {
    rune_lanes_core::scenarios::match_scenarios()
        .into_iter()
        .map(|scenario| (scenario.id.to_string(), scenario.description.to_string()))
        .collect()
}

#[cfg(debug_assertions)]
fn build_scenario(id: &str) -> Option<MatchState> {
    rune_lanes_core::scenarios::build_match_scenario(id)
}

#[cfg(not(debug_assertions))]
fn scenario_catalog() -> Vec<(String, String)> {
    Vec::new()
}

#[cfg(not(debug_assertions))]
fn build_scenario(_id: &str) -> Option<MatchState> {
    None
}
