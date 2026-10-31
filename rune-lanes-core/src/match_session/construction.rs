use std::time::{SystemTime, UNIX_EPOCH};

use crate::rules::CURRENT_RULESET;

use super::{
    Card, Hero, HeroType, HexBoard, HexCoord, MatchMode, MatchProgressionEffects,
    MatchProgressionLoadout, MatchState, Phase, PlayerState, Side,
};

impl PlayerState {
    pub(super) fn new(
        ruleset: &crate::rules::RuneLanesRuleset,
        side: Side,
        board_radius: i32,
        mut rng_seed: u64,
        hero_type: HeroType,
        mut deck: Vec<Card>,
        progression: MatchProgressionLoadout,
    ) -> Self {
        shuffle(&mut deck, &mut rng_seed);
        let mana =
            mana_with_progression(ruleset.turn.base_hero_mana, progression.effects.mana_delta);

        Self {
            side,
            knocked_out: false,
            mana,
            max_mana: mana,
            hero: Hero::new(ruleset, side, board_radius, hero_type, &progression.effects),
            progression,
            hand: Vec::new(),
            deck_count: deck.len(),
            discard_count: 0,
            deck,
            discard: Vec::new(),
            rng_seed,
            has_started_first_turn: false,
            summoned_unit_count: 0,
        }
    }

    pub(super) fn draw(&mut self) -> Option<Card> {
        if self.deck.is_empty() && !self.discard.is_empty() {
            self.deck.append(&mut self.discard);
            shuffle(&mut self.deck, &mut self.rng_seed);
        }

        let drawn = self.deck.pop();
        if let Some(card) = drawn.clone() {
            self.hand.push(card);
        }
        self.deck_count = self.deck.len();
        self.discard_count = self.discard.len();
        drawn
    }
}
impl Hero {
    pub(super) fn new(
        ruleset: &crate::rules::RuneLanesRuleset,
        side: Side,
        board_radius: i32,
        hero_type: HeroType,
        effects: &MatchProgressionEffects,
    ) -> Self {
        let (id, position) = match side {
            Side::Player => (
                "player-hero",
                HexCoord {
                    q: if board_radius > CURRENT_RULESET.arena.duel_radius {
                        -1
                    } else {
                        0
                    },
                    r: board_radius,
                },
            ),
            Side::Opponent => (
                "opponent-hero",
                HexCoord {
                    q: if board_radius > CURRENT_RULESET.arena.duel_radius {
                        1
                    } else {
                        0
                    },
                    r: -board_radius,
                },
            ),
            Side::PlayerTwo => (
                "player-two-hero",
                HexCoord {
                    q: 1,
                    r: board_radius - 1,
                },
            ),
            Side::OpponentTwo => (
                "opponent-two-hero",
                HexCoord {
                    q: -1,
                    r: 1 - board_radius,
                },
            ),
        };
        let profile = ruleset.hero(hero_type);
        let max_hp = (profile.max_hp + effects.max_hp_delta).max(1);
        let attack = (profile.attack + effects.attack_delta).max(0);
        let max_ap = (i16::from(profile.max_ap) + i16::from(effects.max_ap_delta)).max(1) as u8;

        Self {
            id: id.to_string(),
            side,
            knocked_out: false,
            hero_type,
            hp: max_hp,
            max_hp,
            shield: 0,
            attack,
            attack_range: profile.attack_range,
            position,
            ap_remaining: max_ap,
            max_ap,
            has_attacked: false,
            items: Vec::new(),
            stat_markers: Vec::new(),
        }
    }
}
fn shuffle<T>(items: &mut [T], seed: &mut u64) {
    if items.len() <= 1 {
        return;
    }

    for index in (1..items.len()).rev() {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        // Take the modulus before narrowing: wasm32 and native hosts must shuffle identically.
        let swap_index = (*seed % (index as u64 + 1)) as usize;
        items.swap(index, swap_index);
    }
}

impl Default for MatchState {
    fn default() -> Self {
        Self::new()
    }
}

