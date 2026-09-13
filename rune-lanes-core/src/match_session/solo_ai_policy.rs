use std::collections::HashSet;
use std::fs;
use std::path::Path;

use super::{
    ActionTarget, BuildingEffect, Card, CardKind, HexCoord, ItemActiveEffect, PieceView, Side,
    SpellEffect, card_interactions,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub(crate) struct SoloAiPolicy {
    rules: Vec<SoloAiRuleId>,
}

impl Default for SoloAiPolicy {
    fn default() -> Self {
        Self::from_checked_in_config().unwrap_or_else(|_| Self::baseline())
    }
}

impl SoloAiPolicy {
    pub(crate) fn baseline() -> Self {
        Self::new(baseline_rules())
    }

    pub(crate) fn new(rules: Vec<SoloAiRuleId>) -> Self {
        Self { rules }
    }

    #[cfg(test)]
    pub(crate) fn rule_order(&self) -> &[SoloAiRuleId] {
        &self.rules
    }

    pub(crate) fn from_definition(definition: &AiPolicyDefinition) -> Self {
        Self::new(definition.rules.clone())
    }

    pub(crate) fn from_checked_in_config() -> Result<Self, AiPolicyConfigError> {
        let config = AiPolicyConfig::load_from_default_path()?;
        config.default_policy()
    }

    pub(super) fn decide(&self, view: &SoloAiView) -> SoloAiDecision {
        for rule in &self.rules {
            if let Some(intent) = self.evaluate_rule(*rule, view) {
                return SoloAiDecision::TakeAction(intent);
            }
        }

        SoloAiDecision::FinishTurn
    }

    fn evaluate_rule(&self, rule: SoloAiRuleId, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        match rule {
            #[cfg(test)]
            SoloAiRuleId::InvalidAttack => Some(SoloAiActionIntent::Attack {
                attacker_id: "missing-attacker".to_string(),
                target_id: "missing-target".to_string(),
            }),
            SoloAiRuleId::InRangeAttack => self.in_range_attack(view),
            SoloAiRuleId::UsefulSpell => self.useful_spell(view),
            SoloAiRuleId::UsefulItemActivation => self.useful_item_activation(view),
            SoloAiRuleId::UsefulItemEquip => self.useful_item_equip(view),
            SoloAiRuleId::BuildManaSource => self.build_mana_source(view),
            SoloAiRuleId::HighestCostUnitSummon => self.highest_cost_unit_summon(view),
            SoloAiRuleId::MoveTowardPlayerHero => self.move_toward_player_hero(view),
        }
    }

    fn in_range_attack(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        let mut attackers = view.opponent_pieces.clone();
        attackers.sort_by_key(|piece| {
            if piece.id == view.opponent_hero.id {
                1
            } else {
                0
            }
        });

        for attacker in attackers {
            if attacker.ap_remaining == 0 || attacker.has_attacked {
                continue;
            }
            if piece_can_attack(&attacker, &view.player_hero) {
                return Some(SoloAiActionIntent::Attack {
                    attacker_id: attacker.id,
                    target_id: view.player_hero.id.clone(),
                });
            }
            if let Some(target) = view
                .player_units
                .iter()
                .find(|unit| piece_can_attack(&attacker, unit))
            {
                return Some(SoloAiActionIntent::Attack {
                    attacker_id: attacker.id,
                    target_id: target.id.clone(),
                });
            }
        }

        None
    }

    fn useful_spell(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        if view.opponent_hero.ap_remaining == 0 {
            return None;
        }

        for card in view
            .opponent_hand
            .iter()
            .filter(|card| card.cost <= view.opponent_mana)
        {
            let CardKind::Spell { effect, .. } = &card.kind else {
                continue;
            };

            let target = match effect {
                SpellEffect::Damage { .. } => {
                    let legal_targets = legal_spell_targets_for_ai(card, view, &view.player_pieces);
                    legal_targets
                        .iter()
                        .copied()
                        .find(|piece| piece.id == view.player_hero.id)
                        .or_else(|| legal_targets.into_iter().next())
                }
                SpellEffect::AreaDamage { .. } | SpellEffect::LineDamage { .. } => {
                    legal_spell_targets_for_ai(card, view, &view.player_pieces)
                        .into_iter()
                        .next()
                }
                SpellEffect::Heal { .. } => {
                    legal_spell_targets_for_ai(card, view, &view.opponent_pieces)
                        .into_iter()
                        .find(|piece| view.damaged_piece_ids.contains(&piece.id))
                }
                SpellEffect::Buff { .. } | SpellEffect::StatBuff { .. } => {
                    legal_spell_targets_for_ai(card, view, &view.opponent_units)
                        .into_iter()
                        .max_by_key(|piece| piece.attack)
                }
                SpellEffect::Draw { .. } => {
                    if view.opponent_deck_count + view.opponent_discard_count > 0
                        && legal_spell_targets_for_ai(
                            card,
                            view,
                            std::slice::from_ref(&view.opponent_hero),
                        )
                        .into_iter()
                        .next()
                        .is_some()
                    {
                        Some(&view.opponent_hero)
                    } else {
                        None
                    }
                }
            };

            if let Some(target) = target {
                return Some(SoloAiActionIntent::PlayCard {
                    card_id: card.id.clone(),
                    target: ActionTarget::Piece {
                        piece_id: target.id.clone(),
                    },
                });
            }
        }

        None
    }

    fn highest_cost_unit_summon(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        if view.opponent_hero.ap_remaining == 0 {
            return None;
        }

        let card = view
            .opponent_hand
            .iter()
            .filter(|card| card.cost <= view.opponent_mana)
            .filter(|card| matches!(card.kind, CardKind::Unit { .. }))
            .max_by_key(|card| card.cost)?;

        let coord = self.best_summon_hex(view)?;

        Some(SoloAiActionIntent::PlayCard {
            card_id: card.id.clone(),
            target: ActionTarget::Hex { coord },
        })
    }

    fn useful_item_equip(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        if view.opponent_hero.ap_remaining == 0 {
            return None;
        }
        for card in view
            .opponent_hand
            .iter()
            .filter(|card| card.cost <= view.opponent_mana)
        {
            let CardKind::Item { targets, range, .. } = card.kind else {
                continue;
            };
            let target = view
                .opponent_pieces
                .iter()
                .filter(|piece| piece.ap_remaining > 0 || piece.id == view.opponent_hero.id)
                .filter(|piece| card_interactions::target_policy_allows(targets, piece.is_hero))
                .filter(|piece| {
                    view.opponent_hero.position.distance(piece.position) <= i32::from(range)
                })
                .max_by_key(|piece| piece.attack)?;

            return Some(SoloAiActionIntent::PlayCard {
                card_id: card.id.clone(),
                target: ActionTarget::Piece {
                    piece_id: target.id.clone(),
                },
            });
        }
        None
    }

    fn useful_item_activation(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        for item in &view.opponent_carried_items {
            if item.carrier.ap_remaining == 0 || item.active_used_this_turn {
                continue;
            }
            let target = match &item.active {
                ItemActiveEffect::HealCarrier { .. } => view
                    .damaged_piece_ids
                    .contains(&item.carrier.id)
                    .then_some(None),
                ItemActiveEffect::DamageTarget { range, .. } => view
                    .player_pieces
                    .iter()
                    .filter(|piece| {
                        item.carrier.position.distance(piece.position) <= i32::from(*range)
                    })
                    .max_by_key(|piece| {
                        if piece.id == view.player_hero.id {
                            99
                        } else {
                            piece.attack
                        }
                    })
                    .map(|piece| {
                        Some(ActionTarget::Piece {
                            piece_id: piece.id.clone(),
                        })
                    }),
                ItemActiveEffect::Draw { .. } => {
                    (view.opponent_deck_count + view.opponent_discard_count > 0).then_some(None)
                }
                ItemActiveEffect::StatMarker { .. } => view
                    .player_pieces
                    .iter()
                    .any(|piece| item.carrier.position.distance(piece.position) <= 2)
                    .then_some(None),
            }?;

            return Some(SoloAiActionIntent::ActivateItem {
                carrier_id: item.carrier.id.clone(),
                item_id: item.item_id.clone(),
                target,
            });
        }
        None
    }

    fn build_mana_source(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        if view.opponent_hero.ap_remaining == 0 {
            return None;
        }

        let card = view.opponent_hand.iter().find(|card| {
            card.cost <= view.opponent_mana
                && matches!(
                    card.kind,
                    CardKind::ManaSource
                        | CardKind::Building {
                            effect: BuildingEffect::TurnStartMana { .. }
                        }
                )
        })?;
        let coord = self.best_mana_source_hex(view)?;

        Some(SoloAiActionIntent::PlayCard {
            card_id: card.id.clone(),
            target: ActionTarget::Hex { coord },
        })
    }

    fn move_toward_player_hero(&self, view: &SoloAiView) -> Option<SoloAiActionIntent> {
        let player_hero = view.player_hero.position;
        let (piece, destination) = view
            .opponent_pieces
            .iter()
            .filter(|piece| piece.ap_remaining > 0)
            .filter(|piece| {
                !view
                    .player_pieces
                    .iter()
                    .any(|target| piece_can_attack(piece, target))
            })
            .filter_map(|piece| {
                self.empty_neighbors(view, piece.position)
                    .into_iter()
                    .min_by_key(|coord| coord.distance(player_hero))
                    .filter(|coord| {
                        coord.distance(player_hero) < piece.position.distance(player_hero)
                    })
                    .map(|coord| (piece, coord))
            })
            .next()?;

        Some(SoloAiActionIntent::MovePiece {
            piece_id: piece.id.clone(),
            to: destination,
        })
    }

    fn best_summon_hex(&self, view: &SoloAiView) -> Option<HexCoord> {
        self.empty_neighbors(view, view.opponent_hero.position)
            .into_iter()
            .min_by_key(|coord| coord.distance(view.player_hero.position))
    }

    fn best_mana_source_hex(&self, view: &SoloAiView) -> Option<HexCoord> {
        self.empty_neighbors(view, view.opponent_hero.position)
            .into_iter()
            .filter(|coord| !view.mana_sources.contains(coord))
            .min_by_key(|coord| coord.distance(view.player_hero.position))
    }

    fn empty_neighbors(&self, view: &SoloAiView, coord: HexCoord) -> Vec<HexCoord> {
        coord
            .neighbors()
            .into_iter()
            .filter(|neighbor| view.valid_hexes.contains(neighbor))
            .filter(|neighbor| !view.occupied_hexes.contains(neighbor))
            .collect()
    }
}

fn legal_spell_targets_for_ai<'a>(
    card: &Card,
    view: &SoloAiView,
    candidates: &'a [PieceView],
) -> Vec<&'a PieceView> {
    card_interactions::legal_spell_targets(
        card,
        view.controlled_side,
        view.opponent_hero.position,
        &view.opponent_hero.id,
        candidates.iter(),
    )
}

