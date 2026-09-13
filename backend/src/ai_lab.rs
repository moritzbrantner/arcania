use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::card_catalog::card_template_by_id;
use crate::deck_library::ai_lab_system_decks;
use crate::match_session::{
    AiPolicyConfig, Card, CardKind, HeroType, HexBoard, HexCoord, MatchError, MatchState,
    RecordedReplayFrame, Side, Unit, default_policy_config_path,
};

mod self_play;
use self_play::{GameOutcome, GameSpec, SimulationGameResult, run_game};

const DEFAULT_SUITE_PATH: &str = "backend/config/ai-lab-suites.json";
const FALLBACK_SUITE_PATH: &str = "config/ai-lab-suites.json";
const DEFAULT_MAX_ACTIONS: u32 = 300;

pub fn run_from_env() -> Result<(), AiLabError> {
    run(std::env::args().skip(1).collect())
}

pub fn run(args: Vec<String>) -> Result<(), AiLabError> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err(AiLabError::Usage(usage()));
    };
    let options = CliOptions::parse(&args[1..])?;
    match command {
        "validate-config" => {
            let policy_config = load_policy_config()?;
            let suite_config = load_suite_config()?;
            suite_config.validate_with_policies(&policy_config)?;
            println!("AI lab config is valid.");
            Ok(())
        }
        "run" => {
            let out_dir = options.out_dir()?;
            let report = run_suite(&options.suite, &out_dir)?;
            println!(
                "AI lab suite {} wrote {} games to {}",
                report.suite_id,
                report.total_games,
                out_dir.display()
            );
            Ok(())
        }
        "promote" => {
            let out_dir = options.out_dir()?;
            let report = run_suite(&options.suite, &out_dir)?;
            let promoted = promote_from_report(&report)?;
            if let Some(policy_id) = promoted {
                println!("Promoted AI policy {policy_id}.");
            } else {
                println!("No AI policy passed promotion gates.");
            }
            Ok(())
        }
        _ => Err(AiLabError::Usage(usage())),
    }
}

#[derive(Clone, Debug)]
struct CliOptions {
    suite: String,
    out: Option<PathBuf>,
}

impl CliOptions {
    fn parse(args: &[String]) -> Result<Self, AiLabError> {
        let mut suite = "default".to_string();
        let mut out = None;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--suite" => {
                    index += 1;
                    suite = args
                        .get(index)
                        .ok_or_else(|| AiLabError::Usage("--suite requires a value".to_string()))?
                        .clone();
                }
                "--out" => {
                    index += 1;
                    out = Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        AiLabError::Usage("--out requires a value".to_string())
                    })?));
                }
                unknown => return Err(AiLabError::Usage(format!("Unknown option: {unknown}"))),
            }
            index += 1;
        }
        Ok(Self { suite, out })
    }

    fn out_dir(&self) -> Result<PathBuf, AiLabError> {
        if let Some(out) = &self.out {
            return Ok(out.clone());
        }
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        Ok(PathBuf::from(format!("target/ai-lab/{timestamp}")))
    }
}

fn usage() -> String {
    "Usage: ai-lab run|promote|validate-config [--suite default] [--out target/ai-lab/latest]"
        .to_string()
}

#[derive(Debug)]
pub enum AiLabError {
    Usage(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Config(String),
    Deck(String),
    Match(MatchError),
}

impl fmt::Display for AiLabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Config(message) => write!(f, "{message}"),
            Self::Deck(message) => write!(f, "{message}"),
            Self::Match(error) => write!(f, "{error}"),
        }
    }
}

impl Error for AiLabError {}

