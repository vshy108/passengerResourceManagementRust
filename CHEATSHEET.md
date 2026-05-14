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
cargo run --features http --bin serve -- \
  --api-keys 'cl-aria-token:cl-aria,ps-001-token:ps-001' \
  --enable-reset
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

| Flag | Env | Purpose |
|---|---|---|
| `--bind` | `PRMS_BIND` | Listen address |
| `--cors-origins` | `PRMS_CORS_ORIGINS` | CORS allow-list |
| `--enable-reset` | `PRMS_ENABLE_RESET` | Register `/reset` |
| `--api-keys` | `PRMS_API_KEYS` | `token:actor-id` pairs |
| `--db-path` | `PRMS_DB_PATH` | SQLite persistence path |
| `--pg-url` | `PRMS_PG_URL` | PostgreSQL URL with `postgres` feature |
| `--log-format` | `PRMS_LOG_FORMAT` | `text` or `json` logs |

## Review Gates

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo nextest run --all-features
cd web && npm ci && npm run build
```
