//! PostgreSQL-backed entity and event store.
//!
//! Mirrors the `SqliteEntityStore` / `SqliteUsageEventSink` /
//! `SqliteAdminEventSink` pattern from `sqlite_event_store.rs`, but uses
//! **`sqlx` async API** against a `PostgreSQL` connection pool. Every method is
//! `async` and must be `.await`ed from an async context.
//!
//! The store is `Clone` because `PgPool` is `Arc`-backed internally.
//!
//! Compiled only when the `postgres` feature is active.

use sqlx::{PgPool, Row};

use crate::domain::admin_event::{AdminAction, AdminEvent, TargetKind};
use crate::domain::crew_lead::{CrewLead, CrewLeadId};
use crate::domain::passenger::{Passenger, PassengerId};
use crate::domain::resource::{Resource, ResourceId};
use crate::domain::tier::Tier;
use crate::domain::timestamp::Timestamp;
use crate::domain::usage_event::{Outcome, UsageEvent};

// ---------- helpers: domain enum <-> TEXT --------------------------------

fn tier_to_str(t: Tier) -> &'static str {
    match t {
        Tier::Silver => "Silver",
        Tier::Gold => "Gold",
        Tier::Diamond => "Diamond",
        Tier::Platinum => "Platinum",
    }
}

fn tier_from_str(s: &str) -> Result<Tier, sqlx::Error> {
    match s {
        "Silver" => Ok(Tier::Silver),
        "Gold" => Ok(Tier::Gold),
        "Diamond" => Ok(Tier::Diamond),
        "Platinum" => Ok(Tier::Platinum),
        other => Err(sqlx::Error::Decode(
            format!("unknown tier: {other:?}").into(),
        )),
    }
}

fn outcome_to_str(o: Outcome) -> &'static str {
    match o {
        Outcome::Allowed => "Allowed",
        Outcome::Denied => "Denied",
    }
}

fn outcome_from_str(s: &str) -> Result<Outcome, sqlx::Error> {
    match s {
        "Allowed" => Ok(Outcome::Allowed),
        "Denied" => Ok(Outcome::Denied),
        other => Err(sqlx::Error::Decode(
            format!("unknown outcome: {other:?}").into(),
        )),
    }
}

fn action_to_str(a: AdminAction) -> &'static str {
    match a {
        AdminAction::CrewLeadBootstrapped => "CrewLeadBootstrapped",
        AdminAction::CrewLeadAdded => "CrewLeadAdded",
        AdminAction::CrewLeadRemoved => "CrewLeadRemoved",
        AdminAction::CrewLeadReplaced => "CrewLeadReplaced",
        AdminAction::PassengerCreated => "PassengerCreated",
        AdminAction::PassengerTierChanged => "PassengerTierChanged",
        AdminAction::PassengerDeleted => "PassengerDeleted",
        AdminAction::ResourceCreated => "ResourceCreated",
        AdminAction::ResourceMinTierChanged => "ResourceMinTierChanged",
        AdminAction::ResourceDeleted => "ResourceDeleted",
    }
}

fn action_from_str(s: &str) -> Result<AdminAction, sqlx::Error> {
    match s {
        "CrewLeadBootstrapped" => Ok(AdminAction::CrewLeadBootstrapped),
        "CrewLeadAdded" => Ok(AdminAction::CrewLeadAdded),
        "CrewLeadRemoved" => Ok(AdminAction::CrewLeadRemoved),
        "CrewLeadReplaced" => Ok(AdminAction::CrewLeadReplaced),
        "PassengerCreated" => Ok(AdminAction::PassengerCreated),
        "PassengerTierChanged" => Ok(AdminAction::PassengerTierChanged),
        "PassengerDeleted" => Ok(AdminAction::PassengerDeleted),
        "ResourceCreated" => Ok(AdminAction::ResourceCreated),
        "ResourceMinTierChanged" => Ok(AdminAction::ResourceMinTierChanged),
        "ResourceDeleted" => Ok(AdminAction::ResourceDeleted),
        other => Err(sqlx::Error::Decode(
            format!("unknown admin action: {other:?}").into(),
        )),
    }
}

