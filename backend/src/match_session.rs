//! Temporary compatibility facade while backend callers migrate to `rune_lanes_core`.
//!
//! Authoritative Rune Lanes rules live in the standalone core crate. Do not add
//! new game rules, legality checks, or deterministic state transitions here.

pub(crate) use rune_lanes_core::*;
