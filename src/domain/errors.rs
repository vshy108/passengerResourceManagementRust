//! Domain error type. See spec files (CL-E*, TP-E*, …).

use thiserror::Error;

/// Closed sum of all errors the domain can raise. Every public error
/// variant is also documented with its spec ID in the corresponding
/// `specs/*.md` file.
///
/// Marked `#[non_exhaustive]` so adding a new variant in a future slice
/// does not break downstream `match` arms outside this crate.
// `#[non_exhaustive]` on a public enum forces external `match` blocks to
// include a wildcard `_ => ...` arm. Adding a new variant later is then
// a NON-breaking change for consumers. Internal `match`es in this crate
// are still required to be exhaustive.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainError {
    /// CL-E1 — raised by `CrewLeadService::add` when the count is
    /// already 3. Use `replace` to rotate a lead instead.
    #[error("crew lead limit reached (max 3)")]
    CrewLeadLimitReached,

    /// CL-E2 — raised by `CrewLeadService::remove`. Removal is always
    /// rejected because it would break the exactly-3 invariant (CL-I1).
    /// Use `replace(old_id, new_lead)` to swap a lead atomically.
    #[error("crew lead minimum breached (must keep 3)")]
    CrewLeadMinimumBreached,

    /// CL-E3 — raised by `add` / `replace` when the incoming lead's id
    /// duplicates an existing lead. IDs must be globally unique (CL-I2).
    #[error("crew lead already exists")]
    CrewLeadAlreadyExists,

    /// CL-E4 — raised by `remove` / `replace` when `old_id` is not a
    /// current Crew Lead.
    #[error("crew lead not found")]
    CrewLeadNotFound,

    /// CL-E5 — raised by `bootstrap` when the seed slice has fewer or
    /// more than exactly 3 leads, or contains duplicate ids.
    #[error("crew lead bootstrap invalid")]
    CrewLeadBootstrapInvalid,

    /// PS-E1 / RS-E1 / AC-E1 — raised whenever a non-Crew-Lead actor
    /// calls a mutation that requires Crew Lead permission, OR a
    /// non-Passenger actor calls `use_resource`.
    #[error("unauthorized actor")]
    UnauthorizedActor,

    /// PS-E2 — raised by `PassengerService::create` when an active
    /// passenger with the same id already exists. Re-creating a
    /// soft-deleted id is allowed (PS-R6).
    #[error("passenger already exists")]
    PassengerAlreadyExists,

    /// PS-E3 / AC-E3 — raised when a passenger id is not found in the
    /// active list, or the record is soft-deleted.
    #[error("passenger not found")]
    PassengerNotFound,

    /// RS-E2 — raised by `ResourceService::create` when an active
    /// resource with the same id already exists.
    #[error("resource already exists")]
    ResourceAlreadyExists,

    /// RS-E3 / AC-E4 — raised when a resource id is not found in the
    /// active catalog, or the record is soft-deleted.
    #[error("resource not found")]
    ResourceNotFound,

    /// AC-E2 — raised (and a `Denied` `UsageEvent` still emitted) when
    /// the passenger's tier rank is below the resource's `min_tier`
    /// rank. See TP-R2.
    #[error("access denied")]
    AccessDenied,

    /// Raised by HTTP handlers when a client-supplied `If-Match` version
    /// does not match the entity's current version — another mutation
    /// occurred since the client last read the record.
    #[error("version conflict — record was modified by another request")]
    VersionConflict,
}
