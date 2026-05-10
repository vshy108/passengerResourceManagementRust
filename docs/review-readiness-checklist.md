# Review Readiness Checklist

This file records senior-review gaps found while preparing the project for code review / submission.

## Immediate Submission Fixes

- [x] Add `README.md` AI Usage Disclosure.
- [x] Add a reviewer path to `README.md` so reviewers know what to inspect first.
- [x] Fix README drift: IDs are string-backed newtypes, not UUID wrappers.
- [x] Fix README drift: current runnable interface is HTTP + React demo, not CLI.
- [x] Fix README drift: entity persistence is currently service-owned in-memory state; event sinks are behind ports.
- [x] Fix README drift: there is no strict ID format validation yet; request DTOs validate JSON shape and enums.
- [x] Document `cargo nextest` install/fallback path.
- [x] Fix README drift: web app is now a React thin client backed by the Rust API, not a TypeScript port of the services running in the browser.

## Code / Product Follow-Ups

- [x] Clarify whether passenger self-access must be enforced in the Rust service API or only by the HTTP shape.
  Self-access is enforced by design: `passenger_id` is derived from `Actor::Passenger`, not a separate parameter.
  Documented in `specs/05-access.md`.
- [x] Generate TypeScript API types/client from `/openapi.json` to reduce contract drift.
  `openapi-typescript` generates `web/src/services/openapi.generated.ts` from the live spec.
  `api.ts` now re-exports all `ApiXxx` and `Tier` types from the generated file.
  Run `npm run generate:types` (with the Rust server running) to regenerate.
- [x] Add a Playwright end-to-end flow through the React UI and live Rust API.
  8 tests in `web/e2e/prms.spec.ts` cover: page load + ONLINE status, seeded passenger/resource
  tables, health/ready counts, access allowed + denied flows, new passenger creation, and
  refresh reload. Config in `web/playwright.config.ts`; run `npm run test:e2e` (requires Rust
  server running at 127.0.0.1:8080 with `--enable-reset`). `vite.config.ts` corrected to use
  `vite` (not `vitest/config`) import.
- [x] Decide whether to close the remaining coverage gap or keep a documented gate.
  Decision: gate at **96%** lines (CI: `--fail-under-lines 96`). The uncovered lines are
  infrastructure glue (mutex-poison 503 path, CORS `Any`/`List` branch, governor rate-limit
  block, SQLite failure paths) that are impractical to hit without unsafe thread manipulation
  or OS-level I/O failure injection. Only `src/bin/` is excluded from the measurement;
  `sqlite_event_store` is covered via `tests/sqlite_persistence.rs`. Current achieved:
  **96.51%** (182 tests, all green).

## Production Readiness Follow-Ups

- [x] Add real authentication and derive `Actor` from trusted identity, not request body fields.
  `AuthActor` extractor in `src/interface/http.rs` reads `Authorization: Bearer <token>`,
  resolves the token against `PRMS_API_KEYS` (a `HashMap<String, String>` built at startup),
  and returns 401 for missing or unknown tokens. The `actor_id` field was removed from all
  mutating request DTOs (`CreatePassengerReq`, `ChangeTierReq`, `CreateResourceReq`,
  `ReplaceCrewLeadReq`, `UseResourceReq`). All HTTP tests updated to use `auth_req` helper
  with `CL_TOKEN`/`PS_TOKEN` constants. E2E tests require `PRMS_API_KEYS=token:actor-id,...`.
  README updated to remove the "No real authentication" trade-off note.
- [x] Add persistent storage with migrations, backups, and append-only event tables.
  SQLite-backed event sinks added (`src/infrastructure/sqlite_event_store.rs`).
  `SqliteUsageEventSink` and `SqliteAdminEventSink` write-through: every `append()`
  is persisted before the in-memory cache is updated. On startup, existing rows are
  loaded so prior runs' events are immediately visible. Entity state (passengers,
  resources, crew leads) still lives in memory — a documented trade-off.
  Set `PRMS_DB_PATH=/path/to/prms.db` (or `--db-path`) to enable. Without it the
  server falls back to the in-memory demo world. Schema: two append-only tables
  (`usage_events`, `admin_events`), WAL mode enabled. 8 unit tests use `:memory:`.
