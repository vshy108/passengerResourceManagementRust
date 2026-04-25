//! A monotonic timestamp (nanoseconds since some fixed epoch chosen by
//! the injected `Clock`). The domain treats it as an opaque comparable
//! integer — it never converts to wall-clock formats.

/// Nanoseconds (or any monotonic unit) supplied by the `Clock` port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(pub i64);
