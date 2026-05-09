//! Infrastructure adapters: concrete implementations of port traits.

pub mod fake_clock;
pub mod in_memory_admin_event_sink;
pub mod in_memory_usage_event_sink;
pub mod sqlite_event_store;

// Re-export the entity store so composition_root.rs can import it cleanly.
#[cfg(feature = "http")]
pub use sqlite_event_store::SqliteEntityStore;
