# 01 — Tier Policy

**Spec ID prefix:** `TP`

## Purpose
Define the ordering of passenger tiers and the rule that determines
whether a passenger may access a resource.

## Inputs
- `passenger_tier: Tier`
- `resource_min_tier: Tier`

## Outputs
- `Tier::can_access(self, resource_min_tier: Tier) -> bool`
- `Tier::rank(self) -> u8` (returns 1, 2, 3, or 4)

## Rules (normative)
- **TP-R1**: `Tier::Silver.rank() == 1`, `Tier::Gold.rank() == 2`,
  `Tier::Diamond.rank() == 3`, `Tier::Platinum.rank() == 4`.
  Ordering: Silver < Gold < Diamond < Platinum.
- **TP-R2**: `passenger_tier.can_access(resource_min_tier) == true`
  iff `passenger_tier.rank() >= resource_min_tier.rank()`.
- **TP-R3**: Tier ordering is total and stable. No tier equals another.
- **TP-R4**: `Tier` is a closed `enum` with exactly four variants.
  Adding a variant requires inserting it in `TIER_NAMES` and `rank()`.

## Invariants
- **TP-I1**: For any `Tier t`, `t.can_access(t) == true` (reflexive).
- **TP-I2**: For any tiers `a, b`: if `a.can_access(b)` and
  `b.can_access(a)` then `a == b` (antisymmetric).
- **TP-I3**: For any tiers `a, b, c`: if `a.can_access(b)` and
  `b.can_access(c)` then `a.can_access(c)` (transitive).

## Errors
- **TP-E1** `InvalidTier`: Raised at the input boundary when a string is
  parsed via `Tier::try_from(&str)` and does not match any variant.
  Domain code assumes inputs are already valid `Tier` values.

## Acceptance scenarios (Given / When / Then)

### Access matrix (TP-R2)
- **TP-S1**: Given a Silver passenger and a Silver resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S2**: Given a Silver passenger and a Gold resource,
  When `can_access` is evaluated, Then it returns `false`.
- **TP-S3**: Given a Silver passenger and a Platinum resource,
  When `can_access` is evaluated, Then it returns `false`.
- **TP-S4**: Given a Gold passenger and a Silver resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S5**: Given a Gold passenger and a Gold resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S6**: Given a Gold passenger and a Platinum resource,
  When `can_access` is evaluated, Then it returns `false`.
- **TP-S7**: Given a Platinum passenger and a Silver resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S8**: Given a Platinum passenger and a Gold resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S9**: Given a Platinum passenger and a Platinum resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S16**: Given a Diamond passenger and a Silver resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S17**: Given a Diamond passenger and a Gold resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S18**: Given a Diamond passenger and a Diamond resource,
  When `can_access` is evaluated, Then it returns `true`.
- **TP-S19**: Given a Diamond passenger and a Platinum resource,
  When `can_access` is evaluated, Then it returns `false`.
- **TP-S20**: Given a Silver passenger and a Diamond resource,
  When `can_access` is evaluated, Then it returns `false`.
- **TP-S21**: Given a Gold passenger and a Diamond resource,
  When `can_access` is evaluated, Then it returns `false`.
- **TP-S22**: Given a Platinum passenger and a Diamond resource,
  When `can_access` is evaluated, Then it returns `true`.

### Rank (TP-R1)
- **TP-S10**: `Tier::Silver.rank() == 1`.
- **TP-S11**: `Tier::Gold.rank() == 2`.
- **TP-S12**: `Tier::Platinum.rank() == 4`.
- **TP-S23**: `Tier::Diamond.rank() == 3`.

### Parsing (TP-E1)
- **TP-S13**: `Tier::try_from("Silver")` returns `Ok(Tier::Silver)`.
- **TP-S14**: `Tier::try_from("platinum")` is case-sensitive and returns
  `Err(InvalidTier)`.
- **TP-S15**: `Tier::try_from("Bronze")` returns `Err(InvalidTier)`.
- **TP-S24**: `Tier::try_from("Diamond")` returns `Ok(Tier::Diamond)`.

## Out of scope
- Time-boxed / temporary upgrades (not in this challenge).
- Per-resource override of tier rules — a resource's `min_tier` is the
  only gate.
- Group / role permissions beyond the three tiers.

## Traceability
| Rule | Test(s)        | Implementation         |
|------|----------------|------------------------|
| TP-R1 | TP-S10..S12, TP-S23    | `domain/tier.rs: rank` |
| TP-R2 | TP-S1..S9, TP-S16..S22 | `domain/tier.rs: can_access` |
| TP-E1 | TP-S13..S15, TP-S24    | `domain/tier.rs: TryFrom<&str>` |
| TP-I1..I3 | covered by access-matrix scenarios | `domain/tier.rs` |
