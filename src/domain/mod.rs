//! Domain layer — pure types, value objects, and policies.
//!
//! No I/O, no clocks, no logging. Everything here is deterministic and
//! deeply testable.

pub mod crew_lead;
pub mod errors;
pub mod tier;
