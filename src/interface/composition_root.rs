//! Composition root — wires concrete adapters into application
//! services and seeds the demo state. The HTTP server (and any future
//! adapter) consumes the resulting [`World`] inside an `Arc<Mutex<…>>`.

use crate::application::access_service::AccessService;
use crate::application::crew_lead_service::CrewLeadService;
use crate::application::passenger_service::PassengerService;
use crate::application::ports::{AdminEventSink, UsageEventSink, UsageEventSource};
use crate::application::resource_service::ResourceService;
use crate::domain::actor::Actor;
use crate::domain::admin_event::AdminEvent;
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::errors::DomainError;
use crate::domain::passenger::PassengerId;
use crate::domain::resource::ResourceId;
use crate::domain::tier::Tier;
use crate::domain::usage_event::UsageEvent;
#[cfg(feature = "http")]
use crate::infrastructure::SqliteEntityStore;
use crate::infrastructure::fake_clock::FakeClock;
use crate::infrastructure::in_memory_admin_event_sink::InMemoryAdminEventSink;
use crate::infrastructure::in_memory_usage_event_sink::InMemoryUsageEventSink;
#[cfg(feature = "http")]
use crate::infrastructure::sqlite_event_store::{
    SqliteAdminEventSink, SqliteUsageEventSink, open_db,
};
#[cfg(feature = "http")]
use crate::infrastructure::system_clock::SystemClock;

// ---------- sink enums --------------------------------------------------
// These enums dispatch between the in-memory adapters (used in tests and
// default demo mode) and the SQLite adapters (used when PRMS_DB_PATH is
// set). Pattern: enum-as-strategy, per AGENTS.md §10 "when a plain enum
// suffices" for avoiding trait-object towers.

/// Unified usage-event sink/source. Dispatches to in-memory, `SQLite`, or `PostgreSQL`.
pub enum UsageSink {
    InMemory(InMemoryUsageEventSink),
    #[cfg(feature = "http")]
    Sqlite(SqliteUsageEventSink),
    #[cfg(feature = "postgres")]
    Pg(crate::infrastructure::PgUsageEventSink),
}

impl UsageEventSink for UsageSink {
    fn append(&mut self, event: UsageEvent) {
        match self {
            Self::InMemory(s) => s.append(event),
            #[cfg(feature = "http")]
            Self::Sqlite(s) => s.append(event),
            #[cfg(feature = "postgres")]
            Self::Pg(s) => s.append(event),
        }
    }
}

impl UsageEventSource for UsageSink {
    fn list(&self) -> &[UsageEvent] {
        match self {
            Self::InMemory(s) => s.list(),
            #[cfg(feature = "http")]
            Self::Sqlite(s) => s.list(),
            #[cfg(feature = "postgres")]
            Self::Pg(s) => s.list(),
        }
    }
}

/// Unified admin-event sink. Cloneable so multiple services share one
/// buffer. Dispatches to in-memory, `SQLite`, or `PostgreSQL`.
#[derive(Clone)]
pub enum AuditSink {
    InMemory(InMemoryAdminEventSink),
    #[cfg(feature = "http")]
    Sqlite(SqliteAdminEventSink),
    #[cfg(feature = "postgres")]
    Pg(crate::infrastructure::PgAdminEventSink),
}

impl AuditSink {
    /// All admin events recorded so far (cloned snapshot).
    #[must_use]
    pub fn snapshot(&self) -> Vec<AdminEvent> {
        match self {
            Self::InMemory(s) => s.snapshot(),
            #[cfg(feature = "http")]
            Self::Sqlite(s) => s.snapshot(),
            #[cfg(feature = "postgres")]
            Self::Pg(s) => s.snapshot(),
        }
    }

    /// All admin events paired with their hash-chain digests. Events loaded
    /// from the `SQLite` adapter do not carry hashes (empty string), because
    /// the hash chain is recomputed in memory on each server run.
    #[must_use]
    pub fn snapshot_with_hashes(&self) -> Vec<(AdminEvent, String)> {
        match self {
            Self::InMemory(s) => s.snapshot_with_hashes(),
            #[cfg(feature = "http")]
            Self::Sqlite(s) => s
                .snapshot()
                .into_iter()
                .map(|ev| (ev, String::new()))
                .collect(),
            #[cfg(feature = "postgres")]
            Self::Pg(s) => s.snapshot_with_hashes(),
        }
    }
}

impl AdminEventSink for AuditSink {
    fn append(&mut self, event: AdminEvent) {
        match self {
            Self::InMemory(s) => s.append(event),
            #[cfg(feature = "http")]
            Self::Sqlite(s) => s.append(event),
            #[cfg(feature = "postgres")]
            Self::Pg(s) => s.append(event),
        }
    }
}

