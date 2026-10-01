//! Text and JSON rendering of a recorded run. Output contains no timestamps
//! and follows authoritative state order, so it is stable across runs.

use std::fmt::Write as _;

use rune_lanes_core::event_sourcing::{EventEnvelope, MatchEvent};
use rune_lanes_core::{
    ActionTarget, CardKind, HexCoord, MatchState, Phase, PlayerState, ReplayEvent,
    ReplayManaSource, Side, StackAction, StackItem,
};
use serde_json::{Value, json};

use crate::expect::error_code;
use crate::{Outcome, Run, Step};

pub(crate) fn text(run: &Run) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "setup {}", run.setup);
    digest(&mut out, &run.initial_state);

    let mut expectations = 0;
    let mut failed = 0;
    for step in &run.steps {
        match step {
            Step::Command {
                label,
                side,
                by_ai,
                command,
                outcome,
            } => {
                let origin = if *by_ai { " ai" } else { "" };
                let command = command.as_ref().map_or_else(
                    || "(no AI command available)".to_string(),
                    |command| serde_json::to_string(command).unwrap_or_default(),
                );
                let _ = writeln!(out, "\n#{label} {}{origin} {command}", side_name(*side));
                outcome_text(&mut out, outcome);
            }
            Step::AiTurnFinished {
                label,
                side,
                outcome,
            } => {
                let _ = writeln!(out, "\n#{label} {} ai finishTurn", side_name(*side));
                outcome_text(&mut out, outcome);
            }
            Step::Expect {
                label,
                checks,
                failures,
            } => {
                expectations += 1;
                if failures.is_empty() {
                    let _ = writeln!(out, "\n#{label} expect ok ({checks} checks)");
                } else {
                    failed += 1;
                    let _ = writeln!(out, "\n#{label} expect FAILED");
                    for failure in failures {
                        let _ = writeln!(out, "  - {failure}");
                    }
                }
            }
        }
    }

    let _ = writeln!(
        out,
        "\nlegal commands for {} ({}):",
        side_name(run.legal_side),
        run.legal_commands.len()
    );
    for command in &run.legal_commands {
        let _ = writeln!(
            out,
            "  {}",
            serde_json::to_string(command).unwrap_or_default()
        );
    }
    if failed == 0 {
        let _ = writeln!(out, "\nresult: ok ({expectations} expectations passed)");
    } else {
        let _ = writeln!(
            out,
            "\nresult: FAILED ({failed} of {expectations} expectations failed)"
        );
    }
    out
}

fn outcome_text(out: &mut String, outcome: &Outcome) {
    match outcome {
        Outcome::Accepted {
            event,
            replay_events,
            state,
        } => {
            let _ = writeln!(out, "  accepted: {}", domain_event(event));
            for event in replay_events {
                let _ = writeln!(out, "  event {}", replay_event(event));
            }
            digest(out, state);
        }
        Outcome::Rejected(error) => {
            let _ = writeln!(out, "  rejected: {} ({error})", error_code(error));
        }
    }
}

fn domain_event(envelope: &EventEnvelope) -> String {
    let kind = match &envelope.event {
        MatchEvent::Created { .. } => "created",
        MatchEvent::CommandAccepted { .. } => "commandAccepted",
        MatchEvent::ForfeitAccepted { .. } => "forfeitAccepted",
        MatchEvent::AiTurnFinished { .. } => "aiTurnFinished",
    };
    format!("v{} {kind}", envelope.aggregate_version.0)
}

