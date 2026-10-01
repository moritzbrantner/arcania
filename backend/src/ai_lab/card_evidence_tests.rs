use rune_lanes_core::{HeroType, rules::CURRENT_RULESET, workshop::WorkshopSetup};
use serde_json::{Value, json};

use super::card_evidence::{CardEvidenceInput, run_comparison};

fn input_json() -> Value {
    let scenario = rune_lanes_core::workshop::card_scenarios::CardTestScenario {
        workshop: WorkshopSetup {
            seed: 7,
            player_hero: HeroType::Runekeeper,
            opponent_hero: HeroType::Runekeeper,
            player_deck_recipe: rune_lanes_core::deck_library::system_deck_by_id(
                "balanced-starter",
            )
            .expect("known recipe")
            .cards
            .clone(),
            opponent_deck_recipe: rune_lanes_core::deck_library::system_deck_by_id(
                "balanced-starter",
            )
            .expect("known recipe")
            .cards
            .clone(),
            ruleset: CURRENT_RULESET,
            cards: Vec::new(),
        },
        board_setup: rune_lanes_core::workshop::card_scenarios::CardTestBoardSetup::AdjacentEnemy,
    };
    let card = json!({
        "source": {"type": "published", "revisionId": {"cardId": "custom-duelist", "revision": 1}},
        "definition": {"id": "custom-duelist", "name": "Duelist", "rarity": "basic",
            "cost": 1, "text": "", "kind": {"type": "unit", "attack": 2, "armor": 4, "maxAp": 2}}
    });
    json!({
        "candidate": card, "baseline": card, "cases": [scenario],
        "policyConfig": {"defaultPolicyId": "summon", "policies": [{"id": "summon", "rules": ["highestCostUnitSummon"]}]},
        "playerPolicyId": "summon", "opponentPolicyId": "summon", "testedSide": "player", "maxActions": 1
    })
}

#[test]
fn paired_card_evidence_keeps_exact_inputs_and_repeats_real_core_play() {
    let input: CardEvidenceInput = serde_json::from_value(input_json()).expect("valid input");
    let first = serde_json::to_value(run_comparison(input).expect("comparison runs"))
        .expect("report serializes");
    let second = serde_json::to_value(
        run_comparison(serde_json::from_value(input_json()).expect("valid input"))
            .expect("comparison repeats"),
    )
    .expect("report serializes");
    assert_eq!(first, second);
    assert_eq!(first["input"], input_json());
    let pair = &first["pairs"][0];
    assert_eq!(pair["candidate"], pair["baseline"]);
    assert_eq!(pair["candidate"]["cardPlays"], 1);
    assert_eq!(pair["candidate"]["playActionIndices"], json!([0]));
    assert_eq!(pair["candidate"]["survivingUnits"], 1);
    assert_eq!(pair["candidate"]["outcome"]["type"], "timeout");
    assert_eq!(pair["delta"]["cardPlays"], 0);
    assert_eq!(pair["delta"]["survivingUnits"], 0);
    assert!(
        first["limitations"]
            .as_array()
            .expect("limitations")
            .iter()
            .any(|item| { item.as_str().expect("description").contains("source Card") })
    );
}

#[test]
fn card_evidence_rejects_invalid_or_ambiguous_inputs_before_self_play() {
    for (path, replacement, expected) in [
        ("/candidate/definition/cost", json!(31), "30 Mana"),
        ("/candidate/definition/kind/armor", json!(-1), "invalid"),
        (
            "/candidate/definition/id",
            json!("custom-other"),
            "identity",
        ),
        (
            "/candidate/definition/name",
            json!("Different"),
            "same source",
        ),
        ("/cases", json!([]), "case"),
        ("/maxActions", json!(0), "action limit"),
        ("/testedSide", json!("playerTwo"), "Duel"),
        ("/playerPolicyId", json!("missing"), "policy"),
        ("/policyConfig/policies/0/rules", json!([]), "no rules"),
        (
            "/cases/0/workshop/ruleset/turn/baseHeroMana",
            json!(0),
            "turn rules",
        ),
    ] {
        let mut value = input_json();
        *value.pointer_mut(path).expect("fixture path") = replacement;
        let input = serde_json::from_value(value).expect("typed input");
        let error = run_comparison(input)
            .expect_err("invalid input must fail")
            .to_string();
        assert!(error.contains(expected), "{path}: {error}");
    }
    let mut with_card = input_json();
    with_card["cases"][0]["workshop"]["cards"] = json!([with_card["candidate"]["definition"]]);
    let error = run_comparison(serde_json::from_value(with_card).expect("typed input"))
        .expect_err("case cannot replace exact source")
        .to_string();
    assert!(error.contains("empty authored Card list"), "{error}");
    for source in [
        json!({"type": "draft", "draftId": 0, "version": 1, "catalogId": "custom-duelist"}),
        json!({"type": "draft", "draftId": 1, "version": 0, "catalogId": "custom-duelist"}),
        json!({"type": "draft", "draftId": 1, "version": 1, "catalogId": "custom-other"}),
    ] {
        let mut value = input_json();
        value["candidate"]["source"] = source;
        assert!(run_comparison(serde_json::from_value(value).expect("typed input")).is_err());
    }
}

