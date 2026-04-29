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
// `trait` = the Rust equivalent of an interface (a contract). Anyone
// who implements `Clock` must provide a `now()` method.
//
// `Send` and `Sync` are *auto traits* the compiler tracks for thread
// safety:
//   - Send  -> a value of this type can be MOVED to another thread.
//   - Sync  -> a `&T` reference can be SHARED across threads.
// Adding them as supertraits (`trait Clock: Send + Sync`) means every
// implementor must be thread-safe. Without this we couldn't put the
// service inside an `Arc<Mutex<...>>` and share it with HTTP handlers.
pub trait Clock: Send + Sync {
    // `&self` = read-only borrow. The clock isn't mutated when reading
    // the time. Returns an owned `Timestamp` (it's `Copy`, so trivial).
    fn now(&self) -> Timestamp;
}

/// Append-only sink for `UsageEvent`s emitted by `AccessService`.
pub trait UsageEventSink: Send + Sync {
    // `&mut self` = exclusive borrow needed to mutate internal state
    // (e.g. push into a Vec). The borrow checker guarantees no other
    // reference exists during this call.
    fn append(&mut self, event: UsageEvent);
}

/// Read-only view over `UsageEvent`s for reporting queries.
pub trait UsageEventSource: Send + Sync {
    // Returning `&[UsageEvent]` (a borrowed slice) instead of cloning a
    // Vec avoids unnecessary allocations. The slice is valid as long as
    // the caller holds the `&self` borrow on the source.
    fn list(&self) -> &[UsageEvent];
}

/// Append-only sink for `AdminEvent`s. AU-R4 / AU-I1.
pub trait AdminEventSink: Send + Sync {
    fn append(&mut self, event: AdminEvent);
}
