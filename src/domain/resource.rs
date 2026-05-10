//! Resource entity. See `specs/04-resource.md` (RS).

use crate::domain::tier::Tier;
use crate::domain::timestamp::Timestamp;

// `PartialOrd`/`Ord` are derived here (unlike `PassengerId`) because the
// reporting service sorts resources by id when producing deterministic
// listings. Lexicographic order on the underlying String is fine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub String);

impl From<&str> for ResourceId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub id: ResourceId,
    pub name: String,
    // Free-form group label (e.g. "food", "medical"). Kept as String
    // because the spec does not enumerate categories.
    pub category: String,
    // Minimum membership tier required to access this resource (TP-R2).
    pub min_tier: Tier,
    // Soft-delete marker — same semantics as `Passenger::deleted_at`.
    pub deleted_at: Option<Timestamp>,
    /// Optimistic concurrency version. Starts at 0, incremented on every
    /// mutation. Reset to 0 on server restart (in-memory only).
    pub version: u64,
}
