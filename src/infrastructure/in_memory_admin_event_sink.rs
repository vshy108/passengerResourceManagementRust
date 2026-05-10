//! In-memory append-only sink for `AdminEvent`s with a SHA-256 hash chain
//! for tamper-evidence. Each event stores a hash over the previous event's
//! hash concatenated with the canonical event fields. A broken chain
//! (detected by `GET /audit/verify`) indicates deletion or modification.

// `Arc<T>` = Atomically Reference-Counted shared pointer. Multiple
// owners, deallocated when the last clone is dropped. `Send + Sync`
// when `T` is — required for sharing across threads.
// `Mutex<T>` = mutual exclusion lock for interior mutability across
// threads. Combined `Arc<Mutex<T>>` is the textbook "shared mutable
// state across threads" pattern in Rust.
use std::sync::{Arc, Mutex};

use crate::application::ports::AdminEventSink;
use crate::domain::admin_event::AdminEvent;

/// Compute the next link in the hash chain.
/// `prev_hash` is the hex-encoded SHA-256 of the previous event (or the
/// genesis string for the very first event). Fields are separated with `|`
/// to avoid ambiguous concatenation (e.g. "abc" + "def" vs "ab" + "cdef").
///
/// Compiled only when the `http` feature is active (brings in `sha2` / `hex`).
#[cfg(feature = "http")]
fn compute_hash(prev_hash: &str, event: &AdminEvent) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(event.id.as_bytes());
    hasher.update(b"|");
    hasher.update(event.actor_id.0.as_bytes());
    hasher.update(b"|");
    hasher.update(format!("{:?}", event.action).as_bytes());
    hasher.update(b"|");
    hasher.update(format!("{:?}", event.target_kind).as_bytes());
    hasher.update(b"|");
    hasher.update(event.target_id.as_bytes());
    hasher.update(b"|");
    hasher.update(event.timestamp.0.to_string().as_bytes());
    hasher.update(b"|");
    hasher.update(event.details.as_deref().unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

/// Shared, cloneable in-memory sink. Cloning yields another handle on
/// the same underlying buffer, so tests can keep one handle and pass
/// another into a service. Backed by `Arc<Mutex<…>>` so it is also
/// `Send + Sync` and usable from the HTTP server adapter.
// `Clone` here clones the Arc (cheap pointer bump), NOT the Vec inside.
// All clones see the same buffer.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAdminEventSink {
    inner: Arc<Mutex<Vec<AdminEvent>>>,
    /// Parallel hash chain. `hashes[i]` is the SHA-256 hash of event `i`,
    /// computed over `hashes[i-1]` and the canonical event fields.
    /// Empty when the `http` feature is not active (hash chain opt-in).
    hashes: Arc<Mutex<Vec<String>>>,
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
            // `lock()` returns `Result<MutexGuard<...>, PoisonError<...>>`.
            // Ok holds the guard, which auto-unlocks on drop.
            .lock()
            // `.expect("...")` panics with this message if Err. Used here
            // because a poisoned mutex is genuinely unrecoverable — see
            // doc comment above.
            .expect("admin sink mutex poisoned")
            // `.clone()` on the Vec inside the guard. Returns an owned
            // copy so the caller doesn't hold the lock.
            .clone()
    }

    /// Snapshot events paired with their hash-chain digests.
    /// Each entry is `(event, sha256_hex_hash)`. The first event's hash
    /// is computed against the genesis constant; subsequent events chain
    /// on the previous hash.
    ///
    /// Returns an empty `hashes` vec (paired with events) when the `http`
    /// feature is not active (hash chain is a compile-time opt-in).
    ///
    /// # Panics
    /// Panics if any inner mutex is poisoned.
    #[must_use]
    pub fn snapshot_with_hashes(&self) -> Vec<(AdminEvent, String)> {
        let events = self
            .inner
            .lock()
            .expect("admin sink mutex poisoned")
            .clone();
        let hashes = self
            .hashes
            .lock()
            .expect("admin sink hashes mutex poisoned")
            .clone();
        events
            .into_iter()
            .enumerate()
            .map(|(i, ev)| {
                let hash = hashes.get(i).cloned().unwrap_or_default();
                (ev, hash)
            })
            .collect()
    }

    /// # Panics
    /// See [`snapshot`].
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().expect("admin sink mutex poisoned").len()
    }

    /// Overwrite the stored hash at `index` with `bad_hash`.
    ///
    /// Only compiled when the `http` feature is active (the hash chain itself
    /// is gated the same way). Used in integration tests to simulate a tampered
    /// audit log, driving `GET /audit/verify` down its `broken_at` path.
    ///
    /// # Panics
    /// Panics if the inner hashes mutex is poisoned.
    #[cfg(feature = "http")]
    pub fn corrupt_hash_at(&self, index: usize, bad_hash: &str) {
        let mut hashes = self
            .hashes
            .lock()
            .expect("admin sink hashes mutex poisoned");
        if let Some(h) = hashes.get_mut(index) {
            // FIX: avoid clippy::assigning_clones — write via `String::clear` +
            // `push_str` so we reuse the existing allocation rather than
            // creating a new `String` via `to_owned()` and dropping the old one.
            h.clear();
            h.push_str(bad_hash);
        }
    }

    /// # Panics
    /// See [`snapshot`].
    // Clippy enforces: any type with `len()` should also have `is_empty()`.
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
        // Compute the new hash BEFORE acquiring inner lock to minimise
        // contention (hashing is CPU work, not I/O).
        #[cfg(feature = "http")]
        let new_hash = {
            let prev = self
                .hashes
                .lock()
                .expect("admin sink hashes mutex poisoned")
                .last()
                .cloned()
                // Genesis constant — any fixed public value is fine.
                .unwrap_or_else(|| "genesis".to_owned());
            compute_hash(&prev, &event)
        };

        self.inner
            .lock()
            .expect("admin sink mutex poisoned")
            .push(event);

        #[cfg(feature = "http")]
        self.hashes
            .lock()
            .expect("admin sink hashes mutex poisoned")
            .push(new_hash);
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
            id: "test-event-1".into(),
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
