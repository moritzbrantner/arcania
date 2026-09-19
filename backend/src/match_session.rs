//! Temporary compatibility facade while backend callers migrate to `arcania_core`.
//!
//! Authoritative Arcania rules live in the standalone core crate. Do not add
//! new game rules, legality checks, or deterministic state transitions here.

pub(crate) use arcania_core::*;