impl MatchState {
    #[allow(dead_code, reason = "kept as the default rules-engine constructor")]
    pub fn new() -> Self {
        Self::new_with_player_hero_type(HeroType::default())
    }

    pub fn new_with_player_hero_type(player_hero_type: HeroType) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_and_player_hero_type(seed, player_hero_type)
    }

    #[allow(
        dead_code,
        reason = "kept as a progression-free constructor for tests and callers"
    )]
    pub fn new_with_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
    ) -> Self {
        Self::new_with_progression_loadouts(
            player_hero_type,
            opponent_hero_type,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    pub fn new_with_progression_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Solo,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
        )
    }

    #[allow(
        dead_code,
        reason = "kept as the default shared rules-engine constructor"
    )]
    pub fn new_shared_with_hero_types(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_and_mode(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Shared,
        )
    }

    #[allow(
        dead_code,
        reason = "kept as a progression-free constructor for tests and callers"
    )]
    pub fn new_shared_with_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
    ) -> Self {
        Self::new_shared_with_progression_loadouts(
            player_hero_type,
            opponent_hero_type,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    pub fn new_shared_with_progression_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Shared,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "2v2 setup passes mirrored per-seat setup explicitly"
    )]
    pub fn new_shared_two_v_two_with_progression_loadouts(
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_two_hero_type: HeroType,
        opponent_two_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_two_deck: Vec<Card>,
        opponent_two_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
        player_two_progression: MatchProgressionLoadout,
        opponent_two_progression: MatchProgressionLoadout,
    ) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(1);

        Self::new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
            CURRENT_RULESET,
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Shared,
            CURRENT_RULESET.arena.two_v_two_radius,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
            Some((
                player_two_hero_type,
                player_two_deck,
                player_two_progression,
            )),
            Some((
                opponent_two_hero_type,
                opponent_two_deck,
                opponent_two_progression,
            )),
        )
    }

    #[allow(
        dead_code,
        reason = "kept as the deterministic rules-engine test constructor"
    )]
    pub fn new_with_seed(seed: u64) -> Self {
        Self::new_with_seed_and_player_hero_type(seed, HeroType::default())
    }

    pub(super) fn new_with_seed_and_player_hero_type(
        seed: u64,
        player_hero_type: HeroType,
    ) -> Self {
        Self::new_with_seed_hero_types_and_mode(
            seed,
            player_hero_type,
            HeroType::Runekeeper,
            MatchMode::Solo,
        )
    }

    pub(super) fn new_with_seed_hero_types_and_mode(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        mode: MatchMode,
    ) -> Self {
        let player_deck = crate::deck_library::deck_from_snapshot(
            Side::Player,
            &crate::deck_library::starter_deck_snapshot(),
        )
        .expect("starter player deck should be valid");
        let opponent_deck = crate::deck_library::deck_from_snapshot(
            Side::Opponent,
            &crate::deck_library::starter_deck_snapshot(),
        )
        .expect("starter opponent deck should be valid");
        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            mode,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "test and store constructors pass mirrored player/opponent setup explicitly"
    )]
    fn new_with_seed_hero_types_mode_and_decks(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        mode: MatchMode,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
    ) -> Self {
        Self::new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
            CURRENT_RULESET,
            seed,
            player_hero_type,
            opponent_hero_type,
            mode,
            CURRENT_RULESET.arena.duel_radius,
            player_deck,
            opponent_deck,
            player_progression,
            opponent_progression,
            None,
            None,
        )
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "2v2 setup passes mirrored per-seat setup explicitly"
    )]
    fn new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
        ruleset: crate::rules::RuneLanesRuleset,
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        mode: MatchMode,
        board_radius: i32,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        player_progression: MatchProgressionLoadout,
        opponent_progression: MatchProgressionLoadout,
        player_two: Option<(HeroType, Vec<Card>, MatchProgressionLoadout)>,
        opponent_two: Option<(HeroType, Vec<Card>, MatchProgressionLoadout)>,
    ) -> Self {
        let mut game = Self {
            ruleset,
            mode,
            round: 1,
            phase: Phase::Movement,
            active_side: Side::Player,
            player: PlayerState::new(
                &ruleset,
                Side::Player,
                board_radius,
                seed ^ 0xA11C_E551_1234_5678,
                player_hero_type,
                player_deck,
                player_progression,
            ),
            opponent: PlayerState::new(
                &ruleset,
                Side::Opponent,
                board_radius,
                seed ^ 0x0B0E_1234_9876_5432,
                opponent_hero_type,
                opponent_deck,
                opponent_progression,
            ),
            player_two: player_two.map(|(hero_type, deck, progression)| {
                PlayerState::new(
                    &ruleset,
                    Side::PlayerTwo,
                    board_radius,
                    seed ^ 0xA11C_E552_2234_5678,
                    hero_type,
                    deck,
                    progression,
                )
            }),
            opponent_two: opponent_two.map(|(hero_type, deck, progression)| {
                PlayerState::new(
                    &ruleset,
                    Side::OpponentTwo,
                    board_radius,
                    seed ^ 0x0B0E_2234_9876_5432,
                    hero_type,
                    deck,
                    progression,
                )
            }),
            board: HexBoard::new(board_radius),
            action_stack: Vec::new(),
            priority_side: None,
            log: vec!["The heroes enter the hex arena.".to_string()],
            winner: None,
            priority_passes: Vec::new(),
            next_stack_item_id: 1,
            next_unit_id: 1,
            next_item_id: 1,
            next_building_id: 1,
            skip_replay_frames: false,
        };

        for _ in 0..opening_hand_size(&ruleset, &game.player.progression) {
            game.player.draw();
        }
        for _ in 0..opening_hand_size(&ruleset, &game.opponent.progression) {
            game.opponent.draw();
        }
        if let Some(player_two) = &mut game.player_two {
            for _ in 0..opening_hand_size(&ruleset, &player_two.progression) {
                player_two.draw();
            }
        }
        if let Some(opponent_two) = &mut game.opponent_two {
            for _ in 0..opening_hand_size(&ruleset, &opponent_two.progression) {
                opponent_two.draw();
            }
        }
        let mut ignored_frames = Vec::new();
        game.start_turn(Side::Player, &mut ignored_frames, None);

        game
    }

    /// Creates an isolated Solo match with frozen, validated experimental rules.
    pub fn new_with_rule_preset(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
        ruleset: crate::rules::RuneLanesRuleset,
    ) -> Result<Self, crate::rules::RulesetError> {
        ruleset.validate()?;
        Ok(
            Self::new_with_seed_hero_types_mode_decks_radius_and_optional_teammates(
                ruleset,
                seed,
                player_hero_type,
                opponent_hero_type,
                MatchMode::Solo,
                ruleset.arena.duel_radius,
                player_deck,
                opponent_deck,
                MatchProgressionLoadout::default(),
                MatchProgressionLoadout::default(),
                None,
                None,
            ),
        )
    }

    pub fn new_ai_lab_with_seed_and_decks(
        seed: u64,
        player_hero_type: HeroType,
        opponent_hero_type: HeroType,
        player_deck: Vec<Card>,
        opponent_deck: Vec<Card>,
    ) -> Self {
        Self::new_with_seed_hero_types_mode_and_decks(
            seed,
            player_hero_type,
            opponent_hero_type,
            MatchMode::Solo,
            player_deck,
            opponent_deck,
            MatchProgressionLoadout::default(),
            MatchProgressionLoadout::default(),
        )
    }
}

fn opening_hand_size(
    ruleset: &crate::rules::RuneLanesRuleset,
    progression: &MatchProgressionLoadout,
) -> usize {
    usize::from(ruleset.turn.opening_hand_size)
        + usize::from(progression.effects.opening_hand_delta)
}

pub(super) fn mana_with_progression(base: u8, delta: i8) -> u8 {
    let value = i16::from(base) + i16::from(delta);
    value.clamp(0, i16::from(u8::MAX)) as u8
}
