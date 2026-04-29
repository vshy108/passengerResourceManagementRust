//! Deterministic clock for tests. Returns timestamps from a
//! caller-supplied sequence so assertions stay reproducible.

// `std::sync::atomic` provides lock-free atomic primitives that are
// safe to share across threads. We pick `AtomicI64` to match `Timestamp`'s
// inner i64. The alternative would be `Mutex<i64>` — heavier and not
// needed for a simple counter.
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
    // Custom-named constructor (not `new`) — reads better at call sites:
    //   `FakeClock::starting_at(1_000)`.
    #[must_use]
    pub fn starting_at(start: i64) -> Self {
        Self {
            next: AtomicI64::new(start),
        }
    }
}

// `Default` provides a zero-arg constructor: `FakeClock::default()` ==
// `FakeClock::starting_at(0)`. Many APIs (e.g. `or_default()`) rely on
// this trait.
impl Default for FakeClock {
    fn default() -> Self {
        Self::starting_at(0)
    }
}

// Implementing `Clock` for `FakeClock` lets us pass it anywhere a
// generic `C: Clock` is expected.
impl Clock for FakeClock {
    fn now(&self) -> Timestamp {
        // `fetch_add` returns the OLD value AND adds 1 atomically.
        // Equivalent to `let old = next; next += 1; old` but thread-safe.
        // Relaxed is sufficient: we only need a unique increasing
        // sequence per clock; we make no claims about ordering across
        // separate clock instances. Stronger orderings (Acquire/Release)
        // would add memory-fence cost we don't need here.
        Timestamp(self.next.fetch_add(1, Ordering::Relaxed))
    }
}
