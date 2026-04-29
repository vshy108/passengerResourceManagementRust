//! Actor — the subject invoking a service method. Authorisation checks
//! at service boundaries are based on the actor's variant.
//!
//! Actor identity is supplied by the caller; this crate intentionally
//! provides no authentication mechanism.

// `crate::` is the absolute path to THIS crate's root (lib.rs). It's
// the most readable way to reach across modules. Alternatives are
// `super::` (parent module) and `self::` (current module).
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::passenger::PassengerId;

/// Tagged union — either an admin (Crew Lead) or a regular Passenger.
// Note: NOT `Copy` because `CrewLeadId`/`PassengerId` wrap `String`
// (heap-allocated → copying is non-trivial). We must call `.clone()`
// explicitly when we need a duplicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    // Each variant carries one piece of data — Rust enums are sum types
    // (algebraic data types). Pattern matching destructures the inner
    // value, e.g. `match actor { Actor::CrewLead(id) => ... }`.
    CrewLead(CrewLeadId),
    Passenger(PassengerId),
}
