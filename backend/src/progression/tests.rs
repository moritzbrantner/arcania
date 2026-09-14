use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::identity;
use crate::match_store::SqliteMatchStore;

#[test]
fn account_level_uses_fast_early_curve() {
    assert_eq!(summary_for_xp(0).level, 1);
    assert_eq!(summary_for_xp(100).level, 2);
    assert_eq!(summary_for_xp(299).level, 2);
    assert_eq!(summary_for_xp(300).level, 3);
    assert_eq!(summary_for_xp(4_500).level, 10);
    assert_eq!(summary_for_xp(4_500).rune_slots, 2);
}

#[test]
fn new_hero_skill_trees_have_seven_nodes_and_apply_effects() {
    let trees = skill_trees();
    for hero_type in [HeroType::Barbarian, HeroType::Archer, HeroType::Builder] {
        let tree = trees
            .iter()
            .find(|candidate| candidate.hero_type == hero_type)
            .expect("new hero tree should exist");
        assert_eq!(tree.nodes.len(), 7);
    }

    let barbarian_effects = effects_for(
        &[],
        &[
            "barbarian-brutal-stamina".to_string(),
            "barbarian-weapon-practice".to_string(),
            "barbarian-battle-hunger".to_string(),
            "barbarian-warband-hide".to_string(),
            "barbarian-opening-rage".to_string(),
            "barbarian-deep-cuts".to_string(),
        ],
    );
    assert_eq!(barbarian_effects.max_hp_delta, 2);
    assert_eq!(barbarian_effects.attack_delta, 1);
    assert_eq!(barbarian_effects.mana_delta, 1);
    assert_eq!(barbarian_effects.summoned_unit_armor_delta, 1);
    assert_eq!(barbarian_effects.opening_hand_delta, 1);
    assert_eq!(barbarian_effects.spell_damage_delta, 1);

    let archer_effects = effects_for(
        &[],
        &[
            "archer-fleet-footing".to_string(),
            "archer-keen-shot".to_string(),
            "archer-scout-cache".to_string(),
            "archer-trail-rations".to_string(),
            "archer-screening-line".to_string(),
            "archer-light-armor".to_string(),
        ],
    );
    assert_eq!(archer_effects.max_ap_delta, 1);
    assert_eq!(archer_effects.spell_damage_delta, 1);
    assert_eq!(archer_effects.opening_hand_delta, 1);
    assert_eq!(archer_effects.mana_delta, 1);
    assert_eq!(archer_effects.first_summoned_unit_armor_delta, 1);
    assert_eq!(archer_effects.max_hp_delta, 1);

    let builder_effects = effects_for(
        &[],
        &[
            "builder-reinforced-frame".to_string(),
            "builder-supply-cache".to_string(),
            "builder-work-crew-drill".to_string(),
            "builder-first-wall".to_string(),
            "builder-field-manual".to_string(),
            "builder-tool-ready".to_string(),
        ],
    );
    assert_eq!(builder_effects.max_hp_delta, 2);
    assert_eq!(builder_effects.mana_delta, 1);
    assert_eq!(builder_effects.summoned_unit_armor_delta, 1);
    assert_eq!(builder_effects.first_summoned_unit_armor_delta, 1);
    assert_eq!(builder_effects.opening_hand_delta, 1);
    assert_eq!(builder_effects.max_ap_delta, 1);
}

#[test]
fn match_loadout_rejects_locked_and_duplicate_runes() {
    let mut connection = Connection::open_in_memory().expect("in-memory database should open");
    identity::migrate(&connection).expect("identity schema should migrate");
    migrate(&connection).expect("progression schema should migrate");
    insert_user(&connection, 1, 0);

    let mut progression = ProgressionModule::new(&mut connection);
    let locked = progression.match_loadout(
        Some(1),
        HeroType::Runekeeper,
        Some(vec!["vitality".to_string()]),
    );
    assert!(matches!(locked, Err(ProgressionError::LockedRune(_))));

    insert_user(&connection, 2, 10_000);
    let mut progression = ProgressionModule::new(&mut connection);
    let duplicate = progression.match_loadout(
        Some(2),
        HeroType::Runekeeper,
        Some(vec!["vitality".to_string(), "vitality".to_string()]),
    );
    assert!(matches!(duplicate, Err(ProgressionError::DuplicateRune(_))));
}

