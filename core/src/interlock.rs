//! Interlock engine surface. v0.1 scaffolding: the API shape exists so the
//! command path is built around it from day one, but no interlocks are
//! configurable yet.
//!
//! When the engine lands, config validation MUST enforce the law from
//! docs/entity-model.md: an interlock whose subject entity has
//! `criticality: safety` must reference at least one tier-1 or tier-2 guard;
//! a tier-3-only safety interlock is a rejected config, not a warning.

use crate::model::{Command, Entity};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Rejection {
    pub reason: String,
}

/// Tier-3 command filter. Called on every command before publish.
pub fn check(_entity: &Entity, _command: &Command) -> Result<(), Rejection> {
    // ponytail: no interlock config exists yet, so every command passes.
    // The call site and rejection plumbing are the point.
    Ok(())
}
