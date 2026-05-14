# PRMS — HTTP API Examples

Copy-paste `curl` examples for every endpoint. All commands assume the server
is running on `http://localhost:8080`.

---

## 1. Start the server

```sh
# Using dev.env (recommended for local work)
env $(grep -v '^#' dev.env | xargs) \
  cargo run --features http --bin serve -- --enable-reset

# Or inline — token format is  token:actor-id  (comma-separated pairs)
cargo run --features http --bin serve -- \
  --api-keys 'cl-aria:cl-aria,cl-noor:cl-noor,cl-jun:cl-jun,ps-001:ps-001' \
  --enable-reset
```

Bearer tokens from the demo world (token == actor-id):

| Token | Actor | Role |
|---|---|---|
| `cl-aria` | `cl-aria` (Aria Vega) | Crew Lead |
| `cl-noor` | `cl-noor` (Noor Hadid) | Crew Lead |
| `cl-jun` | `cl-jun` (Jun Park) | Crew Lead |
| `ps-001` | `ps-001` (Mira Voss, Silver) | Passenger |
| `ps-002` | `ps-002` (Kai Reeves, Gold) | Passenger |
| `ps-003` | `ps-003` (Lena Ito, Platinum) | Passenger |

---

## 2. Health

```sh
# Basic liveness — no auth required
curl http://localhost:8080/health
# → 200  ok

# Readiness — entity counts + DB liveness
curl http://localhost:8080/health/ready
# → 200
# {
#   "status": "ready",
#   "version": "1.0.0",
#   "crew_leads": 3,
#   "passengers_active": 3,
#   "resources_active": 3,
#   "usage_events": 0,
#   "admin_events": 6
# }
```

---

## 3. Auth check

```sh
# Verify a bearer token and see which actor it resolves to
curl http://localhost:8080/auth/check \
  -H 'Authorization: Bearer cl-aria'
# → 200  {"actor_id": "cl-aria"}

# Missing / unknown token
curl http://localhost:8080/auth/check
# → 401  {"error": "missing or invalid bearer token", "code": "Unauthorized"}
```

---

## 4. Crew leads

```sh
# List all crew leads
curl http://localhost:8080/crew-leads \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{"id":"cl-aria","name":"Aria Vega"}, ...]

# List with pagination
curl 'http://localhost:8080/crew-leads?limit=2&offset=0' \
  -H 'Authorization: Bearer cl-aria'

# POST /crew-leads — always 409 (CL-R2: cap of 3 is full at boot; use PUT to rotate)
curl -X POST http://localhost:8080/crew-leads \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -d '{"lead": {"id": "cl-nova", "name": "Nova Chen"}}'
# → 409  {"error": "crew lead limit reached", "code": "CrewLeadLimitReached"}

# Replace a crew lead (swap old-id for a new record — the only way to change crew leads)
curl -X PUT http://localhost:8080/crew-leads/cl-jun \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -d '{"new_lead": {"id": "cl-jun", "name": "Jun Park-Updated"}}'
# → 204

# DELETE /crew-leads/{id} — always 409 (CL-R3: removal would breach exactly-3 invariant)
curl -X DELETE http://localhost:8080/crew-leads/cl-jun \
  -H 'Authorization: Bearer cl-aria'
# → 409  {"error": "crew lead minimum breached", "code": "CrewLeadMinimumBreached"}
```

---

## 5. Passengers

```sh
# List active passengers
curl http://localhost:8080/passengers \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{"id":"ps-001","name":"Mira Voss","tier":"Silver","deleted_at":null,"version":1}, ...]

# Include soft-deleted records
curl 'http://localhost:8080/passengers?include_deleted=true' \
  -H 'Authorization: Bearer cl-aria'

# Get one passenger
curl http://localhost:8080/passengers/ps-001 \
  -H 'Authorization: Bearer cl-aria'
# → 200  {"id":"ps-001","name":"Mira Voss","tier":"Silver","deleted_at":null,"version":1}

# Create a passenger  (Idempotency-Key makes the POST safe to retry)
curl -X POST http://localhost:8080/passengers \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: create-ps-neo-v1' \
  -d '{"id": "ps-neo", "name": "Neo Park", "tier": "Gold"}'
# → 201  {"id":"ps-neo","name":"Neo Park","tier":"Gold","deleted_at":null,"version":1}

# Change tier  (If-Match prevents lost-update races — use version from GET response)
curl -X PATCH http://localhost:8080/passengers/ps-001/tier \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -H 'If-Match: "1"' \
  -d '{"tier": "Platinum"}'
# → 204

# Soft-delete a passenger
curl -X DELETE http://localhost:8080/passengers/ps-neo \
  -H 'Authorization: Bearer cl-aria' \
  -H 'If-Match: "1"'
# → 204

# Error: wrong If-Match version
curl -X PATCH http://localhost:8080/passengers/ps-001/tier \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -H 'If-Match: "99"' \
  -d '{"tier": "Gold"}'
# → 412  {"error": "version conflict ...", "code": "VersionConflict"}
```

---

## 6. Resources

