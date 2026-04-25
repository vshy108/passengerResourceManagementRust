//! Admin audit event types. See `specs/06-audit.md` (AU).

use crate::domain::crew_lead::CrewLeadId;
use crate::domain::timestamp::Timestamp;

/// AU-R3 — closed set of administrative mutations that emit an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdminAction {
    CrewLeadBootstrapped,
    CrewLeadAdded,
    CrewLeadRemoved,
    CrewLeadReplaced,
    PassengerCreated,
    PassengerTierChanged,
    PassengerDeleted,
    ResourceCreated,
    ResourceMinTierChanged,
    ResourceDeleted,
}

/// AU-R2 — kind of entity referenced by `target_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetKind {
    CrewLead,
    Passenger,
    Resource,
}

/// AU-R2 — append-only record of a successful admin mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminEvent {
    pub id: u64,
    pub actor_id: CrewLeadId,
    pub action: AdminAction,
    pub target_kind: TargetKind,
    pub target_id: String,
    pub timestamp: Timestamp,
    pub details: Option<String>,
}
