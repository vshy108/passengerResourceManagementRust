//! Application layer — services that orchestrate domain types.
//!
//! Services depend on **trait** ports (when needed), never on concrete
//! infrastructure adapters.

// Each service handles one aggregate (Single Responsibility Principle).
// `ports` defines the trait contracts the services depend on; concrete
// adapters live in the `infrastructure` module.
pub mod access_service;
pub mod crew_lead_service;
pub mod guards;
pub mod passenger_service;
pub mod ports;
pub mod reporting_service;
pub mod resource_service;
