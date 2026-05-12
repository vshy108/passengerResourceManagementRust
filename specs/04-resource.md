# 04 — Resource

**Spec ID prefix:** `RS`

## Purpose
Define the Resource catalog: provisioning, updating, and decommissioning
onboard facilities. Only Crew Leads may mutate resources.

## Inputs
- `Resource { id: ResourceId, name: String, category: String,
  min_tier: Tier, deleted_at: Option<Timestamp> }`
- An `Actor` (Crew Lead) for every mutation.
- A `Clock` (port trait) for `deleted_at`.

## Outputs
- `Result<Resource, DomainError>` from mutations.
- `&[Resource]` (active) from queries.

## Rules (normative)
- **RS-R1**: `create(actor, resource)` Crew-Lead-only.
  - Error: `UnauthorizedActor`.
- **RS-R2**: Resource ids unique among **active** resources.
  - Error: `ResourceAlreadyExists`.
- **RS-R3**: `change_min_tier(actor, id, new_tier)` Crew-Lead-only.
  Future access checks use the new value; historical events untouched.
- **RS-R4**: `soft_delete(actor, id)` excludes from `list()` /
  `list_accessible_for()`; `get(id)` still resolves.
- **RS-R5**: Operating on unknown or soft-deleted id is rejected.
  - Error: `ResourceNotFound`.
- **RS-R6**: `list()` returns active resources in insertion order.
- **RS-R7**: `list_accessible_for(tier)` returns active resources
  satisfying `tier.can_access(resource.min_tier)`, in insertion order.

## Invariants
- **RS-I1**: Active resources have a valid `Tier` as `min_tier`.
- **RS-I2**: `deleted_at`, when set, is immutable.

## Errors
- **RS-E1** `UnauthorizedActor`.
- **RS-E2** `ResourceAlreadyExists`.
- **RS-E3** `ResourceNotFound`.

## Acceptance scenarios

### Create
- **RS-S1**: Crew Lead creates `R1(Silver)` → catalog contains it.
- **RS-S2**: Passenger actor → `UnauthorizedActor`.
- **RS-S3**: Active id collision → `ResourceAlreadyExists`.

### Change min tier
- **RS-S4**: Crew Lead changes `R1` Silver → Platinum → `min_tier == Platinum`.
- **RS-S5**: Passenger actor → `UnauthorizedActor`.
- **RS-S6**: Unknown id → `ResourceNotFound`.

### Soft delete
- **RS-S7**: After `soft_delete(R1)`: excluded from `list()`; `get(R1)`
  returns it with `deleted_at = Some(_)`.
- **RS-S8**: `change_min_tier` on soft-deleted → `ResourceNotFound`.

### Listing
- **RS-S9**: Insertion order preserved.
- **RS-S10**: With Silver `S1`, Gold `G1`, Platinum `P1` active,
  `list_accessible_for(Gold)` returns `[S1, G1]`.
- **RS-S11**: Soft-deleted resources are excluded from
  `list_accessible_for`.

## Traceability
| Rule | Test(s) | Implementation |
|------|---------|----------------|
| RS-R1 / RS-E1 | RS-S1, RS-S2, RS-S5 | `application/resource_service.rs` |
| RS-R2 / RS-E2 | RS-S3 | ditto |
| RS-R3 | RS-S4, RS-S6 | ditto |
| RS-R4 / RS-R5 | RS-S7, RS-S8 | ditto |
| RS-R6 | RS-S9 | ditto |
| RS-R7 | RS-S10, RS-S11 | uses `Tier::can_access` |

## Implementation notes

### RS-N1 — `version` counter is persisted
The optimistic-concurrency `version` field on `Resource` is incremented on each
successful `change_min_tier` or `soft_delete` call. SQLite and PostgreSQL
entity stores persist and restore this field so an `If-Match: "N"` check keeps
its conflict-protection semantics across server restarts.
