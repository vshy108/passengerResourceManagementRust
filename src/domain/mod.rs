//! Domain layer — pure types, value objects, and policies.
//!
//! No I/O, no clocks, no logging. Everything here is deterministic and
//! deeply testable.

// Each `pub mod` corresponds to a sibling `.rs` file in this directory.
// The presence of `mod.rs` is what makes `domain/` a module (older Rust
// convention; the newer alternative is a `domain.rs` file beside the
// folder — both work, this project uses `mod.rs`).
pub mod actor;
pub mod admin_event;
pub mod crew_lead;
pub mod errors;
pub mod passenger;
pub mod resource;
pub mod tier;
pub mod timestamp;
pub mod usage_event;