- [x] Remove or strongly protect `/reset` outside demo mode.
  `/reset` is now opt-in via `--enable-reset` / `PRMS_ENABLE_RESET` (default `false`).
  The route is not registered at all unless the flag is set; a `tracing::warn!` fires
  at startup if it is enabled.
- [x] Add pagination for growing endpoints such as `/usage`, `/audit`, and list endpoints.
  Added `?offset=N&limit=N` (default 0/100, max 1000) to `/audit` and `/usage` via
  `PaginationQuery` in dto.rs. OpenAPI spec reflects the new params.
- [x] Add metrics, alerts, and deeper health checks.
  Added `GET /health/ready` — returns JSON with entity counts (crew leads, passengers,
  resources, usage events, admin events); returns 503 if the world mutex is poisoned.
  Added `GET /metrics` — Prometheus text format (no extra crate) with gauges for active
  entities and counters for usage events (allowed/denied split) and admin events.
  Both endpoints covered by integration tests in `tests/http_health.rs`.
- [x] Restrict CORS origins for non-local deployments.
  `PRMS_CORS_ORIGINS` already enforces an allow-list when set; added a
  `tracing::warn!` at startup when CORS is `Any` so operators are alerted.
- [x] Add stable event IDs across restarts (database sequence, UUID, or persisted counter).
  `uuid` crate (v4) added as a dependency. All `AdminEvent` and `UsageEvent` IDs
  changed from `u64` counters (reset on restart) to `Uuid::new_v4().to_string()`.
  DTOs updated: `id` is now `String` in both `AdminEventDto` and `UsageEventDto`.
  Counter fields removed from `PassengerService`, `ResourceService`, `AccessService`,
  and `AuditCfg` in `CrewLeadService`.

## Senior-Review Positioning

- [x] Present the project as complete for the scoped assignment, not production-complete.
- [x] Be explicit that in-memory state and simulated identity are known trade-offs.
- [x] Use the spec -> test -> service -> HTTP -> React path as the primary live review narrative.

## Extras (added during production-readiness pass)

- [x] Pin Node >=22.12 for the web app (`web/.nvmrc` + `engines` field in `web/package.json`).
  Vite 7 requires Node 20.19+ or 22.12+; `.nvmrc` lets contributors run `nvm use` in the
  `web/` directory and get the correct version automatically.

## Remaining Production Gaps (not in scope for this assignment)

Gaps that remain before this could be called production-hardened:

- [x] **Dockerfile / container image.**
  Multi-stage `Dockerfile` (builder: `rust:1-bookworm`, runtime: `debian:bookworm-slim`).
  Runs as non-root uid 10001. `docker-compose.yml` includes the PRMS service plus a
  Caddy reverse-proxy for automatic TLS. `Caddyfile` handles Let's Encrypt certificate
  acquisition and renewal.

- [x] **Rate-limit thresholds configurable.**
  `--rate-limit-rps` / `PRMS_RATE_LIMIT_RPS` (default 10) and
  `--rate-limit-burst` / `PRMS_RATE_LIMIT_BURST` (default 50) added to `Args`.
  `router_with()` accepts both as parameters and passes them to `GovernorConfigBuilder`.
  Startup log records the active values. Committed `9f8b2ec`.

- [x] **Structured JSON log format.**
  `--log-format` / `PRMS_LOG_FORMAT` added to `Args` (`text` | `json`, default `text`).
  `Args::parse()` is now called before subscriber init so the format flag is available.
  `tracing-subscriber` `json` feature added to `Cargo.toml`.
  Committed `9f8b2ec`.

- [x] **Startup warning when `PRMS_API_KEYS` is unset.**
  `tracing::warn!` fires in `serve::main()` when the parsed API key map is empty
  (line 168 of `src/bin/serve.rs`). Already present before this session.

- [x] **Request body size limit.**
  `DefaultBodyLimit::max(64 * 1024)` applied in `router_with()` (http.rs line 238).
  Already present before this session.

- [x] **TLS.**
  `Caddyfile` acts as a TLS-terminating reverse proxy with automatic Let's Encrypt
  certificates. `docker-compose.yml` binds the PRMS backend to `127.0.0.1` inside
  the Docker network; Caddy is the sole external entry point on ports 80/443.
  Already present before this session.
