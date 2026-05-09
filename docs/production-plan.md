# Production Gap Plan — PRMS

Baseline: 148 tests passing, `cargo clippy` clean, RwLock + rate-limit + SQLite
persistence committed (commits up to `b43c839`).

Each item is a concrete, bounded change. Execution order is listed at the bottom.

---

## P1 — `SystemClock` (Critical correctness)

**Problem:** `build_demo_world()` and `build_world_with_sqlite()` both use
`FakeClock::default()`, which starts at `Timestamp(0)` and increments by 1 ns.
Every event timestamp stored in SQLite is 0, 1, 2, … nanoseconds since the Unix
epoch — January 1, 1970. The audit log and usage history are meaningless in
production.

**Fix:**
- Add `src/infrastructure/system_clock.rs` implementing `Clock` via
  `std::time::SystemTime::now()` returning nanoseconds since Unix epoch as `i64`.
- Replace `FakeClock::default()` with `SystemClock` in `build_demo_world()` and
  `build_world_with_sqlite()`.
- Keep `FakeClock` exclusively in tests.

**Files:** `src/infrastructure/system_clock.rs` (new), `src/infrastructure/mod.rs`,
`src/interface/composition_root.rs`.

**Status:** [ ] not started

---

## P2 — Atomic `flush_to_db()` transaction (Data integrity)

**Problem:** `World::flush_to_db()` calls `sync_crew_leads()`, `sync_passengers()`,
`sync_resources()` as three separate DELETE+INSERT operations. A process crash between
any two leaves the DB in a split-brain state (crew leads updated but passengers still
show old state).

**Fix:**
- Add `fn sync_all()` on `SqliteEntityStore` that wraps all three tables inside a
  single `BEGIN IMMEDIATE` / `COMMIT` transaction.
- `flush_to_db()` calls this single method instead of three separate calls.

**Files:** `src/infrastructure/sqlite_event_store.rs`,
`src/interface/composition_root.rs`.

**Status:** [ ] not started

---

## P3 — Constant-time bearer token comparison (Security — OWASP A07)

**Problem:** `state.api_keys.get(t)` is a HashMap lookup. HashMap short-circuits on
hash mismatch and on byte mismatch — a measurable timing signal lets an attacker
enumerate which token prefixes are valid.

**Fix:**
- Add `subtle = "2"` to `[dependencies]` in `Cargo.toml`.
- Replace HashMap lookup in `AuthActor` extractor with a linear scan using
  `subtle::ConstantTimeEq` on the raw bytes of every key; scan always visits all keys
  regardless of match position.

**Files:** `Cargo.toml`, `src/interface/http.rs` (`FromRequestParts` for `AuthActor`).

**Status:** [ ] not started

---

## P4 — SQLite `busy_timeout` + `synchronous` tuning (Reliability)

**Problem:** `open_db()` sets WAL mode but not `PRAGMA busy_timeout`. If two
connections write simultaneously the second gets `SQLITE_BUSY` immediately and
panics rather than retrying.

**Fix:**
- Add to the `open_db()` PRAGMA batch:
  ```sql
  PRAGMA busy_timeout = 5000;
  PRAGMA synchronous = NORMAL;
  ```

**Files:** `src/infrastructure/sqlite_event_store.rs` (`open_db()`).

**Status:** [ ] not started

---

## P5 — Pedantic `clippy` in CI (Quality gate drift)

**Problem:** Local runs use `-W clippy::pedantic` but CI only runs `-D warnings`.
PRs can introduce pedantic violations that pass CI but fail locally.

**Fix:**
- Change the `--features http` clippy step in CI to:
  ```
  cargo clippy --all-targets --features http -- -D warnings -W clippy::pedantic
  ```

**Files:** `.github/workflows/ci.yml`.

**Status:** [ ] not started

---

## P6 — Structured per-handler tracing spans (Observability)

**Problem:** `TraceLayer` logs one span per request but handlers emit no structured
fields. You cannot query logs by `passenger_id`, `resource_id`, or `actor_id`.

**Fix:**
- Add `tracing::info!` calls with structured fields in every mutating handler
  immediately before returning success. Examples:
  - `create_passenger`: `tracing::info!(passenger_id = %req.id, actor = %actor_id, "passenger created");`
  - `use_resource`: `tracing::info!(passenger_id = %actor_id, resource_id = %req.resource_id, outcome = ?ev.outcome, "resource access recorded");`

**Files:** `src/interface/http.rs` (8–10 handler functions).

**Status:** [ ] not started

---

## P7 — SQLite indexes (Performance)

