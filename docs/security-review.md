# PRMS — Security Hardening Review

Covers rate limiting, API key parsing, CORS, body limits, and security
response headers. Every finding includes the relevant OWASP Top 10 category,
the implementation location, and the test that verifies the behavior.

Latest local evidence: [`security-hardening-evidence-2026-05-18.md`](./security-hardening-evidence-2026-05-18.md).

---

## Summary

All security controls are implemented and tested. No missing boundary cases
were found during this review. The table below maps each control to its OWASP
category and test.

| Control | OWASP | Location | Test |
|---|---|---|---|
| API key constant-time comparison | A07 | `http.rs AuthActor::from_request_parts` | `unauthorized_request_returns_typed_error_code` |
| 401 on missing / unknown token | A07 | `http.rs AuthActor` | `create_passenger_missing_token_returns_401` |
| 403 on wrong actor type | A01 | domain `UnauthorizedActor` → `map_err` | `http_admin.rs` non-crew-lead token tests |
| 64 KiB body cap | A04 | `DefaultBodyLimit::max(64 * 1024)` | `oversized_request_body_returns_413` |
| Per-IP token-bucket rate limit | A04 | `GovernorLayer` (opt-in) | `rate_limit_returns_429_after_burst_exhausted` |
| `X-Content-Type-Options: nosniff` | A05 | `SetResponseHeaderLayer` | `security_response_headers_are_set` |
| `X-Frame-Options: DENY` | A05 | `SetResponseHeaderLayer` | `security_response_headers_are_set` |
| `Referrer-Policy: no-referrer` | A05 | `SetResponseHeaderLayer` | `security_response_headers_are_set` |
| `Content-Security-Policy: default-src 'none'` | A05 | `SetResponseHeaderLayer` | `security_response_headers_are_set` |
| CORS origin restriction | A05 | `CorsLayer` with `CorsOrigins::List` | `cors_list_origins_allows_listed_origin` |
| CORS any-origin (dev default) | — | `CorsOrigins::Any` | `http_health.rs` base tests |
| Pagination caps (offset/limit) | A04 | `PaginationQuery`, `TopNQuery` boundary | implicit in pagination tests |
| `/reset` route opt-in only | A05 | `enable_reset` flag, not registered by default | `reset_not_registered_when_disabled` |

---

## API key parsing

**Implementation:** `src/bin/serve.rs` parses `--api-keys` as a
comma-separated list of `token:actor-id` pairs. Keys and actor IDs are split
on the first `:` only, so actor IDs may contain colons if needed. Empty
tokens and empty actor IDs are rejected at startup.

**Constant-time comparison:** `http.rs AuthActor::from_request_parts` uses
`subtle::ConstantTimeEq` in a linear scan over all keys. The scan always
visits every key (never short-circuits on match) to prevent timing-based
enumeration of which token prefix matches (OWASP A07).

**Token scope in idempotency cache:** each idempotency key is prefixed with
`actor_id:` (e.g. `cl-aria:create-ps-001`) so two actors sharing the same
`Idempotency-Key` string cannot collide and receive each other's cached
responses.

---

## Rate limiting

Rate limiting is **enabled by default** (`PRMS_ENABLE_RATE_LIMIT=true`).
Set it to `false` for local dev or integration tests where all requests share
the loopback IP and would exhaust the token bucket immediately:

```sh
# Local dev — disable rate limiting
PRMS_ENABLE_RATE_LIMIT=false cargo run --features http --bin serve -- \
  --api-keys '...' \
  --enable-reset

# Or via dev.env (already sets PRMS_ENABLE_RATE_LIMIT=false)
env $(grep -v '^#' dev.env | xargs) cargo run --features http --bin serve -- --enable-reset

# Production — keep the default (or tune thresholds)
cargo run --features http --bin serve -- \
  --api-keys '...' \
  --rate-limit-rps 10 \
  --rate-limit-burst 50
```

429 response shape:

```
HTTP/1.1 429 Too Many Requests
# (empty body — tower-governor's default)
```

---

## CORS

| Setting | Dev (default) | Production |
|---|---|---|
| `--cors-origins` / `PRMS_CORS_ORIGINS` | unset | comma-separated list |
| `CorsOrigins` variant | `Any` | `List(Vec<HeaderValue>)` |
| `allow_origin` | `Any` (all origins) | explicit list |

Dev.env example:

```sh
PRMS_CORS_ORIGINS="http://localhost:5173"
```

Multi-origin production example:

```sh
--cors-origins 'https://app.example.com,https://admin.example.com'
```

---

## Body limit

`DefaultBodyLimit::max(64 * 1024)` (64 KiB) is applied globally on the axum
router. Every request DTO in this application is tiny (< 1 KiB), so the limit
is a safe defence against accidental or malicious oversized payloads
(OWASP A04).

The limit returns `413 Payload Too Large` before the handler is invoked, so
there is no risk of OOM from a large body.

---

## Security response headers

All four headers are injected via `SetResponseHeaderLayer::if_not_present`,
which preserves any value the handler already set (e.g. `Content-Type`):

| Header | Value | Purpose |
|---|---|---|
| `X-Content-Type-Options` | `nosniff` | Prevents MIME-type sniffing |
| `X-Frame-Options` | `DENY` | Blocks clickjacking via iframes |
| `Referrer-Policy` | `no-referrer` | Suppresses referrer leakage |
| `Content-Security-Policy` | `default-src 'none'` | Blocks all content for JSON-only API |

---

## `/reset` endpoint

`POST /reset` is only registered when `--enable-reset` / `PRMS_ENABLE_RESET`
is set. The route does not exist at all in the default binary:

```sh
# Without --enable-reset:
curl -X POST http://localhost:8080/reset -H 'Authorization: Bearer cl-aria'
# → 404  (route not registered)
```

Never enable in production. Guarded by the `enable_reset` boolean in
`router_with()`.

---

## Verify all security controls

```sh
# Lint and clippy (all features)
cargo clippy --all-targets --all-features -- -D warnings

# Full HTTP feature test suite (includes all security tests above)
cargo nextest run --features http
```