#[test]
fn changed_revision_reports_paired_damage_events_and_matchup_outcome_deltas() {
    let mut value = input_json();
    value["policyConfig"]["policies"][0]["rules"] = json!(["usefulSpell"]);
    value["candidate"]["source"]["revisionId"]["revision"] = json!(2);
    value["candidate"]["definition"]["kind"] = json!({
        "type": "spell", "range": 3, "priority": 0, "effect": {"type": "damage", "amount": 1}
    });
    value["baseline"]["definition"]["kind"] = value["candidate"]["definition"]["kind"].clone();
    value["baseline"]["definition"]["cost"] = json!(4);
    value["cases"][0]["boardSetup"] = json!("lowHpHero");
    let mut second_case = value["cases"][0].clone();
    second_case["workshop"]["seed"] = json!(42);
    value["cases"]
        .as_array_mut()
        .expect("cases")
        .push(second_case);
    let report = serde_json::to_value(
        run_comparison(serde_json::from_value(value.clone()).expect("typed input"))
            .expect("comparison runs"),
    )
    .expect("report serializes");
    assert_eq!(report["input"], value);
    for pair in report["pairs"].as_array().expect("pairs") {
        assert_eq!(pair["candidate"]["cardPlays"], 1);
        assert_eq!(pair["baseline"]["cardPlays"], 0);
        assert_eq!(pair["candidate"]["eventTotals"]["damageAmount"], 1);
        assert_eq!(pair["baseline"]["eventTotals"]["damageAmount"], 0);
        assert_eq!(pair["candidate"]["outcome"]["type"], "testedSideWin");
        assert_eq!(pair["baseline"]["outcome"]["type"], "timeout");
        assert_eq!(pair["delta"]["testedSideWins"], 1);
        assert_eq!(pair["delta"]["eventTotals"]["damageAmount"], 1);
    }
    let repeated = serde_json::to_value(
        run_comparison(serde_json::from_value(value).expect("typed input"))
            .expect("comparison repeats"),
    )
    .expect("report serializes");
    assert_eq!(report, repeated);
}

