//! Parsing of match scripts: JSON lines, one entry per line.
//!
//! A line is a `GameCommand` in its existing serde form (optionally with a
//! `side`), a `{"setup": ...}` header, an `{"advanceAi": ...}` orchestration
//! step, or an `{"expect": {...}}` assertion. Blank lines and lines starting
//! with `#` or `//` are comments.

use std::fmt;

use rune_lanes_core::Side;
use rune_lanes_core::commands::GameCommand;
use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub struct ScriptLine {
    /// 1-based line number in the script file.
    pub line: usize,
    pub entry: ScriptEntry,
}

#[derive(Clone, Debug)]
pub enum ScriptEntry {
    Setup(String),
    Command {
        side: Option<Side>,
        command: GameCommand,
    },
    AdvanceAi {
        side: Side,
        until_done: bool,
    },
    Expect(Expectation),
}

/// Keys accepted inside `{"expect": {...}}`. Each key is one check.
pub const EXPECT_KEYS: &[&str] = &[
    "phase",
    "activeSide",
    "prioritySide",
    "round",
    "winner",
    "mana",
    "pieces",
    "state",
    "legal",
    "illegal",
    "lastResult",
    "side",
];

#[derive(Clone, Debug)]
pub struct Expectation {
    /// Checks in the order written, excluding `side`.
    pub checks: Vec<(String, Value)>,
    /// Side used for `legal`/`illegal`; defaults to the acting side.
    pub side: Option<Side>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "script line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ScriptError {}

pub fn parse_script(text: &str) -> Result<Vec<ScriptLine>, ScriptError> {
    let mut lines = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        let error = |message: String| ScriptError { line, message };
        let value: Value =
            serde_json::from_str(trimmed).map_err(|err| error(format!("invalid JSON: {err}")))?;
        let Value::Object(object) = value else {
            return Err(error("expected a JSON object".to_string()));
        };
        let entry = parse_entry(object).map_err(error)?;
        if matches!(entry, ScriptEntry::Setup(_)) && !lines.is_empty() {
            return Err(error("setup must come before any other entry".to_string()));
        }
        lines.push(ScriptLine { line, entry });
    }
    Ok(lines)
}

fn parse_entry(mut object: Map<String, Value>) -> Result<ScriptEntry, String> {
    if let Some(setup) = object.remove("setup") {
        only_keys(&object, &[])?;
        let Value::String(setup) = setup else {
            return Err("setup must be a string such as \"move-unit\" or \"seed:7\"".to_string());
        };
        return Ok(ScriptEntry::Setup(setup));
    }
    if let Some(expect) = object.remove("expect") {
        only_keys(&object, &[])?;
        return parse_expectation(expect).map(ScriptEntry::Expect);
    }
    if let Some(side) = object.remove("advanceAi") {
        let until_done = match object.remove("untilDone") {
            None => false,
            Some(Value::Bool(flag)) => flag,
            Some(_) => return Err("untilDone must be a boolean".to_string()),
        };
        only_keys(&object, &[])?;
        return Ok(ScriptEntry::AdvanceAi {
            side: parse_side(side)?,
            until_done,
        });
    }
    let side = object.remove("side").map(parse_side).transpose()?;
    let command = parse_command(Value::Object(object))?;
    Ok(ScriptEntry::Command { side, command })
}

pub fn parse_command(value: Value) -> Result<GameCommand, String> {
    let echo = value.to_string();
    let command: GameCommand =
        serde_json::from_value(value).map_err(|err| format!("invalid command {echo}: {err}"))?;
    // Unknown fields are silently ignored by serde; reject them so typos in a
    // script never turn into a different command than the author intended.
    let canonical = serde_json::to_value(&command).map_err(|err| err.to_string())?;
    let parsed: Value = serde_json::from_str(&echo).map_err(|err| err.to_string())?;
    if let (Value::Object(written), Value::Object(canonical)) = (&parsed, &canonical)
        && let Some(unknown) = written.keys().find(|key| !canonical.contains_key(*key))
    {
        return Err(format!("unknown field `{unknown}` in command {echo}"));
    }
    Ok(command)
}

pub fn parse_side(value: Value) -> Result<Side, String> {
    serde_json::from_value(value.clone()).map_err(|_| {
        format!("invalid side {value}; expected player, opponent, playerTwo or opponentTwo")
    })
}

fn parse_expectation(value: Value) -> Result<Expectation, String> {
    let Value::Object(object) = value else {
        return Err("expect must be an object".to_string());
    };
    if object.is_empty() {
        return Err("expect must contain at least one check".to_string());
    }
    let mut checks = Vec::new();
    let mut side = None;
    for (key, value) in object {
        match key.as_str() {
            "side" => side = Some(parse_side(value)?),
            "legal" | "illegal" => {
                let Value::Array(entries) = &value else {
                    return Err(format!("expect.{key} must be an array of commands"));
                };
                for entry in entries {
                    let command = match entry {
                        Value::Object(wrapper) if wrapper.contains_key("command") => {
                            only_keys(wrapper, &["command", "rejection"])?;
                            wrapper["command"].clone()
                        }
                        other => other.clone(),
                    };
                    parse_command(command)?;
                }
                checks.push((key, value));
            }
            known if EXPECT_KEYS.contains(&known) => checks.push((key, value)),
            unknown => {
                return Err(format!(
                    "unknown expect key `{unknown}`; supported: {}",
                    EXPECT_KEYS.join(", ")
                ));
            }
        }
    }
    Ok(Expectation { checks, side })
}

fn only_keys(object: &Map<String, Value>, allowed: &[&str]) -> Result<(), String> {
    match object.keys().find(|key| !allowed.contains(&key.as_str())) {
        Some(key) => Err(format!("unexpected key `{key}`")),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands_in_their_domain_serde_form() {
        let lines = parse_script(
            "# comment\n\n{\"type\":\"movePiece\",\"pieceId\":\"u\",\"to\":{\"q\":1,\"r\":0}}\n{\"side\":\"opponent\",\"type\":\"endTurn\"}\n",
        )
        .unwrap();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line, 3);
        assert!(matches!(
            &lines[1].entry,
            ScriptEntry::Command {
                side: Some(Side::Opponent),
                command: GameCommand::EndTurn
            }
        ));
    }

    #[test]
    fn rejects_unknown_command_fields_and_expect_keys() {
        let typo =
            parse_script("{\"type\":\"movePiece\",\"piece\":\"u\",\"to\":{\"q\":1,\"r\":0}}");
        assert!(typo.is_err());

        let extra = parse_script("{\"type\":\"endTurn\",\"pieceId\":\"u\"}").unwrap_err();
        assert!(extra.message.contains("unknown field `pieceId`"));

        let key = parse_script("{\"expect\":{\"armour\":1}}").unwrap_err();
        assert!(key.message.contains("unknown expect key `armour`"));
    }

    #[test]
    fn setup_must_lead_the_script() {
        let error = parse_script("{\"type\":\"endTurn\"}\n{\"setup\":\"move-unit\"}").unwrap_err();
        assert_eq!(error.line, 2);
    }
}
