# Glossary

Canonical definitions for terms used across all specs. If a term isn't
here, it isn't part of the domain.

## Spec Quick-Reference

| Spec file | Prefix | Topic |
|-----------|--------|-------|
| `01-tier-policy.md` | `TP` | Tier ordering and access rule |
| `02-crew-lead.md` | `CL` | Crew Lead lifecycle (exactly-3 invariant) |
| `03-passenger.md` | `PS` | Passenger create / tier-change / soft-delete |
| `04-resource.md` | `RS` | Resource catalog create / tier-change / soft-delete |
| `05-access.md` | `AC` | Runtime permission check + `UsageEvent` emission |
| `06-audit.md` | `AU` | Admin mutation audit trail (`AdminEvent`) |
| `07-reporting.md` | `RP` | Read-only queries over the `UsageEvent` trail |

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

### CrewLeadId
Newtype wrapping a `Uuid`. Uniquely identifies a `CrewLead`.

### PassengerId
Newtype wrapping a `Uuid`. Uniquely identifies a `Passenger`.

### ResourceId
Newtype wrapping a `Uuid`. Uniquely identifies a `Resource`.

### Timestamp
Newtype wrapping `DateTime<Utc>`. Used for immutable time snapshots on
events. Never created directly in domain or application code — always
sourced from the injected `Clock`.

### Outcome
Enum with two variants: `Allowed` and `Denied`. Recorded on every
`UsageEvent` to capture the result of a resource access attempt.

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
An immutable append-only record emitted whenever a **successful** Crew
Lead mutation occurs. Failed operations emit no event. Fields:
- `id`: monotonically increasing sequence number.
- `actor_id`: `CrewLeadId` of the Crew Lead who performed the action.
- `action`: `AdminAction` variant identifying the mutation type.
- `target_kind`: `TargetKind` — which entity type was affected.
- `target_id`: string id of the affected entity.
- `timestamp`: `Timestamp` from the injected `Clock`.
- `details`: `Option<String>` — free-form context (e.g. `"tier changed Silver → Gold"`).

### TargetKind
Closed enum attached to `AdminEvent` identifying which entity type
`target_id` refers to. Variants: `CrewLead`, `Passenger`, `Resource`.

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

### Bootstrap
The one-time operation that seeds the system with **exactly three**
`CrewLead`s. Rejected if fewer than three, more than three, or duplicate
ids are provided. Emits one `CrewLeadBootstrapped` `AdminEvent` on
success.

### min_tier
The minimum `Tier` a `Resource` requires. A passenger may access a
resource only when `passenger.tier.rank() >= resource.min_tier.rank()`.
Mutating `min_tier` takes effect on the next access attempt; historical
`UsageEvent`s snapshot the value at attempt time.

### rank()
Method on `Tier` returning a `u8`: Silver → 1, Gold → 2, Platinum → 3.
Used for total-order comparison. Never compare tiers with `==` for
ordering; always go through `rank()`.

### can_access(min_tier)
Method on `Tier`. Returns `true` iff `self.rank() >= min_tier.rank()`.
Reflexive (a tier can access itself), antisymmetric, and transitive.

### AdminAction
Closed enum enumerating every administrative mutation type. Recorded in
`AdminEvent.action`. Variants:
- Crew Lead: `CrewLeadBootstrapped`, `CrewLeadAdded`, `CrewLeadRemoved`,
  `CrewLeadReplaced`
- Passenger: `PassengerCreated`, `PassengerTierChanged`,
  `PassengerDeleted`
- Resource: `ResourceCreated`, `ResourceMinTierChanged`,
  `ResourceDeleted`

### TierCounts
Aggregate for reporting. Holds `allowed: u64` and `denied: u64` —
counts of `UsageEvent`s where `tier_at_attempt` matched a given `Tier`.
Used by `aggregate_by_tier()`.

### Port
A trait defined in `application/ports.rs` that abstracts an I/O
dependency (repository, clock, event sink). Services depend on ports
only — never on concrete adapter types. Concrete implementations live in
`infrastructure/`.

