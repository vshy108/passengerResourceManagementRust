# Security Hardening Evidence - 2026-05-18

This note records a fresh validation pass for PRMS auth, request hardening, and rate-limit controls.

## Goal

Prove the service rejects bad or abusive requests predictably and documents the security policy for reviewers.

## Commands Run

```sh
cargo clippy --all-targets --all-features -- -D warnings
make test-http
```

## Results

| Check | Result | Evidence |
|-------|--------|----------|
| All-feature clippy | Passed | `cargo clippy --all-targets --all-features -- -D warnings` finished cleanly |
| HTTP security tests | Passed | `make test-http` completed successfully |
| Unknown bearer token | Covered | `unauthorized_request_returns_typed_error_code` expects `401` and `Unauthorized` |
| Missing bearer token | Covered | `auth_check_rejects_missing_token` and passenger create auth tests expect rejection |
| Wrong actor type | Covered | `/reset` admin tests reject non-crew-lead actors |
| Rate limiting | Covered | `rate_limit_returns_429_after_burst_exhausted` proves strict token-bucket throttling |
| Response headers | Covered | `security_response_headers_are_set` verifies JSON API hardening headers |
| Reset route exposure | Covered | `reset_not_registered_when_disabled` keeps local reset behavior opt-in |

## Reviewer Signal

The hardening controls are implemented, documented, and tested. No code change was needed in this pass because the existing implementation already covers the next slice's auth, actor-scope, request-size, CORS, response-header, and rate-limit boundaries.

Next useful slice: generate a short API transcript for one unauthorized request, one forbidden actor request, and one rate-limited request so the behavior is visible without reading test code.