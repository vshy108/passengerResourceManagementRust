//! Authorisation guards reused across services.

use crate::domain::actor::Actor;
use crate::domain::errors::DomainError;

/// Returns `Ok(())` when `actor` is a Crew Lead, otherwise
/// `Err(DomainError::UnauthorizedActor)`.
///
/// # Errors
/// `DomainError::UnauthorizedActor` if `actor` is not a Crew Lead.
pub fn require_crew_lead(actor: &Actor) -> Result<(), DomainError> {
    match actor {
        Actor::CrewLead(_) => Ok(()),
        Actor::Passenger(_) => Err(DomainError::UnauthorizedActor),
    }
}
