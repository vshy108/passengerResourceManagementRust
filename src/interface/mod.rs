//! Interface layer — adapters that translate between external
//! protocols (HTTP today, possibly CLI later) and the application
//! services. Thin: no business logic lives here.

pub mod composition_root;

// `#[cfg(feature = "http")]` is conditional compilation gated on a
// Cargo *feature* (declared in Cargo.toml under `[features]`). When
// the binary/tests opt out of `http`, these modules are excluded
// entirely — along with their axum/serde/tokio dependencies.
// Run with `cargo build --features http` to enable.
#[cfg(feature = "http")]
pub mod dto;
#[cfg(feature = "http")]
pub mod http;
