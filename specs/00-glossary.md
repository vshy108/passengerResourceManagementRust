# Glossary

Canonical definitions for terms used across all specs. If a term isn't
here, it isn't part of the domain.

## Actors

### Crew Lead
An administrator with exclusive permissions to manage passengers and
resources. The system enforces **exactly three** Crew Leads at all times
after bootstrap (see `02-crew-lead.md`).

### Passenger
A traveler aboard Spaceship X26 who consumes onboard resources. Each
passenger has exactly one **Tier**.

## Value Objects

### Tier
An ordered membership level. One of:

| Tier     | Rank |
|----------|------|
| Silver   | 1    |
| Gold     | 2    |
| Platinum | 3    |

Higher rank inherits access to all lower-rank resources. See
`01-tier-policy.md`.

## Entities

### Resource
An onboard facility (e.g. `Food Station`, `Luxury O2 Pod`). Each resource
has:
- `id`: unique identifier (`ResourceId`).
- `name`: human-readable label.
- `category`: grouping tag (e.g. `hygiene`, `oxygen`).
- `min_tier`: minimum tier required to access.

### UsageEvent
An immutable record emitted whenever a passenger attempts to use a
resource. Fields:
- `id`, `passenger_id`, `resource_id`, `timestamp`, `outcome`.
- `outcome` ∈ `{ Allowed, Denied }`.
- Tier snapshot fields (`tier_at_attempt`, `min_tier_at_attempt`) so
  history is never rewritten when current tiers change.

### AdminEvent
An immutable record emitted whenever a Crew Lead performs an
administrative mutation (create/update/delete passenger or resource,
tier change, etc.).

## Concepts

### Access
A passenger is said to *have access* to a resource iff
`passenger.tier.rank() >= resource.min_tier.rank()`.

### Actor
The subject invoking a service method. Passed explicitly as a parameter —
there is no implicit session. Actors are either a Crew Lead or a Passenger.

### Audit Trail
The append-only collection of all `UsageEvent`s and `AdminEvent`s. Used
for personal history and aggregated reports. Never mutated or deleted.

### Soft delete
A passenger or resource marked as `deleted_at: Option<DateTime<Utc>>` but
retained so historical audit entries referencing it remain resolvable.
Hard deletion is not permitted.

## Out of scope (explicit non-terms)
- **Authentication / Session / Token** — out of scope. Actor identity is
  provided by the caller.
- **Capacity** — resources have no concurrent-use limit.
- **Scheduling / Reservations** — not in this system.
- **Currency / Billing** — not in this system.
