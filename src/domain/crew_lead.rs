//! Crew Lead entity. See `specs/02-crew-lead.md`.

/// Newtype around a Crew Lead identifier. Keeping it as a distinct type
/// (rather than a bare `String`) prevents accidental mix-ups with
/// `PassengerId` or `ResourceId` at the type level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrewLeadId(pub String);

impl From<&str> for CrewLeadId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

/// A Crew Lead administrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewLead {
    pub id: CrewLeadId,
    pub name: String,
}
