use super::{
    ActionTarget, BuffTargetPolicy, Card, CardKind, CardSummary, CarriedItem, Hero, HexBoard,
    HexCoord, ItemActiveEffect, ItemPassiveEffect, MatchError, MatchProgressionLoadout, PieceView,
    Side, SpellEffect, StatMarker, Unit,
};

pub(crate) struct PlannedUnitPlay {
    pub(crate) coord: HexCoord,
}

pub(crate) struct PlannedSpellPlay {
    pub(crate) priority: u8,
    pub(crate) target_id: String,
}

pub(crate) struct PlannedItemPlay {
    pub(crate) carrier_id: String,
}

pub(crate) enum ResolvedSpellEffect {
    Heal {
        piece_id: String,
        amount: i32,
    },
    Buff {
        piece_id: String,
        attack: i32,
        armor: i32,
        max_ap: i8,
        targets: BuffTargetPolicy,
    },
    Damage {
        piece_ids: Vec<String>,
        amount: i32,
    },
    Draw {
        amount: u8,
    },
}

pub(crate) enum SpellLog {
    Heal { piece_id: String },
    Buff { piece_id: String },
    Damage { piece_id: String },
    Draw { amount: u8 },
    AreaDamage { piece_id: String },
    LineDamage,
}

pub(crate) struct ResolvedSpell {
    pub(crate) effect: ResolvedSpellEffect,
    pub(crate) log: SpellLog,
}

pub(crate) enum ResolvedItemActiveEffect {
    HealCarrier {
        carrier_id: String,
        amount: i32,
    },
    DamageTarget {
        target_id: String,
        amount: i32,
    },
    Draw {
        amount: u8,
    },
    StatMarker {
        carrier_id: String,
        marker: StatMarker,
    },
}

pub(crate) struct ResolvedItemActivation {
    pub(crate) item_name: String,
    pub(crate) effect: ResolvedItemActiveEffect,
}

pub(crate) fn plan_unit_play(
    card: &Card,
    target: ActionTarget,
    caster_position: HexCoord,
    board: &HexBoard,
    is_occupied: impl Fn(HexCoord) -> bool,
) -> Result<PlannedUnitPlay, MatchError> {
    if !matches!(card.kind, CardKind::Unit { .. }) {
        return Err(MatchError::InvalidTarget);
    }

    let ActionTarget::Hex { coord } = target else {
        return Err(MatchError::InvalidTarget);
    };

    validate_unit_target(coord, caster_position, board, is_occupied)?;

    Ok(PlannedUnitPlay { coord })
}

pub(crate) fn plan_spell_play(
    card: &Card,
    target: ActionTarget,
    side: Side,
    caster_position: HexCoord,
    caster_hero_id: &str,
    target_piece: &PieceView,
) -> Result<PlannedSpellPlay, MatchError> {
    let CardKind::Spell { priority, .. } = &card.kind else {
        return Err(MatchError::InvalidTarget);
    };

    let ActionTarget::Piece { piece_id } = target else {
        return Err(MatchError::InvalidTarget);
    };

    if piece_id != target_piece.id {
        return Err(MatchError::PieceNotFound);
    }

    if !spell_target_is_legal(card, side, caster_position, caster_hero_id, target_piece) {
        return Err(MatchError::InvalidTarget);
    }

    Ok(PlannedSpellPlay {
        priority: *priority,
        target_id: target_piece.id.clone(),
    })
}

pub(crate) fn plan_item_play(
    card: &Card,
    target: ActionTarget,
    side: Side,
    caster_position: HexCoord,
    target_carrier: &PieceView,
) -> Result<PlannedItemPlay, MatchError> {
    let CardKind::Item {
        range,
        targets,
        passive,
        active,
    } = &card.kind
    else {
        return Err(MatchError::InvalidTarget);
    };

    let ActionTarget::Piece { piece_id } = target else {
        return Err(MatchError::InvalidTarget);
    };

    if piece_id != target_carrier.id {
        return Err(MatchError::PieceNotFound);
    }

    validate_item_target(side, caster_position, *range, *targets, target_carrier)?;
    if target_carrier.is_hero && !item_has_effect_on_hero(passive, active.as_ref()) {
        return Err(MatchError::InvalidTarget);
    }

    Ok(PlannedItemPlay {
        carrier_id: target_carrier.id.clone(),
    })
}

