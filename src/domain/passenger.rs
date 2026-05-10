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
    // `Option<T>` is Rust's null-safe "maybe a value":
    //   - Some(t) -> a value is present
    //   - None    -> no value
    // The compiler forces you to handle both cases (no NullPointerException).
    pub deleted_at: Option<Timestamp>,
    /// Optimistic concurrency version. Starts at 0, incremented on every
    /// mutation so callers can use `If-Match: "<version>"` to prevent
    /// lost-update races. Reset to 0 on server restart (in-memory only).
    pub version: u64,
}
