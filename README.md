# Spaceship X26 — Passenger Resource Management System (PRMS)

A small, layered Rust crate that models tier-based access to ship resources
(Silver / Gold / Platinum), with crew-lead-administered passengers and
resources, an audit trail of administrative changes, a usage-event log of
every access attempt, and reporting queries on top.

The project is spec-driven: every rule in [`specs/`](./specs) maps to one or
more tests. See [`AGENTS.md`](./AGENTS.md) for the house rules.

## Quick start (reviewers)

```bash
rustup show
cargo nextest run                  # core library + integration tests
cargo nextest run --features http  # adds the axum HTTP adapter test suite
```

That's the whole core path — no env vars, no services, no network.
If `cargo nextest` is not installed, run `cargo install cargo-nextest --locked`
or use `cargo test` / `cargo test --features http` as the built-in fallback.

## Review path

For a fast review, follow one vertical slice:

1. Read the access rules in [`specs/05-access.md`](./specs/05-access.md).
2. Open the matching integration tests in [`tests/access.rs`](./tests/access.rs).
3. Inspect the implementation in
  [`src/application/access_service.rs`](./src/application/access_service.rs).
4. Check the HTTP adapter in [`src/interface/http.rs`](./src/interface/http.rs)
  and DTOs in [`src/interface/dto.rs`](./src/interface/dto.rs).
5. Run the React thin client in [`web/`](./web) or read
  [`docs/code-review-qa.md`](./docs/code-review-qa.md) for review prep.

## Layout

```
src/
  domain/          # pure value objects, errors, events
  application/     # services + small port traits (Clock, event sinks)
  infrastructure/  # in-memory adapters, fake clock, event sinks
  interface/       # composition root + HTTP adapter (feature-gated)
  bin/             # binary entrypoints (serve)
specs/             # numbered rules, invariants, scenarios
tests/             # one integration file per spec slice
web/               # React thin client (all content fetched from Rust backend)
```

Dependency direction is inward only:
`interface → application → domain`, with `infrastructure` plugging into
`application` ports.

## Specs covered

| File                               | Slice              |
| ---------------------------------- | ------------------ |
| `specs/01-tier-policy.md`          | Tier ranking       |
| `specs/02-crew-lead.md`            | Crew lead registry |
| `specs/03-passenger.md`            | Passenger admin    |
| `specs/04-resource.md`             | Resource admin     |
| `specs/05-access.md`               | Access checks      |
| `specs/06-audit.md`                | Admin audit trail  |
| `specs/07-reporting.md`            | Reporting queries  |

## React thin client

A React thin client lives in [`web/`](./web). It fetches all content
from the Rust axum backend — no local TypeScript services or in-browser
state. See [`web/README.md`](./web/README.md) for run instructions.

## HTTP server (optional)

An axum-based HTTP adapter exposes the services over JSON. It is
feature-gated so the core test path stays dependency-free.

```bash
cargo run --features http --bin serve
# → PRMS HTTP server listening on http://127.0.0.1:8080
```

### API endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Liveness check |
| `GET` | `/openapi.json` | Full OpenAPI 3.x spec |
| `GET` | `/crew-leads` | List all Crew Leads |
| `POST` | `/crew-leads` | Add a Crew Lead (rejected if count already 3) |
| `PUT` | `/crew-leads/{id}` | Replace a Crew Lead (count stays 3) |
| `DELETE` | `/crew-leads/{id}` | Remove a Crew Lead (always rejected — use replace) |
| `GET` | `/passengers` | List active passengers |
| `POST` | `/passengers` | Create a passenger (Crew Lead only) |
| `DELETE` | `/passengers/{id}` | Soft-delete a passenger (Crew Lead only) |
| `PATCH` | `/passengers/{id}/tier` | Change a passenger's tier (Crew Lead only) |
| `GET` | `/resources` | List active resources |
| `POST` | `/resources` | Create a resource (Crew Lead only) |
| `DELETE` | `/resources/{id}` | Soft-delete a resource (Crew Lead only) |
| `PATCH` | `/resources/{id}/min-tier` | Change a resource's min tier (Crew Lead only) |
| `GET` | `/resources/accessible` | List resources accessible to the caller's tier |
| `POST` | `/access` | Attempt to use a resource (Passenger only) |
| `GET` | `/audit` | List all `AdminEvent`s |
| `GET` | `/usage` | List all `UsageEvent`s |
| `GET` | `/reports/by-tier` | Aggregate usage counts by passenger tier |
| `GET` | `/reports/top-resources` | Top-N resources by allowed-use count |
| `GET` | `/reports/personal-history/{id}` | Personal usage history for a passenger |
| `POST` | `/reset` | Reset in-memory state — only registered when `PRMS_ENABLE_RESET=true` |