fn kind_to_str(k: TargetKind) -> &'static str {
    match k {
        TargetKind::CrewLead => "CrewLead",
        TargetKind::Passenger => "Passenger",
        TargetKind::Resource => "Resource",
    }
}

fn kind_from_str(s: &str) -> Result<TargetKind, sqlx::Error> {
    match s {
        "CrewLead" => Ok(TargetKind::CrewLead),
        "Passenger" => Ok(TargetKind::Passenger),
        "Resource" => Ok(TargetKind::Resource),
        other => Err(sqlx::Error::Decode(
            format!("unknown target kind: {other:?}").into(),
        )),
    }
}

// ---------- PgEntityStore ------------------------------------------------

/// PostgreSQL-backed entity and event store.
///
/// Uses a connection pool (`PgPool`) so each concurrent handler gets its own
/// pooled connection — no global process-level lock is needed for DB access.
///
/// `Clone` is cheap: `PgPool` is `Arc`-wrapped internally.
#[derive(Clone)]
pub struct PgEntityStore {
    pool: PgPool,
}

impl PgEntityStore {
    /// Create a new store around an already-connected pool.
    /// The caller must run [`migrate`] before calling any other method.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Apply the SQL schema. Idempotent (`IF NOT EXISTS` throughout).
    ///
    /// # Errors
    /// `sqlx::Error` if the DDL fails (e.g. permission denied or bad connection).
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        // `raw_sql` executes a multi-statement string — needed because the
        // migration file contains several `CREATE TABLE IF NOT EXISTS` and
        // `CREATE INDEX IF NOT EXISTS` statements separated by semicolons.
        sqlx::raw_sql(include_str!("../../migrations/001_initial.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Returns `true` if the `crew_leads` table is empty (first run).
    ///
    /// # Errors
    /// `sqlx::Error` if the query fails.
    pub async fn is_first_run(&self) -> Result<bool, sqlx::Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM crew_leads")
            .fetch_one(&self.pool)
            .await?;
        Ok(count == 0)
    }

    // ---- load methods ---------------------------------------------------

    /// Load all crew leads in insertion order.
    ///
    /// # Errors
    /// `sqlx::Error` on query or decode failure.
    pub async fn load_crew_leads(&self) -> Result<Vec<CrewLead>, sqlx::Error> {
        let rows = sqlx::query("SELECT id, name FROM crew_leads ORDER BY ctid")
            .fetch_all(&self.pool)
            .await?;
        rows.iter()
            .map(|row| {
                Ok(CrewLead {
                    id: CrewLeadId(row.try_get("id")?),
                    name: row.try_get("name")?,
                })
            })
            .collect()
    }

    /// Load passengers split into `(active, deleted)` lists (insertion order).
    ///
    /// # Errors
    /// `sqlx::Error` on query or decode failure.
    pub async fn load_passengers(&self) -> Result<(Vec<Passenger>, Vec<Passenger>), sqlx::Error> {
        let rows = sqlx::query("SELECT id, name, tier, deleted_at FROM passengers ORDER BY ctid")
            .fetch_all(&self.pool)
            .await?;

        let mut active = Vec::new();
        let mut deleted = Vec::new();
        for row in &rows {
            let tier_s: String = row.try_get("tier")?;
            let p = Passenger {
                id: PassengerId(row.try_get("id")?),
                name: row.try_get("name")?,
                tier: tier_from_str(&tier_s)?,
                deleted_at: row.try_get::<Option<i64>, _>("deleted_at")?.map(Timestamp),
                // FIX: version is in-memory only (not persisted); restore to 0 on load.
                version: 0,
            };
            if p.deleted_at.is_some() {
                deleted.push(p);
            } else {
                active.push(p);
            }
        }
        Ok((active, deleted))
    }

    /// Load resources split into `(active, deleted)` lists (insertion order).
    ///
    /// # Errors
    /// `sqlx::Error` on query or decode failure.
    pub async fn load_resources(&self) -> Result<(Vec<Resource>, Vec<Resource>), sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, name, category, min_tier, deleted_at FROM resources ORDER BY ctid",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut active = Vec::new();
        let mut deleted = Vec::new();
        for row in &rows {
            let min_s: String = row.try_get("min_tier")?;
            let r = Resource {
                id: ResourceId(row.try_get("id")?),
                name: row.try_get("name")?,
                category: row.try_get("category")?,
                min_tier: tier_from_str(&min_s)?,
                deleted_at: row.try_get::<Option<i64>, _>("deleted_at")?.map(Timestamp),
                // FIX: version is in-memory only (not persisted); restore to 0 on load.
                version: 0,
            };
            if r.deleted_at.is_some() {
                deleted.push(r);
            } else {
                active.push(r);
            }
        }
        Ok((active, deleted))
    }