```sh
# List active resources
curl http://localhost:8080/resources \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{"id":"res-lounge","name":"Stardeck Lounge","category":"social","min_tier":"Silver","deleted_at":null,"version":1}, ...]

# List resources accessible to a given tier
curl 'http://localhost:8080/resources/accessible?tier=Gold' \
  -H 'Authorization: Bearer cl-aria'
# → 200  [res-lounge (Silver), res-spa (Gold)]  — Platinum excluded

# Get one resource
curl http://localhost:8080/resources/res-spa \
  -H 'Authorization: Bearer cl-aria'

# Create a resource
curl -X POST http://localhost:8080/resources \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: create-res-gym-v1' \
  -d '{"id": "res-gym", "name": "Gravity Gym", "category": "fitness", "min_tier": "Silver"}'
# → 201

# Change minimum tier
curl -X PATCH http://localhost:8080/resources/res-gym/min-tier \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -H 'If-Match: "1"' \
  -d '{"tier": "Gold"}'
# → 204

# Soft-delete a resource
curl -X DELETE http://localhost:8080/resources/res-gym \
  -H 'Authorization: Bearer cl-aria' \
  -H 'If-Match: "2"'
# → 204
```

---

## 7. Access (use a resource)

The passenger's token identifies them as the actor.

```sh
# Allowed: Gold passenger (ps-002) accesses Gold resource (res-spa)
curl -X POST http://localhost:8080/access \
  -H 'Authorization: Bearer ps-002' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id": "res-spa"}'
# → 200
# {
#   "id": "evt-001",
#   "passenger_id": "ps-002",
#   "resource_id": "res-spa",
#   "outcome": "Allowed",
#   "tier_at_attempt": "Gold",
#   "min_tier_at_attempt": "Gold",
#   "occurred_at": 1000
# }

# Denied: Silver passenger (ps-001) tries Gold resource (res-spa)
curl -X POST http://localhost:8080/access \
  -H 'Authorization: Bearer ps-001' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id": "res-spa"}'
# → 403  {"error": "access denied", "code": "AccessDenied"}

# Crew lead cannot use resources (wrong actor type)
curl -X POST http://localhost:8080/access \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id": "res-lounge"}'
# → 403  {"error": "unauthorized actor", "code": "UnauthorizedActor"}
```

---

## 8. Usage events

```sh
# List all usage events (paginated)
curl 'http://localhost:8080/usage?limit=20&offset=0' \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{...}, ...]
```

---

## 9. Reports

```sh
# Access counts grouped by passenger tier
curl http://localhost:8080/reports/by-tier \
  -H 'Authorization: Bearer cl-aria'
# → 200
# {
#   "Silver":   {"allowed": 0, "denied": 1},
#   "Gold":     {"allowed": 1, "denied": 0},
#   "Diamond":  {"allowed": 0, "denied": 0},
#   "Platinum": {"allowed": 0, "denied": 0}
# }

# Top N most-accessed resources (default n=5, max 1000)
curl 'http://localhost:8080/reports/top-resources?n=3' \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{"resource_id":"res-spa","allowed_count":1}]

# Personal access history for one passenger
curl http://localhost:8080/reports/history/ps-002 \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{...usage events for ps-002...}]
```

---

## 10. Audit trail

```sh
# List admin events (crew lead mutations, tier changes, etc.)
curl 'http://localhost:8080/audit?limit=10' \
  -H 'Authorization: Bearer cl-aria'
# → 200  [{...}, ...]

# Verify audit chain integrity (hash chain validation)
curl http://localhost:8080/audit/verify \
  -H 'Authorization: Bearer cl-aria'
# → 200  {"valid": true, "event_count": 6}
```

---

## 11. Metrics

Prometheus text format — no auth required.

```sh
curl http://localhost:8080/metrics
# → 200
# # HELP prms_crew_leads_total Active crew leads.
# prms_crew_leads_total 3
# # HELP prms_passengers_active_total Active passengers.
# prms_passengers_active_total 3
# ...
```

---

## 12. OpenAPI schema

```sh
curl http://localhost:8080/openapi.json | python3 -m json.tool | head -40
```

---

## 13. Reset (demo / test only)

Requires `--enable-reset` flag at startup. **Never enable in production.**

```sh
curl -X POST http://localhost:8080/reset \
  -H 'Authorization: Bearer cl-aria'
# → 204  (world restored to seeded demo state)
```

---

## Error response shape

All error responses share the same envelope:

```json
{
  "error": "human-readable message",
  "code":  "MachinePascalCaseCode"
}
```

Common status → code mappings:

| HTTP | Code | Cause |
|---|---|---|
| 400 | `InvalidInput` | Validation failure at the boundary |
| 401 | `Unauthorized` | Missing or unknown bearer token |
| 403 | `UnauthorizedActor` / `AccessDenied` | Wrong actor type or tier too low |
| 404 | `PassengerNotFound` / `ResourceNotFound` / `CrewLeadNotFound` | Entity does not exist |
| 409 | `PassengerAlreadyExists` / `CrewLeadLimitReached` / … | Conflict |
| 412 | `VersionConflict` | `If-Match` version mismatch (optimistic concurrency) |
| 429 | — | Rate limit exceeded (tower-governor, per IP) |
| 503 | `InternalError` | DB unreachable or lock poisoned |
