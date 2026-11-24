//! Manifests as they are fetched and stored, and the metadata read from them.

use std::sync::Arc;

use serde::Serialize;
use toml::{Table, Value};

use crate::Error;

/// The `Cargo.toml` of one crate version as it was fetched: what the store
/// holds and what one fetch hands to every waiting caller.
#[derive(Debug, Clone)]
pub enum Manifest {
    /// The file's bytes, unexamined.
    Found(Arc<[u8]>),
    /// The tarball was read in full and has no `Cargo.toml`.
    Absent,
}

impl Manifest {
    /// Parses the manifest and extracts what the proxy reports.
    ///
    /// Parsing is lenient: a field of an unexpected type counts as absent, and
    /// the spellings of old cargo versions (`[project]`, `license_file`,
    /// `proc_macro`, `crate_type`) are accepted.
    ///
    /// # Errors
    ///
    /// - [`Error::NoManifest`] if this is [`Manifest::Absent`].
    /// - [`Error::InvalidManifest`] if the file is not UTF-8, is not TOML or
    ///   has neither a `[package]` nor a `[project]` table.
    pub fn metadata(&self) -> Result<Metadata, Error> {
        match self {
            Manifest::Found(toml) => Metadata::parse(toml),
            Manifest::Absent => Err(Error::NoManifest),
        }
    }
}

/// What the proxy reports about one crate version, derived from its `Cargo.toml`.
#[derive(Debug, Serialize)]
pub struct Metadata {
    /// `package.license` as written: usually an SPDX expression such as
    /// `MIT OR Apache-2.0`, but it is not validated and old crates hold other
    /// forms such as `MIT/Apache-2.0`.
    pub license: Option<String>,
    /// `package.license-file` as written: the path, inside the crate, of a
    /// license text. A crate may set it, `license`, both or neither.
    pub license_file: Option<String>,
    /// Whether the library is a procedural macro: `lib.proc-macro` is true or
    /// `lib.crate-type` contains `proc-macro`.
    pub proc_macro: bool,
}

impl Metadata {
    fn parse(manifest: &[u8]) -> Result<Self, Error> {
        let manifest: Table =
            toml::from_slice(manifest).map_err(|e| Error::InvalidManifest(e.to_string()))?;
        let package = first(&manifest, &["package", "project"], Value::as_table)
            .ok_or_else(|| Error::InvalidManifest("no [package] table".into()))?;
        let lib = first(&manifest, &["lib"], Value::as_table);
        let proc_macro = lib.is_some_and(|lib| {
            first(lib, &["proc-macro", "proc_macro"], Value::as_bool) == Some(true)
                || first(lib, &["crate-type", "crate_type"], Value::as_array)
                    .is_some_and(|types| types.iter().any(|t| t.as_str() == Some("proc-macro")))
        });
        let text = |keys| first(package, keys, Value::as_str).map(str::to_owned);
        Ok(Self {
            license: text(&["license"]),
            license_file: text(&["license-file", "license_file"]),
            proc_macro,
        })
    }
}

/// The value of the first of `keys` that has the type `get` asks for.
///
/// Published manifests span every cargo version ever released, so a value of
/// an unexpected type counts as absent instead of rejecting the manifest, and
/// an old spelling is used only where the current one is unusable.
fn first<'a, T>(
    table: &'a Table,
    keys: &[&str],
    get: impl Fn(&'a Value) -> Option<T>,
) -> Option<T> {
    keys.iter().find_map(|key| table.get(*key).and_then(&get))
}
