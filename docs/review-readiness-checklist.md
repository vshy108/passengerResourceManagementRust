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
- [ ] Add a Playwright end-to-end flow through the React UI and live Rust API.
- [ ] Decide whether to close the remaining coverage gap or keep the 98% line gate with documented rationale.

## Production Readiness Follow-Ups

- [ ] Add real authentication and derive `Actor` from trusted identity, not request body fields.
- [ ] Add persistent storage with migrations, backups, and append-only event tables.
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

- [ ] Present the project as complete for the scoped assignment, not production-complete.
- [ ] Be explicit that in-memory state and simulated identity are known trade-offs.
- [ ] Use the spec -> test -> service -> HTTP -> React path as the primary live review narrative.