State is in-process and resets on restart. Quick smoke test (requires `PRMS_API_KEYS` — see below):

```bash
curl http://127.0.0.1:8080/health
curl -H 'Authorization: Bearer cl-aria-token' http://127.0.0.1:8080/crew-leads
curl -X POST http://127.0.0.1:8080/access \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer ps-001-token' \
  -d '{"resource_id":"res-lounge"}'
```

Start the server with tokens mapped to actor IDs:

```bash
cargo run --features http --bin serve -- \
  --api-keys 'cl-aria-token:cl-aria,ps-001-token:ps-001' \
  --enable-reset
```

### Configuration

All flags also read from the matching environment variable:

| Flag                    | Env                        | Default            | Purpose                                              |
| ----------------------- | -------------------------- | ------------------ | ---------------------------------------------------- |
| `--bind`                | `PRMS_BIND`                | `127.0.0.1:8080`   | Listen address                                       |
| `--cors-origins`        | `PRMS_CORS_ORIGINS`        | _unset_ (`Any`)    | Comma-separated allow-list, e.g. `https://app.x26`   |
| `--enable-reset`        | `PRMS_ENABLE_RESET`        | `false`            | Register the `/reset` route (local dev only)         |
| `--shutdown-grace-secs` | `PRMS_SHUTDOWN_GRACE_SECS` | `10`               | Max seconds to drain in-flight requests after SIGINT |
| `--api-keys`            | `PRMS_API_KEYS`            | _unset_ (all 401)  | Comma-separated `token:actor-id` pairs, e.g. `tok1:cl-aria,tok2:ps-001` |
| `RUST_LOG`              | _(env only)_               | `info`             | tracing-subscriber filter                            |

Every response carries an `x-request-id` header (UUID-v4 if the client
did not supply one) so logs can be correlated. The full OpenAPI 3.1
spec is served at `GET /openapi.json` — point Swagger UI / Redoc /
Stoplight at it.

### Endpoints

Endpoint surface lives in [`src/interface/http.rs`](./src/interface/http.rs)
and the wire shapes in [`src/interface/dto.rs`](./src/interface/dto.rs).

| Method | Path                              | Purpose                              |
| ------ | --------------------------------- | ------------------------------------ |
| GET    | `/health`                         | liveness probe                       |
| GET    | `/health/ready`                   | readiness probe (returns 503 if not ready) |
| GET    | `/metrics`                        | Prometheus text metrics              |
| GET    | `/openapi.json`                   | OpenAPI 3.1 document                 |
| GET    | `/crew-leads`                     | list crew leads                      |
| POST   | `/crew-leads`                     | add crew lead (always 409, capped)   |
| PUT    | `/crew-leads/:old_id`             | replace crew lead                    |
| DELETE | `/crew-leads/:id`                 | remove crew lead (409 if at minimum) |
| GET    | `/passengers`                     | list active passengers               |
| POST   | `/passengers`                     | create passenger                     |
| GET    | `/passengers/:id`                 | fetch one (incl. deleted)            |
| PATCH  | `/passengers/:id/tier`            | change tier                          |
| DELETE | `/passengers/:id`                 | soft-delete                          |
| GET    | `/resources`                      | list active resources                |
| POST   | `/resources`                      | create resource                      |
| GET    | `/resources/accessible?tier=…`    | filter by tier                       |
| GET    | `/resources/:id`                  | fetch one                            |
| PATCH  | `/resources/:id/min-tier`         | change min tier                      |
| DELETE | `/resources/:id`                  | soft-delete                          |
| POST   | `/access`                         | attempt access                       |
| GET    | `/usage`                          | usage event log                      |
| GET    | `/audit`                          | admin event log                      |
| GET    | `/reports/by-tier`                | passenger count per tier             |
| GET    | `/reports/top-resources?n=…`      | top-N resources by allowed access    |
| GET    | `/reports/history/:passenger_id`  | personal history                     |
| POST   | `/reset`                          | reset world to seeded state          |

## Tooling

