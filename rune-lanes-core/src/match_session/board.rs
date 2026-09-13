use super::{Building, BuildingEffect, HexBoard, HexCoord, HexTile, PieceView};

impl HexBoard {
    pub(crate) fn new(radius: i32) -> Self {
        let mut tiles = Vec::new();
        for q in -radius..=radius {
            for r in -radius..=radius {
                let coord = HexCoord { q, r };
                if coord.distance(HexCoord { q: 0, r: 0 }) <= radius {
                    tiles.push(HexTile { coord });
                }
            }
        }
        tiles.sort_by_key(|tile| (tile.coord.r, tile.coord.q));
        let buildings = vec![mana_well_building(
            "natural-mana-2".to_string(),
            HexCoord { q: 0, r: 0 },
        )];
        Self {
            radius,
            tiles,
            mana_sources: Vec::new(),
            buildings,
            units: Vec::new(),
            dropped_items: Vec::new(),
        }
    }

    pub(crate) fn is_valid(&self, coord: HexCoord) -> bool {
        coord.distance(HexCoord { q: 0, r: 0 }) <= self.radius
    }

    pub(crate) fn replace_mana_wells(&mut self, coords: impl IntoIterator<Item = HexCoord>) {
        self.mana_sources.clear();
        self.buildings
            .retain(|building| !matches!(building.effect, BuildingEffect::TurnStartMana { .. }));
        for (index, coord) in coords.into_iter().enumerate() {
            self.buildings.push(mana_well_building(
                format!("preset-mana-{}", index + 1),
                coord,
            ));
        }
    }

    pub(super) fn migrate_legacy_mana_sources(&mut self) {
        if self.mana_sources.is_empty() {
            return;
        }

        let mut next_index = self.buildings.len() + 1;
        for coord in std::mem::take(&mut self.mana_sources) {
            if self
                .buildings
                .iter()
                .any(|building| building.position == coord)
            {
                continue;
            }
            self.buildings.push(mana_well_building(
                format!("legacy-mana-{next_index}"),
                coord,
            ));
            next_index += 1;
        }
    }

    pub(super) fn remove_legacy_outer_natural_mana_wells(&mut self) {
        self.buildings
            .retain(|building| building.id != "natural-mana-1" && building.id != "natural-mana-3");
    }
}

pub(super) fn mana_well_building(id: String, position: HexCoord) -> Building {
    Building {
        id,
        template_id: "mana-well".to_string(),
        name: "Mana Well".to_string(),
        position,
        effect: BuildingEffect::TurnStartMana { amount: 1 },
        activated_this_turn: false,
    }
}

impl HexCoord {
    pub(super) fn distance(self, other: Self) -> i32 {
        let dq = self.q - other.q;
        let dr = self.r - other.r;
        let ds = -self.q - self.r - (-other.q - other.r);
        dq.abs().max(dr.abs()).max(ds.abs())
    }

    pub(crate) fn is_adjacent(self, other: Self) -> bool {
        self.distance(other) == 1
    }

    pub(super) fn neighbors(self) -> [Self; 6] {
        [
            Self {
                q: self.q + 1,
                r: self.r,
            },
            Self {
                q: self.q + 1,
                r: self.r - 1,
            },
            Self {
                q: self.q,
                r: self.r - 1,
            },
            Self {
                q: self.q - 1,
                r: self.r,
            },
            Self {
                q: self.q - 1,
                r: self.r + 1,
            },
            Self {
                q: self.q,
                r: self.r + 1,
            },
        ]
    }

    pub(super) fn direction_to(self, other: Self) -> Option<Self> {
        let distance = self.distance(other);
        if distance == 0 {
            return None;
        }

        Self::directions()
            .into_iter()
            .find(|direction| self.offset(*direction, distance) == other)
    }

    pub(super) fn offset(self, direction: Self, distance: i32) -> Self {
        Self {
            q: self.q + direction.q * distance,
            r: self.r + direction.r * distance,
        }
    }

    pub(super) fn directions() -> [Self; 6] {
        [
            Self { q: 1, r: 0 },
            Self { q: 1, r: -1 },
            Self { q: 0, r: -1 },
            Self { q: -1, r: 0 },
            Self { q: -1, r: 1 },
            Self { q: 0, r: 1 },
        ]
    }
}

pub(super) fn piece_can_attack(attacker: &PieceView, target: &PieceView) -> bool {
    let distance = attacker.position.distance(target.position);
    distance >= 1 && distance <= i32::from(attacker.attack_range)
}
