//! The error type shared by every layer of the proxy.

/// Everything that can go wrong while answering a request.
///
/// `Clone` so one upstream fetch can hand its outcome to every waiting caller.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// The request is malformed: a crate name, version or index path with
    /// characters that are not allowed, or too many crates in one batch.
    #[error("invalid request: {0}")]
    BadRequest(String),
    /// Upstream has no such crate version. Not stored: it may be published later.
    #[error("crate version not found upstream")]
    NotFound,
    /// The tarball was read in full and has no `Cargo.toml`. Permanent, so
    /// the absence is stored.
    #[error("crate tarball contains no Cargo.toml")]
    NoManifest,
    /// The manifest is not UTF-8, is not TOML, or has neither a `[package]`
    /// nor a `[project]` table. The manifest itself is stored; it is parsed
    /// on every request.
    #[error("manifest could not be parsed: {0}")]
    InvalidManifest(String),
    /// crates.io could not be reached, or what it sent could not be used.
    /// Never stored, so the next request tries again.
    #[error("upstream request failed: {0}")]
    Upstream(String),
    /// The database could not be opened, read or written.
    #[error("storage failed: {0}")]
    Store(String),
}
