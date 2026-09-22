use super::*;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

fn test_db_path(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    path.push(format!(
        "rune-lanes-api-{name}-{}.sqlite3",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    path
}

async fn json_request(app: Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = app.oneshot(request).await.expect("request should complete");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should read");
    let json = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, json)
}

async fn post_match_action(
    app: Router,
    match_id: &str,
    action_json: &'static str,
) -> (StatusCode, serde_json::Value) {
    json_request(
        app,
        Request::builder()
            .method("POST")
            .uri(format!("/api/matches/{match_id}/actions"))
            .header("content-type", "application/json")
            .body(Body::from(action_json))
            .expect("request should build"),
    )
    .await
}

async fn advance_solo_match_to_player_turn(app: Router, match_id: &str) -> serde_json::Value {
    let mut last = serde_json::Value::Null;
    for _ in 0..50 {
        let match_state = &last["matchState"];
        if match_state["activeSide"] == "player"
            && match_state["actionStack"]
                .as_array()
                .is_some_and(Vec::is_empty)
        {
            return last;
        }

        let action = if match_state["prioritySide"] == "player" {
            r#"{"type":"passPriority"}"#
        } else {
            r#"{"type":"advanceAi"}"#
        };
        let (status, acted) = post_match_action(app.clone(), match_id, action).await;
        assert_eq!(status, StatusCode::OK);
        last = acted;
    }

    panic!("AI did not return control to the player");
}

async fn end_turn_from_card_play(app: Router, match_id: &str) -> serde_json::Value {
    let (status, _) = post_match_action(app.clone(), match_id, r#"{"type":"startCardPlay"}"#).await;
    assert_eq!(status, StatusCode::OK);
    let (status, acted) = post_match_action(app, match_id, r#"{"type":"endTurn"}"#).await;
    assert_eq!(status, StatusCode::OK);
    acted
}

fn seat_token_from_url(url: &str) -> &str {
    url.rsplit('/')
        .next()
        .expect("seat URL should end in token")
}

fn complete_match_by_forfeit(path: &std::path::Path, match_id: &str, winner: Side) {
    let mut store = SqliteMatchStore::new(path).expect("store should reopen");
    let mut stored = store
        .load_match(match_id)
        .expect("match lookup should succeed")
        .expect("match should exist");
    let frames =
        rune_lanes_core::test_support::forfeit_match(&mut stored.state, winner, 0);
    store
        .save_custom_action_and_replay_frames(
            match_id,
            0,
            r#"{"type":"testForfeit"}"#,
            &stored.state,
            &frames,
        )
        .expect("completed match should save");
}

async fn register_test_account(app: Router, email: &str) -> String {
    let (status, body) = json_request(
        app,
        Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header("content-type", "application/json")
            .body(Body::from(format!(
                r#"{{"email":"{email}","password":"password123"}}"#
            )))
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body["token"]
        .as_str()
        .expect("token should exist")
        .to_string()
}

fn default_preferences_payload() -> &'static str {
    r#"{"theme":"system","motion":"system","animationSpeed":"normal","boardScale":"normal","boardVisualMode":"3d","hotkeys":[{"commandId":"cursorNorthwest","binding":"Q"},{"commandId":"cursorNortheast","binding":"W"},{"commandId":"cursorEast","binding":"E"},{"commandId":"cursorWest","binding":"A"},{"commandId":"cursorSouthwest","binding":"S"},{"commandId":"cursorSoutheast","binding":"D"},{"commandId":"confirm","binding":"Enter"},{"commandId":"cancel","binding":"Escape"},{"commandId":"endTurn","binding":"T"},{"commandId":"passPriority","binding":"P"},{"commandId":"openCardInfo","binding":"I"},{"commandId":"openSettings","binding":","},{"commandId":"openCatalog","binding":"C"},{"commandId":"openDecks","binding":"K"},{"commandId":"openMatchArchive","binding":"M"}]}"#
}

fn custom_preferences_payload() -> &'static str {
    r#"{"theme":"highContrast","motion":"reduced","animationSpeed":"fast","boardScale":"large","boardVisualMode":"2d","hotkeys":[{"commandId":"cursorNorthwest","binding":"U"},{"commandId":"cursorNortheast","binding":"O"},{"commandId":"cursorEast","binding":"L"},{"commandId":"cursorWest","binding":"J"},{"commandId":"cursorSouthwest","binding":"N"},{"commandId":"cursorSoutheast","binding":"B"},{"commandId":"confirm","binding":"Enter"},{"commandId":"cancel","binding":"Escape"},{"commandId":"endTurn","binding":"Y"},{"commandId":"passPriority","binding":"R"},{"commandId":"openCardInfo","binding":"F"},{"commandId":"openSettings","binding":","},{"commandId":"openCatalog","binding":"G"},{"commandId":"openDecks","binding":"H"},{"commandId":"openMatchArchive","binding":"V"}]}"#
}

mod auth;
mod decks;
mod loadout_resolution;
mod matches;
mod preferences;
mod profile;
mod scenarios;
mod shared_matches;