// ---------- World -------------------------------------------------------

/// In-process state owned by exactly one HTTP server. Held inside a
/// `Mutex` by the caller; not internally synchronised.
// Composition Root pattern (Dependency Injection): a single struct
// bundling the wired services. Built once at startup, shared via
// `Arc<Mutex<World>>` to handlers. Services know nothing about each
// other's wiring — only this file does.
pub struct World {
    pub crew_leads: CrewLeadService,
    pub passengers: PassengerService<FakeClock>,
    pub resources: ResourceService<FakeClock>,
    pub access: AccessService<FakeClock, UsageSink>,
    /// Clone-able handle on the same shared admin-event buffer the
    /// services write to — exposed so reporting endpoints can read it.
    pub audit_sink: AuditSink,
    /// Present when `PRMS_DB_PATH` is set. HTTP handlers call
    /// `flush_to_db()` after each entity mutation to persist state.
    #[cfg(feature = "http")]
    pub entity_store: Option<SqliteEntityStore>,
}

impl World {
    /// Flush all entity state to `SQLite`. No-op when `entity_store` is `None`.
    ///
    /// # Panics
    /// Panics if any `SQLite` write fails — a divergence between in-memory
    /// and persistent state is unrecoverable, so crashing is correct.
    #[cfg(feature = "http")]
    pub fn flush_to_db(&self) {
        if let Some(store) = &self.entity_store {
            // FIX: use sync_all() so all three entity tables are replaced inside
            // a single BEGIN IMMEDIATE / COMMIT transaction. Previously three
            // separate DELETE+INSERT calls meant a crash between any two left
            // the DB in a split-brain state (e.g. crew leads updated but
            // passengers still showing old state).
            store.sync_all(
                self.crew_leads.list(),
                self.passengers.list(),
                self.passengers.deleted(),
                self.resources.list(),
                self.resources.deleted(),
            );
        }
    }

    /// Returns `Some(true/false)` if a `SQLite` entity store is configured,
    /// `None` if in-memory only. Used by `GET /health/ready` for DB liveness.
    #[cfg(feature = "http")]
    #[must_use]
    pub fn ping_db(&self) -> Option<bool> {
        // FIX: map(SqliteEntityStore::ping_db) avoids redundant closure
        self.entity_store.as_ref().map(SqliteEntityStore::ping_db)
    }
}

/// Build a fresh demo world: 3 seeded Crew Leads, 3 sample passengers,
/// 3 sample resources. Mirrors the TypeScript demo's `seedWorld`.
///
/// # Errors
/// Propagates any `DomainError` from the underlying services. With the
/// hard-coded seed data this should not happen in practice.
pub fn build_demo_world() -> Result<World, DomainError> {
    // Single audit sink. We `.clone()` the Arc-backed handle into each
    // service so they all write to the SAME underlying buffer. Cloning
    // the sink is cheap (one pointer bump) — see InMemoryAdminEventSink.
    let audit_sink = AuditSink::InMemory(InMemoryAdminEventSink::new());
    let usage_sink = UsageSink::InMemory(InMemoryUsageEventSink::new());
    build_world(audit_sink, usage_sink)
}

