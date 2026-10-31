use super::{
    BuffTargetPolicy, BuildingEffect, Hero, HexCoord, MatchState, PieceView, PlayerState, Side,
    StatBonus, Team, Unit, card_interactions,
};

impl MatchState {
    pub(super) fn has_building(&self, coord: HexCoord) -> bool {
        self.board
            .buildings
            .iter()
            .any(|building| building.position == coord)
            || self.board.mana_sources.contains(&coord)
    }

    pub(super) fn player_ref(&self, side: Side) -> &PlayerState {
        match side {
            Side::Player => &self.player,
            Side::Opponent => &self.opponent,
            Side::PlayerTwo => self
                .player_two
                .as_ref()
                .expect("player two should exist in a 2v2 match"),
            Side::OpponentTwo => self
                .opponent_two
                .as_ref()
                .expect("opponent two should exist in a 2v2 match"),
        }
    }

    pub(super) fn player_mut(&mut self, side: Side) -> &mut PlayerState {
        match side {
            Side::Player => &mut self.player,
            Side::Opponent => &mut self.opponent,
            Side::PlayerTwo => self
                .player_two
                .as_mut()
                .expect("player two should exist in a 2v2 match"),
            Side::OpponentTwo => self
                .opponent_two
                .as_mut()
                .expect("opponent two should exist in a 2v2 match"),
        }
    }

    pub(super) fn participants(&self) -> Vec<&PlayerState> {
        let mut participants = vec![&self.player, &self.opponent];
        if let Some(player_two) = &self.player_two {
            participants.push(player_two);
        }
        if let Some(opponent_two) = &self.opponent_two {
            participants.push(opponent_two);
        }
        participants
    }

    pub(super) fn participant_sides(&self) -> Vec<Side> {
        self.participants()
            .into_iter()
            .map(|participant| participant.side)
            .collect()
    }

    fn active_participant_sides(&self) -> Vec<Side> {
        self.participants()
            .into_iter()
            .filter(|participant| !participant.knocked_out)
            .map(|participant| participant.side)
            .collect()
    }

    /// Compatibility hook used by deterministic AI-lab presets while a typed
    /// experiment-configuration API is extracted into the core.
    pub fn player_mut_for_ai_lab(&mut self, side: Side) -> &mut PlayerState {
        self.player_mut(side)
    }

    pub(super) fn pieces_for_side(&self, side: Side) -> Vec<PieceView> {
        let participant = self.player_ref(side);
        let mut pieces = if participant.knocked_out {
            Vec::new()
        } else {
            vec![self.apply_aura_to_piece_view(PieceView::from(&participant.hero))]
        };
        pieces.extend(
            self.board
                .units
                .iter()
                .filter(|unit| unit.side == side)
                .map(PieceView::from)
                .map(|piece| self.apply_aura_to_piece_view(piece)),
        );
        pieces
    }

    fn pieces_for_team(&self, team: Team) -> Vec<PieceView> {
        self.active_participant_sides()
            .into_iter()
            .filter(|side| side.team() == team)
            .flat_map(|side| self.pieces_for_side(side))
            .collect()
    }

    pub(super) fn pieces_for_opposing_team(&self, side: Side) -> Vec<PieceView> {
        self.pieces_for_team(side.opposing_team())
    }