pub(crate) fn legal_spell_targets<'a>(
    card: &Card,
    side: Side,
    caster_position: HexCoord,
    caster_hero_id: &str,
    candidates: impl IntoIterator<Item = &'a PieceView>,
) -> Vec<&'a PieceView> {
    candidates
        .into_iter()
        .filter(|target| spell_target_is_legal(card, side, caster_position, caster_hero_id, target))
        .collect()
}

fn spell_target_is_legal(
    card: &Card,
    side: Side,
    caster_position: HexCoord,
    caster_hero_id: &str,
    target: &PieceView,
) -> bool {
    let CardKind::Spell { range, effect, .. } = &card.kind else {
        return false;
    };

    caster_position.distance(target.position) <= i32::from(*range)
        && validate_spell_target(side, effect, caster_position, caster_hero_id, target).is_ok()
}

pub(crate) fn validate_spell_target(
    side: Side,
    effect: &SpellEffect,
    caster_position: HexCoord,
    caster_hero_id: &str,
    target: &PieceView,
) -> Result<(), MatchError> {
    if target.is_hero {
        match effect {
            SpellEffect::Buff { .. } => return Err(MatchError::InvalidTarget),
            SpellEffect::StatBuff {
                attack,
                armor,
                max_ap,
                ..
            } if !stat_change_has_effect_on_hero(*attack, *armor, *max_ap) => {
                return Err(MatchError::InvalidTarget);
            }
            _ => {}
        }
    }

    match effect {
        SpellEffect::Heal { .. } | SpellEffect::Buff { .. } | SpellEffect::StatBuff { .. }
            if target.side.team() != side.team() =>
        {
            Err(MatchError::InvalidTarget)
        }
        SpellEffect::Damage { .. }
        | SpellEffect::AreaDamage { .. }
        | SpellEffect::LineDamage { .. }
            if target.side.team() == side.team() =>
        {
            Err(MatchError::InvalidTarget)
        }
        SpellEffect::Draw { .. } if target.side != side || target.id != caster_hero_id => {
            Err(MatchError::InvalidTarget)
        }
        SpellEffect::LineDamage { .. }
            if caster_position.direction_to(target.position).is_none() =>
        {
            Err(MatchError::InvalidTarget)
        }
        SpellEffect::Buff { .. } if target.id == caster_hero_id => Err(MatchError::InvalidTarget),
        SpellEffect::StatBuff { targets, .. }
            if !target_policy_allows(*targets, target.is_hero) =>
        {
            Err(MatchError::InvalidTarget)
        }
        _ => Ok(()),
    }
}

pub(crate) fn validate_item_target(
    side: Side,
    caster_position: HexCoord,
    range: u8,
    targets: BuffTargetPolicy,
    target: &PieceView,
) -> Result<(), MatchError> {
    if target.side.team() != side.team()
        || caster_position.distance(target.position) > i32::from(range)
        || !target_policy_allows(targets, target.is_hero)
    {
        return Err(MatchError::InvalidTarget);
    }

    Ok(())
}

