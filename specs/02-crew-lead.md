# 02 — Crew Lead

**Spec ID prefix:** `CL`

## Purpose
Define the lifecycle of Crew Lead administrators and enforce the
system-wide invariant that there are **exactly three** of them after
bootstrap.

## Inputs
- `CrewLead { id: CrewLeadId, name: String }`
- A `CrewLeadRepo` trait (port) for storage; the service depends on the
  trait, not a concrete adapter.

## Outputs
- `Result<T, DomainError>` from mutation operations.
- `&[CrewLead]` (or `Vec<CrewLead>` clone) from queries.

## Rules (normative)
- **CL-R1**: Bootstrap seeds the system with **exactly three** Crew Leads.
- **CL-R2**: `add(lead)` is rejected when the current count is already 3.
  - Error: `CrewLeadLimitReached`.
- **CL-R3**: `remove(id)` is rejected — the exactly-3 invariant plus the
  cap in `add` (CL-R2) means the count is always ≤ 3, so removal always
  breaches the minimum. Use `replace` to rotate a lead.
  - Error: `CrewLeadMinimumBreached`.
- **CL-R4**: `replace(old_id, new_lead)` atomically removes `old_id` and
  adds `new_lead`. Count remains 3. Rejected if `old_id` is not a current
  lead.
  - Error: `CrewLeadNotFound`.
- **CL-R5**: Crew Lead IDs are unique. Adding an existing id is rejected.
  - Error: `CrewLeadAlreadyExists`.
- **CL-R6**: `list()` returns all current Crew Leads in insertion order.

## Invariants
- **CL-I1**: After bootstrap, `count() == 3` at all times.
- **CL-I2**: No duplicate Crew Lead ids exist at any point.

## Errors
All are variants of the single `domain::DomainError` enum
(`#[non_exhaustive]`).

- **CL-E1** `CrewLeadLimitReached`: attempted to add a 4th lead.
- **CL-E2** `CrewLeadMinimumBreached`: attempted to remove when count is
  3 without replacement.
- **CL-E3** `CrewLeadAlreadyExists`: attempted to add an existing id.
- **CL-E4** `CrewLeadNotFound`: attempted to remove or replace an unknown
  id.
- **CL-E5** `CrewLeadBootstrapInvalid`: bootstrap called with other than
  exactly 3 distinct leads.

## Acceptance scenarios (Given / When / Then)

### Bootstrap (CL-R1, CL-I1, CL-E5)
- **CL-S1**: Given no Crew Leads exist, When `bootstrap` is called with 3
  distinct leads, Then the service contains exactly those 3 leads.
- **CL-S2**: Given no Crew Leads exist, When `bootstrap` is called with 2
  leads, Then it returns `CrewLeadBootstrapInvalid`.
- **CL-S3**: Given no Crew Leads exist, When `bootstrap` is called with 4
  leads, Then it returns `CrewLeadBootstrapInvalid`.
- **CL-S4**: Given no Crew Leads exist, When `bootstrap` is called with 3
  leads containing a duplicate id, Then it returns
  `CrewLeadBootstrapInvalid`.

### Add (CL-R2, CL-E1)
- **CL-S5**: Given 3 Crew Leads exist, When `add` is called, Then it
  returns `CrewLeadLimitReached` and the count remains 3.

### Remove (CL-R3, CL-E2)
- **CL-S6**: Given 3 Crew Leads exist, When `remove(existing_id)` is
  called without replacement, Then it returns `CrewLeadMinimumBreached`
  and the count remains 3.

### Replace (CL-R4, CL-E4, CL-E3)
- **CL-S7**: Given 3 Crew Leads exist, When
  `replace(existing_id, new_lead)` is called, Then the service contains
  the 2 other original leads plus `new_lead`, count stays 3.
- **CL-S8**: Given 3 Crew Leads exist, When
  `replace(unknown_id, new_lead)` is called, Then it returns
  `CrewLeadNotFound` and state is unchanged.
- **CL-S9**: Given 3 Crew Leads exist, When
  `replace(existing_id, new_lead)` is called and `new_lead.id` matches
  another current lead's id, Then it returns `CrewLeadAlreadyExists` and
  state is unchanged.

### Listing (CL-R6)
- **CL-S11**: Given 3 Crew Leads were bootstrapped in order A, B, C,
  When `list()` is called, Then it returns `[A, B, C]` in that order.

## Out of scope
- Persistence beyond the process lifetime.
- Roles beyond Crew Lead (no super-admin, no passenger-admin).
- Any UI for managing leads.

## Traceability
| Rule          | Test(s)        | Implementation                                            |
|---------------|----------------|-----------------------------------------------------------|
| CL-R1 / CL-I1 | CL-S1..S4      | `application/crew_lead_service.rs: bootstrap`             |
| CL-R2         | CL-S5          | `application/crew_lead_service.rs: add`                   |
| CL-R3         | CL-S6          | `application/crew_lead_service.rs: remove`                |
| CL-R4         | CL-S7..S9      | `application/crew_lead_service.rs: replace`               |
| CL-R5 / CL-I2 | CL-S4, CL-S9   | service uniqueness check                                  |
| CL-R6         | CL-S11         | `application/crew_lead_service.rs: list`                  |
