# PRMS — Persistence Matrix

Documents which behaviors are available and tested in each storage mode.

---

## Modes overview

The server selects a storage mode at startup based on environment variables
or CLI flags (priority: PostgreSQL → SQLite → in-memory):

| Mode | Activation | Data across restarts | Tests in CI |
|---|---|---|---|
| **In-memory** | default (no env vars) | No — fresh seeded world every boot | `cargo nextest run` |
| **SQLite** | `PRMS_DB_PATH=/path/to/file.db` | Yes — WAL-backed file | `cargo nextest run --features http` |
| **PostgreSQL** | `PRMS_PG_URL=postgres://...` | Yes — full ACID, pool | Requires live PG; skipped in CI |

---

## Behavior matrix

| Behavior | In-memory | SQLite | PostgreSQL |
|---|---|---|---|
| Crew leads persisted across restart | No | Yes | Yes |
| Passengers persisted across restart | No | Yes | Yes |
| Resources persisted across restart | No | Yes | Yes |
| Soft-deleted entities restored on restart | No | Yes | Yes |
| Optimistic-concurrency versions restored | No | Yes | Yes |
| Usage events persisted across restart | No | Yes | Yes |
| Admin events (audit trail) persisted | No | Yes | Yes |
| `/health/ready` DB liveness check | No (skipped) | Yes (`ping_db`) | Yes (`ping_db`) |
| Concurrent access (multi-handler) | Per-aggregate RwLock | Per-aggregate RwLock + WAL | Connection pool |
| Re-seeding on first boot | Always | Once (is_first_run) | Once (is_first_run) |
| Flush on mutation (flush_to_db) | No-op | Yes (sync_all) | Yes (async tasks) |

---

## Test coverage by mode

### In-memory

| Test | File |
|---|---|
| `health_ready_returns_entity_counts` | `tests/http_health.rs` |
| `metrics_returns_prometheus_text` | `tests/http_health.rs` |
| All `tests/http_*.rs` (non-sqlite) | multiple |

In-memory mode does not exercise `flush_to_db` or `ping_db` — those paths
are guarded by `if let Some(store) = state.world.entity_store` and skip
cleanly when `entity_store` is `None`.

### SQLite

| Test | File | What it covers |
|---|---|---|
| `entity_state_survives_restart_via_sqlite` | `tests/sqlite_persistence.rs` | First-run seeding + second-run restore; tier + soft-delete mutations survive |
| `sqlite_usage_sink_records_access_events` | `tests/sqlite_persistence.rs` | `UsageSink::Sqlite::append` + `list` |
| `sqlite_memory_db_always_seeds_a_fresh_world` | `tests/sqlite_persistence.rs` | `:memory:` always seeds fresh; in-memory connections are independent |
| `sqlite_backed_patch_triggers_flush_to_db` | `tests/http_sqlite.rs` | `flush_to_db` is called on PATCH mutation |
| `sqlite_backed_delete_triggers_flush_to_db` | `tests/http_sqlite.rs` | `flush_to_db` is called on DELETE mutation |
| `sqlite_health_ready_exercises_ping_db` | `tests/http_sqlite.rs` | `/health/ready` calls `ping_db()` and returns entity counts |
| `sqlite_audit_list_exercises_sqlite_snapshot` | `tests/http_sqlite.rs` | `AuditSink::Sqlite::snapshot` path |
| `sqlite_audit_verify_covers_sqlite_skip_path` | `tests/http_sqlite.rs` | `AuditSink::Sqlite::snapshot_with_hashes` + skip-empty-hash path |
| `router_convenience_wrapper_builds_working_app` | `tests/http_sqlite.rs` | `router()` convenience wrapper |

Run the full SQLite suite:

```sh
cargo nextest run --features http
```

### PostgreSQL

No automated tests: PostgreSQL requires a live connection pool (`PRMS_PG_URL`)
which is not available in CI without a sidecar container. The implementation
mirrors the SQLite store (`pg_store.rs`) and is gated behind `--features postgres`.

Repeatable smoke test (starts a temporary PostgreSQL container, runs the HTTP server
with `--features postgres`, writes a passenger, records allowed and denied access,
checks `/health/ready`, and verifies the audit chain):

```sh
scripts/postgres-smoke.sh
```

Manual smoke test (requires a running PostgreSQL instance):

```sh
PRMS_PG_URL="postgres://prms:prms@localhost:5432/prms" \
  cargo run --features postgres,http --bin serve -- \
    --api-keys 'cl-aria:cl-aria' \
    --enable-reset
```

---

## `:memory:` SQLite — a note on testing

`build_world_with_sqlite(":memory:")` is useful for exercising the SQLite
adapter code paths in tests without touching the filesystem. However, SQLite
in-memory databases are connection-local: each call to `open_db(":memory:")`
opens a fresh, empty schema. Therefore:

- `is_first_run()` is always `true` — the demo world is always re-seeded.
- Mutations written in one world instance are **not** visible to any other
  world built from `":memory:"`.
- Use a temp-file path (as `entity_state_survives_restart_via_sqlite` does)
  when you need to verify persistence across a simulated restart.
