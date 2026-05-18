# Reviewer Flow Evidence - 2026-05-18

This note records a fresh local validation pass for the reviewer-facing production flow.

## Goal

A reviewer can open the repo, run the documented commands, hit the health/API flow, and understand the current production-readiness signal without asking for extra context.

## Commands Run

From the repo root:

```sh
make test
make test-http
scripts/postgres-smoke.sh
```

From `web/`:

```sh
npm run build
npm run lint
npm run typecheck
```

## Results

| Check | Result | Evidence |
|-------|--------|----------|
| Core Rust tests | Passed | `make test` completed successfully |
| Rust HTTP adapter tests | Passed | `make test-http` completed successfully |
| PostgreSQL smoke | Passed | Temporary Postgres-backed API passed readiness, write, audit, and health checks |
| React production build | Passed | Vite built `dist/` successfully |
| React lint | Passed | `eslint .` completed successfully |
| React typecheck | Passed | `tsc -b --noEmit` completed successfully |

## PostgreSQL Smoke Snapshot

```text
PostgreSQL smoke passed in 12s
ready_before={"status":"ready","version":"1.0.0","crew_leads":3,"passengers_active":3,"resources_active":3,"usage_events":0,"admin_events":7}
ready_after={"status":"ready","version":"1.0.0","crew_leads":3,"passengers_active":4,"resources_active":3,"usage_events":2,"admin_events":8}
audit_verify={"valid":true,"length":8,"broken_at":null}
```

## Reviewer Signal

The flow proves the Rust API, Postgres persistence path, readiness endpoint, audit verification, and React thin client still validate together on the current machine.

Next useful slice: add a small reviewer smoke script that starts the backend, exercises the documented API examples, and writes a repeatable Markdown transcript under `docs/`.