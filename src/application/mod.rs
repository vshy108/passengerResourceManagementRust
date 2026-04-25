//! Application layer — services that orchestrate domain types.
//!
//! Services depend on **trait** ports (when needed), never on concrete
//! infrastructure adapters.

pub mod crew_lead_service;
pub mod guards;
pub mod passenger_service;
pub mod ports;