/// Build a world backed by `SQLite` at `db_path`.
///
/// On first run (empty entity tables): seeds the demo world and persists
/// all entities to `SQLite`. On subsequent runs: restores entity state
/// from the database without re-seeding, so mutations survive restarts.
/// Usage events and admin events are always loaded from prior runs.
///
/// Use `":memory:"` for a transient `SQLite` database (useful for testing
/// the `SQLite` adapters directly without touching the filesystem).
///
/// # Errors
/// - `BuildError::Sqlite` if any database operation fails.
/// - `BuildError::Domain` if service invariants are violated (should not
///   happen with well-formed persistent data or the seeded demo data).
#[cfg(feature = "http")]
pub fn build_world_with_sqlite(db_path: &str) -> Result<World, BuildError> {
    // Three independent connections to the same file — one per concern.
    // `SQLite` WAL mode (enabled by open_db) handles concurrent access.
    let usage_conn = open_db(db_path).map_err(BuildError::Sqlite)?;
    let admin_conn = open_db(db_path).map_err(BuildError::Sqlite)?;
    let entity_conn = open_db(db_path).map_err(BuildError::Sqlite)?;

    let audit_sink =
        AuditSink::Sqlite(SqliteAdminEventSink::open(admin_conn).map_err(BuildError::Sqlite)?);
    let usage_sink =
        UsageSink::Sqlite(SqliteUsageEventSink::open(usage_conn).map_err(BuildError::Sqlite)?);
    let entity_store = SqliteEntityStore::new(entity_conn);

    if entity_store.is_first_run() {
        // First run: seed demo entities, then persist them to the DB so
        // subsequent restarts can restore state without re-seeding.
        let mut world = build_world(audit_sink, usage_sink).map_err(BuildError::Domain)?;
        world.entity_store = Some(entity_store);
        world.flush_to_db();
        Ok(world)
    } else {
        // Subsequent run: restore entity state from the database.
        let leads = entity_store.load_crew_leads().map_err(BuildError::Sqlite)?;
        let (active_pax, deleted_pax) =
            entity_store.load_passengers().map_err(BuildError::Sqlite)?;
        let (active_res, deleted_res) =
            entity_store.load_resources().map_err(BuildError::Sqlite)?;

        // Restore crew leads WITHOUT emitting bootstrap events (they are
        // already in the admin event log from the original run).
        // FIX: SystemClock ensures future mutations carry real wall-clock timestamps.
        let crew_leads = CrewLeadService::restore(leads)
            .map_err(BuildError::Domain)?
            .with_future_audit(Box::new(SystemClock), Box::new(audit_sink.clone()));

        let passengers = PassengerService::new(FakeClock::starting_at_system_time())
            .with_audit(Box::new(audit_sink.clone()))
            .with_preloaded(active_pax, deleted_pax);

        let resources = ResourceService::new(FakeClock::starting_at_system_time())
            .with_audit(Box::new(audit_sink.clone()))
            .with_preloaded(active_res, deleted_res);

        let access = AccessService::new(FakeClock::starting_at_system_time(), usage_sink);

        Ok(World {
            crew_leads,
            passengers,
            resources,
            access,
            audit_sink,
            entity_store: Some(entity_store),
        })
    }
}

/// Shared wiring for both world builders.
fn build_world(audit_sink: AuditSink, usage_sink: UsageSink) -> Result<World, DomainError> {
    let crew_leads = CrewLeadService::bootstrap_audited(
        // `vec![...]` macro builds a Vec from comma-separated literals.
        vec![
            CrewLead {
                id: CrewLeadId::from("cl-aria"),
                // `"Aria Vega".into()` -> String via `From<&str> for String`.
                name: "Aria Vega".into(),
            },
            CrewLead {
                id: CrewLeadId::from("cl-noor"),
                name: "Noor Hadid".into(),
            },
            CrewLead {
                id: CrewLeadId::from("cl-jun"),
                name: "Jun Park".into(),
            },
        ],
        // `Box::new(...)` heap-allocates so we can store as `Box<dyn Trait>`.
        Box::new(FakeClock::default()),
        Box::new(audit_sink.clone()),
    )?;

    // Builder chain: `new(...).with_audit(...)` — see PassengerService
    // for the with_audit details.
    let mut passengers =
        PassengerService::new(FakeClock::default()).with_audit(Box::new(audit_sink.clone()));
    let mut resources =
        ResourceService::new(FakeClock::default()).with_audit(Box::new(audit_sink.clone()));
    // AccessService doesn't emit ADMIN events (it emits USAGE events),
    // so its sink is the usage-event sink — no audit handle.
    let access = AccessService::new(FakeClock::default(), usage_sink);

    // Synthetic admin used for seeding. Not exposed externally.
    let admin: Actor = Actor::CrewLead(CrewLeadId::from("cl-aria"));

    passengers.create(
        &admin,
        PassengerId::from("ps-001"),
        "Mira Voss".into(),
        Tier::Silver,
    )?;
    passengers.create(
        &admin,
        PassengerId::from("ps-002"),
        "Kai Reeves".into(),
        Tier::Gold,
    )?;
    passengers.create(
        &admin,
        PassengerId::from("ps-003"),
        "Lena Ito".into(),
        Tier::Platinum,
    )?;

    resources.create(
        &admin,
        ResourceId::from("res-lounge"),
        "Stardeck Lounge".into(),
        "social".into(),
        Tier::Silver,
    )?;
    resources.create(
        &admin,
        ResourceId::from("res-spa"),
        "Zero-G Spa".into(),
        "wellness".into(),
        Tier::Gold,
    )?;
    resources.create(
        &admin,
        ResourceId::from("res-bridge"),
        "Bridge Tour".into(),
        "experience".into(),
        Tier::Platinum,
    )?;

    Ok(World {
        crew_leads,
        passengers,
        resources,
        access,
        audit_sink,
        #[cfg(feature = "http")]
        entity_store: None,
    })
}

/// Error from [`build_world_with_sqlite`] or [`build_world_with_postgres`].
#[cfg(feature = "http")]
#[derive(Debug)]
pub enum BuildError {
    Domain(DomainError),
    Sqlite(rusqlite::Error),
    #[cfg(feature = "postgres")]
    Postgres(sqlx::Error),
}

