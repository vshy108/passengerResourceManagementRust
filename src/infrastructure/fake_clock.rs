//! Deterministic clock for tests. Returns timestamps from a
//! caller-supplied sequence so assertions stay reproducible.

use std::sync::atomic::{AtomicI64, Ordering};

use crate::application::ports::Clock;
use crate::domain::timestamp::Timestamp;

/// Increments by 1 ns on every call to `now`, starting from `start`.
/// Cheap, deterministic, and good enough for ordering assertions in
/// integration tests. Backed by `AtomicI64` so it is `Send + Sync`
/// and reusable from the HTTP server adapter.
pub struct FakeClock {
    next: AtomicI64,
}

impl FakeClock {
    #[must_use]
    pub fn starting_at(start: i64) -> Self {
        Self {
            next: AtomicI64::new(start),
        }
    }
}

impl Default for FakeClock {
    fn default() -> Self {
        Self::starting_at(0)
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        // Relaxed is sufficient: we only need a unique increasing
        // sequence per clock; we make no claims about ordering across
        // separate clock instances.
        Timestamp(self.next.fetch_add(1, Ordering::Relaxed))
    }
}
