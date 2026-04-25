//! `UsageEvent` — append-only record of a passenger's resource-access
//! attempt. See `specs/05-access.md` (AC).

use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::domain::timestamp::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Outcome {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageEvent {
    pub id: u64,
    pub passenger_id: PassengerId,
    pub resource_id: ResourceId,
    pub tier_at_attempt: Tier,
    pub min_tier_at_attempt: Tier,
    pub timestamp: Timestamp,
    pub outcome: Outcome,
}
