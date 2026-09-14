use super::*;

pub fn summary_for_xp(total_xp: i64) -> ProgressionSummary {
    let total_xp = total_xp.max(0);
    let mut level = 1_u32;
    loop {
        let next_level_xp = cumulative_xp_for_level(level + 1);
        if total_xp < next_level_xp {
            let current_level_xp = cumulative_xp_for_level(level);
            return ProgressionSummary {
                total_xp,
                level,
                current_level_xp,
                next_level_xp,
                xp_into_level: total_xp - current_level_xp,
                xp_to_next_level: next_level_xp - total_xp,
                rune_slots: rune_slots_for_level(level),
            };
        }
        level += 1;
    }
}

pub(super) fn cumulative_xp_for_level(level: u32) -> i64 {
    let completed_steps = i64::from(level.saturating_sub(1));
    completed_steps * (completed_steps + 1) / 2 * 100
}

pub(super) fn rune_slots_for_level(level: u32) -> usize {
    if level >= 10 { 2 } else { 1 }
}

pub(super) fn rune_definitions() -> Vec<RuneDefinition> {
    vec![
        RuneDefinition {
            id: "vitality",
            name: "Vitality Rune",
            text: "Hero starts with +2 max HP.",
            unlock_level: 2,
            unlocked: false,
        },
        RuneDefinition {
            id: "force",
            name: "Force Rune",
            text: "Hero starts with +1 attack.",
            unlock_level: 4,
            unlocked: false,
        },
        RuneDefinition {
            id: "foresight",
            name: "Foresight Rune",
            text: "Draw +1 opening hand card.",
            unlock_level: 6,
            unlocked: false,
        },
        RuneDefinition {
            id: "wellspring",
            name: "Wellspring Rune",
            text: "Gain +1 natural mana.",
            unlock_level: 8,
            unlocked: false,
        },
        RuneDefinition {
            id: "bulwark",
            name: "Bulwark Rune",
            text: "Summoned units enter with +1 armor.",
            unlock_level: 12,
            unlocked: false,
        },
    ]
}

pub(super) fn rune_definition(rune_id: &str) -> Option<RuneDefinition> {
    rune_definitions()
        .into_iter()
        .find(|rune| rune.id == rune_id)
}

pub(super) fn appearance_definitions_for_hero(
    hero_type: HeroType,
) -> Vec<HeroAppearanceDefinition> {
    hero_appearance_definitions()
        .into_iter()
        .filter(|appearance| appearance.hero_type == hero_type)
        .collect()
}

pub(super) fn hero_appearance_definition(
    hero_type: HeroType,
    appearance_id: &str,
) -> Option<HeroAppearanceDefinition> {
    appearance_definitions_for_hero(hero_type)
        .into_iter()
        .find(|appearance| appearance.id == appearance_id)
}

pub(super) fn hero_appearance_definition_any(
    appearance_id: &str,
) -> Option<HeroAppearanceDefinition> {
    hero_appearance_definitions()
        .into_iter()
        .find(|appearance| appearance.id == appearance_id)
}

pub(super) fn appearance_unlocked(hero_level: u32, definition: &HeroAppearanceDefinition) -> bool {
    definition
        .unlock_level
        .is_none_or(|unlock_level| hero_level >= unlock_level)
}

pub(super) fn base_hero_appearance_id(hero_type: HeroType) -> &'static str {
    match hero_type {
        HeroType::Runekeeper => "runekeeper-base",
        HeroType::Pyromancer => "pyromancer-base",
        HeroType::Chronomancer => "chronomancer-base",
        HeroType::Warden => "warden-base",
        HeroType::Battlemage => "battlemage-base",
        HeroType::Barbarian => "barbarian-base",
        HeroType::Archer => "archer-base",
        HeroType::Builder => "builder-base",
    }
}

