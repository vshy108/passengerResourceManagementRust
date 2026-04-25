# 07 — Reporting

**Spec ID prefix:** `RP`

## Purpose
Read-only queries over the `UsageEvent` trail (see `specs/05-access.md`).
Reporting does not mutate state and does not emit events of its own.

## Inputs
- A `UsageEventSource` (port) — any object exposing
  `list(): &[UsageEvent]`. `InMemoryUsageEventSink` satisfies this.

## Rules (normative)
- **RP-R1**: `personal_history(passenger_id)` returns all `UsageEvent`s
  for that passenger, in chronological order (insertion order is
  chronological by AC-R6). Empty `Vec` when the passenger has no
  events — never an error.
- **RP-R2**: `aggregate_by_tier()` returns a map
  `Tier -> TierCounts { allowed, denied }` counting attempts at which
  the passenger was **at that tier** (`tier_at_attempt`), not their
  current tier. Every `Tier` variant appears in the result (zeros when
  absent).
- **RP-R3**: `top_resources(n)` returns the `n` resources with the most
  **allowed** uses, sorted descending by count. Ties are broken by
  `ResourceId` ascending (deterministic). `n == 0` returns an empty
  `Vec`.
- **RP-R4**: Denied attempts do not count toward `top_resources`.
- **RP-R5**: Reporting is a pure read over the source — calling any
  method twice with the same source yields identical output.

## Invariants
- **RP-I1**: Reporting does not modify the event source.
- **RP-I2**: Output is deterministic given the input events.

## Acceptance scenarios

### Personal history (RP-R1)
- **RP-S1**: Given a passenger with three events (two allowed, one
  denied), `personal_history(id)` returns all three in insertion order.
- **RP-S2**: Given no events for an unknown passenger, returns `vec![]`.
- **RP-S3**: Events for other passengers are excluded.

### Aggregate by tier (RP-R2)
- **RP-S4**: Given two allowed-at-Silver and one denied-at-Silver event,
  `aggregate_by_tier()[&Tier::Silver]` equals
  `TierCounts { allowed: 2, denied: 1 }`.
- **RP-S5**: Tiers with no events still appear with
  `TierCounts { allowed: 0, denied: 0 }`.
- **RP-S6**: A tier change on the passenger does **not** reclassify past
  events — snapshot rule (AC-R5).

### Top resources (RP-R3, RP-R4)
- **RP-S7**: Given allowed counts r1=3, r2=1, r3=2, `top_resources(2)`
  returns `[r1, r3]`.
- **RP-S8**: Denied events are ignored: r1 denied×5 + r2 allowed×1 →
  `top_resources(1) = [r2]`.
- **RP-S9**: Ties break by `ResourceId` ascending.
- **RP-S10**: `top_resources(0)` returns `vec![]`.

## Out of scope
- Date range filters (future).
- Resource metadata joins (category, name).
- Persistence / streaming.

## Traceability
| Rule | Test(s) |
|------|---------|
| RP-R1 | RP-S1, RP-S2, RP-S3 |
| RP-R2 | RP-S4, RP-S5, RP-S6 |
| RP-R3 / RP-R4 | RP-S7, RP-S8, RP-S9, RP-S10 |
