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
    /// UUID v4 assigned at emission — stable across restarts when persisted.
    pub id: String,
    pub passenger_id: PassengerId,
    pub resource_id: ResourceId,
    /// Snapshot of the passenger's tier AT THE TIME of the attempt.
    /// Immutable after emission — later tier changes do not retroactively
    /// reclassify this event (AC-R6 / RP-R3).
    pub tier_at_attempt: Tier,
    /// Snapshot of the resource's `min_tier` AT THE TIME of the attempt.
    /// Same immutability guarantee as `tier_at_attempt`.
    pub min_tier_at_attempt: Tier,
    /// Timestamp from the injected `Clock` at the moment of the call.
    pub timestamp: Timestamp,
    /// Whether the access was allowed or denied. A `Denied` event is
    /// still stored — it serves as the audit record of the failed attempt.
    pub outcome: Outcome,
}