pub(super) fn hero_appearance_definitions() -> Vec<HeroAppearanceDefinition> {
    vec![
        appearance(
            HeroType::Runekeeper,
            "runekeeper-base",
            "Runekeeper",
            "A steady rune caster with a balanced board presence.",
            None,
        ),
        appearance(
            HeroType::Runekeeper,
            "runekeeper-jade-archivist",
            "Jade Archivist",
            "A jade-clad keeper with brighter archive glyphs.",
            Some(3),
        ),
        appearance(
            HeroType::Runekeeper,
            "runekeeper-golden-sigilist",
            "Golden Sigilist",
            "A gilded rune caster framed by ceremonial sigils.",
            Some(6),
        ),
        appearance(
            HeroType::Pyromancer,
            "pyromancer-base",
            "Pyromancer",
            "A direct flame caster with a compact ember focus.",
            None,
        ),
        appearance(
            HeroType::Pyromancer,
            "pyromancer-ember-mantle",
            "Ember Mantle",
            "A hotter mantle and ember aura for an advancing fire mage.",
            Some(3),
        ),
        appearance(
            HeroType::Pyromancer,
            "pyromancer-inferno-crown",
            "Inferno Crown",
            "A crowned pyromancer with a taller inferno focus.",
            Some(6),
        ),
        appearance(
            HeroType::Chronomancer,
            "chronomancer-base",
            "Chronomancer",
            "A mobile caster surrounded by measured time rings.",
            None,
        ),
        appearance(
            HeroType::Chronomancer,
            "chronomancer-glass-hour",
            "Glass Hour",
            "A glassy timekeeper with suspended hour fragments.",
            Some(3),
        ),
        appearance(
            HeroType::Chronomancer,
            "chronomancer-starclock",
            "Starclock",
            "A star-marked chronomancer with a brighter clock halo.",
            Some(6),
        ),
        appearance(
            HeroType::Warden,
            "warden-base",
            "Warden",
            "A rooted defender with a broad protective shield.",
            None,
        ),
        appearance(
            HeroType::Warden,
            "warden-mossguard",
            "Mossguard",
            "A moss-covered protector with softened green armor.",
            Some(3),
        ),
        appearance(
            HeroType::Warden,
            "warden-ironroot",
            "Ironroot",
            "A heavier warden reinforced with ironwood plating.",
            Some(6),
        ),
        appearance(
            HeroType::Battlemage,
            "battlemage-base",
            "Battlemage",
            "A front-line caster balancing weapon and focus.",
            None,
        ),
        appearance(
            HeroType::Battlemage,
            "battlemage-arc-duelist",
            "Arc Duelist",
            "A sharper duelist with brighter arcane edgework.",
            Some(3),
        ),
        appearance(
            HeroType::Battlemage,
            "battlemage-stormplate",
            "Stormplate",
            "A storm-armored battlemage with a charged off-hand.",
            Some(6),
        ),
        appearance(
            HeroType::Barbarian,
            "barbarian-base",
            "Barbarian",
            "A heavy brawler built around a brutal axe silhouette.",
            None,
        ),
        appearance(
            HeroType::Barbarian,
            "barbarian-warpaint",
            "Warpaint",
            "A painted raider with brighter battle markings.",
            Some(3),
        ),
        appearance(
            HeroType::Barbarian,
            "barbarian-ironhide-ravager",
            "Ironhide Ravager",
            "An ironhide champion with heavier armor and axe mass.",
            Some(6),
        ),
        appearance(
            HeroType::Archer,
            "archer-base",
            "Archer",
            "A light skirmisher with a clear bow profile.",
            None,
        ),
        appearance(
            HeroType::Archer,
            "archer-trail-scout",
            "Trail Scout",
            "A scout-marked archer with a leaner trail silhouette.",
            Some(3),
        ),
        appearance(
            HeroType::Archer,
            "archer-moonshot",
            "Moonshot",
            "A moonlit archer with a brighter longbow focus.",
            Some(6),
        ),
        appearance(
            HeroType::Builder,
            "builder-base",
            "Builder",
            "A sturdy support hero carrying tools and a hammer.",
            None,
        ),
        appearance(
            HeroType::Builder,
            "builder-field-engineer",
            "Field Engineer",
            "A field-ready builder with reinforced tool gear.",
            Some(3),
        ),
        appearance(
            HeroType::Builder,
            "builder-runeforge",
            "Runeforge",
            "A runeforge builder with a brighter hammer and frame.",
            Some(6),
        ),
    ]
}

