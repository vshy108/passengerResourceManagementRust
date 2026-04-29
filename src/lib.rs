//! Spaceship X26 — Passenger Resource Management System (PRMS).
//!
//! Library crate root. Modules are organised as layers:
//! `domain` → `application` → `infrastructure` → `interface`.
//! Dependencies point inward only.

// `lib.rs` is the LIBRARY crate root. Cargo also recognises `src/main.rs`
// (binary root) or files under `src/bin/*.rs` (this repo uses
// `src/bin/serve.rs`). Binaries link against the library declared here.
//
// `pub mod foo;` tells Rust: "there is a module `foo` whose code lives
// in either `src/foo.rs` or `src/foo/mod.rs`". Without this declaration
// the compiler IGNORES the file — Rust modules are explicit, not
// auto-discovered like in some other languages.
// `pub` makes the module reachable from outside the crate.
pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod interface;
