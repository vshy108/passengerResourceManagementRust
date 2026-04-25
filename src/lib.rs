//! Spaceship X26 — Passenger Resource Management System (PRMS).
//!
//! Library crate root. Modules are organised as layers:
//! `domain` → `application` → `infrastructure` → `interface`.
//! Dependencies point inward only.

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;