fn appearance(
    hero_type: HeroType,
    id: &'static str,
    name: &'static str,
    text: &'static str,
    unlock_level: Option<u32>,
) -> HeroAppearanceDefinition {
    HeroAppearanceDefinition {
        id,
        hero_type,
        name,
        text,
        unlock_level,
        unlocked: false,
    }
}

pub(super) fn effects_for(rune_ids: &[String], skill_ids: &[String]) -> MatchProgressionEffects {
    let mut effects = MatchProgressionEffects::default();
    for rune_id in rune_ids {
        apply_effect_for_id(&mut effects, rune_id);
    }
    for skill_id in skill_ids {
        apply_effect_for_id(&mut effects, skill_id);
    }
    effects
}

pub(super) fn apply_effect_for_id(effects: &mut MatchProgressionEffects, id: &str) {
    match id {
        "vitality" => effects.max_hp_delta += 2,
        "force" => effects.attack_delta += 1,
        "foresight" => effects.opening_hand_delta += 1,
        "wellspring" => effects.mana_delta += 1,
        "bulwark" => effects.summoned_unit_armor_delta += 1,
        "runekeeper-steady-glyph" => effects.max_hp_delta += 1,
        "runekeeper-channel-stone" => effects.mana_delta += 1,
        "runekeeper-warding-script" => effects.first_summoned_unit_armor_delta += 1,
        "runekeeper-archive-spark" => effects.opening_hand_delta += 1,
        "pyromancer-heated-focus" => effects.attack_delta += 1,
        "pyromancer-kindling-reserve" => effects.mana_delta += 1,
        "pyromancer-scorching-script" => effects.spell_damage_delta += 1,
        "pyromancer-glass-flame" => {
            effects.opening_hand_delta += 1;
            effects.max_hp_delta -= 1;
        }
        "chronomancer-quick-step" => effects.max_ap_delta += 1,
        "chronomancer-stored-moment" => effects.mana_delta += 1,
        "chronomancer-early-loop" => effects.opening_hand_delta += 1,
        "chronomancer-temporal-guard" => effects.max_hp_delta += 1,
        "warden-stone-skin" => effects.max_hp_delta += 2,
        "warden-guard-drill" => effects.summoned_unit_armor_delta += 1,
        "warden-anchored-stance" => {
            effects.max_hp_delta += 1;
            effects.mana_delta += 1;
        }
        "warden-shield-line" => effects.first_summoned_unit_armor_delta += 1,
        "battlemage-weapon-drill" => effects.attack_delta += 1,
        "battlemage-iron-focus" => effects.max_hp_delta += 1,
        "battlemage-battle-rhythm" => effects.mana_delta += 1,
        "battlemage-frontline-command" => effects.summoned_unit_armor_delta += 1,
        "barbarian-brutal-stamina" => effects.max_hp_delta += 2,
        "barbarian-weapon-practice" => effects.attack_delta += 1,
        "barbarian-battle-hunger" => effects.mana_delta += 1,
        "barbarian-warband-hide" => effects.summoned_unit_armor_delta += 1,
        "barbarian-opening-rage" => effects.opening_hand_delta += 1,
        "barbarian-deep-cuts" => effects.spell_damage_delta += 1,
        "archer-fleet-footing" => effects.max_ap_delta += 1,
        "archer-keen-shot" => effects.spell_damage_delta += 1,
        "archer-scout-cache" => effects.opening_hand_delta += 1,
        "archer-trail-rations" => effects.mana_delta += 1,
        "archer-screening-line" => effects.first_summoned_unit_armor_delta += 1,
        "archer-light-armor" => effects.max_hp_delta += 1,
        "builder-reinforced-frame" => effects.max_hp_delta += 2,
        "builder-supply-cache" => effects.mana_delta += 1,
        "builder-work-crew-drill" => effects.summoned_unit_armor_delta += 1,
        "builder-first-wall" => effects.first_summoned_unit_armor_delta += 1,
        "builder-field-manual" => effects.opening_hand_delta += 1,
        "builder-tool-ready" => effects.max_ap_delta += 1,
        _ => {}
    }
}

