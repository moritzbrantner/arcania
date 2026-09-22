use std::fmt;

use serde::{Deserialize, Serialize};

use crate::HeroType;

pub const RULESET_SCHEMA_VERSION: u32 = 1;
const RULESET_VERSION_PREFIX: &str = "rune-lanes-rules";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaRules {
    pub duel_radius: i32,
    pub two_v_two_radius: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnRules {
    pub base_hero_mana: u8,
    pub opening_hand_size: u8,
    pub max_carried_items: u8,
    pub default_attack_range: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroRule {
    pub max_hp: i32,
    pub attack: i32,
    pub max_ap: u8,
    pub attack_range: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeroRules {
    pub runekeeper: HeroRule,
    pub pyromancer: HeroRule,
    pub chronomancer: HeroRule,
    pub warden: HeroRule,
    pub battlemage: HeroRule,
    pub barbarian: HeroRule,
    pub archer: HeroRule,
    pub builder: HeroRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuneLanesRuleset {
    pub schema_version: u32,
    pub arena: ArenaRules,
    pub turn: TurnRules,
    pub heroes: HeroRules,
}

pub const CURRENT_RULESET: RuneLanesRuleset = RuneLanesRuleset {
    schema_version: RULESET_SCHEMA_VERSION,
    arena: ArenaRules {
        duel_radius: 3,
        two_v_two_radius: 4,
    },
    turn: TurnRules {
        base_hero_mana: 3,
        opening_hand_size: 7,
        max_carried_items: 3,
        default_attack_range: 1,
    },
    heroes: HeroRules {
        runekeeper: HeroRule {
            max_hp: 20,
            attack: 1,
            max_ap: 3,
            attack_range: 1,
        },
        pyromancer: HeroRule {
            max_hp: 18,
            attack: 2,
            max_ap: 3,
            attack_range: 1,
        },
        chronomancer: HeroRule {
            max_hp: 16,
            attack: 1,
            max_ap: 4,
            attack_range: 1,
        },
        warden: HeroRule {
            max_hp: 24,
            attack: 1,
            max_ap: 2,
            attack_range: 1,
        },
        battlemage: HeroRule {
            max_hp: 20,
            attack: 2,
            max_ap: 2,
            attack_range: 1,
        },
        barbarian: HeroRule {
            max_hp: 22,
            attack: 3,
            max_ap: 2,
            attack_range: 1,
        },
        archer: HeroRule {
            max_hp: 16,
            attack: 2,
            max_ap: 4,
            attack_range: 2,
        },
        builder: HeroRule {
            max_hp: 24,
            attack: 1,
            max_ap: 2,
            attack_range: 1,
        },
    },
};

impl Default for RuneLanesRuleset {
    fn default() -> Self {
        CURRENT_RULESET
    }
}

#[must_use]
pub fn current_ruleset_version() -> String {
    ruleset_version(&CURRENT_RULESET)
}

fn ruleset_version(ruleset: &RuneLanesRuleset) -> String {
    let encoded =
        serde_json::to_vec(ruleset).expect("the statically defined current ruleset must serialize");
    let fingerprint = encoded
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("{RULESET_VERSION_PREFIX}-v{RULESET_SCHEMA_VERSION}-{fingerprint:016x}")
}

impl RuneLanesRuleset {
    #[must_use]
    pub fn hero(&self, hero_type: HeroType) -> HeroRule {
        match hero_type {
            HeroType::Runekeeper => self.heroes.runekeeper,
            HeroType::Pyromancer => self.heroes.pyromancer,
            HeroType::Chronomancer => self.heroes.chronomancer,
            HeroType::Warden => self.heroes.warden,
            HeroType::Battlemage => self.heroes.battlemage,
            HeroType::Barbarian => self.heroes.barbarian,
            HeroType::Archer => self.heroes.archer,
            HeroType::Builder => self.heroes.builder,
        }
    }

    pub fn validate(&self) -> Result<(), RulesetError> {
        if self.schema_version != RULESET_SCHEMA_VERSION {
            return Err(RulesetError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.arena.duel_radius <= 0 || self.arena.two_v_two_radius < self.arena.duel_radius {
            return Err(RulesetError::InvalidArenaRules);
        }
        if self.turn.base_hero_mana == 0
            || self.turn.opening_hand_size == 0
            || self.turn.max_carried_items == 0
            || self.turn.default_attack_range == 0
        {
            return Err(RulesetError::InvalidTurnRules);
        }
        for hero_type in [
            HeroType::Runekeeper,
            HeroType::Pyromancer,
            HeroType::Chronomancer,
            HeroType::Warden,
            HeroType::Battlemage,
            HeroType::Barbarian,
            HeroType::Archer,
            HeroType::Builder,
        ] {
            let rule = self.hero(hero_type);
            if rule.max_hp <= 0 || rule.attack < 0 || rule.max_ap == 0 || rule.attack_range == 0 {
                return Err(RulesetError::InvalidHeroRule(hero_type));
            }
        }
        Ok(())
    }
}

impl HeroType {
    pub(crate) fn profile(self) -> HeroRule {
        CURRENT_RULESET.hero(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RulesetError {
    UnsupportedSchemaVersion(u32),
    InvalidArenaRules,
    InvalidTurnRules,
    InvalidHeroRule(HeroType),
}

impl fmt::Display for RulesetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "unsupported Rune Lanes ruleset schema version {version}"
                )
            }
            Self::InvalidArenaRules => formatter.write_str("invalid arena rules"),
            Self::InvalidTurnRules => formatter.write_str("invalid turn rules"),
            Self::InvalidHeroRule(hero_type) => {
                write!(formatter, "invalid hero rules for {hero_type:?}")
            }
        }
    }
}

impl std::error::Error for RulesetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_ruleset_is_valid_and_groups_the_editable_game_knobs() {
        assert_eq!(CURRENT_RULESET.validate(), Ok(()));
        assert_eq!(CURRENT_RULESET.arena.duel_radius, 3);
        assert_eq!(CURRENT_RULESET.turn.base_hero_mana, 3);
        assert_eq!(CURRENT_RULESET.turn.opening_hand_size, 7);
        assert_eq!(CURRENT_RULESET.hero(HeroType::Archer).attack_range, 2);
    }

    #[test]
    fn default_match_construction_consumes_the_current_ruleset() {
        let state = crate::MatchState::new_with_seed(7);
        let hero_rule = CURRENT_RULESET.hero(state.player.hero.hero_type);

        assert_eq!(state.board.radius, CURRENT_RULESET.arena.duel_radius);
        assert_eq!(
            state.player.hand.len(),
            usize::from(CURRENT_RULESET.turn.opening_hand_size)
        );
        assert_eq!(state.player.hero.max_hp, hero_rule.max_hp);
        assert_eq!(state.player.hero.attack, hero_rule.attack);
        assert_eq!(state.player.hero.max_ap, hero_rule.max_ap);
        assert_eq!(state.player.hero.attack_range, hero_rule.attack_range);
    }

    #[test]
    fn invalid_typed_rule_configuration_fails_closed() {
        let mut ruleset = CURRENT_RULESET;
        ruleset.turn.opening_hand_size = 0;
        assert_eq!(ruleset.validate(), Err(RulesetError::InvalidTurnRules));
    }

    #[test]
    fn persisted_version_is_derived_from_every_replay_relevant_rule() {
        let current = current_ruleset_version();
        let mut changed = CURRENT_RULESET;
        changed.turn.default_attack_range += 1;

        assert_ne!(current, ruleset_version(&changed));
    }
}
