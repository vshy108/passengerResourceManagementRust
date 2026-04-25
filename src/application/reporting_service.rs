//! Reporting service. See `specs/07-reporting.md` (RP).

use std::collections::HashMap;

use crate::application::ports::UsageEventSource;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::domain::usage_event::UsageEvent;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCounts {
    pub allowed: u64,
    pub denied: u64,
}

pub struct ReportingService<'a, S: UsageEventSource + ?Sized> {
    #[allow(dead_code)] // used in GREEN.
    source: &'a S,
}

impl<'a, S: UsageEventSource + ?Sized> ReportingService<'a, S> {
    #[must_use]
    pub fn new(source: &'a S) -> Self {
        Self { source }
    }

    /// RP-R1.
    #[must_use]
    pub fn personal_history(&self, passenger_id: &PassengerId) -> Vec<UsageEvent> {
        let _ = passenger_id;
        todo!("RP-R1")
    }

    /// RP-R2.
    #[must_use]
    pub fn aggregate_by_tier(&self) -> HashMap<Tier, TierCounts> {
        todo!("RP-R2")
    }

    /// RP-R3 / RP-R4.
    #[must_use]
    pub fn top_resources(&self, n: usize) -> Vec<(ResourceId, u64)> {
        let _ = n;
        todo!("RP-R3")
    }
}