**Problem:** `GET /reports/history/{passenger_id}` does a full table scan of
`usage_events` and filters in Rust. O(n) for large event logs.

**Fix:**
- Add to `open_db()`:
  ```sql
  CREATE INDEX IF NOT EXISTS idx_usage_passenger ON usage_events(passenger_id);
  CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_events(timestamp);
  CREATE INDEX IF NOT EXISTS idx_admin_timestamp ON admin_events(timestamp);
  ```

**Files:** `src/infrastructure/sqlite_event_store.rs` (`open_db()`).

**Status:** [ ] not started

---

## P8 — Security response headers (OWASP A05)

**Problem:** Responses carry no security headers — MIME-sniffing, clickjacking, and
cross-origin data leaks are unmitigated.

**Fix:**
- Use `tower-http`'s `SetResponseHeaderLayer` (already a dependency) in `router_with()`
  to inject:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `Referrer-Policy: no-referrer`

**Files:** `src/interface/http.rs` (`router_with()`). No new dependencies.

**Status:** [ ] not started

---

## P9 — DB liveness in `health/ready` (Operational)

**Problem:** `GET /health/ready` verifies the in-memory `RwLock` but not whether
SQLite is reachable (disk full, file deleted, permissions revoked). A k8s readiness
probe on this endpoint would not catch a dead database.

**Fix:**
- Add `ping_db() -> bool` on `SqliteEntityStore` (runs `SELECT 1`).
- In `health_ready`, if `entity_store.is_some()`, call `ping_db()` and return 503
  with `"code": "DatabaseUnreachable"` on failure.

**Files:** `src/interface/http.rs`, `src/infrastructure/sqlite_event_store.rs`,
`src/interface/composition_root.rs`.

**Status:** [ ] not started

---

## P10 — E2E tests in CI (Regression safety)

**Problem:** 8 Playwright E2E tests exist but are not run in CI. A handler rename or
DTO field change would pass all 148 unit/integration tests but break the frontend.

**Fix:**
- Add an `e2e` job to `.github/workflows/ci.yml` that:
  1. Builds the Rust release binary.
  2. Starts the server with `PRMS_API_KEYS=... PRMS_ENABLE_RESET=true ./serve &`.
  3. Runs `npm ci && npm run test:e2e` in `web/`.

**Files:** `.github/workflows/ci.yml`.

**Status:** [ ] not started

---

## P11 — TLS / reverse-proxy (HTTPS)

**Problem:** The server speaks plain HTTP. Production traffic must be encrypted.

**Fix:**
- Add a `caddy` service to `docker-compose.yml` using `caddy:2-alpine`.
- Add a `Caddyfile` at repo root for automatic Let's Encrypt termination.
- Bind the `prms` container to `127.0.0.1:8080` only; Caddy is the only entry point.
- Document required `PRMS_CORS_ORIGINS` value.

**Files:** `docker-compose.yml`, `Caddyfile` (new), `README.md`.

**Status:** [ ] not started

---

## P12 — Idempotency on create endpoints (Reliability)

**Problem:** If a `POST /passengers` request succeeds server-side but the TCP
connection drops before the client receives 201, the client retries and gets
409 Conflict. No safe-retry mechanism exists.

**Fix:**
- Accept optional `Idempotency-Key: <uuid>` header on all `POST` endpoints.
- Store `(key → response_body, status_code)` in a `HashMap` inside `AppState`
  with a `Timestamp`-based expiry (10 minutes).
- Persist idempotency keys to an `idempotency_cache` SQLite table so they survive
  restarts.

**Files:** `src/interface/http.rs`, `src/infrastructure/sqlite_event_store.rs`,
`src/interface/composition_root.rs`.

**Status:** [ ] not started

---

## Execution Order

| # | Item | Effort | Category |
|---|------|--------|----------|
| 1 | P1 SystemClock | Small | Correctness |
| 2 | P4 busy_timeout | Trivial | Reliability |
| 3 | P2 Atomic flush | Small | Data integrity |
| 4 | P3 Constant-time auth | Small | Security |
| 5 | P7 SQLite indexes | Trivial | Performance |
| 6 | P5 Pedantic CI | Trivial | Quality |
| 7 | P8 Security headers | Small | Security |
| 8 | P9 DB liveness | Small | Operational |
| 9 | P6 Structured tracing | Medium | Observability |
| 10 | P10 E2E in CI | Medium | Testing |
| 11 | P11 TLS/Caddy | Medium | Infrastructure |
| 12 | P12 Idempotency | Large | Reliability |
