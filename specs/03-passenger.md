# 03 — Passenger

**Spec ID prefix:** `PS`

## Purpose
Define the Passenger lifecycle: creation, tier change (upgrade/downgrade),
and soft delete. Only Crew Leads may mutate passengers.

## Inputs
- `Passenger { id: PassengerId, name: String, tier: Tier, deleted_at: Option<DateTime<Utc>> }`
- An `Actor` (Crew Lead) for every mutation.
- A `Clock` (port trait) for stamping `deleted_at`.

## Outputs
- `Result<Passenger, DomainError>` from mutations.
- `&[Passenger]` (active) from queries.

## Rules (normative)
- **PS-R1**: `create(actor, passenger)` creates a passenger with the given
  tier. Only `Actor::CrewLead` may call this.
  - Error: `UnauthorizedActor`.
- **PS-R2**: Passenger ids are unique among **active** passengers.
  - Error: `PassengerAlreadyExists`.
- **PS-R3**: `change_tier(actor, id, new_tier)` updates a passenger's
  tier. Crew-Lead only.
- **PS-R4**: `change_tier` to the same tier is a no-op success.
- **PS-R5**: `soft_delete(actor, id)` stamps `deleted_at` from the clock.
  Crew-Lead only. Soft-deleted passengers are excluded from `list()` but
  still resolvable via `get(id)`.
- **PS-R6**: Re-creating a passenger id whose previous record is
  soft-deleted is allowed — the old record stays for audit.
- **PS-R7**: Operating on an unknown or soft-deleted id is rejected.
  - Error: `PassengerNotFound`.
- **PS-R8**: `list()` returns active passengers in insertion order.
- **PS-R9**: `get(id)` returns the latest record (active or
  soft-deleted) or `PassengerNotFound`.

## Invariants
- **PS-I1**: Every active passenger has a valid `Tier`.
- **PS-I2**: `deleted_at`, when set, is immutable.

## Errors
- **PS-E1** `UnauthorizedActor`.
- **PS-E2** `PassengerAlreadyExists`.
- **PS-E3** `PassengerNotFound`.

## Acceptance scenarios

### Create
- **PS-S1**: Crew Lead creates `P1` (Silver) → service contains `P1`.
- **PS-S2**: Passenger actor calls `create` → `UnauthorizedActor`.
- **PS-S3**: Active `P1` exists, create `P1` again → `PassengerAlreadyExists`.

### Change tier
- **PS-S4**: Crew Lead changes `P1` Silver → Platinum; `get(P1).tier == Platinum`.
- **PS-S5**: Passenger actor calls `change_tier` → `UnauthorizedActor`.
- **PS-S6**: Unknown id → `PassengerNotFound`.
- **PS-S7**: Same-tier change → `Ok` (idempotent).

### Soft delete
- **PS-S8**: Soft-delete `P1` → `list()` excludes `P1`; `get(P1)` returns
  it with `deleted_at = Some(_)`.
- **PS-S9**: Re-create soft-deleted id → succeeds (PS-R6).

### Listing
- **PS-S10**: Insertion order preserved.

## Traceability
| Rule | Test(s) | Implementation |
|------|---------|----------------|
| PS-R1 / PS-E1 | PS-S1, PS-S2, PS-S5 | `application/passenger_service.rs` |
| PS-R2 / PS-E2 | PS-S3 | ditto |
| PS-R3..R4 | PS-S4, PS-S7 | ditto |
| PS-R5..R7 | PS-S6, PS-S8, PS-S9 | ditto |
| PS-R8..R9 | PS-S10 | ditto |