fn digest(out: &mut String, state: &MatchState) {
    let priority = state.priority_side.map_or("-", side_name);
    let winner = state
        .winner
        .map(|side| format!(" winner={}", side_name(side)))
        .unwrap_or_default();
    let _ = writeln!(
        out,
        "  state round={} phase={} active={} priority={priority}{winner}",
        state.round,
        phase_name(&state.phase),
        side_name(state.active_side),
    );
    for participant in participants(state) {
        let mut line = format!(
            "  {} mana {}/{} hand {} deck {} discard {}",
            side_name(participant.side),
            participant.mana,
            participant.max_mana,
            participant.hand.len(),
            participant.deck_count,
            participant.discard_count,
        );
        let hero = &participant.hero;
        let _ = write!(
            line,
            " | {} {} hp {}/{} atk {} ap {}/{}",
            hero.id,
            hex(hero.position),
            hero.hp,
            hero.max_hp,
            hero.attack,
            hero.ap_remaining,
            hero.max_ap
        );
        if hero.shield > 0 {
            let _ = write!(line, " shield {}", hero.shield);
        }
        if hero.has_attacked {
            line.push_str(" attacked");
        }
        if !hero.items.is_empty() {
            let _ = write!(line, " items {}", hero.items.len());
        }
        if participant.knocked_out {
            line.push_str(" knockedOut");
        }
        for unit in state
            .board
            .units
            .iter()
            .filter(|unit| unit.side == participant.side)
        {
            let _ = write!(
                line,
                " | {} {} atk {} arm {}/{} ap {}/{}",
                unit.id,
                hex(unit.position),
                unit.attack,
                unit.armor,
                unit.max_armor,
                unit.ap_remaining,
                unit.max_ap
            );
            if unit.has_attacked {
                line.push_str(" attacked");
            }
            if !unit.items.is_empty() {
                let _ = write!(line, " items {}", unit.items.len());
            }
        }
        let _ = writeln!(out, "{line}");
    }
    if !state.board.buildings.is_empty() {
        let buildings: Vec<String> = state
            .board
            .buildings
            .iter()
            .map(|building| format!("{} {}", building.id, hex(building.position)))
            .collect();
        let _ = writeln!(out, "  buildings {}", buildings.join(" | "));
    }
    if !state.action_stack.is_empty() {
        let stack: Vec<String> = state.action_stack.iter().map(stack_item).collect();
        let _ = writeln!(out, "  stack {}", stack.join(" | "));
    }
}

