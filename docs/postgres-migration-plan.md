# PostgreSQL Migration Plan — PRMS

> **Status (2026-05-14): Phases 1–3 are complete. Phase 4 (full async per-request SQL) remains future work.**

**Goal**: Eliminate the global `Arc<RwLock<World>>` write-lock bottleneck so
concurrent writes to different aggregates proceed without serialization, and lay
a foundation for a fully PostgreSQL-backed future architecture.

---

## Phase 1 — Foundation (no behaviour change, all tests green)

**What changes:**
- `Cargo.toml` — new optional `postgres` feature; adds
  `sqlx = { version = "0.8", features = ["postgres","runtime-tokio","tls-rustls"], optional = true }`.
- `migrations/001_initial.sql` — idempotent PostgreSQL DDL (same 5 tables as
  SQLite: `crew_leads`, `passengers`, `resources`, `usage_events`,
  `admin_events`; uses `$1/$2` placeholders; `IF NOT EXISTS` throughout).
- `src/infrastructure/pg_store.rs` — `PgEntityStore { pool: PgPool }`:
  - `new(pool) → Self`, `migrate(&self) → Result<(), sqlx::Error>` (runs DDL)
  - `is_first_run(&self) → Result<bool, sqlx::Error>`
  - `load_crew_leads / load_passengers / load_resources /`
    `load_usage_events / load_admin_events` (all async)
  - `sync_all(leads, active_pax, deleted_pax, active_res, deleted_res)`
    (async; single transaction: DELETE + INSERT)
  - `append_usage_event / append_admin_event` (async; idempotent via
    `ON CONFLICT (id) DO NOTHING`)
  - `ping() → Result<(), sqlx::Error>`
- `src/infrastructure/mod.rs` — `pub mod pg_store;`
  `#[cfg(feature = "postgres")] pub use pg_store::PgEntityStore;`

**Invariant:** Zero existing tests touched. `cargo nextest run --features http`
stays green.

---

## Phase 2 — Wire `PRMS_PG_URL` (minimal changes)

**What changes:**
- `src/interface/composition_root.rs`:
  - `UsageSink` and `AuditSink` enums gain no new variants yet (PG events go
    through the existing in-memory sinks and are flushed via `sync_all` after
    each mutation — same pattern as SQLite).
  - New `async fn build_world_with_postgres(url: &str) → Result<World, BuildError>`:
    creates a `PgPool`, runs `PgEntityStore::migrate()`, checks `is_first_run`,
    seeds demo world if yes, otherwise loads all entities from PG.
  - `BuildError` gains `Postgres(sqlx::Error)` variant.
- `src/bin/serve.rs`:
  - New `--pg-url` / `PRMS_PG_URL` arg (optional `String`).
  - Priority: `PRMS_PG_URL` → `PRMS_DB_PATH` → in-memory.
  - PG path calls `build_world_with_postgres(url).await`.

**Invariant:** Zero existing tests touched. New PG path is opt-in via env var.

---

## Phase 3 — Per-aggregate locking (eliminates global write lock)

**What changes:** `src/interface/http.rs` only (+ no change to test helpers).

### Design

Introduce `WorldShards` — a private struct that holds one `RwLock` per
aggregate instead of one `RwLock` around the entire `World`:

```rust
struct WorldShards {
    // Canonical lock order (prevents deadlock):
    //   crew_leads → passengers → resources → access → audit_sink
    crew_leads: RwLock<CrewLeadService>,
    passengers: RwLock<PassengerService<FakeClock>>,
    resources:  RwLock<ResourceService<FakeClock>>,
    access:     RwLock<AccessService<FakeClock, UsageSink>>,
    audit_sink: RwLock<AuditSink>,
    entity_store: Option<SqliteEntityStore>,  // not behind a lock; immutable after init
}
```

`AppState::new(world: World, api_keys: HashMap<String, String>)` keeps the same
**public signature** — it decomposes `World` into `WorldShards` internally.
`http_common::app()` and `serve.rs` require **zero changes**.

### Flush helper

```rust
/// Collect entity state under brief per-aggregate read locks, then write to
/// SQLite outside any lock. Calling this after releasing the write lock means
/// I/O never blocks other handlers.
fn flush_to_db(state: &AppState) { ... }
```

### Handler patterns

| Pattern | Lock(s) held | Notes |
|---|---|---|
| Read-only (list, get, report) | one `read()` | Released before return |
| Write (create, change, delete) | one `write()` | Released before `flush_to_db` |
| `use_resource` | passengers(read) + resources(read) + access(write) | Acquired in canonical order to prevent deadlock |
| `reset_world` | all 5 write locks simultaneously | Canonical order; brief inconsistency window acceptable (demo-only) |
| `health_ready` / `metrics` | five separate `read()` | Each released before next acquired |

### Concurrency improvement

Before: ALL mutations serialize through a single `RwLock<World>`.

After: A `POST /passengers` and a `POST /resources` proceed concurrently with
zero lock contention. Only writes to the **same** aggregate type serialize.
`GET` requests never block any handler.

---

## Phase 4 — Full async per-request SQL (future work)

Making port traits async (`UsageEventSink`, `AdminEventSink`, `Clock`) would
allow each handler to read/write SQL directly per request with no in-memory
World at all. This eliminates even the per-aggregate locks.

Requires:
1. Async trait stabilisation (available in Rust 1.75+ via `async fn in traits`).
2. All application services rewritten to `async fn` and injected with `PgPool`.
3. All 185 tests rewritten to use `sqlx::test` with a live PostgreSQL instance.
4. CI `services: postgres:17` in GitHub Actions.

**Estimated scope**: multi-week effort. Not in scope for the current sprint.
