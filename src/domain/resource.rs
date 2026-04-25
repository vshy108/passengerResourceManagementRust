//! Resource entity. See `specs/04-resource.md` (RS).

use crate::domain::tier::Tier;
use crate::domain::timestamp::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    pub category: String,
    pub min_tier: Tier,
    pub deleted_at: Option<Timestamp>,
}