fn participants(state: &MatchState) -> Vec<&PlayerState> {
    [
        Some(&state.player),
        Some(&state.opponent),
        state.player_two.as_ref(),
        state.opponent_two.as_ref(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn stack_item(item: &StackItem) -> String {
    let action = match &item.action {
        StackAction::PlayUnit { card, coord } => {
            format!("playUnit {} {}", card.template_id, hex(*coord))
        }
        StackAction::CastSpell { card, target_id } => {
            format!("castSpell {} -> {target_id}", card.template_id)
        }
        StackAction::MovePiece { piece_id, from, to } => {
            format!("movePiece {piece_id} {}->{}", hex(*from), hex(*to))
        }
        StackAction::Attack {
            attacker_id,
            target_id,
        } => format!("attack {attacker_id} -> {target_id}"),
        StackAction::EquipItem { card, carrier_id } => {
            format!("equipItem {} -> {carrier_id}", card.template_id)
        }
        StackAction::BuildManaSource { card, coord } => {
            format!("buildManaSource {} {}", card.template_id, hex(*coord))
        }
        StackAction::BuildBuilding { card, coord } => {
            format!("buildBuilding {} {}", card.template_id, hex(*coord))
        }
        StackAction::ActivateItem {
            carrier_id,
            item_id,
            target,
        } => format!("activateItem {carrier_id}/{item_id}{}", opt_target(target)),
        StackAction::ActivateBuilding {
            building_id,
            occupant_id,
        } => format!("activateBuilding {building_id} by {occupant_id}"),
    };
    format!(
        "{} {} p{} {action}",
        item.id,
        side_name(item.side),
        item.priority
    )
}

fn replay_event(event: &ReplayEvent) -> String {
    match event {
        ReplayEvent::MatchCreated => "matchCreated".to_string(),
        ReplayEvent::TurnStarted { side, round } => {
            format!("turnStarted {} round {round}", side_name(*side))
        }
        ReplayEvent::TurnEnded { side, round } => {
            format!("turnEnded {} round {round}", side_name(*side))
        }
        ReplayEvent::RoundStarted { round } => format!("roundStarted {round}"),
        ReplayEvent::PhaseChanged { side, phase } => {
            format!("phaseChanged {} {}", side_name(*side), phase_name(phase))
        }
        ReplayEvent::CardDrawn { side, card, hidden } => match card {
            Some(card) => format!("cardDrawn {} {}", side_name(*side), card.template_id),
            None if *hidden => format!("cardDrawn {} (hidden)", side_name(*side)),
            None => format!("cardDrawn {}", side_name(*side)),
        },
        ReplayEvent::CardPlayed { side, card, target } => format!(
            "cardPlayed {} {} ({} cost {}) -> {}",
            side_name(*side),
            card.template_id,
            card_kind(&card.kind),
            card.cost,
            target_text(target)
        ),
        ReplayEvent::ActionQueued { side, item } => {
            format!("actionQueued {} {}", side_name(*side), stack_item(item))
        }
        ReplayEvent::UnitSummoned {
            side,
            unit_id,
            name,
            position,
        } => format!(
            "unitSummoned {} {unit_id} \"{name}\" {}",
            side_name(*side),
            hex(*position)
        ),
        ReplayEvent::PieceMoved {
            side,
            piece_id,
            from,
            to,
        } => format!(
            "pieceMoved {} {piece_id} {}->{}",
            side_name(*side),
            hex(*from),
            hex(*to)
        ),
        ReplayEvent::PieceAttacked {
            side,
            attacker_id,
            target_id,
            damage_to_target,
            counter_damage_to_attacker,
        } => format!(
            "pieceAttacked {} {attacker_id} -> {target_id} damage {damage_to_target} counter {counter_damage_to_attacker}",
            side_name(*side)
        ),
        ReplayEvent::PieceHealed {
            side,
            piece_id,
            amount,
        } => format!("pieceHealed {} {piece_id} +{amount}", side_name(*side)),
        ReplayEvent::UnitArmorRefreshed {
            side,
            unit_id,
            amount,
        } => format!(
            "unitArmorRefreshed {} {unit_id} +{amount}",
            side_name(*side)
        ),
        ReplayEvent::PieceBuffed {
            side,
            piece_id,
            attack_delta,
            armor_delta,
        } => format!(
            "pieceBuffed {} {piece_id} atk {attack_delta:+} arm {armor_delta:+}",
            side_name(*side)
        ),
        ReplayEvent::PieceDamaged {
            side,
            piece_id,
            amount,
        } => format!("pieceDamaged {} {piece_id} -{amount}", side_name(*side)),
        ReplayEvent::UnitDestroyed {
            side,
            unit_id,
            name,
        } => format!("unitDestroyed {} {unit_id} \"{name}\"", side_name(*side)),
        ReplayEvent::ManaSourceBuilt { side, coord } => {
            format!("manaSourceBuilt {} {}", side_name(*side), hex(*coord))
        }
        ReplayEvent::BuildingBuilt {
            side,
            building_id,
            name,
            coord,
        } => format!(
            "buildingBuilt {} {building_id} \"{name}\" {}",
            side_name(*side),
            hex(*coord)
        ),
        ReplayEvent::BuildingActivated {
            side,
            building_id,
            name,
            occupant_id,
        } => format!(
            "buildingActivated {} {building_id} \"{name}\" by {occupant_id}",
            side_name(*side)
        ),
        ReplayEvent::HeroShielded {
            side,
            hero_id,
            amount,
        } => format!("heroShielded {} {hero_id} +{amount}", side_name(*side)),
        ReplayEvent::ManaGained {
            side,
            amount,
            source,
        } => {
            let source = match source {
                ReplayManaSource::BarbarianKill { hero_id, unit_id } => {
                    format!("barbarianKill {hero_id} killed {unit_id}")
                }
            };
            format!("manaGained {} +{amount} ({source})", side_name(*side))
        }
        ReplayEvent::ItemEquipped {
            side,
            carrier_id,
            item_id,
            name,
        } => format!(
            "itemEquipped {} {carrier_id}/{item_id} \"{name}\"",
            side_name(*side)
        ),
        ReplayEvent::ItemDropped {
            side,
            carrier_id,
            item_id,
            name,
            position,
        } => format!(
            "itemDropped {} {carrier_id}/{item_id} \"{name}\" {}",
            side_name(*side),
            hex(*position)
        ),
        ReplayEvent::ItemActivated {
            side,
            carrier_id,
            item_id,
            name,
        } => format!(
            "itemActivated {} {carrier_id}/{item_id} \"{name}\"",
            side_name(*side)
        ),
        ReplayEvent::MatchEnded { winner } => format!("matchEnded winner {}", side_name(*winner)),
    }
}

fn card_kind(kind: &CardKind) -> &'static str {
    match kind {
        CardKind::Unit { .. } => "unit",
        CardKind::Spell { .. } => "spell",
        CardKind::Item { .. } => "item",
        CardKind::Building { .. } => "building",
        CardKind::ManaSource => "manaSource",
    }
}

fn target_text(target: &ActionTarget) -> String {
    match target {
        ActionTarget::Hex { coord } => format!("hex {}", hex(*coord)),
        ActionTarget::Piece { piece_id } => format!("piece {piece_id}"),
    }
}

fn opt_target(target: &Option<ActionTarget>) -> String {
    target
        .as_ref()
        .map(|target| format!(" -> {}", target_text(target)))
        .unwrap_or_default()
}

fn hex(coord: HexCoord) -> String {
    format!("({},{})", coord.q, coord.r)
}

pub(crate) fn side_name(side: Side) -> &'static str {
    match side {
        Side::Player => "player",
        Side::Opponent => "opponent",
        Side::PlayerTwo => "playerTwo",
        Side::OpponentTwo => "opponentTwo",
    }
}

fn phase_name(phase: &Phase) -> &'static str {
    match phase {
        Phase::Movement => "movement",
        Phase::Attack => "attack",
        Phase::CardPlay => "cardPlay",
        Phase::MatchOver => "matchOver",
    }
}