pub(crate) fn resolve_spell(
    card: &CardSummary,
    side: Side,
    caster_position: HexCoord,
    caster_hero_id: &str,
    target: &PieceView,
    enemy_pieces: &[PieceView],
    progression: &MatchProgressionLoadout,
) -> Result<ResolvedSpell, MatchError> {
    let CardKind::Spell { range, effect, .. } = &card.kind else {
        return Err(MatchError::InvalidTarget);
    };

    validate_spell_target(side, effect, caster_position, caster_hero_id, target)?;

    let resolved = match effect {
        SpellEffect::Heal { amount } => ResolvedSpell {
            effect: ResolvedSpellEffect::Heal {
                piece_id: target.id.clone(),
                amount: *amount,
            },
            log: SpellLog::Heal {
                piece_id: target.id.clone(),
            },
        },
        SpellEffect::Buff { attack, armor } => ResolvedSpell {
            effect: ResolvedSpellEffect::Buff {
                piece_id: target.id.clone(),
                attack: *attack,
                armor: *armor,
                max_ap: 0,
                targets: BuffTargetPolicy::UnitsOnly,
            },
            log: SpellLog::Buff {
                piece_id: target.id.clone(),
            },
        },
        SpellEffect::StatBuff {
            attack,
            armor,
            max_ap,
            targets,
        } => ResolvedSpell {
            effect: ResolvedSpellEffect::Buff {
                piece_id: target.id.clone(),
                attack: *attack,
                armor: *armor,
                max_ap: *max_ap,
                targets: *targets,
            },
            log: SpellLog::Buff {
                piece_id: target.id.clone(),
            },
        },
        SpellEffect::Damage { amount } => ResolvedSpell {
            effect: ResolvedSpellEffect::Damage {
                piece_ids: vec![target.id.clone()],
                amount: damage_with_progression(*amount, progression),
            },
            log: SpellLog::Damage {
                piece_id: target.id.clone(),
            },
        },
        SpellEffect::Draw { amount } => ResolvedSpell {
            effect: ResolvedSpellEffect::Draw { amount: *amount },
            log: SpellLog::Draw { amount: *amount },
        },
        SpellEffect::AreaDamage { amount, radius } => ResolvedSpell {
            effect: ResolvedSpellEffect::Damage {
                piece_ids: enemy_piece_ids_in_area(
                    enemy_pieces,
                    target.position,
                    i32::from(*radius),
                ),
                amount: damage_with_progression(*amount, progression),
            },
            log: SpellLog::AreaDamage {
                piece_id: target.id.clone(),
            },
        },
        SpellEffect::LineDamage { amount } => {
            let Some(direction) = caster_position.direction_to(target.position) else {
                return Err(MatchError::InvalidTarget);
            };

            ResolvedSpell {
                effect: ResolvedSpellEffect::Damage {
                    piece_ids: enemy_piece_ids_on_line(
                        enemy_pieces,
                        caster_position,
                        direction,
                        i32::from(*range),
                    ),
                    amount: damage_with_progression(*amount, progression),
                },
                log: SpellLog::LineDamage,
            }
        }
    };

    Ok(resolved)
}

fn stat_change_has_effect_on_hero(attack: i32, armor: i32, max_ap: i8) -> bool {
    attack != 0 || armor > 0 || max_ap != 0
}

fn item_has_effect_on_hero(
    passive: &ItemPassiveEffect,
    active: Option<&ItemActiveEffect>,
) -> bool {
    let passive_has_effect = match passive {
        ItemPassiveEffect::StatBonus {
            attack,
            armor,
            max_ap,
        } => stat_change_has_effect_on_hero(*attack, *armor, *max_ap),
    };
    passive_has_effect
        || active.is_some_and(|active| match active {
            ItemActiveEffect::HealCarrier { .. }
            | ItemActiveEffect::DamageTarget { .. }
            | ItemActiveEffect::Draw { .. } => true,
            ItemActiveEffect::StatMarker {
                attack,
                armor,
                max_ap,
                ..
            } => stat_change_has_effect_on_hero(*attack, *armor, *max_ap),
        })
}

pub(crate) fn target_policy_allows(targets: BuffTargetPolicy, is_hero: bool) -> bool {
    match targets {
        BuffTargetPolicy::UnitsOnly => !is_hero,
        BuffTargetPolicy::HeroesOnly => is_hero,
        BuffTargetPolicy::UnitsAndHeroes => true,
    }
}

pub(crate) fn equip_item_from_card(
    card: CardSummary,
    item_id: String,
) -> Result<CarriedItem, MatchError> {
    let CardKind::Item {
        passive, active, ..
    } = card.kind
    else {
        return Err(MatchError::InvalidTarget);
    };

    Ok(CarriedItem {
        id: item_id,
        template_id: card.template_id,
        name: card.name,
        passive,
        active,
        active_used_this_turn: false,
    })
}

