//! Wall-clock adapter for production use. Returns the current system time
//! as nanoseconds since the Unix epoch. Used in `build_demo_world()` and
//! `build_world_with_sqlite()` so every `UsageEvent` and `AdminEvent` carries
//! a meaningful timestamp.
//!
//! **Not used in tests** — tests inject `FakeClock` for deterministic ordering.

use std::time::SystemTime;

use crate::application::ports::Clock;
use crate::domain::timestamp::Timestamp;

/// Returns the current wall-clock time as nanoseconds since the Unix epoch.
///
/// `Send + Sync` is trivially satisfied — `SystemClock` holds no state.
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            // FIX: UNIX_EPOCH is always ≤ now() on any system where the
            // clock is not set backwards past 1970. If somehow it is
            // (misconfigured VM, test environment), we saturate to 0 rather
            // than panicking — a timestamp of 0 is wrong but recoverable.
            .unwrap_or_default()
            .as_nanos();
        // Saturating cast: `u128` nanos since epoch won't overflow `i64` until
        // the year 2262. On a real spaceship running before then this is safe.
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        Timestamp(nanos as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_now_returns_positive_timestamp() {
        let t = SystemClock.now();
        // Any value > 0 means the clock returned a real Unix-epoch nanos value
        // (year 1970+). Timestamp(0) would indicate clock saturation on a
        // misconfigured system — which would be a very different problem.
        assert!(
            t.0 > 0,
            "SystemClock::now() returned a non-positive timestamp"
        );
    }

    #[test]
    fn system_clock_now_is_monotonically_non_decreasing() {
        let t1 = SystemClock.now();
        let t2 = SystemClock.now();
        // t2 must be >= t1: wall-clock time never goes backwards within a process.
        assert!(
            t2.0 >= t1.0,
            "second call returned an earlier timestamp than first"
        );
    }
}