/// Full structured output. Each accepted step carries the domain event
/// envelope, the replay events, and the authoritative snapshot afterwards.
pub(crate) fn json(run: &Run) -> String {
    let steps: Vec<Value> = run.steps.iter().map(step_json).collect();
    let failed = run
        .steps
        .iter()
        .filter(|step| matches!(step, Step::Expect { failures, .. } if !failures.is_empty()))
        .count();
    let expectations = run
        .steps
        .iter()
        .filter(|step| matches!(step, Step::Expect { .. }))
        .count();
    let document = json!({
        "setup": run.setup,
        "initialState": snapshot(&run.initial_state),
        "steps": steps,
        "final": {
            "aggregateVersion": run.final_version.0,
            "state": snapshot(&run.final_state),
            "legalCommands": {
                "side": run.legal_side,
                "commands": run.legal_commands,
            },
        },
        "expectations": { "total": expectations, "failed": failed },
    });
    let mut text = serde_json::to_string_pretty(&document).unwrap_or_default();
    text.push('\n');
    text
}

fn step_json(step: &Step) -> Value {
    match step {
        Step::Command {
            label,
            side,
            by_ai,
            command,
            outcome,
        } => {
            let mut value = json!({
                "step": label,
                "kind": if *by_ai { "aiCommand" } else { "command" },
                "side": side,
                "command": command,
            });
            merge_outcome(&mut value, outcome);
            value
        }
        Step::AiTurnFinished {
            label,
            side,
            outcome,
        } => {
            let mut value = json!({ "step": label, "kind": "aiTurnFinished", "side": side });
            merge_outcome(&mut value, outcome);
            value
        }
        Step::Expect {
            label,
            checks,
            failures,
        } => json!({
            "step": label,
            "kind": "expect",
            "checks": checks,
            "passed": failures.is_empty(),
            "failures": failures,
        }),
    }
}

fn merge_outcome(value: &mut Value, outcome: &Outcome) {
    let extra = match outcome {
        Outcome::Accepted {
            event,
            replay_events,
            state,
        } => json!({
            "accepted": true,
            "domainEvent": event,
            "replayEvents": replay_events,
            "state": snapshot(state),
        }),
        Outcome::Rejected(error) => json!({
            "accepted": false,
            "rejection": error,
            "rejectionMessage": error.to_string(),
        }),
    };
    if let (Value::Object(target), Value::Object(extra)) = (value, extra) {
        target.extend(extra);
    }
}

fn snapshot(state: &MatchState) -> Value {
    state
        .to_snapshot_json()
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(Value::Null)
}
