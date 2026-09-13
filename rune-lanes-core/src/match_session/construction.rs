use super::{
    BOARD_RADIUS, Card, HERO_MANA, Hero, HeroType, HexCoord, MatchProgressionEffects,
    MatchProgressionLoadout, PlayerState, Side, mana_with_progression,
};

impl PlayerState {
    pub(super) fn new(
        side: Side,
        board_radius: i32,
        mut rng_seed: u64,
        hero_type: HeroType,
        mut deck: Vec<Card>,
        progression: MatchProgressionLoadout,
    ) -> Self {
        shuffle(&mut deck, &mut rng_seed);
        let mana = mana_with_progression(HERO_MANA, progression.effects.mana_delta);

        Self {
            side,
            knocked_out: false,
            mana,
            max_mana: mana,
            hero: Hero::new(side, board_radius, hero_type, &progression.effects),
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
        side: Side,
        board_radius: i32,
        hero_type: HeroType,
        effects: &MatchProgressionEffects,
    ) -> Self {
        let (id, position) = match side {
            Side::Player => (
                "player-hero",
                HexCoord {
                    q: if board_radius > BOARD_RADIUS { -1 } else { 0 },
                    r: board_radius,
                },
            ),
            Side::Opponent => (
                "opponent-hero",
                HexCoord {
                    q: if board_radius > BOARD_RADIUS { 1 } else { 0 },
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
        let profile = hero_type.profile();
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
        let swap_index = (*seed as usize) % (index + 1);
        items.swap(index, swap_index);
    }
}
