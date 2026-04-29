//! `UsageEvent` — append-only record of a passenger's resource-access
//! attempt. See `specs/05-access.md` (AC).

use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::domain::timestamp::Timestamp;

// Unit-only enum (like a C enum) — `Copy` is cheap and convenient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Allowed,
    Denied,
}

// Captures BOTH the tier the passenger had AND the resource's required
// tier *at the moment of attempt*. Recording both makes the audit log
// self-explanatory even after later upgrades/downgrades.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    // Monotonic event id (assigned by the sink, not the caller).
    // `u64` because we expect more events than passengers/resources.
    pub id: u64,
    pub passenger_id: PassengerId,
    pub resource_id: ResourceId,
    pub tier_at_attempt: Tier,
    pub min_tier_at_attempt: Tier,
    pub timestamp: Timestamp,
    pub outcome: Outcome,
}
