use super::{MatchState, Side};

impl MatchState {
    pub(super) fn next_unit_id(&mut self, side: Side) -> String {
        let prefix = match side {
            Side::Player => "P",
            Side::Opponent => "O",
            Side::PlayerTwo => "P2",
            Side::OpponentTwo => "O2",
        };
        let id = format!("{prefix}{}", self.next_unit_id);
        self.next_unit_id += 1;
        id
    }

    pub(super) fn next_item_id(&mut self, side: Side) -> String {
        let prefix = match side {
            Side::Player => "PI",
            Side::Opponent => "OI",
            Side::PlayerTwo => "P2I",
            Side::OpponentTwo => "O2I",
        };
        let id = format!("{prefix}{}", self.next_item_id);
        self.next_item_id += 1;
        id
    }

    pub(super) fn next_building_id(&mut self, side: Side) -> String {
        let prefix = match side {
            Side::Player => "PB",
            Side::Opponent => "OB",
            Side::PlayerTwo => "P2B",
            Side::OpponentTwo => "O2B",
        };
        let id = format!("{prefix}{}", self.next_building_id);
        self.next_building_id += 1;
        id
    }
}
