//! Domain layer — pure types, value objects, and policies.
//!
//! No I/O, no clocks, no logging. Everything here is deterministic and
//! deeply testable.

pub mod actor;
pub mod admin_event;
pub mod crew_lead;
pub mod errors;
pub mod passenger;
pub mod resource;
pub mod tier;
pub mod timestamp;
pub mod usage_event;
