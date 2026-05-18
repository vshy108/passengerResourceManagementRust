# OpenAPI Client Evidence - 2026-05-18

This note records a fresh generated-client validation pass for the PRMS Rust API and React TypeScript client.

## Goal

Prove the backend OpenAPI contract and generated frontend client stay type-aligned without manual request shapes.

## Commands Run

Started the API from the repo root:

```sh
env $(grep -v '^#' dev.env | xargs) cargo run --features http --bin serve -- --enable-reset
```

Verified readiness:

```sh
curl -fsS http://127.0.0.1:8080/health/ready
```

Regenerated and validated the web client from `web/`:

```sh
npm run generate:types
npm run build
npm run lint
npm run typecheck
```

## Results

| Check | Result | Evidence |
|-------|--------|----------|
| API readiness | Passed | `/health/ready` returned `status=ready`, version `1.0.0`, and seeded counts |
| Type generation | Passed | `openapi-typescript` generated `src/services/openapi.generated.ts` from `/openapi.json` |
| Generated client diff | Clean | Regeneration produced no git diff, so the committed client is current |
| Web build | Passed | `npm run build` completed successfully |
| Web lint | Passed | `npm run lint` completed successfully |
| Web typecheck | Passed | `npm run typecheck` completed successfully |

## Reviewer Signal

The Rust OpenAPI document and the checked-in TypeScript client are aligned. A reviewer can regenerate the client from the running API and get the same committed file, then build and typecheck the React client successfully.

Next useful slice: add a tiny contract-refresh script that starts the API, regenerates types, runs the web checks, and fails if `web/src/services/openapi.generated.ts` changes unexpectedly.