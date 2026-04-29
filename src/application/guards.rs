//! Authorisation guards reused across services.

use crate::domain::actor::Actor;
use crate::domain::crew_lead::CrewLeadId;
use crate::domain::errors::DomainError;

/// Returns the Crew Lead's id when `actor` is a Crew Lead, otherwise
/// `Err(DomainError::UnauthorizedActor)`.
///
/// Returning the inner `&CrewLeadId` lets callers downstream (e.g.
/// audit emission) reuse it without re-pattern-matching the actor —
/// which would be a defensive, statically unreachable branch.
///
/// # Errors
/// `DomainError::UnauthorizedActor` if `actor` is not a Crew Lead.
// Free function (not a method on a struct) — guards are pure helpers
// shared across services. Takes `&Actor` (borrow) so the caller keeps
// ownership; returns `&CrewLeadId` borrowed from inside the actor —
// the lifetime is implied (elided) and ties output to input.
pub fn require_crew_lead(actor: &Actor) -> Result<&CrewLeadId, DomainError> {
    match actor {
        // `Actor::CrewLead(id)` destructures and binds `id: &CrewLeadId`
        // because we matched on a `&Actor` (Rust auto-borrows in patterns).
        Actor::CrewLead(id) => Ok(id),
        // `_` ignores the inner PassengerId — we only need to reject.
        Actor::Passenger(_) => Err(DomainError::UnauthorizedActor),
    }
}
