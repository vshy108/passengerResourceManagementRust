# PRMS — Observability Walkthrough

An operator's path through all observability signals: structured logs,
request IDs, readiness, metrics, and audit-chain verification.

---

## 1. Start with structured logging

`tower_http::TraceLayer` emits one structured span per request with method,
URI, status, and latency. The span includes the `x-request-id` value so
every log line for a request carries the same correlation ID.

```sh
# Default: INFO-level spans only
cargo run --features http --bin serve -- --api-keys 'cl-aria:cl-aria'

# Verbose: include DEBUG from the PRMS crate and tower_http
RUST_LOG="passenger_resource_management=debug,tower_http=debug" \
  cargo run --features http --bin serve -- --api-keys 'cl-aria:cl-aria'
```

Typical log line (INFO):

```
2024-01-15T10:30:45.123456Z  INFO request{method=GET uri=/health/ready ...}: tower_http::trace: finished processing request status=200 latency=1ms
```

---

## 2. Request IDs (`x-request-id`)

Every response carries an `x-request-id` header. The middleware
(`SetRequestIdLayer` + `PropagateRequestIdLayer`) assigns a UUID if
the client did not send one, otherwise echoes the client-supplied value.

**Assign (server-generated):**

```sh
curl -v http://localhost:8080/health
# < x-request-id: 3f2504e0-4f89-11d3-9a0c-0305e82c3301
```

**Echo (client-supplied):**

```sh
curl -v http://localhost:8080/health -H 'x-request-id: my-trace-123'
# < x-request-id: my-trace-123
```

**Use in log correlation:** the span logged by `tower_http` includes the
request ID, so searching your log sink for `my-trace-123` returns all log
lines for that request.

Tests that verify this behavior: `request_id_is_assigned_and_propagated`,
`request_id_echoes_client_supplied_value` in `tests/http_health.rs`.

---

## 3. Readiness probe (`/health/ready`)

Returns entity counts and DB liveness. Use this as a Kubernetes
`readinessProbe` or a load-balancer health check.

```sh
curl http://localhost:8080/health/ready
```

Response (200 when ready):

```json
{
  "status": "ready",
  "version": "1.0.0",
  "crew_leads": 3,
  "passengers_active": 3,
  "resources_active": 3,
  "usage_events": 0,
  "admin_events": 6
}
```

503 when the database is unreachable (SQLite/PG only):

```json
{"error": "database unreachable", "code": "DatabaseUnreachable"}
```

In-memory mode: the DB check is skipped entirely (`entity_store = None`),
so `/health/ready` always succeeds as long as the process is running.

---

## 4. Prometheus metrics (`/metrics`)

Scraped in Prometheus text format. No auth required.

```sh
curl http://localhost:8080/metrics
```

Available metrics:

| Metric | Type | Description |
|---|---|---|
| `prms_crew_leads_total` | gauge | Active crew leads |
| `prms_passengers_active_total` | gauge | Active passengers |
| `prms_resources_active_total` | gauge | Active resources |
| `prms_usage_events_total` | counter | Total usage events recorded |
| `prms_usage_events_allowed_total` | counter | Usage events with `Allowed` outcome |
| `prms_usage_events_denied_total` | counter | Usage events with `Denied` outcome |
| `prms_admin_events_total` | counter | Total admin events recorded |

Example Prometheus scrape config:

```yaml
scrape_configs:
  - job_name: prms
    static_configs:
      - targets: ["localhost:8080"]
    metrics_path: /metrics
```

Tests: `metrics_returns_prometheus_text`, `metrics_counts_allowed_and_denied_after_access_events`
in `tests/http_health.rs`.

---

## 5. Audit trail verification

The admin event log is a hash-chained append-only trail of every crew-lead
mutation. Each event carries a SHA-256 hash of `(previous_hash, event_data)`.

**List admin events:**

```sh
curl http://localhost:8080/audit \
  -H 'Authorization: Bearer cl-aria'
```

Each event includes: `id`, `actor_id`, `target_id`, `action`,
`occurred_at`, and `hash` (hex SHA-256).

**Verify chain integrity:**

```sh
curl http://localhost:8080/audit/verify \
  -H 'Authorization: Bearer cl-aria'
# → {"valid": true, "event_count": 6}
```

If any event has been tampered with, `valid` is `false` and `broken_at`
identifies the first event ID where the chain diverges.

Note: `SQLite`-loaded events do not store hashes (written as empty strings).
The verifier skips these and reports `valid: true` — no false tampering
alerts after a restart.

---

## 6. Full observability smoke check

```sh
# 1. Start server
cargo run --features http --bin serve -- \
  --api-keys 'cl-aria:cl-aria,ps-001:ps-001' \
  --enable-reset

# 2. Liveness
curl http://localhost:8080/health

# 3. Readiness + entity counts
curl http://localhost:8080/health/ready

# 4. Metrics (check prms_passengers_active_total == 3)
curl http://localhost:8080/metrics | grep prms_passengers

# 5. Drive one allowed + one denied access event
curl -X POST http://localhost:8080/access \
  -H 'Authorization: Bearer ps-001' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id": "res-lounge"}'   # Silver → Silver: Allowed

curl -X POST http://localhost:8080/access \
  -H 'Authorization: Bearer ps-001' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id": "res-spa"}'      # Silver → Gold: Denied

# 6. Metrics after events
curl http://localhost:8080/metrics | grep prms_usage_events

# 7. Audit
curl http://localhost:8080/audit       -H 'Authorization: Bearer cl-aria'
curl http://localhost:8080/audit/verify -H 'Authorization: Bearer cl-aria'

# 8. Request-id correlation
curl -v http://localhost:8080/passengers -H 'Authorization: Bearer cl-aria' 2>&1 \
  | grep x-request-id
```

---

## Test coverage for this slice

All observability behaviors are covered by the automated test suite:

```sh
cargo nextest run --features http
```

Key tests:

| Test | Signal |
|---|---|
| `request_id_is_assigned_and_propagated` | UUID assigned when absent |
| `request_id_echoes_client_supplied_value` | Client-supplied ID echoed |
| `health_ready_returns_entity_counts` | Readiness counts (in-memory) |
| `sqlite_health_ready_exercises_ping_db` | Readiness + DB liveness (SQLite) |
| `metrics_returns_prometheus_text` | Prometheus text format |
| `metrics_counts_allowed_and_denied_after_access_events` | allowed/denied counters |
| `sqlite_audit_verify_covers_sqlite_skip_path` | Audit verify (SQLite skip path) |
