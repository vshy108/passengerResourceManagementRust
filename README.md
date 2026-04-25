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
cargo nextest run     # or: cargo test
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

Endpoint surface lives in [`src/interface/http.rs`](./src/interface/http.rs)
and the wire shapes in [`src/interface/dto.rs`](./src/interface/dto.rs).
CORS is open by default so the React demo can call it directly.

## Tooling

- Rust 2024, stable channel pinned in [`rust-toolchain.toml`](./rust-toolchain.toml)
- `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`
- Coverage: `cargo llvm-cov nextest`

See [`AGENTS.md`](./AGENTS.md) for full contribution rules.
