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
pub fn require_crew_lead(actor: &Actor) -> Result<&CrewLeadId, DomainError> {
    match actor {
        Actor::CrewLead(id) => Ok(id),
        Actor::Passenger(_) => Err(DomainError::UnauthorizedActor),
    }
}
