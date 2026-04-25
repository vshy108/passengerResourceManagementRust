//! Domain error type. See spec files (CL-E*, TP-E*, …).

use thiserror::Error;

/// Closed sum of all errors the domain can raise. Every public error
/// variant is also documented with its spec ID in the corresponding
/// `specs/*.md` file.
///
/// Marked `#[non_exhaustive]` so adding a new variant in a future slice
/// does not break downstream `match` arms outside this crate.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum DomainError {
    /// CL-E1.
    #[error("crew lead limit reached (max 3)")]
    CrewLeadLimitReached,

    /// CL-E2.
    #[error("crew lead minimum breached (must keep 3)")]
    CrewLeadMinimumBreached,

    /// CL-E3.
    #[error("crew lead already exists")]
    CrewLeadAlreadyExists,

    /// CL-E4.
    #[error("crew lead not found")]
    CrewLeadNotFound,

    /// CL-E5.
    #[error("crew lead bootstrap invalid")]
    CrewLeadBootstrapInvalid,

    /// PS-E1 / RS-E1 / AC-E1 — actor lacked permission for the operation.
    #[error("unauthorized actor")]
    UnauthorizedActor,

    /// PS-E2.
    #[error("passenger already exists")]
    PassengerAlreadyExists,

    /// PS-E3.
    #[error("passenger not found")]
    PassengerNotFound,

    /// RS-E2.
    #[error("resource already exists")]
    ResourceAlreadyExists,

    /// RS-E3.
    #[error("resource not found")]
    ResourceNotFound,
}
