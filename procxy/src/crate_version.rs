//! The validated identity of one crate version.

use std::sync::Arc;

use crate::Error;

/// A crate name and version.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrateVersion {
    name: Arc<str>,
    version: Arc<str>,
}

impl CrateVersion {
    /// Validates a crate's name and version.
    ///
    /// We only check the validity of the characters, not semver.
    ///
    /// # Errors
    ///
    /// [`Error::BadRequest`] unless `name` is 1 to 64 ASCII letters, digits,
    /// `-` or `_`, and `version` is 1 to 256 ASCII letters, digits, `.`, `-`
    /// or `+`.
    pub fn parse(name: &str, version: &str) -> Result<Self, Error> {
        let valid = |s: &str, max: usize, extra: &[u8]| {
            (1..=max).contains(&s.len())
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || extra.contains(&b))
        };
        if !valid(name, 64, b"-_") {
            return Err(Error::BadRequest(format!("invalid crate name {name:?}")));
        }
        if !valid(version, 256, b".-+") {
            return Err(Error::BadRequest(format!("invalid version {version:?}")));
        }
        Ok(Self {
            name: name.into(),
            version: version.into(),
        })
    }

    /// The crate's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The version. A `+` before build metadata is not percent-encoded.
    pub fn version(&self) -> &str {
        &self.version
    }
}