pub(super) fn skill_trees() -> Vec<HeroSkillTree> {
    hero_types()
        .into_iter()
        .map(|hero_type| HeroSkillTree {
            hero_type,
            nodes: skill_nodes(hero_type),
        })
        .collect()
}

pub(super) fn skill_node(hero_type: HeroType, node_id: &str) -> Option<SkillNodeDefinition> {
    skill_nodes(hero_type)
        .into_iter()
        .find(|node| node.id == node_id)
}

pub(super) fn root_skill_id(hero_type: HeroType) -> &'static str {
    match hero_type {
        HeroType::Runekeeper => "runekeeper-runic-balance",
        HeroType::Pyromancer => "pyromancer-ember-path",
        HeroType::Chronomancer => "chronomancer-time-thread",
        HeroType::Warden => "warden-stone-oath",
        HeroType::Battlemage => "battlemage-duelist-oath",
        HeroType::Barbarian => "barbarian-fury-path",
        HeroType::Archer => "archer-long-watch",
        HeroType::Builder => "builder-foundation-plan",
    }
}

pub(super) fn skill_nodes(hero_type: HeroType) -> Vec<SkillNodeDefinition> {
    match hero_type {
        HeroType::Runekeeper => vec![
            skill(
                "runekeeper-runic-balance",
                "Runic Balance",
                "The root of Runekeeper mastery.",
                true,
                None,
            ),
            skill(
                "runekeeper-steady-glyph",
                "Steady Glyph",
                "Hero starts with +1 max HP.",
                false,
                Some("runekeeper-runic-balance"),
            ),
            skill(
                "runekeeper-channel-stone",
                "Channel Stone",
                "Gain +1 natural mana.",
                false,
                Some("runekeeper-runic-balance"),
            ),
            skill(
                "runekeeper-warding-script",
                "Warding Script",
                "First summoned unit each match enters with +1 armor.",
                false,
                Some("runekeeper-steady-glyph"),
            ),
            skill(
                "runekeeper-archive-spark",
                "Archive Spark",
                "Draw +1 opening hand card.",
                false,
                Some("runekeeper-channel-stone"),
            ),
        ],
        HeroType::Pyromancer => vec![
            skill(
                "pyromancer-ember-path",
                "Ember Path",
                "The root of Pyromancer mastery.",
                true,
                None,
            ),
            skill(
                "pyromancer-heated-focus",
                "Heated Focus",
                "Hero starts with +1 attack.",
                false,
                Some("pyromancer-ember-path"),
            ),
            skill(
                "pyromancer-kindling-reserve",
                "Kindling Reserve",
                "Gain +1 natural mana.",
                false,
                Some("pyromancer-ember-path"),
            ),
            skill(
                "pyromancer-scorching-script",
                "Scorching Script",
                "Damaging spells deal +1 damage.",
                false,
                Some("pyromancer-heated-focus"),
            ),
            skill(
                "pyromancer-glass-flame",
                "Glass Flame",
                "Draw +1 opening hand card and start with -1 max HP.",
                false,
                Some("pyromancer-kindling-reserve"),
            ),
        ],
        HeroType::Chronomancer => vec![
            skill(
                "chronomancer-time-thread",
                "Time Thread",
                "The root of Chronomancer mastery.",
                true,
                None,
            ),
            skill(
                "chronomancer-quick-step",
                "Quick Step",
                "Hero starts with +1 max AP.",
                false,
                Some("chronomancer-time-thread"),
            ),
            skill(
                "chronomancer-stored-moment",
                "Stored Moment",
                "Gain +1 natural mana.",
                false,
                Some("chronomancer-time-thread"),
            ),
            skill(
                "chronomancer-early-loop",
                "Early Loop",
                "Draw +1 opening hand card.",
                false,
                Some("chronomancer-quick-step"),
            ),
            skill(
                "chronomancer-temporal-guard",
                "Temporal Guard",
                "Hero starts with +1 max HP.",
                false,
                Some("chronomancer-stored-moment"),
            ),
        ],
        HeroType::Warden => vec![
            skill(
                "warden-stone-oath",
                "Stone Oath",
                "The root of Warden mastery.",
                true,
                None,
            ),
            skill(
                "warden-stone-skin",
                "Stone Skin",
                "Hero starts with +2 max HP.",
                false,
                Some("warden-stone-oath"),
            ),
            skill(
                "warden-guard-drill",
                "Guard Drill",
                "Summoned units enter with +1 armor.",
                false,
                Some("warden-stone-oath"),
            ),
            skill(
                "warden-anchored-stance",
                "Anchored Stance",
                "Hero starts with +1 max HP and gains +1 natural mana.",
                false,
                Some("warden-stone-skin"),
            ),
            skill(
                "warden-shield-line",
                "Shield Line",
                "First summoned unit each match enters with +1 armor.",
                false,
                Some("warden-guard-drill"),
            ),
        ],
        HeroType::Battlemage => vec![
            skill(
                "battlemage-duelist-oath",
                "Duelist Oath",
                "The root of Battlemage mastery.",
                true,
                None,
            ),
            skill(
                "battlemage-weapon-drill",
                "Weapon Drill",
                "Hero starts with +1 attack.",
                false,
                Some("battlemage-duelist-oath"),
            ),
            skill(
                "battlemage-iron-focus",
                "Iron Focus",
                "Hero starts with +1 max HP.",
                false,
                Some("battlemage-duelist-oath"),
            ),
            skill(
                "battlemage-battle-rhythm",
                "Battle Rhythm",
                "Gain +1 natural mana.",
                false,
                Some("battlemage-weapon-drill"),
            ),
            skill(
                "battlemage-frontline-command",
                "Frontline Command",
                "Summoned units enter with +1 armor.",
                false,
                Some("battlemage-iron-focus"),
            ),
        ],
        HeroType::Barbarian => vec![
            skill(
                "barbarian-fury-path",
                "Fury Path",
                "The root of Barbarian mastery.",
                true,
                None,
            ),
            skill(
                "barbarian-brutal-stamina",
                "Brutal Stamina",
                "Hero starts with +2 max HP.",
                false,
                Some("barbarian-fury-path"),
            ),
            skill(
                "barbarian-weapon-practice",
                "Weapon Practice",
                "Hero starts with +1 attack.",
                false,
                Some("barbarian-fury-path"),
            ),
            skill(
                "barbarian-battle-hunger",
                "Battle Hunger",
                "Gain +1 natural mana.",
                false,
                Some("barbarian-weapon-practice"),
            ),
            skill(
                "barbarian-warband-hide",
                "Warband Hide",
                "Summoned units enter with +1 armor.",
                false,
                Some("barbarian-brutal-stamina"),
            ),
            skill(
                "barbarian-opening-rage",
                "Opening Rage",
                "Draw +1 opening hand card.",
                false,
                Some("barbarian-brutal-stamina"),
            ),
            skill(
                "barbarian-deep-cuts",
                "Deep Cuts",
                "Damaging spells deal +1 damage.",
                false,
                Some("barbarian-weapon-practice"),
            ),
        ],
        HeroType::Archer => vec![
            skill(
                "archer-long-watch",
                "Long Watch",
                "The root of Archer mastery.",
                true,
                None,
            ),
            skill(
                "archer-fleet-footing",
                "Fleet Footing",
                "Hero starts with +1 max AP.",
                false,
                Some("archer-long-watch"),
            ),
            skill(
                "archer-keen-shot",
                "Keen Shot",
                "Damaging spells deal +1 damage.",
                false,
                Some("archer-long-watch"),
            ),
            skill(
                "archer-scout-cache",
                "Scout Cache",
                "Draw +1 opening hand card.",
                false,
                Some("archer-fleet-footing"),
            ),
            skill(
                "archer-trail-rations",
                "Trail Rations",
                "Gain +1 natural mana.",
                false,
                Some("archer-keen-shot"),
            ),
            skill(
                "archer-screening-line",
                "Screening Line",
                "First summoned unit each match enters with +1 armor.",
                false,
                Some("archer-fleet-footing"),
            ),
            skill(
                "archer-light-armor",
                "Light Armor",
                "Hero starts with +1 max HP.",
                false,
                Some("archer-keen-shot"),
            ),
        ],
        HeroType::Builder => vec![
            skill(
                "builder-foundation-plan",
                "Foundation Plan",
                "The root of Builder mastery.",
                true,
                None,
            ),
            skill(
                "builder-reinforced-frame",
                "Reinforced Frame",
                "Hero starts with +2 max HP.",
                false,
                Some("builder-foundation-plan"),
            ),
            skill(
                "builder-supply-cache",
                "Supply Cache",
                "Gain +1 natural mana.",
                false,
                Some("builder-foundation-plan"),
            ),
            skill(
                "builder-work-crew-drill",
                "Work Crew Drill",
                "Summoned units enter with +1 armor.",
                false,
                Some("builder-reinforced-frame"),
            ),
            skill(
                "builder-first-wall",
                "First Wall",
                "First summoned unit each match enters with +1 armor.",
                false,
                Some("builder-reinforced-frame"),
            ),
            skill(
                "builder-field-manual",
                "Field Manual",
                "Draw +1 opening hand card.",
                false,
                Some("builder-supply-cache"),
            ),
            skill(
                "builder-tool-ready",
                "Tool Ready",
                "Hero starts with +1 max AP.",
                false,
                Some("builder-supply-cache"),
            ),
        ],
    }
}

