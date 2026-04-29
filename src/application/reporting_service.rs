//! Reporting service. See `specs/07-reporting.md` (RP).

// `std::collections::HashMap` is the standard hash map. Keys must be
// `Eq + Hash`. Iteration order is randomised by default (DoS hardening).
use std::collections::HashMap;

use crate::application::ports::UsageEventSource;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::domain::usage_event::{Outcome, UsageEvent};

// `Default` derive gives us `TierCounts::default()` returning a value
// where every field is zero/empty. Useful for HashMap::entry().or_default().
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TierCounts {
    pub allowed: u64,
    pub denied: u64,
}

// `<'a, S: ... + ?Sized>` introduces:
//   - `'a`     a lifetime parameter — `&'a S` borrows for at least 'a.
//   - `?Sized` opts OUT of the implicit `Sized` bound, letting `S` be
//             a `dyn Trait` (which has unknown size). Without `?Sized`
//             this struct couldn't hold `&dyn UsageEventSource`.
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
        // Type annotation `: HashMap<Tier, TierCounts>` is required
        // here because `HashMap::new()` is generic and the compiler
        // can't infer the types from this line alone.
        let mut out: HashMap<Tier, TierCounts> = HashMap::new();
        // RP-R2 — pre-populate every tier so absent buckets show zeros.
        // `[a, b, c]` is an array literal; `for x in array` iterates
        // by value (each Tier is Copy).
        for t in [Tier::Silver, Tier::Gold, Tier::Platinum] {
            out.insert(t, TierCounts::default());
        }
        for e in self.source.list() {
            // `entry(k).or_default()` is the canonical "get-or-insert"
            // pattern. Returns `&mut V` so we can mutate fields directly.
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
                // `*counts.entry(...).or_insert(0) += 1`:
                //   entry().or_insert(0) returns `&mut u64`, the `*`
                //   dereferences it, and `+= 1` updates the stored value.
                *counts.entry(e.resource_id.clone()).or_insert(0) += 1;
            }
        }
        // `into_iter()` on a HashMap yields `(K, V)` tuples by value.
        // `.collect()` into `Vec<...>` because we need to sort it (a
        // HashMap has no meaningful order).
        let mut entries: Vec<(ResourceId, u64)> = counts.into_iter().collect();
        // RP-R3 — stable order: count desc, then ResourceId asc.
        // `sort_by` takes a closure returning `Ordering` (Less/Equal/Greater).
        // `b.1.cmp(&a.1)` reverses the count comparison (descending).
        // `.then_with(|| ...)` is the secondary sort: only consulted on
        // ties (lazy — closure runs only when needed).
        entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        // `truncate` shortens the Vec in place (no-op if already <= n).
        entries.truncate(n);
        entries
    }
}
