//! Evaluation of `{"expect": {...}}` lines.
//!
//! State checks compare against the authoritative snapshot JSON (the same
//! representation the event store persists), using subset matching: objects
//! compare only the keys written, arrays compare length and each element,
//! scalars compare exactly. Legality checks go through the query facade.

use rune_lanes_core::{MatchError, MatchState};
use serde_json::Value;

use crate::acting_side;
use crate::script::{Expectation, parse_command};

pub(crate) fn evaluate(
    expectation: &Expectation,
    state: &MatchState,
    last_result: Option<&Result<(), MatchError>>,
) -> Vec<String> {
    let snapshot: Value = state
        .to_snapshot_json()
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(Value::Null);
    let side = expectation.side.unwrap_or_else(|| acting_side(state));
    let mut failures = Vec::new();

    for (key, expected) in &expectation.checks {
        match key.as_str() {
            "phase" | "activeSide" | "prioritySide" | "round" | "winner" => {
                compare(key, expected, &snapshot[key.as_str()], &mut failures);
            }
            "state" => compare("state", expected, &snapshot, &mut failures),
            "mana" => match expected {
                Value::Object(sides) => {
                    for (participant, mana) in sides {
                        compare(
                            &format!("mana.{participant}"),
                            mana,
                            &snapshot[participant.as_str()]["mana"],
                            &mut failures,
                        );
                    }
                }
                _ => failures.push("mana: expected an object like {\"player\": 7}".to_string()),
            },
            "pieces" => match expected {
                Value::Object(pieces) => {
                    for (piece_id, piece) in pieces {
                        let path = format!("pieces.{piece_id}");
                        match (find_piece(&snapshot, piece_id), piece) {
                            (None, Value::Null) => {}
                            (None, _) => failures.push(format!("{path}: piece not found")),
                            (Some(_), Value::Null) => {
                                failures.push(format!("{path}: expected no such piece"));
                            }
                            (Some(actual), expected) => {
                                compare(&path, expected, actual, &mut failures);
                            }
                        }
                    }
                }
                _ => failures.push("pieces: expected an object keyed by piece id".to_string()),
            },
            "legal" | "illegal" => {
                let want_legal = key == "legal";
                for entry in expected.as_array().into_iter().flatten() {
                    let (command, rejection) = match entry {
                        Value::Object(wrapper) if wrapper.contains_key("command") => (
                            wrapper["command"].clone(),
                            wrapper.get("rejection").cloned(),
                        ),
                        other => (other.clone(), None),
                    };
                    let echo = command.to_string();
                    let Ok(command) = parse_command(command) else {
                        failures.push(format!("{key}: cannot parse {echo}"));
                        continue;
                    };
                    let availability = state.queries().command_availability(side, &command);
                    let actual_rejection = availability.rejection.as_ref().map(error_code);
                    match (want_legal, availability.allowed) {
                        (true, false) => failures.push(format!(
                            "legal {echo}: rejected {}",
                            actual_rejection.unwrap_or_default()
                        )),
                        (false, true) => failures.push(format!("illegal {echo}: allowed")),
                        (false, false) => {
                            if let Some(rejection) = rejection
                                && rejection.as_str() != actual_rejection.as_deref()
                            {
                                failures.push(format!(
                                    "illegal {echo}: expected rejection {rejection}, got \"{}\"",
                                    actual_rejection.unwrap_or_default()
                                ));
                            }
                        }
                        (true, true) => {}
                    }
                }
            }
            "lastResult" => {
                let actual = match last_result {
                    None => Value::from("none"),
                    Some(Ok(())) => Value::from("accepted"),
                    Some(Err(error)) => Value::from(error_code(error)),
                };
                compare("lastResult", expected, &actual, &mut failures);
            }
            other => failures.push(format!("unsupported check `{other}`")),
        }
    }
    failures
}

pub(crate) fn error_code(error: &MatchError) -> String {
    match serde_json::to_value(error) {
        Ok(Value::String(code)) => code,
        _ => format!("{error:?}"),
    }
}

fn find_piece<'a>(snapshot: &'a Value, piece_id: &str) -> Option<&'a Value> {
    let by_id = |piece: &&Value| piece["id"] == piece_id;
    snapshot["board"]["units"]
        .as_array()
        .into_iter()
        .flatten()
        .find(by_id)
        .or_else(|| {
            ["player", "opponent", "playerTwo", "opponentTwo"]
                .iter()
                .map(|side| &snapshot[*side]["hero"])
                .find(by_id)
        })
}

fn compare(path: &str, expected: &Value, actual: &Value, failures: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (key, value) in expected {
                compare(
                    &format!("{path}.{key}"),
                    value,
                    actual.get(key).unwrap_or(&Value::Null),
                    failures,
                );
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            if expected.len() != actual.len() {
                failures.push(format!(
                    "{path}: expected {} entries, got {}",
                    expected.len(),
                    actual.len()
                ));
                return;
            }
            for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
                compare(&format!("{path}[{index}]"), expected, actual, failures);
            }
        }
        (expected, actual) if expected == actual => {}
        (expected, actual) => {
            failures.push(format!("{path}: expected {expected}, got {actual}"));
        }
    }
}