pub(super) fn skill(
    id: &'static str,
    name: &'static str,
    text: &'static str,
    root: bool,
    prerequisite_id: Option<&'static str>,
) -> SkillNodeDefinition {
    SkillNodeDefinition {
        id,
        name,
        text,
        root,
        prerequisite_id,
    }
}

pub(super) fn hero_types() -> Vec<HeroType> {
    vec![
        HeroType::Runekeeper,
        HeroType::Pyromancer,
        HeroType::Chronomancer,
        HeroType::Warden,
        HeroType::Battlemage,
        HeroType::Barbarian,
        HeroType::Archer,
        HeroType::Builder,
    ]
}

pub(super) fn hero_type_to_db(hero_type: HeroType) -> &'static str {
    match hero_type {
        HeroType::Runekeeper => "runekeeper",
        HeroType::Pyromancer => "pyromancer",
        HeroType::Chronomancer => "chronomancer",
        HeroType::Warden => "warden",
        HeroType::Battlemage => "battlemage",
        HeroType::Barbarian => "barbarian",
        HeroType::Archer => "archer",
        HeroType::Builder => "builder",
    }
}

pub(super) fn hero_type_from_db(value: &str) -> Option<HeroType> {
    match value {
        "runekeeper" => Some(HeroType::Runekeeper),
        "pyromancer" => Some(HeroType::Pyromancer),
        "chronomancer" => Some(HeroType::Chronomancer),
        "warden" => Some(HeroType::Warden),
        "battlemage" => Some(HeroType::Battlemage),
        "barbarian" => Some(HeroType::Barbarian),
        "archer" => Some(HeroType::Archer),
        "builder" => Some(HeroType::Builder),
        _ => None,
    }
}
