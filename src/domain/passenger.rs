//! Passenger entity. See `specs/03-passenger.md` (PS).

use crate::domain::tier::Tier;
use crate::domain::timestamp::Timestamp;

/// Newtype around a passenger identifier. Distinct from `CrewLeadId` /
/// `ResourceId` to prevent type-level mix-ups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PassengerId(pub String);

impl From<&str> for PassengerId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passenger {
    pub id: PassengerId,
    pub name: String,
    pub tier: Tier,
    /// Soft-delete marker. `None` means the record is active. Once set
    /// it is immutable (PS-I2).
    pub deleted_at: Option<Timestamp>,
}