#[test]
fn skill_unlock_spends_hero_mastery_points_and_respec_restores_them() {
    let mut connection = Connection::open_in_memory().expect("in-memory database should open");
    identity::migrate(&connection).expect("identity schema should migrate");
    migrate(&connection).expect("progression schema should migrate");
    insert_user(&connection, 1, 0);
    connection
        .execute(
            "INSERT INTO hero_mastery (user_id, hero_type, xp) VALUES (1, 'pyromancer', 300)",
            [],
        )
        .expect("hero mastery should insert");

    let mut progression = ProgressionModule::new(&mut connection);
    let response = progression
        .unlock_skill(1, HeroType::Pyromancer, "pyromancer-heated-focus")
        .expect("skill should unlock");
    let pyromancer = response
        .heroes
        .iter()
        .find(|hero| hero.hero_type == HeroType::Pyromancer)
        .expect("pyromancer progression should be present");
    assert_eq!(pyromancer.available_skill_points, 1);
    assert!(
        pyromancer
            .unlocked_skill_ids
            .iter()
            .any(|node_id| node_id == "pyromancer-heated-focus")
    );

    let response = progression
        .respec_hero(1, HeroType::Pyromancer)
        .expect("respec should succeed");
    let pyromancer = response
        .heroes
        .iter()
        .find(|hero| hero.hero_type == HeroType::Pyromancer)
        .expect("pyromancer progression should be present");
    assert_eq!(pyromancer.available_skill_points, 2);
    assert!(pyromancer.unlocked_skill_ids.is_empty());
}

#[test]
fn hero_appearance_selection_respects_mastery_unlocks() {
    let mut connection = Connection::open_in_memory().expect("in-memory database should open");
    identity::migrate(&connection).expect("identity schema should migrate");
    migrate(&connection).expect("progression schema should migrate");
    insert_user(&connection, 1, 0);
    connection
        .execute(
            "INSERT INTO hero_mastery (user_id, hero_type, xp) VALUES (1, 'pyromancer', 100)",
            [],
        )
        .expect("hero mastery should insert");

    {
        let mut progression = ProgressionModule::new(&mut connection);
        let response = progression
            .load_for_user(1)
            .expect("progression should load");
        let pyromancer = response
            .hero_appearances
            .iter()
            .find(|hero| hero.hero_type == HeroType::Pyromancer)
            .expect("pyromancer appearances should load");
        assert_eq!(pyromancer.selected_appearance_id, "pyromancer-base");
        assert!(
            pyromancer
                .appearances
                .iter()
                .any(|appearance| appearance.id == "pyromancer-base" && appearance.unlocked)
        );

        let locked = progression.save_hero_appearance_selection(
            1,
            HeroType::Pyromancer,
            SaveHeroAppearanceRequest {
                appearance_id: "pyromancer-ember-mantle".to_string(),
            },
        );
        assert!(matches!(
            locked,
            Err(ProgressionError::LockedHeroAppearance(_))
        ));
    }

    connection
        .execute(
            "UPDATE hero_mastery SET xp = 300 WHERE user_id = 1 AND hero_type = 'pyromancer'",
            [],
        )
        .expect("hero mastery should update");
    {
        let mut progression = ProgressionModule::new(&mut connection);
        let response = progression
            .save_hero_appearance_selection(
                1,
                HeroType::Pyromancer,
                SaveHeroAppearanceRequest {
                    appearance_id: "pyromancer-ember-mantle".to_string(),
                },
            )
            .expect("level 3 appearance should save");
        let pyromancer = response
            .hero_appearances
            .iter()
            .find(|hero| hero.hero_type == HeroType::Pyromancer)
            .expect("pyromancer appearances should load");
        assert_eq!(pyromancer.selected_appearance_id, "pyromancer-ember-mantle");
    }

    connection
        .execute(
            "UPDATE hero_mastery SET xp = 1500 WHERE user_id = 1 AND hero_type = 'pyromancer'",
            [],
        )
        .expect("hero mastery should update");
    let mut progression = ProgressionModule::new(&mut connection);
    let response = progression
        .save_hero_appearance_selection(
            1,
            HeroType::Pyromancer,
            SaveHeroAppearanceRequest {
                appearance_id: "pyromancer-inferno-crown".to_string(),
            },
        )
        .expect("level 6 appearance should save");
    let pyromancer = response
        .hero_appearances
        .iter()
        .find(|hero| hero.hero_type == HeroType::Pyromancer)
        .expect("pyromancer appearances should load");
    assert_eq!(
        pyromancer.selected_appearance_id,
        "pyromancer-inferno-crown"
    );
}