- Rust 2024, stable channel pinned in [`rust-toolchain.toml`](./rust-toolchain.toml)
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`
- Coverage: `cargo llvm-cov nextest --features http --ignore-filename-regex 'src/bin/'`
  — 98%+ line coverage; CI fails below that threshold. The `serve`
  binary entrypoint is excluded (it boots a real socket).
- CI: [`.github/workflows/ci.yml`](./.github/workflows/ci.yml) runs fmt, clippy
  (default + `--features http`), nextest (default + `--features http`), and
  the web build on every push and PR.

See [`AGENTS.md`](./AGENTS.md) for full contribution rules.

## Approach, tradeoffs, and assumptions

**Approach.** Spec-first, hexagonal layering with the dependency arrow
pointing inward (`interface → application → domain`, with
`infrastructure` plugged into `application` via traits). Every spec
file under [`specs/`](./specs) has numbered rules / scenarios; tests
are named after those scenario IDs (e.g. `tp_r1_s10_…`) so a reviewer
can map a failing test back to the rule it enforces. Red → green →
refactor; one commit per spec ID where practical.

**Domain purity.** `src/domain/` has `#![forbid(unsafe_code)]`, no
I/O, no clocks, no logging, and no third-party crates beyond
`thiserror`. IDs are string-backed newtypes (`PassengerId(String)`, …)
so the type system catches mix-ups at compile time. Errors are a single
`#[non_exhaustive]` `DomainError` enum — the compiler points at every
`match` site whenever a variant is added.

**Tradeoffs we made deliberately.**
- **In-memory state only.** Passengers, resources, and crew leads are
  service-owned in-memory collections; usage/admin events use in-memory
  sinks behind small port traits. Adding a real DB is the next adapter
  boundary, but we did not ship one because the brief did not ask for
  durability. See `src/infrastructure/`.
- **Token-based authentication.** Every request must carry
  `Authorization: Bearer <token>`. The server resolves the token to an
  `Actor` via `PRMS_API_KEYS` at startup — unknown tokens return 401.
  For local demo / E2E, pass `--api-keys token:actor-id,...` or the
  `PRMS_API_KEYS` env var. Tokens are never stored on disk; a real
  deployment would back this with a secrets manager.
- **`PartialOrd` via `Tier::rank()`.** We compare tiers through an
  explicit `rank()` rather than deriving `Ord` on the enum so the
  ordering is documented in code and stays stable if variants are
  reordered.
- **HTTP + React thin client.** The HTTP adapter is gated behind the
  `http` Cargo feature, and the React thin client lives in a separate
  `web/` project so the core crate stays dependency-light. CI runs both
  Rust feature configurations and the web build.
- **OpenAPI is generated, not handwritten.** `utoipa` derives the
  schema from the same DTOs the handlers use, so the spec cannot
  drift from the wire format.
- **Coverage gate at 98% lines** (`src/bin/serve.rs` excluded — it
  binds a real socket). Higher gates encouraged us to delete dead
  code rather than write tests for unreachable branches.

**Assumptions.**
- A passenger keeps the same tier between the moment of access and
  the moment the `UsageEvent` is recorded — we snapshot
  `tier_at_attempt` and `min_tier_at_attempt` on the event so audit
  history is stable even if tiers change later.
- Soft delete is sufficient for "remove" operations: passengers and
  resources are flagged inactive but never physically purged, so
  audit trails remain intact (`AGENTS.md` §3, "Every access attempt
  emits a `UsageEvent`").
- Time only flows forward; the injected `Clock` trait returns
  monotonic timestamps. Tests use `FakeClock` to advance time
  deterministically.
- IDs supplied by clients are treated as stable strings within a process
  lifetime. The HTTP layer validates JSON shape and tier enum values at
  the boundary, but it does not currently enforce an ID format beyond
  using distinct domain newtypes internally.
- A single-process deployment is acceptable for the demo; horizontal
  scaling (shared state, leader election) is out of scope.

## Limitations

The HTTP server is a demo affordance, not a production target:

- State is held in a single mutex around `World` — fine for the demo, will not
  scale beyond a handful of concurrent writers.
- `POST /reset` is gated by `PRMS_ENABLE_RESET=true` and "must be a
  known crew lead" — intended for local demo / test use only.
- CORS defaults to `Any` for dev convenience; set `PRMS_CORS_ORIGINS`
  before exposing the server beyond localhost.
- There is no durable storage, pagination, or stable event-ID sequence
  across restarts yet. Those are documented follow-ups in
  [`docs/review-readiness-checklist.md`](./docs/review-readiness-checklist.md).

## AI Usage Disclosure

GitHub Copilot in agent mode was used during development and review
preparation. It helped most with drafting specs, scaffolding tests,
boilerplate DTO/handler code, documentation polish, and the code-review
Q&A guide in [`docs/code-review-qa.md`](./docs/code-review-qa.md).

The domain rules, invariants, service boundaries, error mapping, and
trade-off decisions were reviewed manually against the specs. Suggestions
that added unnecessary infrastructure, hidden global state, broad rewrites,
or `unwrap()`/`expect()` on expected failure paths were rejected.

Verification was done independently by reading diffs, checking spec-to-test
traceability, running the Rust test/build commands and the web build in CI, and
reasoning through edge cases such as denied access, soft-delete, and audit
snapshot stability.
