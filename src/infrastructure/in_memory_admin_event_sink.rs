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
