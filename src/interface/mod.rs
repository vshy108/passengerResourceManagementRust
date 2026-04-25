//! Interface layer — adapters that translate between external
//! protocols (HTTP today, possibly CLI later) and the application
//! services. Thin: no business logic lives here.

pub mod composition_root;

#[cfg(feature = "http")]
pub mod dto;
#[cfg(feature = "http")]
pub mod http;
