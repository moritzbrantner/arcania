use super::*;

#[test]
fn configured_system_recipes_keep_their_ids_and_materialize_sixty_known_cards() {
    assert_eq!(
        system_deck_recipes()
            .iter()
            .map(|deck| deck.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "balanced-starter",
            "ember-burn",
            "tempo-lines",
            "rune-fortress",
            "unit-pressure",
            "barbarian-fury-line",
            "archer-volley-line",
            "builder-worksite",
        ]
    );
    for deck in system_deck_recipes() {
        assert_eq!(
            deck.cards
                .iter()
                .map(|card| u32::from(card.count))
                .sum::<u32>(),
            60,
            "{}",
            deck.id
        );
        let cards = deck_from_counts(Side::Player, &deck.cards).unwrap();
        assert_eq!(cards.len(), 60, "{}", deck.id);
        assert_eq!(
            cards
                .iter()
                .map(|card| card.id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            60
        );
    }
}

#[test]
fn exact_system_lookup_and_starter_counts_preserve_the_default_recipe() {
    let balanced = system_deck_by_id("balanced-starter").unwrap();
    assert_eq!(balanced.name, "Balanced Starter");
    assert_eq!(balanced.hero_type, crate::HeroType::Runekeeper);
    assert_eq!(balanced.cards, starter_deck_snapshot().cards);
    assert_eq!(starter_recipe_count("ember-squire"), 4);
    assert_eq!(starter_recipe_count("iron-colossus"), 1);
    assert_eq!(starter_recipe_count("meteor-bloom"), 0);
    assert_eq!(starter_recipe_count("unknown"), 0);
    assert!(system_deck_by_id("unknown").is_none());
}

#[test]
fn materialization_preserves_supplied_order_and_per_side_instance_ids() {
    let recipe = [
        DeckCardCount {
            template_id: "spark-jolt".to_string(),
            count: 2,
        },
        DeckCardCount {
            template_id: "ember-squire".to_string(),
            count: 1,
        },
    ];
    for (side, prefix) in [
        (Side::Player, "p"),
        (Side::Opponent, "o"),
        (Side::PlayerTwo, "p2"),
        (Side::OpponentTwo, "o2"),
    ] {
        let cards = deck_from_counts(side, &recipe).unwrap();
        assert_eq!(
            cards.iter().map(|card| card.id.clone()).collect::<Vec<_>>(),
            vec![
                format!("{prefix}-0-spark-jolt"),
                format!("{prefix}-1-spark-jolt"),
                format!("{prefix}-0-ember-squire"),
            ]
        );
        assert_eq!(
            cards
                .iter()
                .map(|card| card.template_id.as_str())
                .collect::<Vec<_>>(),
            vec!["spark-jolt", "spark-jolt", "ember-squire"]
        );
        assert_eq!(cards[0].cost, 1);
        assert_eq!(cards[2].name, "Ember Squire");
    }
}

#[test]
fn unknown_templates_return_the_existing_typed_error() {
    let error = deck_from_counts(
        Side::Player,
        &[DeckCardCount {
            template_id: "missing".to_string(),
            count: 1,
        }],
    )
    .unwrap_err();
    assert_eq!(
        error,
        DeckLibraryError::UnknownTemplate("missing".to_string())
    );
    assert_eq!(error.to_string(), "unknown card template missing");
}

#[test]
fn system_recipe_metadata_and_counts_round_trip_without_changing_order() {
    let recipe = system_deck_by_id("tempo-lines").unwrap();
    let value = serde_json::to_value(recipe).unwrap();
    assert_eq!(value["heroType"], "chronomancer");
    assert_eq!(
        value["cards"][0],
        serde_json::json!({ "templateId": "swift-familiar", "count": 4 })
    );
    assert_eq!(
        serde_json::from_value::<SystemDeckRecipe>(value).unwrap(),
        *recipe
    );
}
