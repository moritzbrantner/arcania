//! Compatibility re-exports for callers that still import `rune_lanes_core::cqrs`.
//!
//! New code should use the explicit `commands` and `queries` modules. Keeping
//! this bridge avoids turning the architecture cleanup into a transport/persistence
//! migration.

pub use crate::commands::*;
pub use crate::queries::*;
