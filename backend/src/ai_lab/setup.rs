use std::collections::{HashMap, HashSet};

use rune_lanes_core::card_template_by_id;
use rune_lanes_core::deck_library::{deck_from_counts, system_deck_by_id};
use rune_lanes_core::rules::CURRENT_RULESET;
use serde::{Deserialize, Serialize};

use super::AiLabError;
use crate::match_session::{Card, CardKind, HeroType, HexBoard, HexCoord, MatchState, Side, Unit};

#[derive(Clone, Debug)]
pub(super) struct GameSpec {
    pub(super) candidate_policy_id: String,
    pub(super) player_policy_id: String,
    pub(super) opponent_policy_id: String,
    pub(super) player_deck_id: String,
    pub(super) opponent_deck_id: String,
    pub(super) candidate_side: Side,
    pub(super) seed: u64,
    pub(super) rule_preset_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RulePreset {
    id: String,
    #[serde(default)]
    player: Option<SideSetup>,
    #[serde(default)]
    opponent: Option<SideSetup>,
    #[serde(default)]
    units: Vec<UnitSetup>,
    #[serde(default)]
    mana_sources: Vec<HexCoord>,
    #[serde(default)]
    card_overrides: Vec<CardOverride>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SideSetup {
    #[serde(default)]
    hero_type: Option<HeroType>,
    #[serde(default)]
    hero: Option<HeroOverride>,
    #[serde(default)]
    mana: Option<u8>,
    #[serde(default)]
    max_mana: Option<u8>,
    #[serde(default)]
    hand: Option<Vec<String>>,
    #[serde(default)]
    deck: Option<Vec<String>>,
    #[serde(default)]
    discard: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HeroOverride {
    #[serde(default)]
    hp: Option<i32>,
    #[serde(default)]
    max_hp: Option<i32>,
    #[serde(default)]
    attack: Option<i32>,
    #[serde(default)]
    attack_range: Option<u8>,
    #[serde(default)]
    ap_remaining: Option<u8>,
    #[serde(default)]
    max_ap: Option<u8>,
    #[serde(default)]
    position: Option<HexCoord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UnitSetup {
    id: String,
    side: Side,
    template_id: String,
    position: HexCoord,
    #[serde(default)]
    attack: Option<i32>,
    #[serde(default)]
    attack_range: Option<u8>,
    #[serde(default)]
    armor: Option<i32>,
    #[serde(default)]
    max_armor: Option<i32>,
    #[serde(default)]
    ap_remaining: Option<u8>,
    #[serde(default)]
    max_ap: Option<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CardOverride {
    template_id: String,
    #[serde(default)]
    cost: Option<u8>,
    #[serde(default)]
    attack: Option<i32>,
    #[serde(default)]
    armor: Option<i32>,
    #[serde(default)]
    max_ap: Option<u8>,
}

/// Frozen, validated simulation setup. Each constructed match owns its mutable state.
#[derive(Debug, Default)]
pub(super) struct SimulationSetup {
    presets: HashMap<String, PreparedRulePreset>,
}

impl SimulationSetup {
    pub(super) fn from_presets(presets: &[RulePreset]) -> Result<Self, AiLabError> {
        let mut prepared = HashMap::new();
        for preset in presets {
            if prepared.contains_key(&preset.id) {
                return Err(AiLabError::Config(format!(
                    "duplicate rule preset id: {}",
                    preset.id
                )));
            }
            prepared.insert(preset.id.clone(), preset.prepare()?);
        }
        Ok(Self { presets: prepared })
    }

    pub(super) fn validate_preset_id(&self, id: &str) -> Result<(), AiLabError> {
        self.preset(id).map(|_| ())
    }

    fn preset(&self, id: &str) -> Result<&PreparedRulePreset, AiLabError> {
        self.presets
            .get(id)
            .ok_or_else(|| AiLabError::Config(format!("rule preset was not found: {id}")))
    }

    pub(super) fn new_match(&self, spec: &GameSpec) -> Result<MatchState, AiLabError> {
        let player_deck = system_deck_by_id(&spec.player_deck_id)
            .ok_or_else(|| AiLabError::Deck(format!("unknown deck {}", spec.player_deck_id)))?;
        let opponent_deck = system_deck_by_id(&spec.opponent_deck_id)
            .ok_or_else(|| AiLabError::Deck(format!("unknown deck {}", spec.opponent_deck_id)))?;
        let player_cards = deck_from_counts(Side::Player, &player_deck.cards)
            .map_err(|error| AiLabError::Deck(error.to_string()))?;
        let opponent_cards = deck_from_counts(Side::Opponent, &opponent_deck.cards)
            .map_err(|error| AiLabError::Deck(error.to_string()))?;
        let mut game = MatchState::new_ai_lab_with_seed_and_decks(
            spec.seed,
            player_deck.hero_type,
            opponent_deck.hero_type,
            player_cards,
            opponent_cards,
        );
        if let Some(id) = &spec.rule_preset_id {
            self.preset(id)?.apply(&mut game);
        }
        Ok(game)
    }
}

#[derive(Debug)]
struct PreparedRulePreset {
    player: Option<PreparedSideSetup>,
    opponent: Option<PreparedSideSetup>,
    units: Vec<Unit>,
    mana_sources: Vec<HexCoord>,
    card_overrides: HashMap<String, CardOverride>,
}

impl RulePreset {
    fn prepare(&self) -> Result<PreparedRulePreset, AiLabError> {
        let board = HexBoard::new(CURRENT_RULESET.arena.duel_radius);
        let mut occupied = HashSet::new();
        for (side, setup) in [
            (Side::Player, &self.player),
            (Side::Opponent, &self.opponent),
        ] {
            let position = setup
                .as_ref()
                .and_then(|setup| setup.hero.as_ref())
                .and_then(|hero| hero.position)
                .unwrap_or_else(|| default_hero_position(side));
            validate_hex(position, &board)?;
            if !occupied.insert(position) {
                return Err(AiLabError::Config(format!(
                    "rule preset {} places multiple pieces on {},{}",
                    self.id, position.q, position.r
                )));
            }
        }
        let player = self
            .player
            .as_ref()
            .map(|setup| setup.prepare(Side::Player, &board))
            .transpose()?;
        let opponent = self
            .opponent
            .as_ref()
            .map(|setup| setup.prepare(Side::Opponent, &board))
            .transpose()?;
        let mut units = Vec::with_capacity(self.units.len());
        for unit in &self.units {
            validate_hex(unit.position, &board)?;
            if !occupied.insert(unit.position) {
                return Err(AiLabError::Config(format!(
                    "rule preset {} places multiple pieces on {},{}",
                    self.id, unit.position.q, unit.position.r
                )));
            }
            units.push(unit.to_unit()?);
        }
        for coord in &self.mana_sources {
            validate_hex(*coord, &board)?;
        }
        let mut card_overrides = HashMap::new();
        for override_ in &self.card_overrides {
            if card_template_by_id(&override_.template_id).is_none() {
                return Err(AiLabError::Config(format!(
                    "rule preset {} references unknown card {}",
                    self.id, override_.template_id
                )));
            }
            // Retain the existing last-override-wins behavior for duplicate entries.
            card_overrides.insert(override_.template_id.clone(), override_.clone());
        }
        Ok(PreparedRulePreset {
            player,
            opponent,
            units,
            mana_sources: self.mana_sources.clone(),
            card_overrides,
        })
    }
}

impl PreparedRulePreset {
    fn apply(&self, game: &mut MatchState) {
        if let Some(player) = &self.player {
            apply_side_setup(game, Side::Player, player);
        }
        if let Some(opponent) = &self.opponent {
            apply_side_setup(game, Side::Opponent, opponent);
        }
        if !self.units.is_empty() {
            game.board.units.clone_from(&self.units);
        }
        if !self.mana_sources.is_empty() {
            game.board
                .replace_mana_wells(self.mana_sources.iter().copied());
        }
        if !self.card_overrides.is_empty() {
            for side in [Side::Player, Side::Opponent] {
                let player = game.player_mut_for_ai_lab(side);
                apply_card_overrides(&mut player.hand, &self.card_overrides);
                apply_card_overrides(player.deck_mut_for_ai_lab(), &self.card_overrides);
                apply_card_overrides(player.discard_mut_for_ai_lab(), &self.card_overrides);
            }
        }
    }
}

#[derive(Debug)]
struct PreparedSideSetup {
    hero_type: Option<HeroType>,
    hero: Option<HeroOverride>,
    mana: Option<u8>,
    max_mana: Option<u8>,
    hand: Option<Vec<Card>>,
    deck: Option<Vec<Card>>,
    discard: Option<Vec<Card>>,
}

impl SideSetup {
    fn prepare(&self, side: Side, board: &HexBoard) -> Result<PreparedSideSetup, AiLabError> {
        if let Some(hero) = &self.hero {
            validate_hero(hero, board)?;
        }
        Ok(PreparedSideSetup {
            hero_type: self.hero_type,
            hero: self.hero.clone(),
            mana: self.mana,
            max_mana: self.max_mana,
            hand: self
                .hand
                .as_ref()
                .map(|ids| cards_from_template_ids(side, ids))
                .transpose()?,
            deck: self
                .deck
                .as_ref()
                .map(|ids| cards_from_template_ids(side, ids))
                .transpose()?,
            discard: self
                .discard
                .as_ref()
                .map(|ids| cards_from_template_ids(side, ids))
                .transpose()?,
        })
    }
}

fn default_hero_position(side: Side) -> HexCoord {
    let radius = CURRENT_RULESET.arena.duel_radius;
    match side {
        Side::Player => HexCoord { q: 0, r: radius },
        Side::Opponent => HexCoord { q: 0, r: -radius },
        Side::PlayerTwo => HexCoord {
            q: 1,
            r: radius - 1,
        },
        Side::OpponentTwo => HexCoord {
            q: -1,
            r: 1 - radius,
        },
    }
}

fn validate_hero(hero: &HeroOverride, board: &HexBoard) -> Result<(), AiLabError> {
    if hero.max_hp.is_some_and(|value| value <= 0)
        || hero.hp.is_some_and(|value| value <= 0)
        || hero.max_ap.is_some_and(|value| value == 0)
        || hero.ap_remaining.is_some_and(|value| value == 0)
        || hero.attack_range.is_some_and(|value| value == 0)
    {
        return Err(AiLabError::Config(
            "rule preset has invalid hero stats".to_string(),
        ));
    }
    if let Some(position) = hero.position {
        validate_hex(position, board)?;
    }
    Ok(())
}

fn validate_hex(coord: HexCoord, board: &HexBoard) -> Result<(), AiLabError> {
    if !board.is_valid(coord) {
        return Err(AiLabError::Config(format!(
            "rule preset references invalid hex {},{}",
            coord.q, coord.r
        )));
    }
    Ok(())
}

fn apply_side_setup(game: &mut MatchState, side: Side, setup: &PreparedSideSetup) {
    if let Some(hero_type) = setup.hero_type {
        game.player_mut_for_ai_lab(side).hero.hero_type = hero_type;
    }
    if let Some(hero) = &setup.hero {
        let target = &mut game.player_mut_for_ai_lab(side).hero;
        if let Some(value) = hero.max_hp {
            target.max_hp = value;
        }
        if let Some(value) = hero.hp {
            target.hp = value;
        }
        if let Some(value) = hero.attack {
            target.attack = value;
        }
        if let Some(value) = hero.attack_range {
            target.attack_range = value;
        }
        if let Some(value) = hero.max_ap {
            target.max_ap = value;
        }
        if let Some(value) = hero.ap_remaining {
            target.ap_remaining = value;
        }
        if let Some(value) = hero.position {
            target.position = value;
        }
    }
    {
        let player = game.player_mut_for_ai_lab(side);
        if let Some(value) = setup.max_mana {
            player.max_mana = value;
        }
        if let Some(value) = setup.mana {
            player.mana = value;
        }
        if let Some(zone) = &setup.hand {
            player.hand = zone.clone();
        }
        if let Some(zone) = &setup.deck {
            player.replace_deck_for_ai_lab(zone.clone());
        }
        if let Some(zone) = &setup.discard {
            player.replace_discard_for_ai_lab(zone.clone());
        }
    }
}

fn cards_from_template_ids(side: Side, template_ids: &[String]) -> Result<Vec<Card>, AiLabError> {
    template_ids
        .iter()
        .enumerate()
        .map(|(index, template_id)| {
            let mut card = card_template_by_id(template_id).ok_or_else(|| {
                AiLabError::Config(format!("rule preset references unknown card {template_id}"))
            })?;
            card.id = format!("{}-preset-{}-{index}", side.card_prefix(), card.template_id);
            Ok(card)
        })
        .collect()
}

impl UnitSetup {
    fn to_unit(&self) -> Result<Unit, AiLabError> {
        let card = card_template_by_id(&self.template_id)
            .ok_or_else(|| AiLabError::Config(format!("unknown unit card {}", self.template_id)))?;
        let CardKind::Unit {
            attack,
            armor,
            max_ap,
        } = card.kind
        else {
            return Err(AiLabError::Config(format!(
                "card {} is not a unit card",
                self.template_id
            )));
        };
        let max_armor = self.max_armor.unwrap_or(self.armor.unwrap_or(armor));
        Ok(Unit {
            id: self.id.clone(),
            side: self.side,
            name: card.name,
            template_id: Some(self.template_id.clone()),
            attack: self.attack.unwrap_or(attack),
            attack_range: self
                .attack_range
                .unwrap_or(CURRENT_RULESET.turn.default_attack_range),
            armor: self.armor.unwrap_or(max_armor),
            max_armor,
            position: self.position,
            ap_remaining: self.ap_remaining.unwrap_or(self.max_ap.unwrap_or(max_ap)),
            max_ap: self.max_ap.unwrap_or(max_ap),
            has_attacked: false,
            items: Vec::new(),
            stat_markers: Vec::new(),
        })
    }
}

fn apply_card_overrides(cards: &mut [Card], overrides: &HashMap<String, CardOverride>) {
    for card in cards {
        let Some(override_) = overrides.get(card.template_id.as_str()) else {
            continue;
        };
        if let Some(cost) = override_.cost {
            card.cost = cost;
        }
        match &mut card.kind {
            CardKind::Unit {
                attack,
                armor,
                max_ap,
            } => {
                if let Some(value) = override_.attack {
                    *attack = value;
                }
                if let Some(value) = override_.armor {
                    *armor = value;
                }
                if let Some(value) = override_.max_ap {
                    *max_ap = value;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::SuiteConfigFile;
    use super::*;
    use crate::match_session::BuildingEffect;

    fn test_spec(preset: Option<&str>) -> GameSpec {
        GameSpec {
            candidate_policy_id: "candidate".to_string(),
            player_policy_id: "candidate".to_string(),
            opponent_policy_id: "baseline-v1".to_string(),
            player_deck_id: "balanced-starter".to_string(),
            opponent_deck_id: "balanced-starter".to_string(),
            candidate_side: Side::Player,
            seed: 42,
            rule_preset_id: preset.map(str::to_string),
        }
    }

    #[test]
    fn match_factory_preserves_core_seeded_setup_for_every_deck_pair() {
        let setup = SimulationSetup::default();
        for player in rune_lanes_core::deck_library::system_deck_recipes() {
            for opponent in rune_lanes_core::deck_library::system_deck_recipes() {
                for seed in [1, 42] {
                    let spec = GameSpec {
                        player_deck_id: player.id.clone(),
                        opponent_deck_id: opponent.id.clone(),
                        seed,
                        ..test_spec(None)
                    };
                    let actual = setup
                        .new_match(&spec)
                        .expect("known decks should construct");
                    let expected = MatchState::new_ai_lab_with_seed_and_decks(
                        seed,
                        player.hero_type,
                        opponent.hero_type,
                        deck_from_counts(Side::Player, &player.cards)
                            .expect("Player deck should build"),
                        deck_from_counts(Side::Opponent, &opponent.cards)
                            .expect("Opponent deck should build"),
                    );
                    assert_eq!(
                        actual.initial_replay_frame().snapshot_json,
                        expected.initial_replay_frame().snapshot_json
                    );
                }
            }
        }
    }

    #[test]
    fn prepared_setup_creates_independent_mutable_matches() {
        let preset: RulePreset = serde_json::from_value(serde_json::json!({
            "id": "copies", "player": {"hand": ["ember-squire"], "deck": ["rune-charm"], "discard": ["spark-jolt"]},
            "opponent": {"hand": ["spark-jolt"]}, "units": [{
                "id": "unit", "side": "player", "templateId": "ember-squire", "position": {"q": 0, "r": 2}
            }]
        })).expect("preset should parse");
        let setup = SimulationSetup::from_presets(&[preset]).expect("preset should prepare");
        let mut first = setup
            .new_match(&test_spec(Some("copies")))
            .expect("first match should construct");
        let initial = first.initial_replay_frame().snapshot_json;
        first.player.hero.hp = 1;
        first.player.hand.clear();
        first.player.replace_deck_for_ai_lab(Vec::new());
        first.player.replace_discard_for_ai_lab(Vec::new());
        first.opponent.hand.clear();
        first.board.units[0].armor = 0;
        let second = setup
            .new_match(&test_spec(Some("copies")))
            .expect("second match should construct");
        assert_eq!(second.initial_replay_frame().snapshot_json, initial);
    }

    #[test]
    fn construction_rejects_unknown_decks_and_presets() {
        let setup = SimulationSetup::default();
        for spec in [
            GameSpec {
                player_deck_id: "missing".to_string(),
                ..test_spec(None)
            },
            GameSpec {
                opponent_deck_id: "missing".to_string(),
                ..test_spec(None)
            },
            test_spec(Some("missing")),
        ] {
            assert!(setup.new_match(&spec).is_err());
        }
        assert!(setup.validate_preset_id("missing").is_err());
        let preset: RulePreset = serde_json::from_value(serde_json::json!({"id": "duplicate"}))
            .expect("preset should parse");
        assert!(SimulationSetup::from_presets(&[preset.clone(), preset]).is_err());
    }

    #[test]
    fn rule_preset_rejects_invalid_hex_and_unknown_card() {
        let invalid_hex = RulePreset {
            id: "bad-hex".to_string(),
            player: Some(SideSetup {
                hero: Some(HeroOverride {
                    position: Some(HexCoord { q: 9, r: 0 }),
                    ..HeroOverride::default()
                }),
                ..SideSetup::default()
            }),
            opponent: None,
            units: Vec::new(),
            mana_sources: Vec::new(),
            card_overrides: Vec::new(),
        };
        assert!(SimulationSetup::from_presets(&[invalid_hex]).is_err());

        let unknown_card = RulePreset {
            id: "bad-card".to_string(),
            player: Some(SideSetup {
                hand: Some(vec!["missing-card".to_string()]),
                ..SideSetup::default()
            }),
            opponent: None,
            units: Vec::new(),
            mana_sources: Vec::new(),
            card_overrides: Vec::new(),
        };
        assert!(SimulationSetup::from_presets(&[unknown_card]).is_err());
    }

    #[test]
    fn rule_preset_rejects_duplicate_default_hero_hex_and_unknown_effect_fields() {
        let duplicate_default_hero_hex = RulePreset {
            id: "duplicate".to_string(),
            player: None,
            opponent: None,
            units: vec![UnitSetup {
                id: "unit".to_string(),
                side: Side::Player,
                template_id: "ember-squire".to_string(),
                position: HexCoord { q: 0, r: 3 },
                attack: None,
                attack_range: None,
                armor: None,
                max_armor: None,
                ap_remaining: None,
                max_ap: None,
            }],
            mana_sources: Vec::new(),
            card_overrides: Vec::new(),
        };
        assert!(SimulationSetup::from_presets(&[duplicate_default_hero_hex]).is_err());

        let unknown_effect = serde_json::from_str::<SuiteConfigFile>(
            r#"{"suites":[],"rulePresets":[{"id":"bad","cardOverrides":[{"templateId":"ember-squire","effect":{"type":"newEffect"}}]}]}"#,
        )
        .expect_err("new effect fields should be rejected");
        assert!(unknown_effect.to_string().contains("unknown field"));
    }

    #[test]
    fn rule_preset_rejects_unknown_fields_at_every_setup_boundary() {
        for preset in [
            serde_json::json!({"id": "invalid", "effect": {"type": "newEffect"}}),
            serde_json::json!({"id": "invalid", "player": {"effect": {}}}),
            serde_json::json!({"id": "invalid", "player": {"hero": {"effect": {}}}}),
            serde_json::json!({"id": "invalid", "units": [{
                "id": "unit", "side": "player", "templateId": "ember-squire",
                "position": {"q": 0, "r": 2}, "effect": {}
            }]}),
            serde_json::json!({"id": "invalid", "cardOverrides": [{
                "templateId": "ember-squire", "effect": {}
            }]}),
        ] {
            let error = serde_json::from_value::<SuiteConfigFile>(serde_json::json!({
                "suites": [], "rulePresets": [preset]
            }))
            .expect_err("unsupported effect fields must fail instead of being ignored");
            assert!(error.to_string().contains("unknown field `effect`"));
        }
    }

    #[test]
    fn rule_preset_rejects_invalid_hero_stats() {
        for hero in [
            serde_json::json!({"hp": 0}),
            serde_json::json!({"maxHp": -1}),
            serde_json::json!({"maxAp": 0}),
            serde_json::json!({"apRemaining": 0}),
            serde_json::json!({"attackRange": 0}),
        ] {
            let preset: RulePreset = serde_json::from_value(serde_json::json!({
                "id": "invalid-stats", "player": {"hero": hero}
            }))
            .expect("numeric Hero overrides should deserialize");
            assert!(
                SimulationSetup::from_presets(&[preset])
                    .expect_err("invalid Hero stats must fail validation")
                    .to_string()
                    .contains("invalid hero stats")
            );
        }
    }

    #[test]
    fn preset_units_use_the_core_default_attack_range() {
        let preset: RulePreset = serde_json::from_value(serde_json::json!({
            "id": "unit", "units": [{"id": "unit", "side": "player", "templateId": "ember-squire", "position": {"q": 0, "r": 2}}]
        })).expect("unit setup should parse");
        let setup = SimulationSetup::from_presets(&[preset]).expect("unit setup should prepare");
        let game = setup
            .new_match(&test_spec(Some("unit")))
            .expect("game should construct");
        assert_eq!(
            game.board.units[0].attack_range,
            CURRENT_RULESET.turn.default_attack_range
        );
    }

    #[test]
    fn rule_preset_mana_sources_apply_as_building_backed_mana_wells() {
        let preset = RulePreset {
            id: "mana".to_string(),
            player: None,
            opponent: None,
            units: Vec::new(),
            mana_sources: vec![HexCoord { q: 0, r: 2 }],
            card_overrides: Vec::new(),
        };

        let setup = SimulationSetup::from_presets(&[preset]).expect("preset should prepare");
        let game = setup
            .new_match(&test_spec(Some("mana")))
            .expect("game should construct");

        assert!(game.board.mana_sources.is_empty());
        assert!(game.board.buildings.iter().any(|building| {
            building.id == "preset-mana-1"
                && building.position == HexCoord { q: 0, r: 2 }
                && matches!(building.effect, BuildingEffect::TurnStartMana { amount: 1 })
        }));
    }
}
