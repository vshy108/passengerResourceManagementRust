//! Application-layer port traits. Concrete adapters live in
//! `crate::infrastructure`.

use crate::domain::admin_event::AdminEvent;
use crate::domain::timestamp::Timestamp;
use crate::domain::usage_event::UsageEvent;

/// Source of monotonically non-decreasing timestamps. Injected at the
/// composition root so domain and application code stay deterministic.
pub trait Clock {
    fn now(&self) -> Timestamp;
}

/// Append-only sink for `UsageEvent`s emitted by `AccessService`.
pub trait UsageEventSink {
    fn append(&mut self, event: UsageEvent);
}

/// Read-only view over `UsageEvent`s for reporting queries.
pub trait UsageEventSource {
    fn list(&self) -> &[UsageEvent];
}

/// Append-only sink for `AdminEvent`s. AU-R4 / AU-I1.
pub trait AdminEventSink {
    fn append(&mut self, event: AdminEvent);
}
