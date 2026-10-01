use super::*;

#[tokio::test]
async fn profile_can_update_display_name_and_generated_avatar() {
    let path = test_db_path("profile-update");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "profile@example.com").await;

    let (status, updated) = json_request(
            app.clone(),
            Request::builder()
                .method("PATCH")
                .uri("/api/profile")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"displayName":"Rune Pilot","handle":"rune-pilot","avatar":{"symbol":"shield","color":"indigo"},"preferredHeroType":"chronomancer","boardVisualMode":"2d"}"#,
                ))
                .expect("request should build"),
        )
        .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["handle"], "rune-pilot");
    assert_eq!(updated["displayName"], "Rune Pilot");
    assert_eq!(updated["avatar"]["symbol"], "shield");
    assert_eq!(updated["avatar"]["color"], "indigo");
    assert_eq!(updated["preferredHeroType"], "chronomancer");
    assert_eq!(updated["boardVisualMode"], "2d");

    let (status, loaded) = json_request(
        app,
        Request::builder()
            .uri("/api/profile")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(loaded["handle"], "rune-pilot");
    assert_eq!(loaded["displayName"], "Rune Pilot");
    assert_eq!(loaded["avatar"]["symbol"], "shield");
    assert_eq!(loaded["avatar"]["color"], "indigo");
    assert_eq!(loaded["preferredHeroType"], "chronomancer");
    assert_eq!(loaded["boardVisualMode"], "2d");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_can_update_preferred_hero_without_resubmitting_identity() {
    let path = test_db_path("profile-preferred-hero");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "hero@example.com").await;

    let (status, updated) = json_request(
        app,
        Request::builder()
            .method("PATCH")
            .uri("/api/profile/preferred-hero")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"pyromancer"}"#))
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["preferredHeroType"], "pyromancer");
    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_rejects_invalid_or_duplicate_public_handles() {
    let path = test_db_path("profile-handle-validation");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let first_token = register_test_account(app.clone(), "first@example.com").await;
    let second_token = register_test_account(app.clone(), "second@example.com").await;

    let (status, body) = json_request(
            app.clone(),
            Request::builder()
                .method("PATCH")
                .uri("/api/profile")
                .header("authorization", format!("Bearer {first_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"displayName":"First","handle":"no","avatar":{"symbol":"wand","color":"sky"},"preferredHeroType":"runekeeper","boardVisualMode":"3d"}"#,
                ))
                .expect("request should build"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(
        body["message"]
            .as_str()
            .expect("message should be a string")
            .contains("Public handle")
    );

    let (status, body) = json_request(
            app,
            Request::builder()
                .method("PATCH")
                .uri("/api/profile")
                .header("authorization", format!("Bearer {second_token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"displayName":"Second","handle":"first","avatar":{"symbol":"wand","color":"sky"},"preferredHeroType":"runekeeper","boardVisualMode":"3d"}"#,
                ))
                .expect("request should build"),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "Public handle is already taken.");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn existing_profiles_migrate_to_default_board_visual_mode() {
    let path = test_db_path("profile-board-mode-migration");
    {
        let connection = rusqlite::Connection::open(&path).expect("db should open");
        connection
            .execute_batch(
                "
                    CREATE TABLE users (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        email TEXT NOT NULL,
                        email_normalized TEXT NOT NULL UNIQUE,
                        password_hash TEXT NOT NULL,
                        display_name TEXT NOT NULL DEFAULT '',
                        avatar_symbol TEXT NOT NULL DEFAULT 'sparkles',
                        avatar_color TEXT NOT NULL DEFAULT 'emerald',
                        preferred_hero_type TEXT NOT NULL DEFAULT 'runekeeper',
                        created_at INTEGER NOT NULL DEFAULT (unixepoch())
                    );
                    INSERT INTO users (
                        email,
                        email_normalized,
                        password_hash,
                        display_name,
                        avatar_symbol,
                        avatar_color,
                        preferred_hero_type
                    )
                    VALUES (
                        'migrated@example.com',
                        'migrated@example.com',
                        'unused',
                        'Migrated',
                        'rune',
                        'sky',
                        'warden'
                    );
                    ",
            )
            .expect("old users table should seed");
    }

    let _store = SqliteMatchStore::new(&path).expect("store should migrate");
    let connection = rusqlite::Connection::open(&path).expect("db should reopen");
    let board_visual_mode: String = connection
        .query_row(
            "SELECT board_visual_mode FROM users WHERE email_normalized = 'migrated@example.com'",
            [],
            |row| row.get(0),
        )
        .expect("migrated user should load");
    let public_handle: String = connection
        .query_row(
            "SELECT public_handle FROM users WHERE email_normalized = 'migrated@example.com'",
            [],
            |row| row.get(0),
        )
        .expect("migrated public handle should load");

    assert_eq!(board_visual_mode, "3d");
    assert_eq!(public_handle, "migrated");

    let _ = fs::remove_file(path);
}

#[test]
fn hero_migration_preserves_durable_data_and_discards_legacy_matches() {
    let path = test_db_path("hero-migration");
    {
        let connection = rusqlite::Connection::open(&path).expect("database should open");
        connection
                .execute_batch(
                    r#"
                    CREATE TABLE users (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        email TEXT NOT NULL,
                        email_normalized TEXT NOT NULL UNIQUE,
                        public_handle TEXT UNIQUE,
                        password_hash TEXT NOT NULL,
                        display_name TEXT NOT NULL DEFAULT '',
                        avatar_symbol TEXT NOT NULL DEFAULT 'sparkles',
                        avatar_color TEXT NOT NULL DEFAULT 'emerald',
                        preferred_wizard_type TEXT NOT NULL DEFAULT 'runekeeper',
                        board_visual_mode TEXT NOT NULL DEFAULT '3d',
                        total_xp INTEGER NOT NULL DEFAULT 0,
                        created_at INTEGER NOT NULL DEFAULT (unixepoch())
                    );
                    INSERT INTO users (id, email, email_normalized, public_handle, password_hash, display_name, preferred_wizard_type)
                    VALUES (1, 'hero@example.com', 'hero@example.com', 'hero', 'hash', 'Hero', 'pyromancer');

                    CREATE TABLE deck_recipes (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id INTEGER NOT NULL,
                        name TEXT NOT NULL,
                        is_default INTEGER NOT NULL DEFAULT 0,
                        wizard_type TEXT NOT NULL DEFAULT 'runekeeper',
                        rune_ids_json TEXT NOT NULL DEFAULT '[]',
                        created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                        updated_at INTEGER NOT NULL DEFAULT (unixepoch())
                    );
                    INSERT INTO deck_recipes (id, user_id, name, wizard_type, rune_ids_json)
                    VALUES (10, 1, 'Legacy Deck', 'chronomancer', '["force"]');
                    CREATE TABLE deck_recipe_cards (
                        deck_id INTEGER NOT NULL,
                        template_id TEXT NOT NULL,
                        count INTEGER NOT NULL,
                        PRIMARY KEY (deck_id, template_id)
                    );

                    CREATE TABLE wizard_mastery (
                        user_id INTEGER NOT NULL,
                        wizard_type TEXT NOT NULL,
                        xp INTEGER NOT NULL DEFAULT 0,
                        PRIMARY KEY (user_id, wizard_type)
                    );
                    INSERT INTO wizard_mastery (user_id, wizard_type, xp) VALUES (1, 'pyromancer', 300);
                    CREATE TABLE wizard_skill_unlocks (
                        user_id INTEGER NOT NULL,
                        wizard_type TEXT NOT NULL,
                        node_id TEXT NOT NULL,
                        unlocked_at INTEGER NOT NULL,
                        PRIMARY KEY (user_id, wizard_type, node_id)
                    );
                    INSERT INTO wizard_skill_unlocks (user_id, wizard_type, node_id, unlocked_at)
                    VALUES (1, 'pyromancer', 'pyromancer-heated-focus', 123);
                    CREATE TABLE wizard_rune_loadouts (
                        user_id INTEGER NOT NULL,
                        wizard_type TEXT NOT NULL,
                        rune_ids_json TEXT NOT NULL,
                        updated_at INTEGER NOT NULL,
                        PRIMARY KEY (user_id, wizard_type)
                    );
                    INSERT INTO wizard_rune_loadouts (user_id, wizard_type, rune_ids_json, updated_at)
                    VALUES (1, 'pyromancer', '["vitality"]', 456);
                    CREATE TABLE match_xp_awards (
                        match_id TEXT NOT NULL,
                        user_id INTEGER NOT NULL,
                        account_xp INTEGER NOT NULL,
                        wizard_type TEXT NOT NULL,
                        wizard_xp INTEGER NOT NULL,
                        won INTEGER NOT NULL,
                        awarded_at INTEGER NOT NULL,
                        PRIMARY KEY (match_id, user_id)
                    );
                    INSERT INTO match_xp_awards (match_id, user_id, account_xp, wizard_type, wizard_xp, won, awarded_at)
                    VALUES ('legacy-match', 1, 150, 'pyromancer', 150, 1, 789);

                    CREATE TABLE matches (
                        id TEXT PRIMARY KEY NOT NULL,
                        snapshot_json TEXT NOT NULL,
                        initial_snapshot_json TEXT,
                        completed_at INTEGER,
                        mode TEXT NOT NULL DEFAULT 'solo',
                        owner_user_id INTEGER,
                        created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                        updated_at INTEGER NOT NULL DEFAULT (unixepoch())
                    );
                    INSERT INTO matches (id, snapshot_json, initial_snapshot_json)
                    VALUES ('legacy-match', '{"player":{"wizard":{"id":"player-wizard","wizardType":"pyromancer"}}}', '{}');
                    CREATE TABLE match_actions (
                        match_id TEXT NOT NULL,
                        action_index INTEGER NOT NULL,
                        request_json TEXT NOT NULL,
                        accepted_at INTEGER NOT NULL,
                        PRIMARY KEY (match_id, action_index)
                    );
                    INSERT INTO match_actions (match_id, action_index, request_json, accepted_at)
                    VALUES ('legacy-match', 1, '{"pieceId":"player-wizard"}', 1);
                    CREATE TABLE match_replay_frames (
                        match_id TEXT NOT NULL,
                        frame_index INTEGER NOT NULL,
                        action_index INTEGER,
                        event_json TEXT NOT NULL,
                        snapshot_json TEXT NOT NULL,
                        created_at INTEGER NOT NULL,
                        PRIMARY KEY (match_id, frame_index)
                    );
                    INSERT INTO match_replay_frames (match_id, frame_index, event_json, snapshot_json, created_at)
                    VALUES ('legacy-match', 0, '{"type":"matchCreated"}', '{"player":{"wizard":{}}}', 1);
                    CREATE TABLE shared_matches (
                        match_id TEXT PRIMARY KEY NOT NULL,
                        status TEXT NOT NULL,
                        creator_user_id INTEGER,
                        created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                        updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
                        forfeit_winner TEXT
                    );
                    INSERT INTO shared_matches (match_id, status) VALUES ('legacy-match', 'setup');
                    CREATE TABLE match_seats (
                        match_id TEXT NOT NULL,
                        side TEXT NOT NULL,
                        seat_token TEXT NOT NULL UNIQUE,
                        wizard_type TEXT,
                        joined_at INTEGER,
                        last_seen_at INTEGER,
                        disconnected_at INTEGER,
                        deck_recipe_name TEXT,
                        deck_recipe_snapshot_json TEXT,
                        progression_loadout_json TEXT,
                        PRIMARY KEY (match_id, side)
                    );
                    INSERT INTO match_seats (match_id, side, seat_token, wizard_type)
                    VALUES ('legacy-match', 'player', 'seat', 'pyromancer');
                    "#,
                )
                .expect("legacy schema should seed");
    }

    let mut store = SqliteMatchStore::new(&path).expect("migration should succeed");
    let connection = store.connection_mut();

    let preferred_hero_type: String = connection
        .query_row(
            "SELECT preferred_hero_type FROM users WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .expect("preferred hero should migrate");
    assert_eq!(preferred_hero_type, "pyromancer");
    let deck_hero_type: String = connection
        .query_row(
            "SELECT hero_type FROM deck_recipes WHERE id = 10",
            [],
            |row| row.get(0),
        )
        .expect("deck hero should migrate");
    assert_eq!(deck_hero_type, "chronomancer");
    let mastery_xp: i64 = connection
        .query_row(
            "SELECT xp FROM hero_mastery WHERE user_id = 1 AND hero_type = 'pyromancer'",
            [],
            |row| row.get(0),
        )
        .expect("hero mastery should migrate");
    assert_eq!(mastery_xp, 300);
    let unlocked_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM hero_skill_unlocks", [], |row| {
            row.get(0)
        })
        .expect("hero skill unlocks should migrate");
    assert_eq!(unlocked_count, 1);
    let rune_ids_json: String = connection
            .query_row(
                "SELECT rune_ids_json FROM hero_rune_loadouts WHERE user_id = 1 AND hero_type = 'pyromancer'",
                [],
                |row| row.get(0),
            )
            .expect("hero rune loadout should migrate");
    assert_eq!(rune_ids_json, "[\"vitality\"]");
    let award_hero_type: String = connection
        .query_row(
            "SELECT hero_type FROM match_xp_awards WHERE match_id = 'legacy-match'",
            [],
            |row| row.get(0),
        )
        .expect("match award hero type should migrate");
    assert_eq!(award_hero_type, "pyromancer");

    for table in [
        "matches",
        "match_actions",
        "match_replay_frames",
        "shared_matches",
        "match_seats",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("table should be queryable");
        assert_eq!(count, 0, "{table} should be discarded");
    }

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_rejects_unknown_generated_avatar_values() {
    let path = test_db_path("profile-avatar-invalid");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "avatar@example.com").await;

    let (status, body) = json_request(
            app,
            Request::builder()
                .method("PATCH")
                .uri("/api/profile")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"displayName":"Avatar","handle":"avatar","avatar":{"symbol":"dragon","color":"void"},"preferredHeroType":"runekeeper"}"#,
                ))
                .expect("request should build"),
        )
        .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["message"], "Choose a valid generated avatar.");

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_rejects_unknown_preferred_hero_type() {
    let path = test_db_path("profile-hero-invalid");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "hero-invalid@example.com").await;

    let (status, _body) = json_request(
            app,
            Request::builder()
                .method("PATCH")
                .uri("/api/profile")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"displayName":"Avatar","handle":"avatar","avatar":{"symbol":"wand","color":"sky"},"preferredHeroType":"stormcaller"}"#,
                ))
                .expect("request should build"),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_rejects_unknown_board_visual_mode() {
    let path = test_db_path("profile-board-mode-invalid");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "board-invalid@example.com").await;

    let (status, _body) = json_request(
            app,
            Request::builder()
                .method("PATCH")
                .uri("/api/profile")
                .header("authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"displayName":"Avatar","handle":"avatar","avatar":{"symbol":"wand","color":"sky"},"preferredHeroType":"runekeeper","boardVisualMode":"cinematic"}"#,
                ))
                .expect("request should build"),
        )
        .await;

    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn signed_in_match_archive_only_lists_that_users_matches() {
    let path = test_db_path("auth-archive-scope");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let first_token = register_test_account(app.clone(), "first@example.com").await;
    let second_token = register_test_account(app.clone(), "second@example.com").await;

    let (_, first_match) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("authorization", format!("Bearer {first_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let first_match_id = first_match["matchId"].as_str().unwrap().to_string();

    let (_, second_match) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("authorization", format!("Bearer {second_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let second_match_id = second_match["matchId"].as_str().unwrap().to_string();

    let (status, first_archive) = json_request(
        app.clone(),
        Request::builder()
            .uri("/api/matches")
            .header("authorization", format!("Bearer {first_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first_archive["matches"].as_array().unwrap().len(), 1);
    assert_eq!(first_archive["matches"][0]["matchId"], first_match_id);

    let (status, second_load_from_first_user) = json_request(
        app,
        Request::builder()
            .uri(format!("/api/matches/{second_match_id}"))
            .header("authorization", format!("Bearer {first_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        second_load_from_first_user["message"],
        format!("Match {second_match_id} was not found")
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_matches_list_owned_solo_matches_only() {
    let path = test_db_path("profile-solo-history");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));

    let first_token = register_test_account(app.clone(), "history-first@example.com").await;
    let second_token = register_test_account(app.clone(), "history-second@example.com").await;

    let (_, first_match) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("authorization", format!("Bearer {first_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;
    let first_match_id = first_match["matchId"].as_str().unwrap().to_string();

    let _ = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/matches")
            .header("authorization", format!("Bearer {second_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    let (status, profile_matches) = json_request(
        app,
        Request::builder()
            .uri("/api/profile/matches")
            .header("authorization", format!("Bearer {first_token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile_matches["matches"].as_array().unwrap().len(), 1);
    assert_eq!(profile_matches["matches"][0]["matchId"], first_match_id);
    assert_eq!(
        profile_matches["matches"][0]["viewerHeroTypes"][0],
        "runekeeper"
    );
    assert_eq!(
        profile_matches["matches"][0]["opposingHeroTypes"][0],
        "runekeeper"
    );
    assert_eq!(
        profile_matches["matches"][0]["viewerDeckName"],
        "Balanced Starter"
    );

    let _ = fs::remove_file(path);
}

#[tokio::test]
async fn profile_matches_include_completed_shared_matches_joined_by_account() {
    let path = test_db_path("profile-shared-history");
    let app = create_app(SqliteMatchStore::new(&path).expect("store should open"));
    let token = register_test_account(app.clone(), "shared-creator@example.com").await;

    let (_, created) = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri("/api/shared-matches")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;
    let match_id = created["matchId"].as_str().expect("match id should exist");
    let player_token = seat_token_from_url(
        created["playerSeatUrl"]
            .as_str()
            .expect("player URL exists"),
    );
    let opponent_token = seat_token_from_url(
        created["inviteSeatUrl"]
            .as_str()
            .expect("invite URL exists"),
    );

    let _ = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{opponent_token}/join"
            ))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"pyromancer"}"#))
            .expect("request should build"),
    )
    .await;
    let _ = json_request(
        app.clone(),
        Request::builder()
            .method("POST")
            .uri(format!(
                "/api/shared-matches/{match_id}/seats/{player_token}/join"
            ))
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"heroType":"chronomancer"}"#))
            .expect("request should build"),
    )
    .await;

    let connection = rusqlite::Connection::open(&path).expect("test database should open");
    connection
        .execute(
            "UPDATE shared_matches SET status = 'completed' WHERE match_id = ?1",
            rusqlite::params![match_id],
        )
        .expect("shared match should be markable complete");

    let (status, profile_matches) = json_request(
        app,
        Request::builder()
            .uri("/api/profile/matches")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .expect("request should build"),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(profile_matches["matches"].as_array().unwrap().len(), 1);
    assert_eq!(profile_matches["matches"][0]["matchId"], match_id);

    let _ = fs::remove_file(path);
}
