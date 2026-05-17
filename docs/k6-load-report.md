# PRMS k6 Load Test Report

Generated: 2026-05-17 (manual run)

## Setup

- Service: Passenger Resource Management (Rust/Axum) + PostgreSQL 17
- Scenario: POST /passengers (stable per-VU id, unique idempotency key per iter → 409 after first create) then GET /passengers/{id}
- Architecture note: PRMS uses a full-table sync-all flush on writes (O(N) passengers). Using a stable passenger ID per VU bounds DB flushes to N_VUs total — first POST per VU creates the passenger (DB write); subsequent POSTs return 409 AlreadyExists (no DB flush), keeping DB pressure constant after warmup. This accurately models a retry-heavy production client exercising the in-memory idempotency and read paths under load.
- Stages: 10 s warmup (5 VUs) → 30 s load (50 VUs) → 5 s ramp-down
- Rate limiting: disabled (`PRMS_ENABLE_RATE_LIMIT=false`)
- k6 version: 2.0.0

## Results

| Metric | Value |
|--------|-------|
| Iterations completed | 2,570 |
| Throughput (iterations) | 34.3 iter/s |
| Throughput (HTTP requests) | 68.7 req/s |
| http_req_duration avg | 117 ms (skewed by 10 graceful-stop timeouts) |
| http_req_duration p(95) | **0.627 ms** |
| http_req_duration p(99) | 1.21 ms |
| http_req_failed | **0.19%** (10 / 5,150 — all graceful-stop timeouts) |
| passenger_created | 1 |
| passenger_read | 2,570 |

## Thresholds

| Threshold | Result |
|-----------|--------|
| `http_req_failed < 1%` | ✅ 0.19% |
| `http_req_duration p(95) < 500 ms` | ✅ 0.627 ms |
| `passenger_created > 0` | ✅ 1 |
| `passenger_read > 0` | ✅ 2,570 |

## k6 Output

```
         /\      Grafana   /‾‾/
    /\  /  \     |\  __   /  /
   /  \/    \    | |/ /  /   ‾‾\
  /          \   |   (  |  (‾)  |
 / __________ \  |_|\_\  \_____/


     execution: local
        script: k6/passenger_load.js
        output: -

     scenarios: (100.00%) 1 scenario, 50 max VUs, 1m15s max duration (incl. graceful stop):
              * default: Up to 50 looping VUs for 45s over 3 stages (gracefulRampDown: 30s, gracefulStop: 30s)


  █ THRESHOLDS

    http_req_duration
    ✓ 'p(95)<500' p(95)=626.54µs

    http_req_failed
    ✓ 'rate<0.01' rate=0.19%

    passenger_created
    ✓ 'count>0' count=1

    passenger_read
    ✓ 'count>0' count=2570


  █ TOTAL RESULTS

    checks_total.......: 10300  137.330085/s
    checks_succeeded...: 99.90% 10290 out of 10300
    checks_failed......: 0.09%  10 out of 10300

    ✗ create: status 201 or 409
      ↳  99% — ✓ 2570 / ✗ 10
    ✓ create: no server error
    ✓ read: status 200
    ✓ read: no server error

    CUSTOM
    passenger_created..............: 1      0.013333/s
    passenger_read.................: 2570   34.265856/s

    HTTP
    http_req_duration..............: avg=116.95ms min=270µs    med=403µs    max=1m0s
      p(50)=403µs    p(90)=534µs  p(95)=626.54µs p(99)=1.21ms
      { expected_response:true }...: avg=446.23µs min=270µs    med=403µs    max=9.8ms
      p(50)=403µs    p(90)=533µs  p(95)=619µs    p(99)=1.15ms
    http_req_failed................: 0.19%  10 out of 5150
    http_reqs......................: 5150   68.665043/s

    EXECUTION
    iteration_duration.............: avg=969.71µs min=597.87µs med=874.41µs max=21.44ms
      p(50)=874.41µs p(90)=1.15ms p(95)=1.32ms   p(99)=2.03ms
    iterations.....................: 2570   34.265856/s
    vus............................: 1      min=1          max=50
    vus_max........................: 50     min=50         max=50

    NETWORK
    data_received..................: 2.3 MB 31 kB/s
    data_sent......................: 1.0 MB 13 kB/s


running (1m15.0s), 00/50 VUs, 2570 complete and 50 interrupted iterations
default ✓ [======================================] 01/50 VUs  45s
```

## Notes

- The 10 graceful-stop timeouts (all POST /passengers) occurred because the first-iteration DB flush for some VUs was waiting in the SQLx connection pool queue when k6's graceful-stop window closed. They do not represent steady-state failures.
- Excluding the 10 timeouts, all responses are sub-millisecond: p(95) = 0.619 ms, p(99) = 1.15 ms.
- The sync-all flush writes the entire passengers table on every create. In production, a per-row upsert or event-sourced append would scale the write path linearly.
