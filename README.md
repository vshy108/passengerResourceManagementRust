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

## Layout

```
src/
  domain/          # pure value objects, errors, events
  application/     # services + port traits (PassengerRepo, …)
  infrastructure/  # in-memory adapters, fake clock, event sinks
  interface/       # composition root + HTTP adapter (feature-gated)
  bin/             # binary entrypoints (serve)
specs/             # numbered rules, invariants, scenarios
tests/             # one integration file per spec slice
web/               # optional React + TypeScript browser demo
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

## Web demo (optional)

A TypeScript port of the same services with a small React UI lives in
[`web/`](./web). See [`web/README.md`](./web/README.md) for run instructions.

## HTTP server (optional)

An axum-based HTTP adapter exposes the services over JSON. It is
feature-gated so the core test path stays dependency-free.

```bash
cargo run --features http --bin serve
# → PRMS HTTP server listening on http://127.0.0.1:8080
```

State is in-process and resets on restart. Quick smoke test:

```bash
curl http://127.0.0.1:8080/health
curl http://127.0.0.1:8080/crew-leads
curl -X POST http://127.0.0.1:8080/access \
  -H 'Content-Type: application/json' \
  -d '{"passenger_id":"ps-001","resource_id":"res-lounge"}'
```

### Configuration

All flags also read from the matching environment variable:

| Flag                    | Env                        | Default            | Purpose                                              |
| ----------------------- | -------------------------- | ------------------ | ---------------------------------------------------- |
| `--bind`                | `PRMS_BIND`                | `127.0.0.1:8080`   | Listen address                                       |
| `--cors-origins`        | `PRMS_CORS_ORIGINS`        | _unset_ (`Any`)    | Comma-separated allow-list, e.g. `https://app.x26`   |
| `--shutdown-grace-secs` | `PRMS_SHUTDOWN_GRACE_SECS` | `10`               | Max seconds to drain in-flight requests after SIGINT |
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
`thiserror` / `chrono`. IDs are newtypes (`PassengerId(Uuid)`, …) so
the type system catches mix-ups at compile time. Errors are a single
`#[non_exhaustive]` `DomainError` enum — the compiler points at every
`match` site whenever a variant is added.

**Tradeoffs we made deliberately.**
- **In-memory adapters only.** Persistence is a `Mutex<World>` behind
  the repository traits. Adding a real DB is a port-swap, not a
  rewrite, but we did not ship one because the brief did not ask for
  durability. See `src/infrastructure/`.
- **No authn/authz.** Admin endpoints accept `actor_id` at face value
  and verify only that it maps to a known crew lead. Real auth was
  out of scope; the `AccessPolicy` strategy makes adding it later a
  matter of inserting a guard at the boundary, not rewriting services.
- **`PartialOrd` via `Tier::rank()`.** We compare tiers through an
  explicit `rank()` rather than deriving `Ord` on the enum so the
  ordering is documented in code and stays stable if variants are
  reordered.
- **Two demo surfaces (CLI + HTTP + React).** The HTTP adapter and the
  web demo are gated behind the `http` Cargo feature and a separate
  `web/` workspace so the core crate stays dependency-light. CI runs
  both feature configurations.
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
- IDs supplied by clients are unique and stable within a process
  lifetime. The HTTP layer rejects malformed IDs at the boundary via
  `TryFrom`.
- A single-process deployment is acceptable for the demo; horizontal
  scaling (shared state, leader election) is out of scope.

## Limitations

The HTTP server is a demo affordance, not a production target:

- State is held in a single `Mutex<World>` — fine for the demo, will not
  scale beyond a handful of concurrent writers.
- All admin endpoints accept `actor_id` at face value; this crate ships
  no authentication layer (see [`AGENTS.md`](./AGENTS.md) §8).
- `POST /reset` is gated by "must be a known crew lead" but is still
  intended for local demo / test use only.
- The web client and the HTTP server keep **independent** in-process
  state — mutations in one are not visible in the other.
- CORS defaults to `Any` for dev convenience; set `PRMS_CORS_ORIGINS`
  before exposing the server beyond localhost.
