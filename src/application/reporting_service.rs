//! Reporting service. See `specs/07-reporting.md` (RP).

use std::collections::HashMap;

use crate::application::ports::UsageEventSource;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::domain::usage_event::{Outcome, UsageEvent};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCounts {
    pub allowed: u64,
    pub denied: u64,
}

pub struct ReportingService<'a, S: UsageEventSource + ?Sized> {
    source: &'a S,
}

impl<'a, S: UsageEventSource + ?Sized> ReportingService<'a, S> {
    #[must_use]
    pub fn new(source: &'a S) -> Self {
        Self { source }
    }

    /// RP-R1 — passenger-scoped chronological history.
    #[must_use]
    pub fn personal_history(&self, passenger_id: &PassengerId) -> Vec<UsageEvent> {
        self.source
            .list()
            .iter()
            .filter(|e| e.passenger_id == *passenger_id)
            .cloned()
            .collect()
    }

    /// RP-R2 — counts by `tier_at_attempt` snapshot. Every `Tier`
    /// variant appears in the result.
    #[must_use]
    pub fn aggregate_by_tier(&self) -> HashMap<Tier, TierCounts> {
        let mut out: HashMap<Tier, TierCounts> = HashMap::new();
        // RP-R2 — pre-populate every tier so absent buckets show zeros.
        for t in [Tier::Silver, Tier::Gold, Tier::Platinum] {
            out.insert(t, TierCounts::default());
        }
        for e in self.source.list() {
            let entry = out.entry(e.tier_at_attempt).or_default();
            match e.outcome {
                Outcome::Allowed => entry.allowed += 1,
                Outcome::Denied => entry.denied += 1,
            }
        }
        out
    }

    /// RP-R3 / RP-R4 — top `n` resources by allowed-use count, ties
    /// broken by `ResourceId` ascending. `n == 0` short-circuits.
    #[must_use]
    pub fn top_resources(&self, n: usize) -> Vec<(ResourceId, u64)> {
        if n == 0 {
            return Vec::new();
        }
        let mut counts: HashMap<ResourceId, u64> = HashMap::new();
        for e in self.source.list() {
            if e.outcome == Outcome::Allowed {
                *counts.entry(e.resource_id.clone()).or_insert(0) += 1;
            }
        }
        let mut entries: Vec<(ResourceId, u64)> = counts.into_iter().collect();
        // RP-R3 — stable order: count desc, then ResourceId asc.
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        entries.truncate(n);
        entries
    }
}
