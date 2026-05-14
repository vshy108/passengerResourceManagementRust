# PRMS — Web Client Reviewer Flow

Step-by-step guide to running the Rust API and the React thin client
side-by-side, with seeded state, API key examples, and reset behavior.

---

## Prerequisites

- Rust toolchain (`rustup show` to verify)
- Node.js ≥ 20 (`node --version` to verify; `.nvmrc` pins the version)

---

## Step 1 — Start the Rust backend

In one terminal, from the **repo root**:

```sh
# Use the dev.env file for a one-liner:
env $(grep -v '^#' dev.env | xargs) \
  cargo run --features http --bin serve -- --enable-reset

# Or inline (same settings):
cargo run --features http --bin serve -- \
  --api-keys 'cl-aria:cl-aria,cl-noor:cl-noor,cl-jun:cl-jun,ps-001:ps-001,ps-002:ps-002,ps-003:ps-003' \
  --cors-origins 'http://localhost:5173' \
  --enable-reset
```

The server prints: `PRMS HTTP server listening on http://127.0.0.1:8080`

Verify it's up:

```sh
curl http://127.0.0.1:8080/health/ready
# → {"status":"ready","crew_leads":3,"passengers_active":3,"resources_active":3,...}
```

---

## Step 2 — Start the React thin client

In a second terminal, from the **`web/`** directory:

```sh
cd web
npm ci       # install locked deps from package-lock.json
npm run dev  # starts Vite dev server on http://localhost:5173
```

Open `http://localhost:5173` in a browser.

The page has two independent areas:

| Area | Data source |
|---|---|
| **In-browser panels** (Crew Leads, Passengers, Resources, Access, Reports, Audit) | TypeScript services running fully in the browser |
| **Live Rust server panel** (bottom of page) | Calls `GET /passengers`, `/resources`, `/usage`, `/reports/by-tier`, `POST /access`, `POST /reset` on the Rust backend |

---

## Step 3 — Seeded demo state

Both areas start with the same demo entities:

| Type | ID | Name | Tier / Role |
|---|---|---|---|
| Crew lead | `cl-aria` | Aria Vega | Crew Lead |
| Crew lead | `cl-noor` | Noor Hadid | Crew Lead |
| Crew lead | `cl-jun` | Jun Park | Crew Lead |
| Passenger | `ps-001` | Mira Voss | Silver |
| Passenger | `ps-002` | Kai Reeves | Gold |
| Passenger | `ps-003` | Lena Ito | Platinum |
| Resource | `res-lounge` | Stardeck Lounge | min Silver |
| Resource | `res-spa` | Zero-G Spa | min Gold |
| Resource | `res-bridge` | Bridge Tour | min Platinum |

API keys (token == actor-id in demo mode):

| Bearer token | Actor |
|---|---|
| `cl-aria` | Crew lead Aria Vega |
| `ps-001` | Passenger Mira Voss (Silver) |
| `ps-002` | Passenger Kai Reeves (Gold) |
| `ps-003` | Passenger Lena Ito (Platinum) |

---

## Step 4 — Review the Live Rust server panel

1. The panel auto-pings `GET /health` on load.
2. Click **Load data** to fetch passengers, resources, usage events, and the
   by-tier report.
3. Use the **"POST /access"** button to drive access events. The panel shows
   the outcome (Allowed / Denied).
4. Use **"Accessible resources for tier"** to see which resources a given
   tier can reach.
5. Click **"Reset server state"** to restore the seeded demo world via
   `POST /reset`. Reload the page to confirm all counts return to 3/3/3.

---

## Step 5 — Build check

Verify the TypeScript build is clean before reviewing the source:

```sh
cd web
npm run build     # tsc -b && vite build → dist/
npm run lint      # eslint
npm run typecheck # tsc --noEmit
```

---

## Reset behavior

`POST /reset` rebuilds the world from the same seed as startup:

```sh
curl -X POST http://127.0.0.1:8080/reset \
  -H 'Authorization: Bearer cl-aria'
# → 204

# Confirm counts are back to 3/3/3
curl http://127.0.0.1:8080/health/ready
```

The in-browser TypeScript panels are independent of the Rust backend — they
have their own in-memory state and are not affected by `/reset`.

---

## OpenAPI type generation (optional)

If you change DTOs in the Rust backend and want to regenerate the TypeScript
types used by the live panel:

```sh
# Requires the Rust server to be running on port 8080
cd web
npm run generate:types
# → src/services/openapi.generated.ts (overwritten)
```