pub(crate) fn resolve_item_activation(
    carrier: &PieceView,
    item: &CarriedItem,
    item_id: &str,
    target: Option<&PieceView>,
) -> Result<ResolvedItemActivation, MatchError> {
    if item.id != item_id {
        return Err(MatchError::ItemNotFound);
    }
    let active = item.active.clone().ok_or(MatchError::InvalidTarget)?;

    let effect = match active {
        ItemActiveEffect::HealCarrier { amount, .. } => ResolvedItemActiveEffect::HealCarrier {
            carrier_id: carrier.id.clone(),
            amount,
        },
        ItemActiveEffect::DamageTarget { amount, range, .. } => {
            let target = target.ok_or(MatchError::InvalidTarget)?;
            if target.side.team() == carrier.side.team()
                || carrier.position.distance(target.position) > i32::from(range)
            {
                return Err(MatchError::InvalidTarget);
            }
            ResolvedItemActiveEffect::DamageTarget {
                target_id: target.id.clone(),
                amount,
            }
        }
        ItemActiveEffect::Draw { amount, .. } => ResolvedItemActiveEffect::Draw { amount },
        ItemActiveEffect::StatMarker {
            attack,
            armor,
            max_ap,
            ..
        } => {
            if carrier.is_hero && !stat_change_has_effect_on_hero(attack, armor, max_ap) {
                return Err(MatchError::InvalidTarget);
            }
            ResolvedItemActiveEffect::StatMarker {
                carrier_id: carrier.id.clone(),
                marker: StatMarker {
                    id: String::new(),
                    source_item_id: item.id.clone(),
                    attack,
                    armor,
                    max_ap,
                },
            }
        }
    };

    Ok(ResolvedItemActivation {
        item_name: item.name.clone(),
        effect,
    })
}

pub(crate) fn can_resolve_unit_play(
    coord: HexCoord,
    caster_position: HexCoord,
    board: &HexBoard,
    is_occupied: impl Fn(HexCoord) -> bool,
) -> bool {
    validate_unit_target(coord, caster_position, board, is_occupied).is_ok()
}

pub(crate) fn summon_unit_from_card(
    card: &CardSummary,
    side: Side,
    next_unit_id: impl FnOnce() -> String,
    coord: HexCoord,
    progression: &MatchProgressionLoadout,
    is_first_summoned_unit: bool,
) -> Option<Unit> {
    let CardKind::Unit {
        attack,
        armor,
        max_ap,
    } = &card.kind
    else {
        return None;
    };

    let mut armor = armor
        .saturating_add(progression.effects.summoned_unit_armor_delta)
        .max(1);
    if is_first_summoned_unit {
        armor = armor
            .saturating_add(progression.effects.first_summoned_unit_armor_delta)
            .max(1);
    }

    Some(Unit {
        id: next_unit_id(),
        side,
        name: card.name.clone(),
        template_id: Some(card.template_id.clone()),
        attack: *attack,
        attack_range: crate::rules::CURRENT_RULESET.turn.default_attack_range,
        armor,
        max_armor: armor,
        position: coord,
        ap_remaining: *max_ap / 2,
        max_ap: *max_ap,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    })
}

pub(crate) fn apply_item_passive(unit: &mut Unit, passive: &ItemPassiveEffect) {
    match passive {
        ItemPassiveEffect::StatBonus {
            attack,
            armor,
            max_ap,
        } => {
            unit.attack = unit.attack.saturating_add(*attack).max(0);
            unit.armor = unit.armor.saturating_add(*armor).max(1);
            unit.max_armor = unit.max_armor.saturating_add(*armor).max(1);
            if *max_ap >= 0 {
                let amount = *max_ap as u8;
                unit.ap_remaining = unit.ap_remaining.saturating_add(amount);
                unit.max_ap = unit.max_ap.saturating_add(amount);
            } else {
                let amount = max_ap.unsigned_abs();
                unit.ap_remaining = unit.ap_remaining.saturating_sub(amount);
                unit.max_ap = unit.max_ap.saturating_sub(amount);
            }
        }
    }
}