    /// Load all usage events in chronological order.
    ///
    /// # Errors
    /// `sqlx::Error` on query or decode failure.
    pub async fn load_usage_events(&self) -> Result<Vec<UsageEvent>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, passenger_id, resource_id, \
                    tier_at_attempt, min_tier_at_attempt, timestamp, outcome \
             FROM usage_events \
             ORDER BY timestamp ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let tier_s: String = row.try_get("tier_at_attempt")?;
                let min_s: String = row.try_get("min_tier_at_attempt")?;
                let out_s: String = row.try_get("outcome")?;
                Ok(UsageEvent {
                    id: row.try_get("id")?,
                    passenger_id: PassengerId(row.try_get("passenger_id")?),
                    resource_id: ResourceId(row.try_get("resource_id")?),
                    tier_at_attempt: tier_from_str(&tier_s)?,
                    min_tier_at_attempt: tier_from_str(&min_s)?,
                    timestamp: Timestamp(row.try_get("timestamp")?),
                    outcome: outcome_from_str(&out_s)?,
                })
            })
            .collect()
    }

    /// Load all admin events in chronological order.
    ///
    /// # Errors
    /// `sqlx::Error` on query or decode failure.
    pub async fn load_admin_events(&self) -> Result<Vec<AdminEvent>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, actor_id, action, target_kind, target_id, timestamp, details \
             FROM admin_events \
             ORDER BY timestamp ASC, id ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let act_s: String = row.try_get("action")?;
                let kind_s: String = row.try_get("target_kind")?;
                Ok(AdminEvent {
                    id: row.try_get("id")?,
                    actor_id: CrewLeadId(row.try_get("actor_id")?),
                    action: action_from_str(&act_s)?,
                    target_kind: kind_from_str(&kind_s)?,
                    target_id: row.try_get("target_id")?,
                    timestamp: Timestamp(row.try_get("timestamp")?),
                    details: row.try_get("details")?,
                })
            })
            .collect()
    }

    // ---- sync methods ---------------------------------------------------

    /// Atomically replace all three entity tables in a single transaction.
    ///
    /// Uses `DELETE` + `INSERT` inside a `BEGIN`/`COMMIT`. Usage events and
    /// admin events are append-only and are NOT touched by this method.
    ///
    /// # Errors
    /// `sqlx::Error` if the transaction fails.
    pub async fn sync_all(
        &self,
        leads: &[CrewLead],
        active_pax: &[Passenger],
        deleted_pax: &[Passenger],
        active_res: &[Resource],
        deleted_res: &[Resource],
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // ---- crew_leads ----
        sqlx::query("DELETE FROM crew_leads")
            .execute(&mut *tx)
            .await?;
        for lead in leads {
            sqlx::query("INSERT INTO crew_leads (id, name) VALUES ($1, $2)")
                .bind(&lead.id.0)
                .bind(&lead.name)
                .execute(&mut *tx)
                .await?;
        }

        // ---- passengers (active + deleted in one pass) ----
        sqlx::query("DELETE FROM passengers")
            .execute(&mut *tx)
            .await?;
        for p in active_pax.iter().chain(deleted_pax.iter()) {
            sqlx::query(
                "INSERT INTO passengers (id, name, tier, deleted_at) \
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(&p.id.0)
            .bind(&p.name)
            .bind(tier_to_str(p.tier))
            .bind(p.deleted_at.map(|t| t.0))
            .execute(&mut *tx)
            .await?;
        }

        // ---- resources (active + deleted in one pass) ----
        sqlx::query("DELETE FROM resources")
            .execute(&mut *tx)
            .await?;
        for r in active_res.iter().chain(deleted_res.iter()) {
            sqlx::query(
                "INSERT INTO resources (id, name, category, min_tier, deleted_at) \
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(&r.id.0)
            .bind(&r.name)
            .bind(&r.category)
            .bind(tier_to_str(r.min_tier))
            .bind(r.deleted_at.map(|t| t.0))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Append a usage event. Idempotent: duplicate `id` is silently ignored.
    ///
    /// # Errors
    /// `sqlx::Error` if the insert fails.
    pub async fn append_usage_event(&self, event: &UsageEvent) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO usage_events \
             (id, passenger_id, resource_id, tier_at_attempt, min_tier_at_attempt, timestamp, outcome) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&event.id)
        .bind(&event.passenger_id.0)
        .bind(&event.resource_id.0)
        .bind(tier_to_str(event.tier_at_attempt))
        .bind(tier_to_str(event.min_tier_at_attempt))
        .bind(event.timestamp.0)
        .bind(outcome_to_str(event.outcome))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Append an admin event. Idempotent: duplicate `id` is silently ignored.
    ///
    /// # Errors
    /// `sqlx::Error` if the insert fails.
    pub async fn append_admin_event(&self, event: &AdminEvent) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO admin_events \
             (id, actor_id, action, target_kind, target_id, timestamp, details) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&event.id)
        .bind(&event.actor_id.0)
        .bind(action_to_str(event.action))
        .bind(kind_to_str(event.target_kind))
        .bind(&event.target_id)
        .bind(event.timestamp.0)
        .bind(event.details.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Verify connectivity with a trivial query.
    ///
    /// # Errors
    /// `sqlx::Error` if the pool cannot acquire a connection or the query fails.
    pub async fn ping(&self) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await?;
        Ok(())
    }
}

// ---------- PgAdminEventSink ---------------------------------------------

/// Write-through `PostgreSQL` admin-event sink.
///
/// Each `append` call:
/// 1. Immediately stores the event in the in-memory buffer (for fast reads).
/// 2. Sends the event to a background tokio task that writes it to `PostgreSQL`.
///
/// Events are visible to `snapshot()` immediately; PG persistence is
/// best-effort on a background task (fire-and-forget). A channel-closed
/// error is logged — it means the background task exited, which is unrecoverable.
///
/// `Clone` is cheap: `PgPool` and `UnboundedSender` are `Arc`-backed.
#[derive(Clone)]
pub struct PgAdminEventSink {
    mem: crate::infrastructure::in_memory_admin_event_sink::InMemoryAdminEventSink,
    tx: tokio::sync::mpsc::UnboundedSender<AdminEvent>,
}

impl PgAdminEventSink {
    /// Construct the sink, pre-loading historical events from `existing`.
    /// The background writer task is spawned immediately on the current tokio runtime.
    #[must_use]
    pub fn new(pool: &PgPool, existing: Vec<AdminEvent>) -> Self {
        use crate::application::ports::AdminEventSink as _;
        let mut mem =
            crate::infrastructure::in_memory_admin_event_sink::InMemoryAdminEventSink::new();
        // Pre-load historical events into the memory buffer without re-writing to PG.
        for ev in existing {
            mem.append(ev);
        }
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AdminEvent>();
        let pool_bg = pool.clone();
        tokio::spawn(async move {
            let store = PgEntityStore::new(pool_bg);
            while let Some(event) = rx.recv().await {
                if let Err(e) = store.append_admin_event(&event).await {
                    tracing::error!(
                        event_id = %event.id,
                        error = %e,
                        "PgAdminEventSink: failed to persist admin event to PostgreSQL"
                    );
                }
            }
        });
        Self { mem, tx }
    }

    /// Snapshot of all events (historical + current session).
    #[must_use]
    pub fn snapshot(&self) -> Vec<AdminEvent> {
        self.mem.snapshot()
    }

    /// Snapshot with hash-chain digests. Delegates to the in-memory buffer.
    #[must_use]
    pub fn snapshot_with_hashes(&self) -> Vec<(AdminEvent, String)> {
        self.mem.snapshot_with_hashes()
    }
}

impl crate::application::ports::AdminEventSink for PgAdminEventSink {
    fn append(&mut self, event: AdminEvent) {
        // Write to in-memory first for immediate read visibility.
        self.mem.append(event.clone());
        // FIX: log on channel-closed rather than silently discarding the event.
        // A closed channel means the background PG writer task has exited
        // (unrecoverable), so the error is surfaced to the operator via logs.
        if let Err(e) = self.tx.send(event) {
            tracing::error!(
                event_id = %e.0.id,
                "PgAdminEventSink: background writer channel closed; event not persisted to PostgreSQL"
            );
        }
    }
}

// ---------- PgUsageEventSink ---------------------------------------------

/// Write-through `PostgreSQL` usage-event sink.
///
/// Same design as [`PgAdminEventSink`]: in-memory buffer for fast reads,
/// background tokio task for PG persistence.
#[derive(Clone)]
pub struct PgUsageEventSink {
    mem: crate::infrastructure::in_memory_usage_event_sink::InMemoryUsageEventSink,
    tx: tokio::sync::mpsc::UnboundedSender<UsageEvent>,
}

impl PgUsageEventSink {
    /// Construct the sink, pre-loading historical events from `existing`.
    #[must_use]
    pub fn new(pool: &PgPool, existing: Vec<UsageEvent>) -> Self {
        use crate::application::ports::UsageEventSink as _;
        let mut mem =
            crate::infrastructure::in_memory_usage_event_sink::InMemoryUsageEventSink::new();
        for ev in existing {
            mem.append(ev);
        }
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<UsageEvent>();
        let pool_bg = pool.clone();
        tokio::spawn(async move {
            let store = PgEntityStore::new(pool_bg);
            while let Some(event) = rx.recv().await {
                if let Err(e) = store.append_usage_event(&event).await {
                    tracing::error!(
                        event_id = %event.id,
                        error = %e,
                        "PgUsageEventSink: failed to persist usage event to PostgreSQL"
                    );
                }
            }
        });
        Self { mem, tx }
    }

    /// List all usage events (historical + current session).
    #[must_use]
    pub fn list(&self) -> &[UsageEvent] {
        use crate::application::ports::UsageEventSource;
        self.mem.list()
    }
}

impl crate::application::ports::UsageEventSink for PgUsageEventSink {
    fn append(&mut self, event: UsageEvent) {
        self.mem.append(event.clone());
        if let Err(e) = self.tx.send(event) {
            tracing::error!(
                event_id = %e.0.id,
                "PgUsageEventSink: background writer channel closed; event not persisted to PostgreSQL"
            );
        }
    }
}

impl crate::application::ports::UsageEventSource for PgUsageEventSink {
    fn list(&self) -> &[UsageEvent] {
        self.mem.list()
    }
}
