# Passenger Resource Management Rust Improvement Plan

This plan tracks future PRMS Rust slices. Keep changes vertical, spec-backed, and verifiable through public seams.

## S1 — API Example Collection

- [x] Add copy/paste HTTP examples for health, crew leads, passengers, resources, access, reports, audit, and reset.
- [x] Include required `Authorization: Bearer ...` headers and expected status codes.
- [x] Verify against `cargo run --features http --bin serve -- --api-keys ... --enable-reset`.

## S2 — Persistence Matrix

- [ ] Document which behavior is covered by in-memory, SQLite, and PostgreSQL modes.
- [ ] Add focused tests for mode-specific failure cases and startup readiness.
- [ ] Verify with: `cargo nextest run --all-features`.

## S3 — Observability Walkthrough

- [ ] Document request IDs, log formats, readiness, metrics, and audit verification as one operator path.
- [ ] Add smoke checks for `/health/ready`, `/metrics`, and `x-request-id` behavior if gaps remain.
- [ ] Verify with the HTTP feature test suite.

## S4 — Web Client Reviewer Flow

- [ ] Add a reviewer script for running the Rust API and React thin client together.
- [ ] Include seeded state, API key examples, and reset behavior.
- [ ] Verify with: `cd web && npm ci && npm run build`.

## S5 — Security Hardening Review

- [ ] Review rate limits, API key parsing, CORS configuration, body limits, and security headers as one slice.
- [ ] Add tests for any missing boundary cases before changing code.
- [ ] Verify with: `cargo clippy --all-targets --all-features -- -D warnings` and `cargo nextest run --features http`.