    pub(super) fn piece_view_at(&self, coord: HexCoord) -> Option<PieceView> {
        for participant in self.participants() {
            if !participant.knocked_out && participant.hero.position == coord {
                return Some(self.apply_aura_to_piece_view(PieceView::from(&participant.hero)));
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.position == coord)
            .map(PieceView::from)
            .map(|piece| self.apply_aura_to_piece_view(piece))
    }

    pub(super) fn piece_view(&self, piece_id: &str) -> Option<PieceView> {
        for participant in self.participants() {
            if !participant.knocked_out && participant.hero.id == piece_id {
                return Some(self.apply_aura_to_piece_view(PieceView::from(&participant.hero)));
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == piece_id)
            .map(PieceView::from)
            .map(|piece| self.apply_aura_to_piece_view(piece))
    }

    fn apply_aura_to_piece_view(&self, mut piece: PieceView) -> PieceView {
        let bonus = self.aura_stat_bonus_for(piece.side, piece.position, piece.is_hero);
        piece.attack = piece.attack.saturating_add(bonus.attack).max(0);
        piece
    }

    pub(super) fn aura_stat_bonus_for(
        &self,
        side: Side,
        position: HexCoord,
        is_hero: bool,
    ) -> StatBonus {
        let mut bonus = StatBonus {
            attack: 0,
            armor: 0,
            max_ap: 0,
            targets: BuffTargetPolicy::UnitsAndHeroes,
        };
        for building in &self.board.buildings {
            let BuildingEffect::AuraStatBonus {
                range,
                targets,
                attack,
                armor,
                max_ap,
                ..
            } = building.effect
            else {
                continue;
            };
            if building.position.distance(position) > i32::from(range) {
                continue;
            }
            if self.building_occupant_side(building.position) != Some(side) {
                continue;
            }
            if !card_interactions::target_policy_allows(targets, is_hero) {
                continue;
            }
            bonus.attack = bonus.attack.saturating_add(attack);
            bonus.armor = bonus.armor.saturating_add(armor);
            bonus.max_ap = bonus.max_ap.saturating_add(max_ap);
        }
        bonus
    }

    fn building_occupant_side(&self, position: HexCoord) -> Option<Side> {
        for participant in self.participants() {
            if !participant.knocked_out && participant.hero.position == position {
                return Some(participant.side);
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.position == position)
            .map(|unit| unit.side)
    }

    pub(super) fn best_building_line_targets(
        &self,
        side: Side,
        origin: HexCoord,
        range: u8,
    ) -> Vec<String> {
        HexCoord::directions()
            .into_iter()
            .map(|direction| {
                self.pieces_for_opposing_team(side)
                    .into_iter()
                    .filter(|piece| {
                        origin.distance(piece.position) <= i32::from(range)
                            && origin.direction_to(piece.position) == Some(direction)
                    })
                    .map(|piece| piece.id)
                    .collect::<Vec<_>>()
            })
            .max_by_key(Vec::len)
            .unwrap_or_default()
    }

    pub(super) fn best_building_heal_target(
        &self,
        side: Side,
        origin: HexCoord,
        range: u8,
        targets: BuffTargetPolicy,
    ) -> Option<String> {
        self.pieces_for_team(side.team())
            .into_iter()
            .filter(|piece| origin.distance(piece.position) <= i32::from(range))
            .filter(|piece| card_interactions::target_policy_allows(targets, piece.is_hero))
            .filter(|piece| self.piece_is_damaged(&piece.id))
            .min_by_key(|piece| origin.distance(piece.position))
            .map(|piece| piece.id)
    }

    pub(super) fn piece_is_damaged(&self, piece_id: &str) -> bool {
        for participant in self.participants() {
            if participant.hero.id == piece_id {
                return participant.hero.hp < participant.hero.max_hp;
            }
        }
        self.board
            .units
            .iter()
            .find(|unit| unit.id == piece_id)
            .is_some_and(|unit| unit.armor < unit.max_armor)
    }

    pub(super) fn is_occupied(&self, coord: HexCoord) -> bool {
        self.participants()
            .into_iter()
            .any(|participant| !participant.knocked_out && participant.hero.position == coord)
            || self.board.units.iter().any(|unit| unit.position == coord)
    }
}

impl From<&Hero> for PieceView {
    fn from(hero: &Hero) -> Self {
        Self {
            id: hero.id.clone(),
            side: hero.side,
            position: hero.position,
            attack: hero.attack,
            attack_range: hero.attack_range,
            ap_remaining: hero.ap_remaining,
            has_attacked: hero.has_attacked,
            is_hero: true,
        }
    }
}

impl From<&Unit> for PieceView {
    fn from(unit: &Unit) -> Self {
        Self {
            id: unit.id.clone(),
            side: unit.side,
            position: unit.position,
            attack: unit.attack,
            attack_range: unit.attack_range,
            ap_remaining: unit.ap_remaining,
            has_attacked: unit.has_attacked,
            is_hero: false,
        }
    }
}