#[test]
fn hero_appearance_selection_rejects_unknown_and_wrong_hero_ids() {
    let mut connection = Connection::open_in_memory().expect("in-memory database should open");
    identity::migrate(&connection).expect("identity schema should migrate");
    migrate(&connection).expect("progression schema should migrate");
    insert_user(&connection, 1, 0);
    connection
        .execute(
            "INSERT INTO hero_mastery (user_id, hero_type, xp) VALUES (1, 'pyromancer', 1500)",
            [],
        )
        .expect("hero mastery should insert");

    let mut progression = ProgressionModule::new(&mut connection);
    let unknown = progression.save_hero_appearance_selection(
        1,
        HeroType::Pyromancer,
        SaveHeroAppearanceRequest {
            appearance_id: "pyromancer-missing".to_string(),
        },
    );
    assert!(matches!(
        unknown,
        Err(ProgressionError::UnknownHeroAppearance(_))
    ));

    let wrong_hero = progression.save_hero_appearance_selection(
        1,
        HeroType::Pyromancer,
        SaveHeroAppearanceRequest {
            appearance_id: "warden-mossguard".to_string(),
        },
    );
    assert!(matches!(
        wrong_hero,
        Err(ProgressionError::MismatchedHeroAppearance { .. })
    ));
}

#[test]
fn reward_summary_includes_hero_appearance_unlocks() {
    let mut connection = Connection::open_in_memory().expect("in-memory database should open");
    identity::migrate(&connection).expect("identity schema should migrate");
    migrate(&connection).expect("progression schema should migrate");
    insert_user(&connection, 1, 100);
    connection
        .execute(
            "INSERT INTO hero_mastery (user_id, hero_type, xp) VALUES (1, 'pyromancer', 350)",
            [],
        )
        .expect("hero mastery should insert");
    connection
        .execute(
            "
            INSERT INTO match_xp_awards (
                match_id, user_id, account_xp, hero_type, hero_xp, won, awarded_at
            )
            VALUES ('match-appearance', 1, 100, 'pyromancer', 100, 0, 1)
            ",
            [],
        )
        .expect("award should insert");

    let progression = ProgressionModule::new(&mut connection);
    let reward = progression
        .match_reward_summary(1, "match-appearance", Side::Player)
        .expect("reward should load")
        .expect("reward should exist");
    assert!(reward.unlocks.iter().any(|unlock| {
        matches!(
            unlock,
            MatchUnlockCallout::HeroAppearanceUnlocked {
                hero_type: HeroType::Pyromancer,
                appearance_id: "pyromancer-ember-mantle",
                ..
            }
        )
    }));
}

#[test]
fn completed_match_awards_account_and_hero_xp_once() {
    let path = test_db_path("progression-award");
    let mut store = SqliteMatchStore::new(&path).expect("store should open");
    store
        .connection_mut()
        .execute(
            "
                INSERT INTO users (id, email, email_normalized, password_hash, display_name)
                VALUES (1, 'xp@example.com', 'xp@example.com', 'hash', 'XP')
                ",
            [],
        )
        .expect("user should insert");
    let mut stored = store
        .create_match_for_user(HeroType::Pyromancer, Some(1))
        .expect("match should create");
    let frames = stored.state.forfeit_recording(Side::Player, 0);
    store
        .save_custom_action_and_replay_frames(
            &stored.id,
            0,
            r#"{"type":"testForfeit"}"#,
            &stored.state,
            &frames,
        )
        .expect("completed match should save");
    award_completed_match(store.connection_mut(), &stored.id)
        .expect("second award attempt should be idempotent");

    let total_xp: i64 = store
        .connection_mut()
        .query_row("SELECT total_xp FROM users WHERE id = 1", [], |row| {
            row.get(0)
        })
        .expect("total xp should load");
    let hero_xp: i64 = store
        .connection_mut()
        .query_row(
            "SELECT xp FROM hero_mastery WHERE user_id = 1 AND hero_type = 'pyromancer'",
            [],
            |row| row.get(0),
        )
        .expect("hero xp should load");
    let award_count: i64 = store
        .connection_mut()
        .query_row("SELECT COUNT(*) FROM match_xp_awards", [], |row| row.get(0))
        .expect("award count should load");

    assert_eq!(total_xp, 150);
    assert_eq!(hero_xp, 150);
    assert_eq!(award_count, 1);

    let _ = std::fs::remove_file(path);
}

fn insert_user(connection: &Connection, user_id: i64, total_xp: i64) {
    connection
        .execute(
            "
                INSERT INTO users (
                    id,
                    email,
                    email_normalized,
                    password_hash,
                    display_name,
                    total_xp
                )
                VALUES (?1, ?2, ?2, 'hash', ?3, ?4)
                ",
            params![
                user_id,
                format!("user-{user_id}@example.com"),
                format!("User {user_id}"),
                total_xp
            ],
        )
        .expect("user should insert");
}

fn test_db_path(name: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("rune-lanes-{name}-{millis}.sqlite3"))
}
