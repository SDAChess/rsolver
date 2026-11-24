//! Metadata proxy for crates.io.
//!
//! Passes upstream's index through unchanged and serves per-version metadata
//! such as license or proc-macro information derived from the `Cargo.toml`
//! inside each crate.

pub mod crate_version;
pub mod error;
pub mod index;
pub mod manifest;
pub mod server;
pub mod service;
pub mod store;
pub mod upstream;

pub use crate_version::CrateVersion;
pub use error::Error;
