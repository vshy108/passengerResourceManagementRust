//! A monotonic timestamp (nanoseconds since some fixed epoch chosen by
//! the injected `Clock`). The domain treats it as an opaque comparable
//! integer — it never converts to wall-clock formats.

/// Nanoseconds (or any monotonic unit) supplied by the `Clock` port.
// Newtype pattern: wrapping `i64` in a struct makes it a distinct type
// (you can't accidentally pass a random integer where a Timestamp is
// expected). Adding `PartialOrd`/`Ord` lets us use `<`, `>`, `.min()`,
// sort vectors, etc. — used by reporting which orders events by time.
// `i64` (not `u64`) because `chrono` exposes nanos as i64; matching the
// width avoids conversion friction at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(pub i64);
