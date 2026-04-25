//! Application-layer port traits. Concrete adapters live in
//! `crate::infrastructure`.

use crate::domain::admin_event::AdminEvent;
use crate::domain::timestamp::Timestamp;
use crate::domain::usage_event::UsageEvent;

/// Source of monotonically non-decreasing timestamps. Injected at the
/// composition root so domain and application code stay deterministic.
///
/// `Send + Sync` is required so the same adapter can back both the
/// in-process test harness and the multi-threaded HTTP server (where
/// services are shared inside a `Mutex`).
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// Append-only sink for `UsageEvent`s emitted by `AccessService`.
pub trait UsageEventSink: Send + Sync {
    fn append(&mut self, event: UsageEvent);
}

/// Read-only view over `UsageEvent`s for reporting queries.
pub trait UsageEventSource: Send + Sync {
    fn list(&self) -> &[UsageEvent];
}

/// Append-only sink for `AdminEvent`s. AU-R4 / AU-I1.
pub trait AdminEventSink: Send + Sync {
    fn append(&mut self, event: AdminEvent);
}
