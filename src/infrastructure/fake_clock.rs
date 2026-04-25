//! Deterministic clock for tests. Returns timestamps from a
//! caller-supplied sequence so assertions stay reproducible.

use std::cell::Cell;

use crate::application::ports::Clock;
use crate::domain::timestamp::Timestamp;

/// Increments by 1 ns on every call to `now`, starting from `start`.
/// Cheap, deterministic, and good enough for ordering assertions in
/// integration tests.
pub struct FakeClock {
    next: Cell<i64>,
}

impl FakeClock {
    #[must_use]
    pub fn starting_at(start: i64) -> Self {
        Self {
            next: Cell::new(start),
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
        let t = self.next.get();
        self.next.set(t + 1);
        Timestamp(t)
    }
}
