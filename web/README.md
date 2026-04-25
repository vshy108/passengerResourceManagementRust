# PRMS Web Demo

A browser-only demo of the Spaceship X26 Passenger Resource Management
System. The TypeScript services under `src/services/` mirror the Rust
specs in [`../specs/`](../specs/) one-for-one — same rule IDs, same
errors, same audit semantics.

It runs entirely in the browser; refresh the page to reset state.

## Quick start

```bash
cd web
npm install
npm run dev
```

Then open the URL Vite prints (defaults to <http://localhost:5173>).

## What you can do

| Panel       | Spec    | Demos                                                                                |
| ----------- | ------- | ------------------------------------------------------------------------------------ |
| Crew Leads  | CL      | seeded with 3 leads; rotate one via `replace`. `add`/`remove` are forbidden.         |
| Tier matrix | TP      | TP-R2 access matrix rendered live.                                                   |
| Passengers  | PS      | create / change tier / soft delete (Crew-Lead-only).                                 |
| Resources   | RS      | create / change min tier / soft delete (Crew-Lead-only).                             |
| Access      | AC      | passenger acts on themselves; every attempt is recorded with tier-at-time (RP-R3).   |
| Reports     | RP      | aggregate by tier, top-N resources, personal history.                                |
| Audit log   | AU      | one `AdminEvent` per admin mutation (bootstrap, replace, create/change/soft-delete). |

## Layout

```
web/
├── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── src/
    ├── domain/        # Tier, IDs, Actor, errors, value-object types
    ├── services/      # CrewLead/Passenger/Resource/Access/Reporting + clock + composition root
    ├── components/    # one panel per feature area
    ├── state/store.tsx
    ├── App.tsx
    ├── main.tsx
    └── styles.css
```

## Why not call the Rust code?

The Rust crate is a pure library (no HTTP server, no wasm bindings yet —
that would expand scope per `AGENTS.md` §8). To keep this demo
zero-backend the services are reimplemented in TypeScript against the
same specs. The Rust test suite (`cargo test`) is the source of truth;
this UI is a visual cross-check.

## Live Rust server (optional)

The bottom panel ("Live Rust server (HTTP)") talks to the optional axum
adapter. Start it from the repo root in a second terminal:

```bash
cargo run --features http --bin serve
# → PRMS HTTP server listening on http://127.0.0.1:8080
```

The panel auto-pings `GET /health`, then loads passengers, resources,
usage events and the by-tier report from the running server. The
"POST /access" button drives the live `AccessService`. Override the
base URL with `VITE_API_BASE=http://host:port npm run dev`.
