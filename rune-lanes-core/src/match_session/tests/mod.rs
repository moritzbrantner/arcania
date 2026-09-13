use super::*;
use crate::card_catalog::starter_card_templates;

fn hex(q: i32, r: i32) -> HexCoord {
    HexCoord { q, r }
}

fn player_unit_card(game: &MatchState, template_id: &str) -> Card {
    game.player
        .hand
        .iter()
        .chain(game.player.deck.iter())
        .find(|card| card.template_id == template_id)
        .expect("card should exist in starter deck")
        .clone()
}

fn put_card_in_hand(game: &mut MatchState, card: Card) -> String {
    let id = card.id.clone();
    game.player.hand.push(card);
    game.phase = Phase::CardPlay;
    id
}

fn put_card_in_side_hand(game: &mut MatchState, side: Side, card: Card) -> String {
    let id = card.id.clone();
    game.player_mut(side).hand.push(card);
    id
}

fn enter_attack_phase(game: &mut MatchState) {
    game.phase = Phase::Attack;
}

fn enter_card_play(game: &mut MatchState) {
    game.phase = Phase::CardPlay;
}

fn advance_solo_ai_until_player_turn(game: &mut MatchState) {
    for _ in 0..50 {
        if game.active_side == Side::Player && game.action_stack.is_empty() {
            return;
        }

        if game.priority_side == Some(Side::Player) {
            game.apply_action(MatchActionRequest::PassPriority)
                .expect("player can pass priority");
        } else {
            game.apply_action(MatchActionRequest::AdvanceAi)
                .expect("AI can advance");
        }
    }

    panic!("AI did not return control to the player");
}

fn unit_armor(game: &MatchState, unit_id: &str) -> Option<i32> {
    game.board
        .units
        .iter()
        .find(|unit| unit.id == unit_id)
        .map(|unit| unit.armor)
}

fn board_unit(
    id: &str,
    side: Side,
    position: HexCoord,
    attack: i32,
    attack_range: u8,
    armor: i32,
) -> Unit {
    Unit {
        id: id.to_string(),
        side,
        name: id.to_string(),
        template_id: Some(id.to_string()),
        attack,
        attack_range,
        armor,
        max_armor: armor,
        position,
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    }
}

fn damaged_board_unit(
    id: &str,
    side: Side,
    position: HexCoord,
    armor: i32,
    max_armor: i32,
) -> Unit {
    Unit {
        id: id.to_string(),
        side,
        name: id.to_string(),
        template_id: Some(id.to_string()),
        attack: 1,
        attack_range: 1,
        armor,
        max_armor,
        position,
        ap_remaining: 2,
        max_ap: 2,
        has_attacked: false,
        items: Vec::new(),
        stat_markers: Vec::new(),
    }
}

fn card_play_event_names(frames: &[RecordedReplayFrame]) -> Vec<&'static str> {
    frames
        .iter()
        .filter_map(|frame| match frame.event {
            ReplayEvent::CardPlayed { .. } => Some("cardPlayed"),
            ReplayEvent::ActionQueued { .. } => Some("actionQueued"),
            ReplayEvent::UnitSummoned { .. } => Some("unitSummoned"),
            ReplayEvent::PieceHealed { .. } => Some("pieceHealed"),
            ReplayEvent::PieceBuffed { .. } => Some("pieceBuffed"),
            ReplayEvent::PieceDamaged { .. } => Some("pieceDamaged"),
            ReplayEvent::UnitArmorRefreshed { .. } => Some("unitArmorRefreshed"),
            ReplayEvent::CardDrawn { .. } => Some("cardDrawn"),
            ReplayEvent::ItemEquipped { .. } => Some("itemEquipped"),
            ReplayEvent::ItemActivated { .. } => Some("itemActivated"),
            ReplayEvent::BuildingBuilt { .. } => Some("buildingBuilt"),
            ReplayEvent::BuildingActivated { .. } => Some("buildingActivated"),
            ReplayEvent::HeroShielded { .. } => Some("heroShielded"),
            ReplayEvent::ManaSourceBuilt { .. } => Some("manaSourceBuilt"),
            _ => None,
        })
        .collect()
}

mod cards;
mod combat;
mod replay;
mod serialization;
mod shared;
mod turn_flow;
