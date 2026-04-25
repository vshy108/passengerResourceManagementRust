//! In-memory append-only sink for `AdminEvent`s.

use std::cell::RefCell;
use std::rc::Rc;

use crate::application::ports::AdminEventSink;
use crate::domain::admin_event::AdminEvent;

/// Shared, cloneable in-memory sink. Cloning yields another handle on
/// the same underlying buffer, so tests can keep one handle and pass
/// another into a service.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAdminEventSink {
    inner: Rc<RefCell<Vec<AdminEvent>>>,
}

impl InMemoryAdminEventSink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot the events recorded so far.
    #[must_use]
    pub fn snapshot(&self) -> Vec<AdminEvent> {
        self.inner.borrow().clone()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.borrow().len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty()
    }
}

impl AdminEventSink for InMemoryAdminEventSink {
    fn append(&mut self, event: AdminEvent) {
        self.inner.borrow_mut().push(event);
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
