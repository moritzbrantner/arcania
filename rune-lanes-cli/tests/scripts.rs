//! Golden tests for the match script runner.
//!
//! Every `tests/scripts/*.jsonl` is played through the real rules path and its
//! text output compared with the sibling `.expected` file. After an intentional
//! rules or format change, regenerate with:
//!
//! ```sh
//! UPDATE_EXPECT=1 cargo test -p rune-lanes-cli --test scripts
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use rune_lanes_cli::{OutputFormat, play, run_cli};

fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/scripts")
}

fn scripts() -> Vec<PathBuf> {
    let mut scripts: Vec<PathBuf> = fs::read_dir(scripts_dir())
        .expect("tests/scripts should exist")
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    scripts.sort();
    scripts
}

#[test]
fn example_scripts_match_expected_output() {
    let update = std::env::var_os("UPDATE_EXPECT").is_some_and(|value| value == "1");
    let scripts = scripts();
    assert!(
        scripts.len() >= 5,
        "expected the checked-in example scripts"
    );

    let mut mismatches = Vec::new();
    for script in &scripts {
        let text = fs::read_to_string(script).expect("script should be readable");
        let report = play(None, &text, OutputFormat::Text)
            .unwrap_or_else(|error| panic!("{}: {error}", script.display()));
        let expected_path = script.with_extension("expected");
        if update {
            fs::write(&expected_path, &report.output).expect("expected file should be writable");
            continue;
        }
        let expected = fs::read_to_string(&expected_path).unwrap_or_default();
        if expected != report.output {
            mismatches.push(format!(
                "{} differs from {}\n--- actual ---\n{}",
                script.display(),
                expected_path.display(),
                report.output
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{}\nRe-run with UPDATE_EXPECT=1 if the change is intentional.",
        mismatches.join("\n")
    );
}

#[test]
fn output_is_deterministic_across_runs() {
    for script in scripts() {
        let text = fs::read_to_string(&script).unwrap();
        let first = play(None, &text, OutputFormat::Json).unwrap().output;
        let second = play(None, &text, OutputFormat::Json).unwrap().output;
        assert_eq!(first, second, "{}", script.display());
    }
}

#[test]
fn json_output_carries_domain_events_and_legal_commands() {
    let text = fs::read_to_string(scripts_dir().join("turn-phases.jsonl")).unwrap();
    let report = play(None, &text, OutputFormat::Json).unwrap();
    let value: serde_json::Value = serde_json::from_str(&report.output).unwrap();

    let first_command = value["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| step["kind"] == "command")
        .unwrap();
    assert_eq!(first_command["accepted"], true);
    assert_eq!(
        first_command["domainEvent"]["event"]["type"],
        "commandAccepted"
    );
    assert_eq!(first_command["replayEvents"][1]["type"], "pieceMoved");
    assert_eq!(value["final"]["legalCommands"]["side"], "opponent");
    assert_eq!(value["expectations"]["failed"], 0);
}

#[test]
fn cli_exit_codes_distinguish_failed_expectations_from_errors() {
    let run = |args: &[&str]| {
        let args: Vec<String> = args.iter().map(ToString::to_string).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        run_cli(&args, &mut out, &mut err)
    };
    let script = |name: &str| scripts_dir().join(name).display().to_string();

    assert_eq!(run(&["play", &script("turn-phases.jsonl")]), 0);
    assert_eq!(run(&["play", &script("assertion-failure.jsonl")]), 1);
    assert_eq!(
        run(&["play", "no-such-scenario", &script("turn-phases.jsonl")]),
        2
    );
    assert_eq!(run(&["play"]), 2);
    assert_eq!(run(&["setups"]), 0);
}

#[test]
fn command_line_setup_overrides_the_script_header() {
    let text =
        "{\"setup\": \"move-unit\"}\n{\"expect\": {\"state\": {\"board\": {\"units\": []}}}}\n";
    let from_header = play(None, text, OutputFormat::Text).unwrap();
    let overridden = play(Some("seed:3"), text, OutputFormat::Text).unwrap();

    assert_eq!(from_header.failed_expectations, 1);
    assert_eq!(overridden.failed_expectations, 0);
    assert!(overridden.output.starts_with("setup seed:3\n"));
}