pub(crate) fn apply_item_passive_to_hero(hero: &mut Hero, passive: &ItemPassiveEffect) {
    match passive {
        ItemPassiveEffect::StatBonus {
            attack,
            armor,
            max_ap,
        } => {
            hero.attack = hero.attack.saturating_add(*attack).max(0);
            if *armor > 0 {
                hero.shield = hero.shield.saturating_add(*armor);
            }
            if *max_ap >= 0 {
                let amount = *max_ap as u8;
                hero.ap_remaining = hero.ap_remaining.saturating_add(amount);
                hero.max_ap = hero.max_ap.saturating_add(amount);
            } else {
                let amount = max_ap.unsigned_abs();
                hero.ap_remaining = hero.ap_remaining.saturating_sub(amount);
                hero.max_ap = hero.max_ap.saturating_sub(amount);
            }
        }
    }
}

pub(crate) fn item_active_priority(active: &ItemActiveEffect) -> u8 {
    match active {
        ItemActiveEffect::HealCarrier { priority, .. }
        | ItemActiveEffect::DamageTarget { priority, .. }
        | ItemActiveEffect::Draw { priority, .. }
        | ItemActiveEffect::StatMarker { priority, .. } => *priority,
    }
}

fn validate_unit_target(
    coord: HexCoord,
    caster_position: HexCoord,
    board: &HexBoard,
    is_occupied: impl Fn(HexCoord) -> bool,
) -> Result<(), MatchError> {
    if !board.is_valid(coord) {
        return Err(MatchError::InvalidHex);
    }
    if !caster_position.is_adjacent(coord) {
        return Err(MatchError::InvalidTarget);
    }
    if is_occupied(coord) {
        return Err(MatchError::OccupiedHex);
    }

    Ok(())
}

fn damage_with_progression(amount: i32, progression: &MatchProgressionLoadout) -> i32 {
    amount
        .saturating_add(progression.effects.spell_damage_delta)
        .max(0)
}

fn enemy_piece_ids_in_area(
    enemy_pieces: &[PieceView],
    center: HexCoord,
    radius: i32,
) -> Vec<String> {
    enemy_pieces
        .iter()
        .filter(|piece| center.distance(piece.position) <= radius)
        .map(|piece| piece.id.clone())
        .collect()
}

fn enemy_piece_ids_on_line(
    enemy_pieces: &[PieceView],
    origin: HexCoord,
    direction: HexCoord,
    range: i32,
) -> Vec<String> {
    enemy_pieces
        .iter()
        .filter(|piece| origin.distance(piece.position) <= range)
        .filter(|piece| origin.direction_to(piece.position) == Some(direction))
        .map(|piece| piece.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extreme_authored_effect_arithmetic_saturates() {
        let mut progression = MatchProgressionLoadout::default();
        progression.effects.spell_damage_delta = 1;
        assert_eq!(damage_with_progression(i32::MAX, &progression), i32::MAX);

        let mut unit = Unit {
            id: "overflow-unit".to_string(),
            side: Side::Player,
            name: "Overflow Unit".to_string(),
            template_id: Some("overflow-unit".to_string()),
            attack: i32::MAX,
            attack_range: 1,
            armor: i32::MAX,
            max_armor: i32::MAX,
            position: HexCoord { q: 0, r: 0 },
            ap_remaining: 1,
            max_ap: 1,
            has_attacked: false,
            items: Vec::new(),
            stat_markers: Vec::new(),
        };
        apply_item_passive(
            &mut unit,
            &ItemPassiveEffect::StatBonus {
                attack: 1,
                armor: 1,
                max_ap: 0,
            },
        );

        assert_eq!(unit.attack, i32::MAX);
        assert_eq!(unit.armor, i32::MAX);
        assert_eq!(unit.max_armor, i32::MAX);
    }
}
