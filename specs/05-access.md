# 05 — Access

**Spec ID prefix:** `AC`

## Purpose
Define the runtime permission check that gates passenger use of
resources, and the audit event emitted on every attempt (allowed or
denied).

## Inputs
- `actor: &Actor` — the passenger attempting use.
- `resource_id: &ResourceId`.
- Access to `PassengerService` and `ResourceService` (via DI).
- A `UsageEventSink` (port) for emitting events.
- A `Clock` (port) for timestamps.

> **Self-access is enforced by design.** The service derives
> `passenger_id` exclusively from the `Actor::Passenger` variant — there
> is no separate `passenger_id` parameter. A passenger therefore can only
> attempt access as themselves; impersonation is structurally impossible
> at the service layer. At the HTTP boundary the `passenger_id` is still
> caller-supplied (no real auth yet); this is a known limitation tracked
> in `docs/review-readiness-checklist.md`.

## Outputs
- `Result<UsageEvent, DomainError>` — `Ok` for allowed, `Err` for
  denied. The denial **still emits** a `UsageEvent` with
  `outcome = Denied`.

## Rules (normative)
- **AC-R1**: `use_resource(actor, resource_id)` is only callable by
  `Actor::Passenger(_)`. A Crew Lead actor is rejected with
  `UnauthorizedActor` and **no `UsageEvent` is emitted**.
- **AC-R2**: The passenger must exist and not be soft-deleted; otherwise
  `PassengerNotFound`. **No event emitted.**
- **AC-R3**: The resource must exist and not be soft-deleted; otherwise
  `ResourceNotFound`. **No event emitted.**
- **AC-R4**: Allowed iff `passenger.tier.can_access(resource.min_tier)`.
  - Allowed → emit `UsageEvent { outcome: Allowed, … }` and return
    `Ok(event)`.
  - Denied → emit `UsageEvent { outcome: Denied, … }` and return
    `Err(DomainError::AccessDenied)`. Event is recorded **before**
    the result is returned.
- **AC-R5**: `UsageEvent` fields: `id`, `passenger_id`, `resource_id`,
  `tier_at_attempt`, `min_tier_at_attempt`, `timestamp`, `outcome`.
  Snapshots — never rewritten.
- **AC-R6**: Tier changes take effect on the **next** call. Past events
  remain unchanged.
- **AC-R7**: Timestamps come from the injected `Clock`.

## Invariants
- **AC-I1**: For every `use_resource` call where subject and target both
  exist (active), exactly one `UsageEvent` is emitted.
- **AC-I2**: `UsageEvent` is append-only.

## Errors
- **AC-E1** `UnauthorizedActor`.
- **AC-E2** `AccessDenied`.
- **AC-E3** `PassengerNotFound`.
- **AC-E4** `ResourceNotFound`.

## Acceptance scenarios

- **AC-S1**: Crew Lead actor → `UnauthorizedActor`, sink empty.
- **AC-S2**: Unknown passenger id → `PassengerNotFound`, sink empty.
- **AC-S3**: Soft-deleted passenger → `PassengerNotFound`, sink empty.
- **AC-S4**: Unknown resource → `ResourceNotFound`, sink empty.
- **AC-S5**: Soft-deleted resource → `ResourceNotFound`, sink empty.
- **AC-S6**: Platinum passenger on Silver resource →
  `Ok(Allowed)`, sink has 1 `Allowed` event.
- **AC-S7**: Silver passenger on Gold resource →
  `Err(AccessDenied)`, sink has 1 `Denied` event.
- **AC-S8**: Silver passenger uses Silver resource (allowed); later
  upgraded to Platinum. Original event still has
  `tier_at_attempt = Silver`.
- **AC-S9**: Silver denied on Gold at t0; upgraded to Gold; allowed at
  t1. t0 event stays `Denied`; t1 returns `Ok(Allowed)`.
- **AC-S10**: With `FakeClock` starting at 42, the emitted event's
  timestamp is `Timestamp(42)`.

## Traceability
| Rule  | Test(s)             | Implementation                       |
|-------|---------------------|--------------------------------------|
| AC-R1 | AC-S1               | `application/access_service.rs`      |
| AC-R2 | AC-S2, AC-S3        | ditto                                |
| AC-R3 | AC-S4, AC-S5        | ditto                                |
| AC-R4 | AC-S6, AC-S7        | ditto                                |
| AC-R5 / AC-R6 / AC-I2 | AC-S8, AC-S9 | ditto                            |
| AC-R7 | AC-S10              | ditto                                |
