//! In-memory append-only sink for `UsageEvent`s. Used in tests and as
//! the default infrastructure adapter.

use crate::application::ports::{UsageEventSink, UsageEventSource};
use crate::domain::usage_event::UsageEvent;

#[derive(Debug, Default)]
pub struct InMemoryUsageEventSink {
    events: Vec<UsageEvent>,
}

impl InMemoryUsageEventSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

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
