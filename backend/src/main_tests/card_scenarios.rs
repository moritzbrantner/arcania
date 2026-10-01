use super::card_transfers::{publish_transfer_fixture, transfer_request};
use super::*;
use rune_lanes_core::{
    ActionTarget, HexCoord, Side, commands::GameCommand,
    workshop::card_scenarios::CardTestBoardSetup,
};

fn scenario_request(
    version: u64,
    board_setup: CardTestBoardSetup,
    command: GameCommand,
) -> serde_json::Value {
    serde_json::json!({
        "version": version,
        "scenario": {
            "workshop": {
                "seed": 42, "playerHero": "runekeeper", "opponentHero": "runekeeper",
                "ruleset": rune_lanes_core::rules::CURRENT_RULESET, "cards": []
            },
            "boardSetup": board_setup
        },
        "commands": [{"side": Side::Player, "command": command}]
    })
}

fn summon_request(version: u64) -> serde_json::Value {
    scenario_request(
        version,
        CardTestBoardSetup::Empty,
        GameCommand::PlayCard {
            card_id: "player-custom-0".into(),
            target: ActionTarget::Hex {
                coord: HexCoord { q: 0, r: 1 },
            },
        },
    )
}

#[tokio::test]
async fn draft_scenario_previews_and_command_traces_reproduce_authoritative_event_state() {
    let path = test_db_path("draft-scenario-replay");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "scenario-replay@example.com").await;
    let (draft, _) = publish_transfer_fixture(app.clone(), &owner).await;
    let uri = format!("/api/card-drafts/{}/scenarios/run", draft["id"]);
    for layout in [
        CardTestBoardSetup::Empty,
        CardTestBoardSetup::AdjacentEnemy,
        CardTestBoardSetup::ClusteredEnemies,
        CardTestBoardSetup::DamagedAlly,
        CardTestBoardSetup::LowHpHero,
    ] {
        let mut preview_request = summon_request(1);
        preview_request["scenario"]["boardSetup"] = serde_json::json!(layout);
        preview_request["commands"] = serde_json::json!([]);
        let (status, preview) =
            transfer_request(app.clone(), &owner, "POST", &uri, Some(preview_request)).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(preview["initialState"], preview["finalState"]);
        assert_eq!(preview["events"].as_array().unwrap().len(), 1);
        assert_eq!(preview["replayFrames"].as_array().unwrap().len(), 1);
    }
    let mut request = summon_request(1);
    for command in [
        GameCommand::StartAttackPhase,
        GameCommand::StartCardPlay,
        GameCommand::EndTurn,
    ] {
        request["commands"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"side": Side::Player, "command": command}));
    }
    let (status, result) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(request.clone())).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["finalState"]["activeSide"], "opponent");
    assert_eq!(result["events"].as_array().unwrap().len(), 5);
    let events: Vec<rune_lanes_core::event_sourcing::EventEnvelope> =
        serde_json::from_value(result["events"].clone()).unwrap();
    let restored = rune_lanes_core::event_sourcing::EventSourcedMatch::rehydrate(&events).unwrap();
    assert_eq!(
        restored.queries().public_match(Side::Player),
        result["finalState"]
    );
    let frames = result["replayFrames"].as_array().unwrap();
    assert_eq!(frames.last().unwrap()["matchState"], result["finalState"]);
    let (status, repeated) = transfer_request(app, &owner, "POST", &uri, Some(request)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated, result);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn owned_persisted_draft_runs_real_core_commands_without_changing_content() {
    let path = test_db_path("draft-card-scenario");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "draft-scenario@example.com").await;
    let (draft, _) = publish_transfer_fixture(app.clone(), &owner).await;
    let (_, history_before) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        "/api/card-revisions/ash-duelist",
        None,
    )
    .await;
    let uri = format!("/api/card-drafts/{}/scenarios/run", draft["id"]);
    let (status, result) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(summon_request(1))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["draftId"], draft["id"]);
    assert_eq!(result["draftVersion"], 1);
    assert_eq!(result["initialState"]["player"]["mana"], 3);
    assert_eq!(result["finalState"]["player"]["mana"], 2);
    assert_eq!(
        result["finalState"]["board"]["units"][0]["templateId"],
        draft["catalogId"]
    );
    assert_eq!(result["events"].as_array().unwrap().len(), 2);
    assert_eq!(result["events"][1]["event"]["type"], "commandAccepted");
    assert!(result["replayFrames"].as_array().unwrap().len() >= 2);
    let (_, unchanged) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(unchanged, draft);
    let (_, history_after) =
        transfer_request(app, &owner, "GET", "/api/card-revisions/ash-duelist", None).await;
    assert_eq!(history_after, history_before);
    let reopened = create_app(SqliteMatchStore::new(&path).unwrap());
    let (status, repeated) =
        transfer_request(reopened, &owner, "POST", &uri, Some(summon_request(1))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(repeated, result);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn draft_scenarios_enforce_account_and_exact_draft_version_access() {
    let path = test_db_path("draft-scenario-access");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "scenario-owner@example.com").await;
    let other = register_test_account(app.clone(), "scenario-other@example.com").await;
    let (draft, _) = publish_transfer_fixture(app.clone(), &owner).await;
    let uri = format!("/api/card-drafts/{}/scenarios/run", draft["id"]);
    for (token, target, expected) in [
        (other.as_str(), uri.as_str(), StatusCode::NOT_FOUND),
        ("", uri.as_str(), StatusCode::UNAUTHORIZED),
        (
            owner.as_str(),
            "/api/card-drafts/999999/scenarios/run",
            StatusCode::NOT_FOUND,
        ),
    ] {
        let (status, _) =
            transfer_request(app.clone(), token, "POST", target, Some(summon_request(1))).await;
        assert_eq!(status, expected);
    }
    let mut definition = draft["definition"].clone();
    definition["cost"] = serde_json::json!(2);
    definition["name"] = serde_json::json!("Updated Duelist");
    let (status, updated) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &format!("/api/card-drafts/{}", draft["id"]),
        Some(serde_json::json!({"version": 1, "definition": definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, error) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(summon_request(1))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("current version is 2")
    );
    let (status, result) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(summon_request(2))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["draftVersion"], 2);
    assert_eq!(result["finalState"]["player"]["mana"], 1);
    assert_eq!(
        result["finalState"]["board"]["units"][0]["name"],
        "Updated Duelist"
    );
    let (_, unchanged) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(unchanged, updated);
    let (_, history) =
        transfer_request(app, &owner, "GET", "/api/card-revisions/ash-duelist", None).await;
    assert_eq!(history["revisions"].as_array().unwrap().len(), 1);
    assert_eq!(history["revisions"][0]["revision"]["definition"]["cost"], 1);
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn invalid_scenario_inputs_and_actions_return_core_errors_without_persistence() {
    let path = test_db_path("draft-scenario-errors");
    let app = create_app(SqliteMatchStore::new(&path).unwrap());
    let owner = register_test_account(app.clone(), "scenario-errors@example.com").await;
    let (draft, _) = publish_transfer_fixture(app.clone(), &owner).await;
    let uri = format!("/api/card-drafts/{}/scenarios/run", draft["id"]);
    let mut invalid_action = summon_request(1);
    invalid_action["commands"][0]["command"]["cardId"] = serde_json::json!("missing-card");
    let (status, error) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(invalid_action)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(
        error["message"],
        "Card test command 0 failed: rule rejected event-sourced command: card is no longer in hand"
    );
    let mut invalid_setup = summon_request(1);
    invalid_setup["scenario"]["workshop"]["ruleset"]["turn"]["baseHeroMana"] = serde_json::json!(0);
    let (status, _) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(invalid_setup)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let mut supplied_card = summon_request(1);
    supplied_card["scenario"]["workshop"]["cards"] = serde_json::json!([draft["definition"]]);
    let (status, error) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(supplied_card)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("owned persisted draft")
    );
    for (pointer, value) in [
        ("/scenario/boardSetup", serde_json::json!("missing-layout")),
        (
            "/commands/0/command/type",
            serde_json::json!("missing-command"),
        ),
    ] {
        let mut malformed = summon_request(1);
        *malformed.pointer_mut(pointer).unwrap() = value;
        let (status, _) =
            transfer_request(app.clone(), &owner, "POST", &uri, Some(malformed)).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let (_, unchanged) = transfer_request(
        app.clone(),
        &owner,
        "GET",
        &format!("/api/card-drafts/{}", draft["id"]),
        None,
    )
    .await;
    assert_eq!(unchanged, draft);
    let mut invalid_definition = draft["definition"].clone();
    invalid_definition["kind"]["armor"] = serde_json::json!(-1);
    let (status, invalid_draft) = transfer_request(
        app.clone(),
        &owner,
        "PATCH",
        &format!("/api/card-drafts/{}", draft["id"]),
        Some(serde_json::json!({"version": 1, "definition": invalid_definition})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        !invalid_draft["validationErrors"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let (status, _) =
        transfer_request(app.clone(), &owner, "POST", &uri, Some(summon_request(2))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (_, history) =
        transfer_request(app, &owner, "GET", "/api/card-revisions/ash-duelist", None).await;
    assert_eq!(history["revisions"].as_array().unwrap().len(), 1);
    let _ = fs::remove_file(path);
}
