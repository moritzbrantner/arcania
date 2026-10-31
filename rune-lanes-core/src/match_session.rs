mod actions;
mod ai_turn;
mod board;
#[path = "card_interactions.rs"]
mod card_interactions;
mod card_play;
mod combat;
mod construction;
mod effects;
mod ids;
mod items;
mod pieces;
mod projection;
mod replay;
#[cfg(any(test, debug_assertions))]
pub mod scenarios;
mod serialization;
mod solo_ai_policy;
mod stack;
mod turn_flow;
mod types;
pub use replay::*;
pub use types::*;
pub(crate) use types::{DestroyedUnit, PieceView, StatBonus};

use board::piece_can_attack;
use construction::mana_with_progression;
use effects::{AppliedStatDelta, apply_ap_delta};
pub use solo_ai_policy::{
    AiPolicyConfig, AiPolicyConfigError, AiPolicyDefinition, SoloAiPolicy, SoloAiRuleId,
    default_policy_config_path,
};
use solo_ai_policy::{SoloAiActionIntent, SoloAiCarriedItem, SoloAiDecision, SoloAiView};

#[cfg(test)]
use serde_json::json;

#[cfg(test)]
mod tests;
