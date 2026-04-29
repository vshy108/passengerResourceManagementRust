//! In-memory append-only sink for `UsageEvent`s. Used in tests and as
//! the default infrastructure adapter.

use crate::application::ports::{UsageEventSink, UsageEventSource};
use crate::domain::usage_event::UsageEvent;

// `Default` derive auto-generates `new()-equivalent` factories: every
// field gets its type's default (`Vec::new()` for the events vec).
#[derive(Debug, Default)]
pub struct InMemoryUsageEventSink {
    events: Vec<UsageEvent>,
}

impl InMemoryUsageEventSink {
    #[must_use]
    pub fn new() -> Self {
        // Idiomatic: defer to `Default` instead of duplicating field
        // initialisers. Keeps `new()` and `default()` in sync.
        Self::default()
    }
}

// Implementing TWO traits on the same struct is fine and idiomatic.
// One type can play multiple roles (writer + reader here).
impl UsageEventSink for InMemoryUsageEventSink {
    fn append(&mut self, event: UsageEvent) {
        self.events.push(event);
    }
}

impl UsageEventSource for InMemoryUsageEventSink {
    fn list(&self) -> &[UsageEvent] {
        &self.events
    }
}
