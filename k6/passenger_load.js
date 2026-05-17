// k6/passenger_load.js
// Measures throughput and latency of the PRMS passenger API against the
// Postgres-backed compose stack. Exercises a write+read cycle per iteration:
//   1. POST /passengers — create or confirm a stable per-VU passenger
//   2. GET  /passengers/{id} — read back the passenger
//
// Architecture note: PRMS uses a full-table sync-all flush to Postgres on
// every write (O(N) where N = passenger count). Using a STABLE passenger ID
// per VU (ps-load-{VU}) bounds DB flush count to N_VUs total — first POST
// per VU creates the passenger (flush); all subsequent POSTs with a new
// idempotency key return 409 AlreadyExists WITHOUT calling flush_to_db,
// keeping DB load constant after warmup. 409 is marked non-failure because
// it is the expected idempotency contract response.
//
// Stages: 10s warmup (5 VUs) → 30s sustained load (50 VUs) → 5s ramp-down.
//
// Run:
//   k6 run k6/passenger_load.js
// Override target:
//   k6 run k6/passenger_load.js --env BASE_URL=http://localhost:8080 --env BEARER_TOKEN=load-test-token

import http from 'k6/http';
import { check } from 'k6';
import { Counter } from 'k6/metrics';

const BASE_URL = __ENV.BASE_URL || 'http://localhost:8080';
const BEARER_TOKEN = __ENV.BEARER_TOKEN || 'load-test-token';

export const passengerCreated = new Counter('passenger_created');
export const passengerRead = new Counter('passenger_read');

export const options = {
  stages: [
    { duration: '10s', target: 5 },  // warmup
    { duration: '30s', target: 50 }, // sustained load
    { duration: '5s', target: 0 },   // ramp-down
  ],
  summaryTrendStats: ['avg', 'min', 'med', 'max', 'p(50)', 'p(90)', 'p(95)', 'p(99)'],
  thresholds: {
    http_req_failed: ['rate<0.01'],    // <1% 5xx or network errors (409 excluded via setResponseCallback)
    http_req_duration: ['p(95)<500'], // p(95) under 500 ms
    passenger_created: ['count>0'],   // at least one passenger created in first warmup pass
    passenger_read: ['count>0'],      // at least one read must succeed
  },
};

// Mark 409 (AlreadyExists) as a non-failure: it is the expected response when
// a stable passenger ID is POSTed with a new idempotency key after creation.
http.setResponseCallback(http.expectedStatuses({ min: 200, max: 299 }, 409));

export default function () {
  // Stable passenger ID per VU: bounds DB flushes to N_VUs total.
  // Unique idempotency key per iteration: exercises the write-path check
  // (acquire write lock → AlreadyExists → 409) without triggering flush_to_db.
  const passengerId = `ps-load-${__VU}`;
  const idempotencyKey = `ik-${__VU}-${__ITER}`;

  const writeHeaders = {
    'Content-Type': 'application/json',
    'Authorization': `Bearer ${BEARER_TOKEN}`,
    'Idempotency-Key': idempotencyKey,
  };

  // --- write ---
  const createRes = http.post(
    `${BASE_URL}/passengers`,
    JSON.stringify({ id: passengerId, name: `Load User ${__VU}`, tier: 'Silver' }),
    { headers: writeHeaders, tags: { endpoint: 'create-passenger' } },
  );

  if (createRes.status === 201) passengerCreated.add(1);

  check(createRes, {
    // 201 on first VU iter (created), 409 on subsequent (already exists) — both ok
    'create: status 201 or 409': (r) => r.status === 201 || r.status === 409,
    'create: no server error': (r) => r.status < 500,
  });

  // --- read ---
  const readRes = http.get(
    `${BASE_URL}/passengers/${passengerId}`,
    {
      headers: { 'Authorization': `Bearer ${BEARER_TOKEN}` },
      tags: { endpoint: 'get-passenger' },
    },
  );

  if (readRes.status === 200) passengerRead.add(1);

  check(readRes, {
    'read: status 200': (r) => r.status === 200,
    'read: no server error': (r) => r.status < 500,
  });
}
