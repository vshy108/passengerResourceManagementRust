//! Tier — ordered membership level. See `specs/01-tier-policy.md` (TP).

use thiserror::Error;

/// A passenger's membership tier. Higher rank inherits access to all
/// lower-rank resources (TP-R2). The set of variants is closed (TP-R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    Silver,
    Gold,
    Platinum,
}

/// Boundary error raised when an external string fails to parse as a
/// valid `Tier`. Domain code never constructs this — only the boundary
/// (interface layer / serde) does. See TP-E1.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid tier: {0:?}")]
pub struct InvalidTier(pub String);

impl Tier {
    /// TP-R1 — numeric rank used for ordering.
    #[must_use]
    pub fn rank(self) -> u8 {
        match self {
            Tier::Silver => 1,
            Tier::Gold => 2,
            Tier::Platinum => 3,
        }
    }

    /// TP-R2 — `self` (passenger tier) can access a resource whose
    /// minimum required tier is `resource_min_tier`.
    #[must_use]
    pub fn can_access(self, resource_min_tier: Tier) -> bool {
        self.rank() >= resource_min_tier.rank()
    }
}

impl TryFrom<&str> for Tier {
    type Error = InvalidTier;

    /// TP-E1 — case-sensitive parse from the canonical capitalised form.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Silver" => Ok(Tier::Silver),
            "Gold" => Ok(Tier::Gold),
            "Platinum" => Ok(Tier::Platinum),
            other => Err(InvalidTier(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- TP-R2 access matrix (TP-S1..S9) -------------------------------

    #[test]
    fn tp_r2_s1_silver_can_access_silver() {
        assert!(Tier::Silver.can_access(Tier::Silver));
    }

    #[test]
    fn tp_r2_s2_silver_cannot_access_gold() {
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
        assert_eq!(Tier::Silver.rank(), 1);
    }

    #[test]
    fn tp_r1_s11_rank_gold_is_two() {
        assert_eq!(Tier::Gold.rank(), 2);
    }

    #[test]
    fn tp_r1_s12_rank_platinum_is_three() {
        assert_eq!(Tier::Platinum.rank(), 3);
    }

    // -- TP-E1 parsing (TP-S13..S15) -----------------------------------

    #[test]
    fn tp_e1_s13_try_from_silver_ok() {
        assert_eq!(Tier::try_from("Silver"), Ok(Tier::Silver));
    }

    #[test]
    fn tp_e1_s14_try_from_lowercase_is_err() {
        assert!(Tier::try_from("platinum").is_err());
    }

    #[test]
    fn tp_e1_s15_try_from_unknown_is_err() {
        assert!(Tier::try_from("Bronze").is_err());
    }
}
