//! Crew Lead entity. See `specs/02-crew-lead.md`.

/// Newtype around a Crew Lead identifier. Keeping it as a distinct type
/// (rather than a bare `String`) prevents accidental mix-ups with
/// `PassengerId` or `ResourceId` at the type level.
// `Hash` is included so the type can be used as a HashMap key (the
// in-memory crew-lead repository keys by `CrewLeadId`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrewLeadId(pub String);

// Implementing `From<&str>` is the conventional Rust way to provide a
// conversion. It also gives us the inverse `Into<CrewLeadId>` for free.
// Tests can write `CrewLeadId::from("alice")` or `"alice".into()`.
impl From<&str> for CrewLeadId {
    fn from(value: &str) -> Self {
        // `Self` here is `CrewLeadId`. `Self(...)` constructs the tuple
        // struct. `to_owned()` on a `&str` allocates a heap `String` —
        // necessary because we can't store the borrow long-term.
        Self(value.to_owned())
    }
}

/// A Crew Lead administrator.
// Plain data struct. Public fields are fine because invariants (count
// limits, uniqueness, etc.) are enforced by the *service*, not the
// struct itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewLead {
    pub id: CrewLeadId,
    pub name: String,
}