### Clock
Port trait providing the current time as a `Timestamp`. Injected at the
composition root. Domain and application code never call
`SystemTime::now()` or `Utc::now()` directly. Tests use `FakeClock`.

### Composition Root
The single wiring point (`src/interface/composition_root.rs`) where
concrete infrastructure adapters are instantiated and injected into
application services. The only place allowed to know both ports and
concrete adapters simultaneously.

## Services

### CrewLeadService
Application service orchestrating the Crew Lead lifecycle: `bootstrap`,
`add`, `remove`, `replace`, `list`. See `02-crew-lead.md`.

### PassengerService
Application service orchestrating the passenger lifecycle: `create`,
`change_tier`, `soft_delete`, `list`, `get`. See `03-passenger.md`.

### ResourceService
Application service orchestrating the resource catalog: `create`,
`change_min_tier`, `soft_delete`, `list`, `list_accessible_for`, `get`.
See `04-resource.md`.

### AccessService
Application service evaluating runtime resource access. Calls
`Tier::can_access`, emits a `UsageEvent` for every attempt (allowed or
denied) via `UsageEventSink`. See `05-access.md`.

### ReportingService
Read-only application service over the `UsageEvent` trail. Methods:
`personal_history`, `aggregate_by_tier`, `top_resources`. See
`07-reporting.md`.

## Ports (Traits)

### CrewLeadRepo
Persistence port for `CrewLead` entities. Methods: `all`, `save`,
`remove`.

### PassengerRepo
Persistence port for `Passenger` entities. Methods: `get`, `list_active`,
`save`.

### ResourceRepo
Persistence port for `Resource` entities. Methods: `get`, `list_active`,
`save`.

### UsageEventSink
Write port for appending `UsageEvent`s. Method: `record(event)`.

### UsageEventSource
Read port for querying `UsageEvent`s. Method: `list() -> &[UsageEvent]`.
`InMemoryUsageEventSink` satisfies both `UsageEventSink` and
`UsageEventSource`.

### AdminEventSink
Write port for appending `AdminEvent`s. Method: `record(event)`.

## Errors

All errors are variants of the single `DomainError` enum in
`domain/errors.rs` (`#[non_exhaustive]`). Spec IDs are shown for
traceability.

| Variant | Spec | Meaning |
|---------|------|---------|
| `InvalidTier` | TP-E1 | String does not map to a known `Tier` variant. |
| `CrewLeadLimitReached` | CL-E1 | Attempted to add a 4th Crew Lead. |
| `CrewLeadMinimumBreached` | CL-E2 | Attempted to remove a lead without replacement. |
| `CrewLeadAlreadyExists` | CL-E3 | Duplicate Crew Lead id. |
| `CrewLeadNotFound` | CL-E4 | Unknown Crew Lead id on remove or replace. |
| `CrewLeadBootstrapInvalid` | CL-E5 | Bootstrap called with other than exactly 3 distinct leads. |
| `UnauthorizedActor` | PS-E1, RS-E1, AC-E1 | Actor does not have permission for the operation. |
| `PassengerAlreadyExists` | PS-E2 | Active passenger with the same id already exists. |
| `PassengerNotFound` | PS-E3, AC-E3 | Unknown or soft-deleted passenger id. |
| `ResourceAlreadyExists` | RS-E2 | Active resource with the same id already exists. |
| `ResourceNotFound` | RS-E3, AC-E4 | Unknown or soft-deleted resource id. |
| `AccessDenied` | AC-E2 | Passenger tier is below the resource's `min_tier`. |

## Out of scope (explicit non-terms)
- **Authentication / Session / Token** — out of scope. Actor identity is
  provided by the caller.
- **Capacity** — resources have no concurrent-use limit.
- **Scheduling / Reservations** — not in this system.
- **Currency / Billing** — not in this system.
- **Hard delete** — passengers and resources are soft-deleted only so
  historical audit entries remain resolvable.
- **Event replay / projection** — the audit trail is append-only storage,
  not an event-sourced aggregate.
- **Temporary / time-boxed tier upgrades** — tier is set once and changed
  only by an explicit Crew Lead action.
- **Per-resource capacity limits** — a resource has no concurrent-use cap.