#[cfg(feature = "http")]
impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Domain(e) => write!(f, "domain error: {e}"),
            Self::Sqlite(e) => write!(f, "sqlite error: {e}"),
            #[cfg(feature = "postgres")]
            Self::Postgres(e) => write!(f, "postgres error: {e}"),
        }
    }
}

/// Build a world backed by `PostgreSQL` at `pg_url`.
///
/// On first run (empty `crew_leads` table): seeds the demo world and persists
/// all entities to `PostgreSQL`. On subsequent runs: restores entity state from
/// the database. Usage events and admin events are loaded from prior runs.
///
/// The returned `World` uses **in-memory sinks** for events emitted during the
/// current process lifetime. `flush_to_pg()` in Phase 4 will replace this with
/// per-request async SQL; for now the same `sync_all` flush pattern used for
/// `SQLite` is available on `PgEntityStore`.
///
/// # Errors
/// - `BuildError::Postgres` if any database operation fails.
/// - `BuildError::Domain` if service invariants are violated.
#[cfg(feature = "postgres")]
pub async fn build_world_with_postgres(pg_url: &str) -> Result<World, BuildError> {
    use crate::infrastructure::PgEntityStore;
    use sqlx::PgPool;

    let pool = PgPool::connect(pg_url)
        .await
        .map_err(BuildError::Postgres)?;

    let store = PgEntityStore::new(pool.clone());
    store.migrate().await.map_err(BuildError::Postgres)?;

    let is_first = store.is_first_run().await.map_err(BuildError::Postgres)?;
    if is_first {
        // First run: seed demo world, then persist entities to PG.
        // Use PG write-through sinks so new events in this session are
        // persisted per-request, not just on batch sync.
        let admin_events_existing: Vec<crate::domain::admin_event::AdminEvent> = vec![];
        let usage_events_existing: Vec<crate::domain::usage_event::UsageEvent> = vec![];
        let pg_admin_sink =
            crate::infrastructure::PgAdminEventSink::new(&pool, admin_events_existing);
        let pg_usage_sink =
            crate::infrastructure::PgUsageEventSink::new(&pool, usage_events_existing);
        let audit_sink = AuditSink::Pg(pg_admin_sink);
        let usage_sink = UsageSink::Pg(pg_usage_sink);
        let mut world = build_world(audit_sink, usage_sink).map_err(BuildError::Domain)?;
        store
            .sync_all(
                world.crew_leads.list(),
                world.passengers.list(),
                world.passengers.deleted(),
                world.resources.list(),
                world.resources.deleted(),
            )
            .await
            .map_err(BuildError::Postgres)?;
        #[cfg(feature = "http")]
        {
            world.entity_store = None;
        }
        Ok(world)
    } else {
        // Subsequent run: restore from PG, wire PG write-through sinks.
        let leads = store
            .load_crew_leads()
            .await
            .map_err(BuildError::Postgres)?;
        let (active_pax, deleted_pax) = store
            .load_passengers()
            .await
            .map_err(BuildError::Postgres)?;
        let (active_res, deleted_res) =
            store.load_resources().await.map_err(BuildError::Postgres)?;
        let usage_events = store
            .load_usage_events()
            .await
            .map_err(BuildError::Postgres)?;
        let admin_events = store
            .load_admin_events()
            .await
            .map_err(BuildError::Postgres)?;

        // FIX: Use PG write-through sinks so NEW events during this session
        // are persisted per-request. Historical events are pre-loaded into the
        // in-memory buffer inside each sink (so snapshot() returns them too).
        let pg_admin_sink = crate::infrastructure::PgAdminEventSink::new(&pool, admin_events);
        let pg_usage_sink = crate::infrastructure::PgUsageEventSink::new(&pool, usage_events);
        let audit_sink_pg = AuditSink::Pg(pg_admin_sink);
        let usage_sink_pg = UsageSink::Pg(pg_usage_sink);

        let crew_leads = CrewLeadService::restore(leads)
            .map_err(BuildError::Domain)?
            .with_future_audit(Box::new(SystemClock), Box::new(audit_sink_pg.clone()));

        let passengers = PassengerService::new(FakeClock::starting_at_system_time())
            .with_audit(Box::new(audit_sink_pg.clone()))
            .with_preloaded(active_pax, deleted_pax);

        let resources = ResourceService::new(FakeClock::starting_at_system_time())
            .with_audit(Box::new(audit_sink_pg.clone()))
            .with_preloaded(active_res, deleted_res);

        let access = AccessService::new(FakeClock::starting_at_system_time(), usage_sink_pg);

        Ok(World {
            crew_leads,
            passengers,
            resources,
            access,
            audit_sink: audit_sink_pg,
            #[cfg(feature = "http")]
            entity_store: None,
        })
    }
}
