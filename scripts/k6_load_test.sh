#!/usr/bin/env bash
# k6_load_test.sh — start PRMS + Postgres, run k6 passenger load test,
# save a Markdown report to docs/k6-load-report.md, then tear down.
#
# Usage:
#   bash scripts/k6_load_test.sh
#
# Env overrides:
#   BASE_URL     target base URL (default: http://localhost:8080)
#   REPORT_FILE  output Markdown path (default: docs/k6-load-report.md)
#   KEEP_STACK   set to 1 to leave compose stack running after the test
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

BASE_URL="${BASE_URL:-http://localhost:18084}"
REPORT_FILE="${REPORT_FILE:-$REPO_ROOT/docs/k6-load-report.md}"
KEEP_STACK="${KEEP_STACK:-0}"
BEARER_TOKEN="load-test-token"
PRMS_HOST_PORT="${PRMS_HOST_PORT:-18084}"

cd "$REPO_ROOT"

cleanup() {
  if [[ "$KEEP_STACK" != 1 ]]; then
    printf 'stopping compose stack...\n'
    PRMS_API_KEYS="$BEARER_TOKEN:cl-aria" \
    PRMS_CORS_ORIGINS="http://localhost" \
    PRMS_DOMAIN="localhost" \
    PRMS_HOST_PORT="$PRMS_HOST_PORT" \
    docker compose -f docker-compose.yml -f docker-compose.loadtest.yml down -v \
      >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

printf 'starting PRMS compose stack (rate limit disabled)...\n'
PRMS_API_KEYS="$BEARER_TOKEN:cl-aria" \
PRMS_CORS_ORIGINS="http://localhost" \
PRMS_DOMAIN="localhost" \
PRMS_HOST_PORT="$PRMS_HOST_PORT" \
docker compose -f docker-compose.yml -f docker-compose.loadtest.yml up -d --build --wait \
  >/dev/null 2>&1

printf 'waiting for /health/ready...\n'
ready_response=""
for _ in $(seq 1 30); do
  if ready_response="$(curl -fsS "${BASE_URL}/health/ready" 2>/dev/null)"; then
    printf 'ready: %s\n' "$ready_response"
    break
  fi
  sleep 1
done

if [[ -z "$ready_response" ]]; then
  printf 'FAIL: PRMS not ready after 30s\n' >&2
  exit 1
fi

printf 'running k6 load test (50 VUs, 45 s total)...\n'
K6_OUTPUT="$(k6 run \
  --env BASE_URL="$BASE_URL" \
  --env BEARER_TOKEN="$BEARER_TOKEN" \
  "$REPO_ROOT/k6/passenger_load.js" 2>&1)"
printf '%s\n' "$K6_OUTPUT"

mkdir -p "$(dirname "$REPORT_FILE")"
cat > "$REPORT_FILE" << EOF
# PRMS k6 Load Test Report

Generated: $(date -u "+%Y-%m-%d %H:%M UTC")

## Setup

- Service: Passenger Resource Management (Rust/Axum) + PostgreSQL 17
- Scenario: POST /passengers (stable per-VU id, unique idempotency key per iter → 409 after first create) then GET /passengers/{id}
- Architecture note: 409 on duplicate id bypasses sync_all flush; subsequent iterations test in-memory idempotency + read paths
- Stages: 10 s warmup (5 VUs) → 30 s load (50 VUs) → 5 s ramp-down
- Rate limiting: disabled for load test
- Thresholds: p(95) < 500 ms, http error rate < 1 % (409 excluded)

## k6 Output

\`\`\`
$K6_OUTPUT
\`\`\`
EOF

printf 'report saved to %s\n' "$REPORT_FILE"
