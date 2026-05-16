#!/usr/bin/env bash
set -euo pipefail

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

need cargo
need curl
need docker

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
container_name="${PRMS_PG_SMOKE_CONTAINER:-prms-pg-smoke-$$}"
pg_port="${PRMS_PG_SMOKE_PORT:-$((55432 + RANDOM % 1000))}"
app_port="${PRMS_PG_SMOKE_APP_PORT:-$((18080 + RANDOM % 1000))}"
pg_url="postgres://prms:prms@127.0.0.1:${pg_port}/prms"
base_url="http://127.0.0.1:${app_port}"
log_file="${TMPDIR:-/tmp}/prms-postgres-smoke-${container_name}.log"
app_pid=""
started_at="$(date +%s)"

cleanup() {
  if [[ -n "$app_pid" ]] && kill -0 "$app_pid" >/dev/null 2>&1; then
    kill "$app_pid" >/dev/null 2>&1 || true
    wait "$app_pid" >/dev/null 2>&1 || true
  fi
  docker rm -f "$container_name" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run \
  --detach \
  --rm \
  --name "$container_name" \
  --env POSTGRES_USER=prms \
  --env POSTGRES_PASSWORD=prms \
  --env POSTGRES_DB=prms \
  --publish "127.0.0.1:${pg_port}:5432" \
  postgres:18-alpine >/dev/null

for _ in {1..40}; do
  if docker exec "$container_name" pg_isready -U prms -d prms >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! docker exec "$container_name" pg_isready -U prms -d prms >/dev/null 2>&1; then
  echo "PostgreSQL did not become ready" >&2
  exit 1
fi

(
  cd "$repo_root"
  # FIX: clap boolean flags are presence-only on the CLI, so
  # `--enable-rate-limit=false` is rejected. Set the env var to false instead.
  PRMS_ENABLE_RATE_LIMIT=false cargo run --features postgres --bin serve -- \
    --bind "127.0.0.1:${app_port}" \
    --pg-url "$pg_url" \
    --api-keys "cl-aria:cl-aria,ps-001:ps-001,ps-002:ps-002" \
    --enable-reset
) >"$log_file" 2>&1 &
app_pid="$!"

for _ in {1..60}; do
  if curl -fsS "${base_url}/health/ready" >/dev/null 2>&1; then
    break
  fi
  if ! kill -0 "$app_pid" >/dev/null 2>&1; then
    echo "PRMS server exited before readiness; see $log_file" >&2
    exit 1
  fi
  sleep 1
done

curl -fsS "${base_url}/health" >/dev/null
ready_before="$(curl -fsS "${base_url}/health/ready")"

curl -fsS -X POST "${base_url}/passengers" \
  -H 'Authorization: Bearer cl-aria' \
  -H 'Content-Type: application/json' \
  -H 'Idempotency-Key: postgres-smoke-create-ps-neo' \
  -d '{"id":"ps-neo","name":"Neo Park","tier":"Gold"}' >/dev/null

curl -fsS -X POST "${base_url}/access" \
  -H 'Authorization: Bearer ps-002' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id":"res-spa"}' >/dev/null

denied_code="$(curl -sS -o /dev/null -w '%{http_code}' -X POST "${base_url}/access" \
  -H 'Authorization: Bearer ps-001' \
  -H 'Content-Type: application/json' \
  -d '{"resource_id":"res-spa"}')"

if [[ "$denied_code" != "403" ]]; then
  echo "expected denied access to return 403, got $denied_code" >&2
  exit 1
fi

ready_after="$(curl -fsS "${base_url}/health/ready")"
audit_verify="$(curl -fsS "${base_url}/audit/verify" -H 'Authorization: Bearer cl-aria')"
elapsed_secs="$(( $(date +%s) - started_at ))"

cat <<REPORT
PostgreSQL smoke passed in ${elapsed_secs}s
container=${container_name}
pg_url=${pg_url}
base_url=${base_url}
log_file=${log_file}
ready_before=${ready_before}
ready_after=${ready_after}
audit_verify=${audit_verify}
REPORT