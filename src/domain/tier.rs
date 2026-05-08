// `//!` is an INNER doc comment: it documents the *file/module* itself
// (everything below). `///` (seen later) documents the *next item*.
//! Tier — ordered membership level. See `specs/01-tier-policy.md` (TP).

// `use` brings a path into scope so we can write `Error` instead of
// `thiserror::Error`. `thiserror` is a crate (3rd-party library) listed
// in Cargo.toml. `Error` here is a *derive macro* used below.
use thiserror::Error;

/// A passenger's membership tier. Higher rank inherits access to all
/// lower-rank resources (TP-R2). The set of variants is closed (TP-R4).
// `#[derive(...)]` auto-implements traits so we don't write the impls by hand:
//   Debug      -> printable with {:?} (for logs/tests/asserts)
//   Clone      -> explicit `.clone()` produces a copy
//   Copy       -> implicit bitwise copy on assignment/move (cheap, 1 byte here)
//                 Copy requires Clone. Only OK for tiny POD-like types.
//   PartialEq  -> enables `==` and `!=`
//   Eq         -> marker saying equality is reflexive (every value == itself).
//                 Required for use as a HashMap key (with Hash).
//   Hash       -> can be hashed; needed for HashMap/HashSet keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
// `pub` = visible outside this module. `enum` = sum type: a value is
// EXACTLY ONE of the listed variants (no nulls, no subclasses).
pub enum Tier {
    // Each variant is a unit variant (no payload). Order in source does
    // NOT define ordering for `<`/`>`; we use `rank()` instead (TP-R1).
    Silver,
    Gold,
    Diamond,
    Platinum,
}

/// Boundary error raised when an external string fails to parse as a
/// valid `Tier`. Domain code never constructs this — only the boundary
/// (interface layer / serde) does. See TP-E1.
// `Error` (from thiserror) auto-implements `std::error::Error` for us.
// We also derive PartialEq/Eq so tests can compare error values directly.
#[derive(Debug, Error, PartialEq, Eq)]
// `#[error("...")]` is thiserror's attribute that generates the
// `Display` impl. `{0:?}` formats the first tuple field with Debug
// (so the bad input is shown in quotes, including weird characters).
#[error("invalid tier: {0:?}")]
// Tuple struct with one public `String` field — accessed as `.0`.
// `String` (heap-allocated, owned) rather than `&str` so the error can
// outlive the input slice that produced it.
pub struct InvalidTier(pub String);

// `impl Tier { ... }` adds methods/associated functions to the type.
impl Tier {
    /// TP-R1 — numeric rank used for ordering.
    // `#[must_use]` makes the compiler warn if the caller ignores the
    // return value (this method has no side effects, so ignoring it is
    // almost always a bug).
    #[must_use]
    // `self` (no `&`) takes the value by value. Fine because Tier is
    // `Copy`, so the caller still has their original after the call.
    // Returns `u8` — smallest unsigned int (0..=255), plenty for 1..=3.
    pub fn rank(self) -> u8 {
        // `match` MUST be exhaustive: if we add `Diamond` to the enum
        // and forget this arm, the compiler errors here. That's the
        // safety net the spec relies on (TP-R4 "closed set").
        match self {
            // `=>` separates pattern from result expression.
            // No semicolon at the end of an arm's expression — it's the
            // value yielded by that arm, and the whole `match` becomes
            // the function's return value (no explicit `return` needed).
            Tier::Silver   => 1,
            Tier::Gold     => 2,
            Tier::Diamond  => 3,
            Tier::Platinum => 4,
        }
    }

    /// TP-R2 — `self` (passenger tier) can access a resource whose
    /// minimum required tier is `resource_min_tier`.
    #[must_use]
    // Two `Tier` params by value (Copy makes this free). Returns bool.
    pub fn can_access(self, resource_min_tier: Tier) -> bool {
        // We deliberately compare via `rank()` instead of deriving
        // PartialOrd on the enum. Reason: declaration order is a fragile
        // way to express domain ordering — `rank()` makes the intent
        // explicit and is the single source of truth (AGENTS.md §3).
        self.rank() >= resource_min_tier.rank()
    }
}

/// Canonical name → variant mapping. **To add a new tier: insert one entry
/// here in rank order.** `TryFrom<&str>` below is table-driven and needs no
/// other changes. You still need to add the variant above and a `rank()` arm
/// (the compiler will point at both).
const TIER_NAMES: &[(&str, Tier)] = &[
    ("Silver",   Tier::Silver),
    ("Gold",     Tier::Gold),
    ("Diamond",  Tier::Diamond),
    ("Platinum", Tier::Platinum),
];

// Implementing the `TryFrom<&str>` trait gives us `Tier::try_from("Gold")`
// AND, for free, `"Gold".try_into()` (via the blanket `Into`/`TryInto`).
// We use TryFrom (not From) because parsing can fail.
impl TryFrom<&str> for Tier {
    // Associated type: which error type `try_from` returns on failure.
    type Error = InvalidTier;

    /// TP-E1 — case-sensitive parse driven by `TIER_NAMES`. Adding a new
    /// tier requires only one line in that table, not a new match arm here.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        TIER_NAMES
            .iter()
            .find(|(name, _)| *name == value)
            .map(|(_, tier)| *tier)
            .ok_or_else(|| InvalidTier(value.to_owned()))
    }
}

