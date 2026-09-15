use crate::{Card, PlayerState};

impl PlayerState {
    /// Mutable deck access for deterministic AI-lab scenario construction only.
    ///
    /// Product/application code should issue commands rather than mutating zones.
    pub fn deck_mut_for_ai_lab(&mut self) -> &mut [Card] {
        &mut self.deck
    }

    /// Mutable discard access for deterministic AI-lab scenario construction only.
    pub fn discard_mut_for_ai_lab(&mut self) -> &mut [Card] {
        &mut self.discard
    }

    /// Replace the deterministic draw pile for an AI-lab scenario while keeping
    /// the public count projection consistent.
    pub fn replace_deck_for_ai_lab(&mut self, cards: Vec<Card>) {
        self.deck = cards;
        self.deck_count = self.deck.len();
    }

    /// Replace the deterministic discard pile for an AI-lab scenario while
    /// keeping the public count projection consistent.
    pub fn replace_discard_for_ai_lab(&mut self, cards: Vec<Card>) {
        self.discard = cards;
        self.discard_count = self.discard.len();
    }
}