impl From<std::io::Error> for AiLabError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for AiLabError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<MatchError> for AiLabError {
    fn from(error: MatchError) -> Self {
        Self::Match(error)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SuiteConfigFile {
    suites: Vec<SimulationSuite>,
    #[serde(default)]
    rule_presets: Vec<RulePreset>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SimulationSuite {
    id: String,
    baseline_policy_id: String,
    candidate_policy_ids: Vec<String>,
    system_deck_matrix: bool,
    seat_directions: Vec<SeatDirection>,
    minimum_completed_games: u32,
    minimum_candidate_win_rate: f64,
    maximum_draw_rate: f64,
    minimum_median_actions: u32,
    maximum_median_actions: u32,
    seeds: Vec<u64>,
    #[serde(default)]
    rule_preset_ids: Vec<String>,
    #[serde(default = "default_max_actions")]
    max_actions: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum SeatDirection {
    CandidateAsPlayer,
    CandidateAsOpponent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RulePreset {
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

fn default_max_actions() -> u32 {
    DEFAULT_MAX_ACTIONS
}

fn load_policy_config() -> Result<AiPolicyConfig, AiLabError> {
    AiPolicyConfig::load_from_path(default_policy_config_path())
        .map_err(|error| AiLabError::Config(error.to_string()))
}

fn load_suite_config() -> Result<SuiteConfigFile, AiLabError> {
    let path = if Path::new(DEFAULT_SUITE_PATH).exists() {
        DEFAULT_SUITE_PATH
    } else {
        FALLBACK_SUITE_PATH
    };
    let text = fs::read_to_string(path)?;
    let config: SuiteConfigFile = serde_json::from_str(&text)?;
    config.validate()?;
    Ok(config)
}

impl SuiteConfigFile {
    fn validate(&self) -> Result<(), AiLabError> {
        let mut suite_ids = HashSet::new();
        for suite in &self.suites {
            if !suite_ids.insert(&suite.id) {
                return Err(AiLabError::Config(format!(
                    "duplicate simulation suite id: {}",
                    suite.id
                )));
            }
            if suite.candidate_policy_ids.is_empty() {
                return Err(AiLabError::Config(format!(
                    "suite {} has no candidate policies",
                    suite.id
                )));
            }
            if suite.seat_directions.is_empty() {
                return Err(AiLabError::Config(format!(
                    "suite {} has no seat directions",
                    suite.id
                )));
            }
            if suite.seeds.is_empty() {
                return Err(AiLabError::Config(format!(
                    "suite {} has no seeds",
                    suite.id
                )));
            }
        }
        let mut preset_ids = HashSet::new();
        for preset in &self.rule_presets {
            if !preset_ids.insert(&preset.id) {
                return Err(AiLabError::Config(format!(
                    "duplicate rule preset id: {}",
                    preset.id
                )));
            }
            preset.validate()?;
        }
        Ok(())
    }

    fn validate_with_policies(&self, policy_config: &AiPolicyConfig) -> Result<(), AiLabError> {
        self.validate()?;
        for suite in &self.suites {
            if policy_config.policy(&suite.baseline_policy_id).is_none() {
                return Err(AiLabError::Config(format!(
                    "suite {} references missing baseline policy {}",
                    suite.id, suite.baseline_policy_id
                )));
            }
            for candidate_id in &suite.candidate_policy_ids {
                if policy_config.policy(candidate_id).is_none() {
                    return Err(AiLabError::Config(format!(
                        "suite {} references missing candidate policy {}",
                        suite.id, candidate_id
                    )));
                }
            }
            for preset_id in &suite.rule_preset_ids {
                self.rule_preset(preset_id)?;
            }
        }
        Ok(())
    }

    fn suite(&self, id: &str) -> Result<&SimulationSuite, AiLabError> {
        self.suites
            .iter()
            .find(|suite| suite.id == id)
            .ok_or_else(|| AiLabError::Config(format!("simulation suite was not found: {id}")))
    }

    fn rule_preset(&self, id: &str) -> Result<&RulePreset, AiLabError> {
        self.rule_presets
            .iter()
            .find(|preset| preset.id == id)
            .ok_or_else(|| AiLabError::Config(format!("rule preset was not found: {id}")))
    }
}

impl RulePreset {
    fn validate(&self) -> Result<(), AiLabError> {
        let mut occupied = HashSet::new();
        for (side, setup) in [
            (Side::Player, &self.player),
            (Side::Opponent, &self.opponent),
        ] {
            let hero_position = setup
                .as_ref()
                .and_then(|setup| setup.hero.as_ref())
                .and_then(|hero| hero.position)
                .unwrap_or_else(|| default_hero_position(side));
            validate_hex(hero_position)?;
            if !occupied.insert(hero_position) {
                return Err(AiLabError::Config(format!(
                    "rule preset {} places multiple pieces on {},{}",
                    self.id, hero_position.q, hero_position.r
                )));
            }
            if let Some(side_setup) = setup {
                validate_side_setup(side_setup)?;
            }
            if let Some(hero) = setup.as_ref().and_then(|setup| setup.hero.as_ref()) {
                validate_hero(hero)?;
            }
        }
        for unit in &self.units {
            validate_hex(unit.position)?;
            if !occupied.insert(unit.position) {
                return Err(AiLabError::Config(format!(
                    "rule preset {} places multiple pieces on {},{}",
                    self.id, unit.position.q, unit.position.r
                )));
            }
            if card_template_by_id(&unit.template_id).is_none() {
                return Err(AiLabError::Config(format!(
                    "rule preset {} references unknown unit card {}",
                    self.id, unit.template_id
                )));
            }
        }
        for coord in &self.mana_sources {
            validate_hex(*coord)?;
        }
        for override_ in &self.card_overrides {
            if card_template_by_id(&override_.template_id).is_none() {
                return Err(AiLabError::Config(format!(
                    "rule preset {} references unknown card {}",
                    self.id, override_.template_id
                )));
            }
        }
        Ok(())
    }

    fn apply(&self, game: &mut MatchState) -> Result<(), AiLabError> {
        if let Some(player) = &self.player {
            apply_side_setup(game, Side::Player, player)?;
        }
        if let Some(opponent) = &self.opponent {
            apply_side_setup(game, Side::Opponent, opponent)?;
        }
        if !self.units.is_empty() {
            game.board.units.clear();
            for unit in &self.units {
                game.board.units.push(unit.to_unit()?);
            }
        }
        if !self.mana_sources.is_empty() {
            game.board
                .replace_mana_wells(self.mana_sources.iter().copied());
        }
        if !self.card_overrides.is_empty() {
            let overrides: HashMap<_, _> = self
                .card_overrides
                .iter()
                .map(|override_| (override_.template_id.as_str(), override_))
                .collect();
            for side in [Side::Player, Side::Opponent] {
                let player = game.player_mut_for_ai_lab(side);
                apply_card_overrides(&mut player.hand, &overrides);
                apply_card_overrides(player.deck_mut_for_ai_lab(), &overrides);
                apply_card_overrides(player.discard_mut_for_ai_lab(), &overrides);
            }
        }
        Ok(())
    }
}

fn default_hero_position(side: Side) -> HexCoord {
    match side {
        Side::Player => HexCoord { q: 0, r: 3 },
        Side::Opponent => HexCoord { q: 0, r: -3 },
        Side::PlayerTwo => HexCoord { q: 1, r: 2 },
        Side::OpponentTwo => HexCoord { q: -1, r: -2 },
    }
}

fn validate_side_setup(setup: &SideSetup) -> Result<(), AiLabError> {
    for zone in [&setup.hand, &setup.deck, &setup.discard]
        .into_iter()
        .flatten()
    {
        for template_id in zone {
            if card_template_by_id(template_id).is_none() {
                return Err(AiLabError::Config(format!(
                    "rule preset references unknown card {template_id}"
                )));
            }
        }
    }
    Ok(())
}

fn validate_hero(hero: &HeroOverride) -> Result<(), AiLabError> {
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
        validate_hex(position)?;
    }
    Ok(())
}

fn validate_hex(coord: HexCoord) -> Result<(), AiLabError> {
    if !HexBoard::new(3).is_valid(coord) {
        return Err(AiLabError::Config(format!(
            "rule preset references invalid hex {},{}",
            coord.q, coord.r
        )));
    }
    Ok(())
}

fn apply_side_setup(
    game: &mut MatchState,
    side: Side,
    setup: &SideSetup,
) -> Result<(), AiLabError> {
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
            player.hand = cards_from_template_ids(side, zone)?;
        }
        if let Some(zone) = &setup.deck {
            player.replace_deck_for_ai_lab(cards_from_template_ids(side, zone)?);
        }
        if let Some(zone) = &setup.discard {
            player.replace_discard_for_ai_lab(cards_from_template_ids(side, zone)?);
        }
    }
    Ok(())
}

fn cards_from_template_ids(side: Side, template_ids: &[String]) -> Result<Vec<Card>, AiLabError> {
    template_ids
        .iter()
        .enumerate()
        .map(|(index, template_id)| {
            let mut card = card_template_by_id(template_id)
                .ok_or_else(|| AiLabError::Config(format!("unknown card {template_id}")))?;
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
            attack_range: self.attack_range.unwrap_or(1),
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

fn apply_card_overrides(cards: &mut [Card], overrides: &HashMap<&str, &CardOverride>) {
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SimulationReport {
    suite_id: String,
    baseline_policy_id: String,
    candidate_reports: Vec<CandidateReport>,
    total_games: usize,
    promoted_policy_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateReport {
    candidate_policy_id: String,
    total_games: usize,
    completed_games: usize,
    candidate_wins: usize,
    baseline_wins: usize,
    draws: usize,
    timeouts: usize,
    illegal_actions: usize,
    median_actions: u32,
    candidate_win_rate: f64,
    draw_rate: f64,
    gate: PromotionGateReport,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PromotionGateReport {
    passed: bool,
    reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySample<'a> {
    candidate_policy_id: &'a str,
    player_policy_id: &'a str,
    opponent_policy_id: &'a str,
    player_deck_id: &'a str,
    opponent_deck_id: &'a str,
    candidate_side: Side,
    seed: u64,
    rule_preset_id: &'a Option<String>,
    outcome: &'a str,
    action_count: u32,
    frames: Vec<ReplaySampleFrame>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySampleFrame {
    action_index: Option<u32>,
    event: serde_json::Value,
    match_state: serde_json::Value,
}

fn run_suite(suite_id: &str, out_dir: &Path) -> Result<SimulationReport, AiLabError> {
    fs::create_dir_all(out_dir)?;
    let policy_config = load_policy_config()?;
    let suite_config = load_suite_config()?;
    suite_config.validate_with_policies(&policy_config)?;
    let suite = suite_config.suite(suite_id)?;
    let baseline = policy_config
        .policy(&suite.baseline_policy_id)
        .ok_or_else(|| {
            AiLabError::Config(format!(
                "baseline policy was not found: {}",
                suite.baseline_policy_id
            ))
        })?;
    let baseline_policy = crate::match_session::SoloAiPolicy::from_definition(baseline);
    let mut by_candidate: HashMap<String, Vec<SimulationGameResult>> = HashMap::new();

    for candidate_id in &suite.candidate_policy_ids {
        let candidate = policy_config.policy(candidate_id).ok_or_else(|| {
            AiLabError::Config(format!("candidate policy was not found: {candidate_id}"))
        })?;
        let candidate_policy = crate::match_session::SoloAiPolicy::from_definition(candidate);
        let specs = expand_specs(suite, &suite_config, candidate_id)?;
        for spec in specs {
            let result = run_game(
                &spec,
                &baseline_policy,
                &candidate_policy,
                &suite_config,
                suite.max_actions,
            )?;
            by_candidate
                .entry(candidate_id.clone())
                .or_default()
                .push(result);
        }
    }

    let mut candidate_reports = Vec::new();
    let mut promoted_policy_id = None;
    for candidate_id in &suite.candidate_policy_ids {
        let results = by_candidate.get(candidate_id).cloned().unwrap_or_default();
        write_samples(candidate_id, &results, out_dir)?;
        let report = summarize_candidate(candidate_id, &results, suite);
        if report.gate.passed && promoted_policy_id.is_none() {
            promoted_policy_id = Some(candidate_id.clone());
        }
        candidate_reports.push(report);
    }

    let report = SimulationReport {
        suite_id: suite.id.clone(),
        baseline_policy_id: suite.baseline_policy_id.clone(),
        total_games: candidate_reports
            .iter()
            .map(|report| report.total_games)
            .sum(),
        candidate_reports,
        promoted_policy_id,
    };
    fs::write(
        out_dir.join("summary.json"),
        serde_json::to_string_pretty(&report)?,
    )?;
    Ok(report)
}

fn expand_specs(
    suite: &SimulationSuite,
    config: &SuiteConfigFile,
    candidate_policy_id: &str,
) -> Result<Vec<GameSpec>, AiLabError> {
    let decks = if suite.system_deck_matrix {
        ai_lab_system_decks()
    } else {
        ai_lab_system_decks().into_iter().take(1).collect()
    };
    let preset_ids = if suite.rule_preset_ids.is_empty() {
        vec![None]
    } else {
        suite
            .rule_preset_ids
            .iter()
            .map(|id| {
                config.rule_preset(id)?;
                Ok(Some(id.clone()))
            })
            .collect::<Result<Vec<_>, AiLabError>>()?
    };
    let mut specs = Vec::new();
    let mut generation = 0_u64;
    while specs.len() < suite.minimum_completed_games as usize || generation == 0 {
        for seed in &suite.seeds {
            for player_deck in &decks {
                for opponent_deck in &decks {
                    for direction in &suite.seat_directions {
                        for preset_id in &preset_ids {
                            let (player_policy_id, opponent_policy_id, candidate_side) =
                                match direction {
                                    SeatDirection::CandidateAsPlayer => (
                                        candidate_policy_id.to_string(),
                                        suite.baseline_policy_id.clone(),
                                        Side::Player,
                                    ),
                                    SeatDirection::CandidateAsOpponent => (
                                        suite.baseline_policy_id.clone(),
                                        candidate_policy_id.to_string(),
                                        Side::Opponent,
                                    ),
                                };
                            specs.push(GameSpec {
                                candidate_policy_id: candidate_policy_id.to_string(),
                                player_policy_id,
                                opponent_policy_id,
                                player_deck_id: player_deck.id.clone(),
                                opponent_deck_id: opponent_deck.id.clone(),
                                candidate_side,
                                seed: seed.wrapping_add(generation * 1_000_003),
                                rule_preset_id: preset_id.clone(),
                            });
                        }
                    }
                }
            }
        }
        generation += 1;
    }
    Ok(specs)
}

fn summarize_candidate(
    candidate_policy_id: &str,
    results: &[SimulationGameResult],
    suite: &SimulationSuite,
) -> CandidateReport {
    let total_games = results.len();
    let candidate_wins = results
        .iter()
        .filter(|result| result.outcome == GameOutcome::CandidateWin)
        .count();
    let baseline_wins = results
        .iter()
        .filter(|result| result.outcome == GameOutcome::BaselineWin)
        .count();
    let draws = results
        .iter()
        .filter(|result| result.outcome == GameOutcome::Draw)
        .count();
    let timeouts = results
        .iter()
        .filter(|result| result.outcome == GameOutcome::Timeout)
        .count();
    let illegal_actions = results
        .iter()
        .filter(|result| matches!(result.outcome, GameOutcome::IllegalAction(_)))
        .count();
    let completed_games = candidate_wins + baseline_wins;
    let mut actions = results
        .iter()
        .map(|result| result.action_count)
        .collect::<Vec<_>>();
    actions.sort_unstable();
    let median_actions = actions
        .get(actions.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(0);
    let candidate_win_rate = if completed_games == 0 {
        0.0
    } else {
        candidate_wins as f64 / completed_games as f64
    };
    let draw_rate = if total_games == 0 {
        0.0
    } else {
        draws as f64 / total_games as f64
    };
    let gate = promotion_gate(
        completed_games,
        candidate_win_rate,
        draw_rate,
        illegal_actions,
        timeouts,
        median_actions,
        suite,
    );
    CandidateReport {
        candidate_policy_id: candidate_policy_id.to_string(),
        total_games,
        completed_games,
        candidate_wins,
        baseline_wins,
        draws,
        timeouts,
        illegal_actions,
        median_actions,
        candidate_win_rate,
        draw_rate,
        gate,
    }
}

fn promotion_gate(
    completed_games: usize,
    candidate_win_rate: f64,
    draw_rate: f64,
    illegal_actions: usize,
    timeouts: usize,
    median_actions: u32,
    suite: &SimulationSuite,
) -> PromotionGateReport {
    let mut reasons = Vec::new();
    if completed_games < suite.minimum_completed_games as usize {
        reasons.push(format!(
            "completed games {completed_games} below required {}",
            suite.minimum_completed_games
        ));
    }
    if candidate_win_rate < suite.minimum_candidate_win_rate {
        reasons.push(format!(
            "candidate win rate {candidate_win_rate:.3} below required {:.3}",
            suite.minimum_candidate_win_rate
        ));
    }
    if draw_rate > suite.maximum_draw_rate {
        reasons.push(format!(
            "draw rate {draw_rate:.3} above maximum {:.3}",
            suite.maximum_draw_rate
        ));
    }
    if illegal_actions > 0 {
        reasons.push(format!("illegal actions {illegal_actions} above zero"));
    }
    if timeouts > 0 {
        reasons.push(format!("timeouts {timeouts} above zero"));
    }
    if median_actions < suite.minimum_median_actions {
        reasons.push(format!(
            "median actions {median_actions} below minimum {}",
            suite.minimum_median_actions
        ));
    }
    if median_actions > suite.maximum_median_actions {
        reasons.push(format!(
            "median actions {median_actions} above maximum {}",
            suite.maximum_median_actions
        ));
    }
    PromotionGateReport {
        passed: reasons.is_empty(),
        reasons,
    }
}

fn write_samples(
    candidate_id: &str,
    results: &[SimulationGameResult],
    out_dir: &Path,
) -> Result<(), AiLabError> {
    let sample_dir = out_dir.join("samples").join(candidate_id);
    fs::create_dir_all(&sample_dir)?;
    write_named_sample(
        "shortest-win",
        results
            .iter()
            .filter(|result| result.outcome == GameOutcome::CandidateWin)
            .min_by_key(|result| result.action_count),
        &sample_dir,
    )?;
    write_named_sample(
        "longest-completed",
        results
            .iter()
            .filter(|result| {
                matches!(
                    result.outcome,
                    GameOutcome::CandidateWin | GameOutcome::BaselineWin
                )
            })
            .max_by_key(|result| result.action_count),
        &sample_dir,
    )?;
    write_named_sample(
        "first-draw",
        results
            .iter()
            .find(|result| result.outcome == GameOutcome::Draw),
        &sample_dir,
    )?;
    write_named_sample(
        "first-illegal-action",
        results
            .iter()
            .find(|result| matches!(result.outcome, GameOutcome::IllegalAction(_))),
        &sample_dir,
    )?;
    write_named_sample(
        "first-timeout",
        results
            .iter()
            .find(|result| result.outcome == GameOutcome::Timeout),
        &sample_dir,
    )?;
    Ok(())
}

fn write_named_sample(
    name: &str,
    result: Option<&SimulationGameResult>,
    sample_dir: &Path,
) -> Result<(), AiLabError> {
    let Some(result) = result else {
        return Ok(());
    };
    let outcome = match &result.outcome {
        GameOutcome::CandidateWin => "candidateWin",
        GameOutcome::BaselineWin => "baselineWin",
        GameOutcome::Draw => "draw",
        GameOutcome::Timeout => "timeout",
        GameOutcome::IllegalAction(_) => "illegalAction",
    };
    let frames = result
        .frames
        .iter()
        .map(sample_frame)
        .collect::<Result<Vec<_>, AiLabError>>()?;
    let sample = ReplaySample {
        candidate_policy_id: &result.spec.candidate_policy_id,
        player_policy_id: &result.spec.player_policy_id,
        opponent_policy_id: &result.spec.opponent_policy_id,
        player_deck_id: &result.spec.player_deck_id,
        opponent_deck_id: &result.spec.opponent_deck_id,
        candidate_side: result.spec.candidate_side,
        seed: result.spec.seed,
        rule_preset_id: &result.spec.rule_preset_id,
        outcome,
        action_count: result.action_count,
        frames,
    };
    fs::write(
        sample_dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(&sample)?,
    )?;
    Ok(())
}

fn sample_frame(frame: &RecordedReplayFrame) -> Result<ReplaySampleFrame, AiLabError> {
    Ok(ReplaySampleFrame {
        action_index: frame.action_index,
        event: serde_json::to_value(&frame.event)?,
        match_state: serde_json::from_str(&frame.snapshot_json)?,
    })
}

fn promote_from_report(report: &SimulationReport) -> Result<Option<String>, AiLabError> {
    let Some(policy_id) = &report.promoted_policy_id else {
        return Ok(None);
    };
    let path = default_policy_config_path();
    let mut config = load_policy_config()?;
    config
        .set_default_policy_id(policy_id)
        .map_err(|error| AiLabError::Config(error.to_string()))?;
    fs::write(path, serde_json::to_string_pretty(&config)? + "\n")?;
    Ok(Some(policy_id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck_library::deck_from_counts;
    use crate::match_session::{
        AiAdvanceOutcome, AiPolicyConfig, BuildingEffect, SoloAiPolicy, SoloAiRuleId,
    };

    fn test_game() -> MatchState {
        let deck = ai_lab_system_decks()
            .into_iter()
            .next()
            .expect("AI lab system deck should exist");
        let player_cards =
            deck_from_counts(Side::Player, &deck.cards).expect("player deck should build");
        let opponent_cards =
            deck_from_counts(Side::Opponent, &deck.cards).expect("opponent deck should build");
        MatchState::new_ai_lab_with_seed_and_decks(
            42,
            deck.hero_type,
            deck.hero_type,
            player_cards,
            opponent_cards,
        )
    }

    #[test]
    fn policy_config_rejects_unknown_rule_ids() {
        let error = AiPolicyConfig::from_json(
            r#"{"defaultPolicyId":"bad","policies":[{"id":"bad","rules":["missingRule"]}]}"#,
        )
        .expect_err("unknown rule should be rejected");

        assert!(error.to_string().contains("unknown variant"));
    }

    #[test]
    fn suite_config_validates_policy_and_preset_references() {
        let policy_config = AiPolicyConfig::from_json(
            r#"{"defaultPolicyId":"baseline-v1","policies":[{"id":"baseline-v1","rules":["inRangeAttack"]},{"id":"candidate","rules":["inRangeAttack"]}]}"#,
        )
        .expect("policy config should parse");
        let suite = SimulationSuite {
            id: "default".to_string(),
            baseline_policy_id: "missing-baseline".to_string(),
            candidate_policy_ids: vec!["candidate".to_string()],
            system_deck_matrix: false,
            seat_directions: vec![SeatDirection::CandidateAsPlayer],
            minimum_completed_games: 1,
            minimum_candidate_win_rate: 0.5,
            maximum_draw_rate: 1.0,
            minimum_median_actions: 1,
            maximum_median_actions: 20,
            seeds: vec![1],
            rule_preset_ids: Vec::new(),
            max_actions: DEFAULT_MAX_ACTIONS,
        };
        let config = SuiteConfigFile {
            suites: vec![suite],
            rule_presets: Vec::new(),
        };
        assert!(
            config
                .validate_with_policies(&policy_config)
                .expect_err("missing baseline should fail")
                .to_string()
                .contains("missing baseline policy")
        );

        let suite = SimulationSuite {
            id: "default".to_string(),
            baseline_policy_id: "baseline-v1".to_string(),
            candidate_policy_ids: vec!["missing-candidate".to_string()],
            system_deck_matrix: false,
            seat_directions: vec![SeatDirection::CandidateAsPlayer],
            minimum_completed_games: 1,
            minimum_candidate_win_rate: 0.5,
            maximum_draw_rate: 1.0,
            minimum_median_actions: 1,
            maximum_median_actions: 20,
            seeds: vec![1],
            rule_preset_ids: Vec::new(),
            max_actions: DEFAULT_MAX_ACTIONS,
        };
        let config = SuiteConfigFile {
            suites: vec![suite],
            rule_presets: Vec::new(),
        };
        assert!(
            config
                .validate_with_policies(&policy_config)
                .expect_err("missing candidate should fail")
                .to_string()
                .contains("missing candidate policy")
        );

        let suite = SimulationSuite {
            id: "default".to_string(),
            baseline_policy_id: "baseline-v1".to_string(),
            candidate_policy_ids: vec!["candidate".to_string()],
            system_deck_matrix: false,
            seat_directions: vec![SeatDirection::CandidateAsPlayer],
            minimum_completed_games: 1,
            minimum_candidate_win_rate: 0.5,
            maximum_draw_rate: 1.0,
            minimum_median_actions: 1,
            maximum_median_actions: 20,
            seeds: vec![1],
            rule_preset_ids: vec!["missing-preset".to_string()],
            max_actions: DEFAULT_MAX_ACTIONS,
        };
        let config = SuiteConfigFile {
            suites: vec![suite],
            rule_presets: Vec::new(),
        };
        assert!(
            config
                .validate_with_policies(&policy_config)
                .expect_err("missing preset should fail")
                .to_string()
                .contains("rule preset was not found")
        );
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
        assert!(invalid_hex.validate().is_err());

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
        assert!(unknown_card.validate().is_err());
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
        assert!(duplicate_default_hero_hex.validate().is_err());

        let unknown_effect = serde_json::from_str::<SuiteConfigFile>(
            r#"{"suites":[],"rulePresets":[{"id":"bad","cardOverrides":[{"templateId":"ember-squire","effect":{"type":"newEffect"}}]}]}"#,
        )
        .expect_err("new effect fields should be rejected");
        assert!(unknown_effect.to_string().contains("unknown field"));
    }

    #[test]
    fn rule_preset_mana_sources_apply_as_building_backed_mana_wells() {
        let mut game = test_game();
        game.board.mana_sources.push(HexCoord { q: 2, r: 0 });
        let preset = RulePreset {
            id: "mana".to_string(),
            player: None,
            opponent: None,
            units: Vec::new(),
            mana_sources: vec![HexCoord { q: 0, r: 2 }],
            card_overrides: Vec::new(),
        };

        preset.apply(&mut game).expect("preset should apply");

        assert!(game.board.mana_sources.is_empty());
        assert!(game.board.buildings.iter().any(|building| {
            building.id == "preset-mana-1"
                && building.position == HexCoord { q: 0, r: 2 }
                && matches!(building.effect, BuildingEffect::TurnStartMana { amount: 1 })
        }));
    }

    #[test]
    fn self_play_records_invalid_ai_intent_as_illegal_action() {
        let invalid_policy = SoloAiPolicy::new(vec![SoloAiRuleId::InvalidAttack]);
        let baseline_policy = SoloAiPolicy::baseline();
        let config = SuiteConfigFile {
            suites: Vec::new(),
            rule_presets: Vec::new(),
        };
        let spec = GameSpec {
            candidate_policy_id: "candidate".to_string(),
            player_policy_id: "candidate".to_string(),
            opponent_policy_id: "baseline-v1".to_string(),
            player_deck_id: "balanced-starter".to_string(),
            opponent_deck_id: "balanced-starter".to_string(),
            candidate_side: Side::Player,
            seed: 42,
            rule_preset_id: None,
        };

        let result = run_game(&spec, &baseline_policy, &invalid_policy, &config, 10)
            .expect("game should return a result");

        assert!(matches!(
            result.outcome,
            GameOutcome::IllegalAction(ref reason) if reason.contains("piece not found")
        ));
        assert_eq!(result.action_count, 1);
    }

    #[test]
    fn live_ai_advancement_still_finishes_turn_for_invalid_intent() {
        let invalid_policy = SoloAiPolicy::new(vec![SoloAiRuleId::InvalidAttack]);
        let mut forgiving_game = test_game();
        let mut forgiving_frames = Vec::new();

        forgiving_game
            .advance_ai_for_side_with_policy(
                Side::Player,
                &invalid_policy,
                &mut forgiving_frames,
                Some(0),
            )
            .expect("forgiving live path should start attack phase");
        forgiving_game
            .advance_ai_for_side_with_policy(
                Side::Player,
                &invalid_policy,
                &mut forgiving_frames,
                Some(1),
            )
            .expect("forgiving live path should not error");

        assert_eq!(forgiving_game.active_side, Side::Opponent);

        let mut strict_game = test_game();
        let mut strict_frames = Vec::new();
        let strict_outcome = strict_game
            .advance_ai_for_side_with_policy_strict(
                Side::Player,
                &invalid_policy,
                &mut strict_frames,
                Some(0),
            )
            .expect("strict path should start attack phase");
        assert!(matches!(strict_outcome, AiAdvanceOutcome::ActionApplied));

        let strict_outcome = strict_game
            .advance_ai_for_side_with_policy_strict(
                Side::Player,
                &invalid_policy,
                &mut strict_frames,
                Some(1),
            )
            .expect("strict path should return an outcome");

        assert!(matches!(
            strict_outcome,
            AiAdvanceOutcome::IllegalIntent { reason } if reason.contains("piece not found")
        ));
        assert_eq!(strict_game.active_side, Side::Player);
        assert_eq!(strict_frames.len(), 1);
    }

    #[test]
    fn self_play_result_is_deterministic_for_same_spec() {
        let policy_config = AiPolicyConfig::from_json(
            r#"{"defaultPolicyId":"baseline-v1","policies":[{"id":"baseline-v1","rules":["inRangeAttack","usefulSpell","buildManaSource","highestCostUnitSummon","moveTowardPlayerHero"]},{"id":"candidate","rules":["inRangeAttack","highestCostUnitSummon","usefulSpell","moveTowardPlayerHero","buildManaSource"]}]}"#,
        )
        .expect("policy config should parse");
        let baseline = crate::match_session::SoloAiPolicy::from_definition(
            policy_config
                .policy("baseline-v1")
                .expect("baseline exists"),
        );
        let candidate = crate::match_session::SoloAiPolicy::from_definition(
            policy_config.policy("candidate").expect("candidate exists"),
        );
        let config = SuiteConfigFile {
            suites: Vec::new(),
            rule_presets: Vec::new(),
        };
        let spec = GameSpec {
            candidate_policy_id: "candidate".to_string(),
            player_policy_id: "candidate".to_string(),
            opponent_policy_id: "baseline-v1".to_string(),
            player_deck_id: "balanced-starter".to_string(),
            opponent_deck_id: "balanced-starter".to_string(),
            candidate_side: Side::Player,
            seed: 42,
            rule_preset_id: None,
        };

        let first =
            run_game(&spec, &baseline, &candidate, &config, 40).expect("first game should run");
        let second =
            run_game(&spec, &baseline, &candidate, &config, 40).expect("second game should run");

        assert_eq!(first.outcome, second.outcome);
        assert_eq!(first.action_count, second.action_count);
        assert_eq!(first.frames.len(), second.frames.len());
        assert_eq!(
            first.frames.last().map(|frame| &frame.snapshot_json),
            second.frames.last().map(|frame| &frame.snapshot_json)
        );
    }

    #[test]
    fn promotion_gate_requires_all_thresholds() {
        let suite = SimulationSuite {
            id: "default".to_string(),
            baseline_policy_id: "baseline-v1".to_string(),
            candidate_policy_ids: vec!["candidate".to_string()],
            system_deck_matrix: true,
            seat_directions: vec![SeatDirection::CandidateAsPlayer],
            minimum_completed_games: 200,
            minimum_candidate_win_rate: 0.6,
            maximum_draw_rate: 0.1,
            minimum_median_actions: 8,
            maximum_median_actions: 240,
            seeds: vec![1],
            rule_preset_ids: Vec::new(),
            max_actions: DEFAULT_MAX_ACTIONS,
        };

        assert!(
            promotion_gate(200, 0.6, 0.1, 0, 0, 20, &suite).passed,
            "threshold boundary should pass"
        );
        assert!(
            !promotion_gate(199, 0.6, 0.1, 0, 0, 20, &suite).passed,
            "insufficient completed games should fail"
        );
        assert!(
            !promotion_gate(200, 0.59, 0.1, 0, 0, 20, &suite).passed,
            "low win rate should fail"
        );
        assert!(
            !promotion_gate(200, 0.6, 0.11, 0, 0, 20, &suite).passed,
            "high draw rate should fail"
        );
        assert!(
            !promotion_gate(200, 0.6, 0.1, 1, 0, 20, &suite).passed,
            "illegal actions should fail"
        );
        assert!(
            !promotion_gate(200, 0.6, 0.1, 0, 1, 20, &suite).passed,
            "timeouts should fail"
        );
    }
}
