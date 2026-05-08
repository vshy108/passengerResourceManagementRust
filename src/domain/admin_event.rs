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
    /// UUID v4 assigned at emission — stable across restarts when persisted.
    pub id: String,
    // Who performed the action. Only crew leads can mutate state, so the
    // actor type is fixed (no Actor enum needed here).
    pub actor_id: CrewLeadId,
    pub action: AdminAction,
    // `target_kind` + `target_id` together identify the affected entity.
    // We use a plain `String` for the id because it could refer to any
    // of CrewLeadId/PassengerId/ResourceId (all wrap String).
    pub target_kind: TargetKind,
    pub target_id: String,
    pub timestamp: Timestamp,
    // Optional free-form context (e.g. "tier changed Silver → Gold").
    pub details: Option<String>,
}
