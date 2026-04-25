//! In-memory append-only sink for `AdminEvent`s.

use std::sync::{Arc, Mutex};

use crate::application::ports::AdminEventSink;
use crate::domain::admin_event::AdminEvent;

/// Shared, cloneable in-memory sink. Cloning yields another handle on
/// the same underlying buffer, so tests can keep one handle and pass
/// another into a service. Backed by `Arc<Mutex<…>>` so it is also
/// `Send + Sync` and usable from the HTTP server adapter.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAdminEventSink {
    inner: Arc<Mutex<Vec<AdminEvent>>>,
}

impl InMemoryAdminEventSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the events recorded so far.
    ///
    /// # Panics
    /// Panics if the inner mutex is poisoned. The infrastructure layer
    /// is the only place panics on poisoned mutexes are tolerated
    /// (AGENTS.md §3) — a poisoned mutex means another thread panicked
    /// while writing, which is unrecoverable for an audit sink.
    #[must_use]
    pub fn snapshot(&self) -> Vec<AdminEvent> {
        self.inner
            .lock()
            .expect("admin sink mutex poisoned")
            .clone()
    }

    /// # Panics
    /// See [`snapshot`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("admin sink mutex poisoned").len()
    }

    /// # Panics
    /// See [`snapshot`].
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner
            .lock()
            .expect("admin sink mutex poisoned")
            .is_empty()
    }
}

impl AdminEventSink for InMemoryAdminEventSink {
    fn append(&mut self, event: AdminEvent) {
        self.inner
            .lock()
            .expect("admin sink mutex poisoned")
            .push(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::admin_event::{AdminAction, TargetKind};
    use crate::domain::crew_lead::CrewLeadId;
    use crate::domain::timestamp::Timestamp;

    fn sample_event() -> AdminEvent {
        AdminEvent {
            id: 1,
            actor_id: CrewLeadId("cl-1".into()),
            action: AdminAction::CrewLeadBootstrapped,
            target_kind: TargetKind::CrewLead,
            target_id: "cl-1".into(),
            timestamp: Timestamp(0),
            details: None,
        }
    }

    #[test]
    fn new_sink_is_empty_with_zero_len() {
        let sink = InMemoryAdminEventSink::new();
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);
    }

    #[test]
    fn append_increments_len_and_clears_is_empty() {
        let mut sink = InMemoryAdminEventSink::new();
        sink.append(sample_event());
        assert!(!sink.is_empty());
        assert_eq!(sink.len(), 1);
    }
}
