//! Actor — the subject invoking a service method. Authorisation checks
//! at service boundaries are based on the actor's variant.
//!
//! Actor identity is supplied by the caller; this crate intentionally
//! provides no authentication mechanism.

use crate::domain::crew_lead::CrewLeadId;
use crate::domain::passenger::PassengerId;

/// Tagged union — either an admin (Crew Lead) or a regular Passenger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Actor {
    CrewLead(CrewLeadId),
    Passenger(PassengerId),
}
