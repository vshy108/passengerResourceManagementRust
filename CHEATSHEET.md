# Passenger Resource Management Rust Cheatsheet

## Commands

```sh
rustup show
cargo nextest run
cargo nextest run --features http
cargo test
cargo test --features http
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo llvm-cov nextest --features http --ignore-filename-regex 'src/bin/'
```

Run the HTTP server:

```sh
# Recommended: use dev.env (sets API keys, disables rate limiting, sets CORS)
env $(grep -v '^#' dev.env | xargs) \
  cargo run --features http --bin serve -- --enable-reset

# Or inline:
cargo run --features http --bin serve -- \
  --api-keys 'cl-aria:cl-aria,ps-001:ps-001' \
  --enable-reset \
  --enable-rate-limit=false
```

## Domain Rules

- Crew Leads administer passengers and resources.
- Passenger tiers order as `Silver < Gold < Diamond < Platinum`.
- A passenger can use a resource when their tier meets or exceeds the resource minimum.
- Every access attempt emits a usage event.
- Admin mutations emit admin events.
- Domain code has no I/O, clocks, logging, or unsafe code.

## Architecture Boundary

```text
interface -> application -> domain
                 ^
           infrastructure
```

- `src/domain/`: pure value objects, errors, and events.
- `src/application/`: services and port traits.
- `src/infrastructure/`: in-memory adapters, persistence, clocks, event sinks.
- `src/interface/`: composition root and feature-gated HTTP adapter.
- `web/`: React thin client driven by the Rust backend.

## HTTP Flags

| Flag | Env | Default | Purpose |
|---|---|---|---|
| `--bind` | `PRMS_BIND` | `127.0.0.1:8080` | Listen address |
| `--cors-origins` | `PRMS_CORS_ORIGINS` | any | CORS allow-list |
| `--enable-reset` | `PRMS_ENABLE_RESET` | `false` | Register `/reset` |
| `--api-keys` | `PRMS_API_KEYS` | — (all 401) | `token:actor-id` pairs |
| `--db-path` | `PRMS_DB_PATH` | — (in-memory) | SQLite persistence path |
| `--pg-url` | `PRMS_PG_URL` | — | PostgreSQL URL (`postgres` feature) |
| `--enable-rate-limit` | `PRMS_ENABLE_RATE_LIMIT` | `true` | Per-IP token-bucket rate limiting |
| `--rate-limit-rps` | `PRMS_RATE_LIMIT_RPS` | `10` | Tokens replenished per second per IP |
| `--rate-limit-burst` | `PRMS_RATE_LIMIT_BURST` | `50` | Initial token burst per IP |
| `--shutdown-grace-secs` | `PRMS_SHUTDOWN_GRACE_SECS` | `10` | Drain timeout after SIGINT |
| `--log-format` | `PRMS_LOG_FORMAT` | `text` | `text` or `json` structured logs |

## Review Gates

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cd web && npm ci && npm run build
```

## Reference Docs

| Doc | What it covers |
|---|---|
| [`docs/api-examples.md`](docs/api-examples.md) | Copy-paste `curl` for every endpoint |
| [`docs/persistence-matrix.md`](docs/persistence-matrix.md) | In-memory vs SQLite vs PostgreSQL |
| [`docs/observability.md`](docs/observability.md) | Logs, request-id, metrics, audit |
| [`docs/security-review.md`](docs/security-review.md) | Rate limits, CORS, headers, auth |
| [`docs/web-reviewer-flow.md`](docs/web-reviewer-flow.md) | Rust API + React thin client together |
| [`docs/code-review-qa.md`](docs/code-review-qa.md) | Design Q&A for code reviewers |
