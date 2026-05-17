# Passenger Resource Management Rust Improvement Plan

This plan tracks future PRMS Rust slices. Keep changes vertical, spec-backed, and verifiable through public seams.

## S1 — API Example Collection

- [x] Add copy/paste HTTP examples for health, crew leads, passengers, resources, access, reports, audit, and reset.
- [x] Include required `Authorization: Bearer ...` headers and expected status codes.
- [x] Verify against `cargo run --features http --bin serve -- --api-keys ... --enable-reset`.

## S2 — Persistence Matrix

- [x] Document which behavior is covered by in-memory, SQLite, and PostgreSQL modes.
- [x] Add focused tests for mode-specific failure cases and startup readiness.
- [x] Verify with: `cargo nextest run --all-features`.

## S3 — Observability Walkthrough

- [x] Document request IDs, log formats, readiness, metrics, and audit verification as one operator path.
- [x] Add smoke checks for `/health/ready`, `/metrics`, and `x-request-id` behavior if gaps remain.
- [x] Verify with the HTTP feature test suite.

## S4 — Web Client Reviewer Flow

- [x] Add a reviewer script for running the Rust API and React thin client together.
- [x] Include seeded state, API key examples, and reset behavior.
- [x] Verify with: `cd web && npm ci && npm run build`.

## S5 — Security Hardening Review

- [x] Review rate limits, API key parsing, CORS configuration, body limits, and security headers as one slice.
- [x] Add tests for any missing boundary cases before changing code.
- [x] Verify with: `cargo clippy --all-targets --all-features -- -D warnings` and `cargo nextest run --features http`.

## S6 — Production Compose Guardrails

- [x] Remove demo bearer tokens from `docker-compose.yml` and require operator-supplied `PRMS_API_KEYS`.
- [x] Require explicit `PRMS_DOMAIN` and `PRMS_CORS_ORIGINS` so compose deployments cannot silently run with localhost TLS/open CORS assumptions.
- [x] Enable the `/data/prms.db` named volume by default so compose does not fall back to in-memory demo mode.
- [x] Verify with: `PRMS_DOMAIN=prms.example.com PRMS_CORS_ORIGINS=https://prms.example.com PRMS_API_KEYS=prod-token:cl-aria docker compose config`.

## S7 — PostgreSQL Smoke Harness

- [x] Add a repeatable script that starts temporary PostgreSQL, runs the API with the `postgres` feature, and waits for `/health/ready`.
- [x] Exercise a write path, allowed access, denied access, and audit verification against the Postgres-backed server.
- [x] Print timing and evidence output so the smoke can be attached to future deployment or persistence reviews.
- [x] Verify with: `bash -n scripts/postgres-smoke.sh`, `cargo check --features postgres`, and `git diff --check`.

## S8 — k6 Load Test Evidence

- [x] Add `k6/passenger_load.js` — write+read cycle with stable per-VU passenger IDs (bounds DB flushes to N_VUs total; subsequent POSTs return 409 without flush).
- [x] Add `docker-compose.loadtest.yml` override — disables rate limiting for the load run.
- [x] Add `scripts/k6_load_test.sh` — tears down leftover state, starts compose, waits for `/health/ready`, runs k6, saves Markdown report to `docs/k6-load-report.md`, then tears down.
- [x] Add `load-test` target to `Makefile`.
- [x] Run and capture results: **2,570 iterations in 45 s · 34.3 iter/s · 68.7 req/s · p(95) 0.627 ms · 0.19% errors** (all thresholds ✅).
- [x] Save report to `docs/k6-load-report.md`.

## S9 — Published Docker Image (ghcr.io)

- [x] Add `.github/workflows/publish.yml` — builds `ghcr.io/vshy108/passengerresourcemanagementrust:latest` with `--features http,postgres` on every push to `main` using Docker Buildx + GHA cache.
- [x] Update `README.md` — document `docker run` usage for both PostgreSQL-backed and in-memory demo modes, and add published image row to the proof table.
- [x] Verify: `bash -n .github/workflows/publish.yml` (syntax OK).
