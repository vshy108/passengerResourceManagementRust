.PHONY: test test-http postgres-smoke load-test

## Run core library + integration tests (no HTTP adapter)
test:
	cargo nextest run

## Run tests including the Axum HTTP adapter suite
test-http:
	cargo nextest run --features http

## Start a temporary Postgres, run PRMS against it, then tear down
postgres-smoke:
	bash scripts/postgres-smoke.sh

## Run k6 load test against the Postgres-backed compose stack
load-test:
	bash scripts/k6_load_test.sh