// `#[cfg(test)]` = compile this module ONLY when running tests
// (`cargo test`/`cargo nextest run`). It is removed entirely from
// release builds, so test code never bloats the shipped binary.
#[cfg(test)]
mod tests {
    // `super` = the parent module (this file's top level). The `*` glob
    // imports everything `pub` from there — Tier, InvalidTier, etc.
    // Glob imports are usually discouraged, but inside a private test
    // module they're idiomatic for convenience.
    use super::*;

    // -- TP-R2 access matrix (TP-S1..S9) -------------------------------

    // `#[test]` marks a function as a test case. It must take no args
    // and return `()` (or `Result`). Test names mirror spec scenario
    // IDs per AGENTS.md §4 — makes failures traceable to the spec.
    #[test]
    fn tp_r2_s1_silver_can_access_silver() {
        // `assert!(expr)` panics if `expr` is false, which fails the test.
        // A panicking test = a failing test (that's how the runner detects it).
        assert!(Tier::Silver.can_access(Tier::Silver));
    }

    #[test]
    fn tp_r2_s2_silver_cannot_access_gold() {
        // `!` is boolean NOT — we expect `can_access` to return false here.
        assert!(!Tier::Silver.can_access(Tier::Gold));
    }

    #[test]
    fn tp_r2_s3_silver_cannot_access_platinum() {
        assert!(!Tier::Silver.can_access(Tier::Platinum));
    }

    #[test]
    fn tp_r2_s4_gold_can_access_silver() {
        assert!(Tier::Gold.can_access(Tier::Silver));
    }

    #[test]
    fn tp_r2_s5_gold_can_access_gold() {
        assert!(Tier::Gold.can_access(Tier::Gold));
    }

    #[test]
    fn tp_r2_s6_gold_cannot_access_platinum() {
        assert!(!Tier::Gold.can_access(Tier::Platinum));
    }

    #[test]
    fn tp_r2_s7_platinum_can_access_silver() {
        assert!(Tier::Platinum.can_access(Tier::Silver));
    }

    #[test]
    fn tp_r2_s8_platinum_can_access_gold() {
        assert!(Tier::Platinum.can_access(Tier::Gold));
    }

    #[test]
    fn tp_r2_s9_platinum_can_access_platinum() {
        assert!(Tier::Platinum.can_access(Tier::Platinum));
    }

    // -- TP-R1 rank (TP-S10..S12) --------------------------------------

    #[test]
    fn tp_r1_s10_rank_silver_is_one() {
        // `assert_eq!(a, b)` checks `a == b` and, on failure, prints
        // BOTH values via Debug — much friendlier than `assert!(a == b)`.
        // This is why we derived `Debug` and `PartialEq` on Tier.
        assert_eq!(Tier::Silver.rank(), 1);
    }

    #[test]
    fn tp_r1_s11_rank_gold_is_two() {
        assert_eq!(Tier::Gold.rank(), 2);
    }

    #[test]
    fn tp_r1_s12_rank_platinum_is_four() {
        assert_eq!(Tier::Platinum.rank(), 4);
    }

    #[test]
    fn tp_r1_s23_rank_diamond_is_three() {
        assert_eq!(Tier::Diamond.rank(), 3);
    }

    // -- TP-E1 parsing (TP-S13..S15) -----------------------------------

    #[test]
    fn tp_e1_s13_try_from_silver_ok() {
        // We can compare the whole Result<Tier, InvalidTier> with
        // `Ok(Tier::Silver)` because Result, Tier, AND InvalidTier all
        // implement PartialEq. Remove any of those derives and this
        // line stops compiling.
        assert_eq!(Tier::try_from("Silver"), Ok(Tier::Silver));
    }

    #[test]
    fn tp_e1_s14_try_from_lowercase_is_err() {
        // `.is_err()` is a Result method that returns true if it's an
        // Err variant. Used when we don't care which specific error.
        assert!(Tier::try_from("platinum").is_err());
    }

    #[test]
    fn tp_e1_s15_try_from_unknown_is_err() {
        assert!(Tier::try_from("Bronze").is_err());
    }

    #[test]
    fn tp_e1_s24_try_from_diamond_ok() {
        assert_eq!(Tier::try_from("Diamond"), Ok(Tier::Diamond));
    }

    // -- TP-S16..S22: Diamond access matrix ----------------------------

    #[test]
    fn tp_r2_s16_diamond_can_access_silver() {
        assert!(Tier::Diamond.can_access(Tier::Silver));
    }

    #[test]
    fn tp_r2_s17_diamond_can_access_gold() {
        assert!(Tier::Diamond.can_access(Tier::Gold));
    }

    #[test]
    fn tp_r2_s18_diamond_can_access_diamond() {
        assert!(Tier::Diamond.can_access(Tier::Diamond));
    }

    #[test]
    fn tp_r2_s19_diamond_cannot_access_platinum() {
        assert!(!Tier::Diamond.can_access(Tier::Platinum));
    }

    #[test]
    fn tp_r2_s20_silver_cannot_access_diamond() {
        assert!(!Tier::Silver.can_access(Tier::Diamond));
    }

    #[test]
    fn tp_r2_s21_gold_cannot_access_diamond() {
        assert!(!Tier::Gold.can_access(Tier::Diamond));
    }

    #[test]
    fn tp_r2_s22_platinum_can_access_diamond() {
        assert!(Tier::Platinum.can_access(Tier::Diamond));
    }
}
