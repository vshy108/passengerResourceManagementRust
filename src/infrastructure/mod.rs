//! Infrastructure adapters: concrete implementations of port traits.

pub mod fake_clock;
pub mod in_memory_admin_event_sink;
pub mod in_memory_usage_event_sink;
#[cfg(feature = "postgres")]
pub mod pg_store;
pub mod sqlite_event_store;
pub mod system_clock;

// Re-export frequently-used types so callers don't need to know the sub-module
// paths (e.g. `infrastructure::FakeClock` instead of
// `infrastructure::fake_clock::FakeClock`).
pub use fake_clock::FakeClock;

// Re-export the entity store so composition_root.rs can import it cleanly.
#[cfg(feature = "postgres")]
pub use pg_store::PgAdminEventSink;
#[cfg(feature = "postgres")]
pub use pg_store::PgEntityStore;
#[cfg(feature = "postgres")]
pub use pg_store::PgUsageEventSink;
#[cfg(feature = "http")]
pub use sqlite_event_store::SqliteEntityStore;