#[test]
fn dev_cli_writes_repeatable_comparison_reports_and_rejects_invalid_input() {
    let directory = std::env::temp_dir().join(format!(
        "card-evidence-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).expect("fixture directory");
    let input_path = directory.join("input.json");
    let output_path = directory.join("report");
    std::fs::write(
        &input_path,
        serde_json::to_vec(&input_json()).expect("fixture JSON"),
    )
    .expect("input file");
    let args = vec![
        "compare-cards".to_string(),
        "--input".to_string(),
        input_path.display().to_string(),
        "--out".to_string(),
        output_path.display().to_string(),
    ];
    super::run(args.clone()).expect("CLI comparison");
    let first = std::fs::read(output_path.join("card-comparison.json")).expect("report file");
    super::run(args).expect("CLI repeat");
    assert_eq!(
        first,
        std::fs::read(output_path.join("card-comparison.json")).expect("report file")
    );
    let report: Value = serde_json::from_slice(&first).expect("report JSON");
    assert_eq!(report["input"], input_json());
    let mut invalid = input_json();
    invalid["candidate"]["definition"]["cost"] = json!(31);
    std::fs::write(
        &input_path,
        serde_json::to_vec(&invalid).expect("fixture JSON"),
    )
    .expect("invalid input file");
    let invalid_output = directory.join("invalid-report");
    let error = super::run(vec![
        "compare-cards".into(),
        "--input".into(),
        input_path.display().to_string(),
        "--out".into(),
        invalid_output.display().to_string(),
    ])
    .expect_err("invalid input rejected");
    assert!(error.to_string().contains("30 Mana"));
    assert!(!invalid_output.exists());
    std::fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn later_draws_keep_the_exact_tested_revision_for_a_builtin_template() {
    let definition = rune_lanes_core::starter_card_definitions()
        .into_iter()
        .find(|card| card.id == "stoneguard")
        .expect("known template");
    let mut value = input_json();
    for arm in ["candidate", "baseline"] {
        value[arm]["definition"] = serde_json::to_value(&definition).expect("definition JSON");
        value[arm]["source"]["revisionId"]["cardId"] = json!("stoneguard");
    }
    value["candidate"]["source"]["revisionId"]["revision"] = json!(2);
    value["candidate"]["definition"]["cost"] = json!(1);
    value["baseline"]["definition"]["cost"] = json!(30);
    value["cases"][0]["boardSetup"] = json!("empty");
    for recipe in ["playerDeckRecipe", "opponentDeckRecipe"] {
        value["cases"][0]["workshop"][recipe] = json!([{"templateId": "stoneguard", "count": 30}]);
    }
    value["maxActions"] = json!(30);
    let report = serde_json::to_value(
        run_comparison(serde_json::from_value(value).expect("typed input"))
            .expect("comparison runs"),
    )
    .expect("report serializes");
    assert_eq!(report["pairs"][0]["baseline"]["cardPlays"], 0);
    assert!(
        report["pairs"][0]["candidate"]["cardPlays"]
            .as_u64()
            .expect("count")
            >= 2
    );
}

#[test]
fn card_evidence_records_healing_and_draw_events_without_effect_reimplementation() {
    for (effect, layout, field, expected) in [
        (
            json!({"type": "heal", "amount": 2}),
            "damagedAlly",
            "healingAmount",
            2,
        ),
        (
            json!({"type": "draw", "amount": 2}),
            "empty",
            "drawEvents",
            2,
        ),
    ] {
        let mut value = input_json();
        for arm in ["candidate", "baseline"] {
            value[arm]["definition"]["kind"] =
                json!({"type": "spell", "range": 3, "priority": 0, "effect": effect});
        }
        value["policyConfig"]["policies"][0]["rules"] = json!(["usefulSpell"]);
        value["cases"][0]["boardSetup"] = json!(layout);
        let report = serde_json::to_value(
            run_comparison(serde_json::from_value(value).expect("typed input"))
                .expect("comparison runs"),
        )
        .expect("report serializes");
        assert_eq!(
            report["pairs"][0]["candidate"]["eventTotals"][field],
            expected
        );
        assert_eq!(report["pairs"][0]["delta"]["eventTotals"][field], 0);
    }
}

#[test]
fn opponent_evidence_uses_real_priority_resolution_and_its_own_policy() {
    let mut value = input_json();
    value["testedSide"] = json!("opponent");
    value["maxActions"] = json!(6);
    value["cases"][0]["boardSetup"] = json!("lowHpHero");
    value["policyConfig"]["policies"]
        .as_array_mut()
        .expect("policies")
        .push(json!({"id": "spell", "rules": ["usefulSpell"]}));
    value["opponentPolicyId"] = json!("spell");
    for arm in ["candidate", "baseline"] {
        value[arm]["definition"]["kind"] = json!({"type": "spell", "range": 3, "priority": 0,
            "effect": {"type": "damage", "amount": 1}});
    }
    let report = serde_json::to_value(
        run_comparison(serde_json::from_value(value).expect("typed input"))
            .expect("comparison runs"),
    )
    .expect("report serializes");
    let game = &report["pairs"][0]["candidate"];
    assert_eq!(game["cardPlays"], 1);
    assert_eq!(game["playActionIndices"], json!([3]));
    assert_eq!(game["eventTotals"]["damageAmount"], 1);
    assert_eq!(game["outcome"]["type"], "testedSideWin");
}

#[test]
fn a_draft_version_cannot_claim_two_different_catalog_identities() {
    let mut value = input_json();
    value["candidate"]["source"] =
        json!({"type": "draft", "draftId": 1, "version": 1, "catalogId": "custom-duelist"});
    value["baseline"]["source"] =
        json!({"type": "draft", "draftId": 1, "version": 1, "catalogId": "custom-other"});
    value["baseline"]["definition"]["id"] = json!("custom-other");
    let error = run_comparison(serde_json::from_value(value).expect("typed input"))
        .expect_err("one draft version identifies one definition")
        .to_string();
    assert!(error.contains("same source"), "{error}");
}