#[derive(Clone, Debug)]
pub(super) struct SoloAiView {
    pub(super) controlled_side: Side,
    pub(super) opponent_hero: PieceView,
    pub(super) player_hero: PieceView,
    pub(super) opponent_pieces: Vec<PieceView>,
    pub(super) player_pieces: Vec<PieceView>,
    pub(super) opponent_units: Vec<PieceView>,
    pub(super) player_units: Vec<PieceView>,
    pub(super) opponent_carried_items: Vec<SoloAiCarriedItem>,
    pub(super) opponent_hand: Vec<Card>,
    pub(super) opponent_mana: u8,
    pub(super) opponent_deck_count: usize,
    pub(super) opponent_discard_count: usize,
    pub(super) valid_hexes: HashSet<HexCoord>,
    pub(super) occupied_hexes: HashSet<HexCoord>,
    pub(super) mana_sources: HashSet<HexCoord>,
    pub(super) damaged_piece_ids: HashSet<String>,
}

#[derive(Clone, Debug)]
pub(super) struct SoloAiCarriedItem {
    pub(super) carrier: PieceView,
    pub(super) item_id: String,
    pub(super) active: ItemActiveEffect,
    pub(super) active_used_this_turn: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SoloAiDecision {
    TakeAction(SoloAiActionIntent),
    FinishTurn,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SoloAiActionIntent {
    Attack {
        attacker_id: String,
        target_id: String,
    },
    PlayCard {
        card_id: String,
        target: ActionTarget,
    },
    MovePiece {
        piece_id: String,
        to: HexCoord,
    },
    ActivateItem {
        carrier_id: String,
        item_id: String,
        target: Option<ActionTarget>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SoloAiRuleId {
    #[cfg(test)]
    InvalidAttack,
    InRangeAttack,
    UsefulSpell,
    UsefulItemActivation,
    UsefulItemEquip,
    BuildManaSource,
    HighestCostUnitSummon,
    MoveTowardPlayerHero,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiPolicyConfig {
    pub(crate) default_policy_id: String,
    pub(crate) policies: Vec<AiPolicyDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AiPolicyDefinition {
    pub(crate) id: String,
    pub(crate) rules: Vec<SoloAiRuleId>,
}

#[derive(Debug)]
pub(crate) enum AiPolicyConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
    MissingDefault(String),
    EmptyPolicy(String),
    DuplicatePolicy(String),
}

impl std::fmt::Display for AiPolicyConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not read AI policy config: {error}"),
            Self::Json(error) => write!(f, "could not parse AI policy config: {error}"),
            Self::MissingDefault(id) => write!(f, "default AI policy was not found: {id}"),
            Self::EmptyPolicy(id) => write!(f, "AI policy has no rules: {id}"),
            Self::DuplicatePolicy(id) => write!(f, "duplicate AI policy id: {id}"),
        }
    }
}

impl std::error::Error for AiPolicyConfigError {}

impl From<std::io::Error> for AiPolicyConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for AiPolicyConfigError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl AiPolicyConfig {
    pub(crate) fn load_from_default_path() -> Result<Self, AiPolicyConfigError> {
        Self::load_from_path(default_policy_config_path())
    }

    pub(crate) fn load_from_path(path: impl AsRef<Path>) -> Result<Self, AiPolicyConfigError> {
        let text = fs::read_to_string(path)?;
        Self::from_json(&text)
    }

    pub(crate) fn from_json(text: &str) -> Result<Self, AiPolicyConfigError> {
        let config: Self = serde_json::from_str(text)?;
        config.validate()?;
        Ok(config)
    }

    pub(crate) fn validate(&self) -> Result<(), AiPolicyConfigError> {
        let mut ids = HashSet::new();
        for policy in &self.policies {
            if !ids.insert(policy.id.clone()) {
                return Err(AiPolicyConfigError::DuplicatePolicy(policy.id.clone()));
            }
            if policy.rules.is_empty() {
                return Err(AiPolicyConfigError::EmptyPolicy(policy.id.clone()));
            }
        }
        if !self
            .policies
            .iter()
            .any(|policy| policy.id == self.default_policy_id)
        {
            return Err(AiPolicyConfigError::MissingDefault(
                self.default_policy_id.clone(),
            ));
        }
        Ok(())
    }

    pub(crate) fn default_policy(&self) -> Result<SoloAiPolicy, AiPolicyConfigError> {
        let policy = self
            .policy(&self.default_policy_id)
            .ok_or_else(|| AiPolicyConfigError::MissingDefault(self.default_policy_id.clone()))?;
        Ok(SoloAiPolicy::from_definition(policy))
    }

    pub(crate) fn policy(&self, id: &str) -> Option<&AiPolicyDefinition> {
        self.policies.iter().find(|policy| policy.id == id)
    }

    pub(crate) fn set_default_policy_id(&mut self, id: &str) -> Result<(), AiPolicyConfigError> {
        if self.policy(id).is_none() {
            return Err(AiPolicyConfigError::MissingDefault(id.to_string()));
        }
        self.default_policy_id = id.to_string();
        Ok(())
    }
}

pub(crate) fn default_policy_config_path() -> &'static str {
    if Path::new("backend/config/ai-policies.json").exists() {
        "backend/config/ai-policies.json"
    } else {
        "config/ai-policies.json"
    }
}

fn baseline_rules() -> Vec<SoloAiRuleId> {
    vec![
        SoloAiRuleId::InRangeAttack,
        SoloAiRuleId::UsefulSpell,
        SoloAiRuleId::UsefulItemActivation,
        SoloAiRuleId::UsefulItemEquip,
        SoloAiRuleId::BuildManaSource,
        SoloAiRuleId::HighestCostUnitSummon,
        SoloAiRuleId::MoveTowardPlayerHero,
    ]
}

fn piece_can_attack(attacker: &PieceView, target: &PieceView) -> bool {
    let distance = attacker.position.distance(target.position);
    distance >= 1 && distance <= i32::from(attacker.attack_range)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_session::Rarity;

    fn policy_decision(view: SoloAiView) -> SoloAiDecision {
        SoloAiPolicy::default().decide(&view)
    }

    fn base_view() -> SoloAiView {
        let opponent_hero = piece("opponent-hero", Side::Opponent, hex(0, -3));
        let player_hero = piece("player-hero", Side::Player, hex(0, 3));
        let opponent_pieces = vec![opponent_hero.clone()];
        let player_pieces = vec![player_hero.clone()];
        let valid_hexes = (-3..=3)
            .flat_map(|q| (-3..=3).map(move |r| hex(q, r)))
            .filter(|coord| coord.distance(hex(0, 0)) <= 3)
            .collect();
        let occupied_hexes = vec![opponent_hero.position, player_hero.position]
            .into_iter()
            .collect();

        SoloAiView {
            controlled_side: Side::Opponent,
            opponent_hero,
            player_hero,
            opponent_pieces,
            player_pieces,
            opponent_units: Vec::new(),
            player_units: Vec::new(),
            opponent_carried_items: Vec::new(),
            opponent_hand: Vec::new(),
            opponent_mana: 0,
            opponent_deck_count: 0,
            opponent_discard_count: 0,
            valid_hexes,
            occupied_hexes,
            mana_sources: HashSet::new(),
            damaged_piece_ids: HashSet::new(),
        }
    }

    fn piece(id: &str, side: Side, position: HexCoord) -> PieceView {
        PieceView {
            id: id.to_string(),
            side,
            position,
            attack: 1,
            attack_range: 1,
            ap_remaining: 1,
            has_attacked: false,
            is_hero: id.ends_with("hero"),
        }
    }

    fn hex(q: i32, r: i32) -> HexCoord {
        HexCoord { q, r }
    }

    fn unit_card(id: &str, cost: u8) -> Card {
        card(
            id,
            cost,
            CardKind::Unit {
                attack: 1,
                armor: 1,
                max_ap: 1,
            },
        )
    }

    fn spell_card(id: &str, cost: u8, effect: SpellEffect) -> Card {
        card(
            id,
            cost,
            CardKind::Spell {
                range: 6,
                priority: 1,
                effect,
            },
        )
    }

    fn card(id: &str, cost: u8, kind: CardKind) -> Card {
        Card {
            id: id.to_string(),
            template_id: id.to_string(),
            name: id.to_string(),
            rarity: Rarity::Basic,
            cost,
            text: String::new(),
            kind,
        }
    }

    #[test]
    fn rule_order_is_stable() {
        assert_eq!(
            SoloAiPolicy::baseline().rule_order(),
            &[
                SoloAiRuleId::InRangeAttack,
                SoloAiRuleId::UsefulSpell,
                SoloAiRuleId::UsefulItemActivation,
                SoloAiRuleId::UsefulItemEquip,
                SoloAiRuleId::BuildManaSource,
                SoloAiRuleId::HighestCostUnitSummon,
                SoloAiRuleId::MoveTowardPlayerHero,
            ]
        );
    }

    #[test]
    fn adjacent_attack_beats_spell_summon_and_move() {
        let mut view = base_view();
        let opponent_unit = piece("opponent-unit", Side::Opponent, hex(0, 2));
        view.opponent_pieces.push(opponent_unit.clone());
        view.opponent_units.push(opponent_unit.clone());
        view.occupied_hexes.insert(opponent_unit.position);
        view.opponent_hand = vec![
            spell_card("damage", 1, SpellEffect::Damage { amount: 1 }),
            unit_card("unit", 1),
        ];
        view.opponent_mana = 2;

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::Attack {
                attacker_id: "opponent-unit".to_string(),
                target_id: "player-hero".to_string(),
            })
        );
    }

    #[test]
    fn spent_or_already_attacked_pieces_are_ignored() {
        let mut view = base_view();
        let mut spent = piece("spent-unit", Side::Opponent, hex(0, 2));
        spent.ap_remaining = 0;
        let mut attacked = piece("attacked-unit", Side::Opponent, hex(1, 2));
        attacked.has_attacked = true;
        view.opponent_pieces
            .extend([spent.clone(), attacked.clone()]);
        view.opponent_units
            .extend([spent.clone(), attacked.clone()]);
        view.occupied_hexes
            .extend([spent.position, attacked.position]);

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::MovePiece {
                piece_id: "opponent-hero".to_string(),
                to: hex(0, -2),
            })
        );
    }

    #[test]
    fn damage_spell_targets_player_hero_when_legal() {
        let mut view = base_view();
        let player_unit = piece("player-unit", Side::Player, hex(0, 2));
        view.player_pieces.push(player_unit.clone());
        view.player_units.push(player_unit.clone());
        view.occupied_hexes.insert(player_unit.position);
        view.opponent_hand = vec![spell_card("damage", 1, SpellEffect::Damage { amount: 1 })];
        view.opponent_mana = 1;

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard {
                card_id: "damage".to_string(),
                target: ActionTarget::Piece {
                    piece_id: "player-hero".to_string(),
                },
            })
        );
    }

    #[test]
    fn heal_spell_only_fires_for_damaged_allied_pieces() {
        let mut view = base_view();
        view.opponent_hand = vec![spell_card("heal", 1, SpellEffect::Heal { amount: 1 })];
        view.opponent_mana = 1;

        assert_eq!(
            policy_decision(view.clone()),
            SoloAiDecision::TakeAction(SoloAiActionIntent::MovePiece {
                piece_id: "opponent-hero".to_string(),
                to: hex(0, -2),
            })
        );

        view.damaged_piece_ids.insert("opponent-hero".to_string());

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard {
                card_id: "heal".to_string(),
                target: ActionTarget::Piece {
                    piece_id: "opponent-hero".to_string(),
                },
            })
        );
    }

    #[test]
    fn buff_spell_picks_highest_attack_allied_unit() {
        let mut view = base_view();
        let mut low_attack = piece("low", Side::Opponent, hex(0, -2));
        low_attack.attack = 1;
        let mut high_attack = piece("high", Side::Opponent, hex(1, -2));
        high_attack.attack = 4;
        view.opponent_pieces
            .extend([low_attack.clone(), high_attack.clone()]);
        view.opponent_units
            .extend([low_attack.clone(), high_attack.clone()]);
        view.occupied_hexes
            .extend([low_attack.position, high_attack.position]);
        view.opponent_hand = vec![spell_card(
            "buff",
            1,
            SpellEffect::Buff {
                attack: 1,
                armor: 1,
            },
        )];
        view.opponent_mana = 1;

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard {
                card_id: "buff".to_string(),
                target: ActionTarget::Piece {
                    piece_id: "high".to_string(),
                },
            })
        );
    }

    #[test]
    fn selected_spell_target_is_legal_through_card_interaction_query() {
        let mut view = base_view();
        let player_unit = piece("player-unit", Side::Player, hex(0, 2));
        view.player_pieces.push(player_unit.clone());
        view.player_units.push(player_unit);
        view.occupied_hexes.insert(hex(0, 2));
        let damage_spell = spell_card("damage", 1, SpellEffect::Damage { amount: 1 });
        view.opponent_hand = vec![damage_spell.clone()];
        view.opponent_mana = 1;

        let SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard {
            target: ActionTarget::Piece { piece_id },
            ..
        }) = policy_decision(view.clone())
        else {
            panic!("damage spell should choose a piece target");
        };

        let legal_target_ids: HashSet<_> = card_interactions::legal_spell_targets(
            &damage_spell,
            Side::Opponent,
            view.opponent_hero.position,
            &view.opponent_hero.id,
            view.player_pieces.iter(),
        )
        .into_iter()
        .map(|piece| piece.id.clone())
        .collect();

        assert!(legal_target_ids.contains(&piece_id));
    }

    #[test]
    fn summon_picks_highest_cost_affordable_unit_and_closest_valid_hex() {
        let mut view = base_view();
        view.opponent_hand = vec![unit_card("cheap", 1), unit_card("expensive", 3)];
        view.opponent_mana = 3;

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard {
                card_id: "expensive".to_string(),
                target: ActionTarget::Hex { coord: hex(0, -2) },
            })
        );
    }

    #[test]
    fn mana_source_card_builds_before_unit_summon() {
        let mut view = base_view();
        view.opponent_hand = vec![
            unit_card("unit", 2),
            card("mana-well", 2, CardKind::ManaSource),
        ];
        view.opponent_mana = 3;

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::PlayCard {
                card_id: "mana-well".to_string(),
                target: ActionTarget::Hex { coord: hex(0, -2) },
            })
        );
    }

    #[test]
    fn move_picks_a_closer_empty_neighboring_hex() {
        let view = base_view();

        assert_eq!(
            policy_decision(view),
            SoloAiDecision::TakeAction(SoloAiActionIntent::MovePiece {
                piece_id: "opponent-hero".to_string(),
                to: hex(0, -2),
            })
        );
    }

    #[test]
    fn no_legal_rule_returns_finish_turn() {
        let mut view = base_view();
        view.opponent_hero.ap_remaining = 0;
        view.opponent_pieces[0].ap_remaining = 0;

        assert_eq!(policy_decision(view), SoloAiDecision::FinishTurn);
    }
}
