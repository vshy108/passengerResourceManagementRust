//! Application-layer port traits. Concrete adapters live in
//! `crate::infrastructure`.

use crate::domain::timestamp::Timestamp;

/// Source of monotonically non-decreasing timestamps. Injected at the
/// composition root so domain and application code stay deterministic.
pub trait Clock {
    fn now(&self) -> Timestamp;
}
