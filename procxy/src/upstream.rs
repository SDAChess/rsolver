//! Reading manifests from crates.io without downloading whole crates.

mod archive;
mod prefix;

use std::time::Duration;

use tokio::sync::Semaphore;

use crate::{CrateVersion, Error, manifest::Manifest};

use archive::Scan;
use prefix::Prefix;

const USER_AGENT: &str = concat!(
    "procxy/",
    env!("CARGO_PKG_VERSION"),
    " (github.com/sdachess/rsolver)"
);
const CONCURRENT_FETCHES: usize = 16;

/// The HTTP client for every request to crates.io.
/// It identifies the proxy in its `User-Agent`.
///
/// # Errors
///
/// Fails if the TLS backend cannot be initialised.
pub fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
}

/// Reads manifests out of the `.crate` tarballs on static.crates.io.
pub struct Upstream {
    client: reqwest::Client,
    fetches: Semaphore,
}

impl Upstream {
    /// Creates the reader over an HTTP client, normally the one from [`client`].
    pub fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            fetches: Semaphore::new(CONCURRENT_FETCHES),
        }
    }

    /// Fetches the `Cargo.toml` of one crate version, downloading only as much
    /// of the tarball as it takes to reach it.
    ///
    /// [`Manifest::Absent`] is only returned once the whole tarball has been
    /// downloaded and decoded through its checksum.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`] if crates.io has no such crate version.
    /// - [`Error::Upstream`] if a request fails, the responses do not fit
    ///   together (e.g. the file changed during the download), the tarball is
    ///   corrupt, or the tarball or its manifest exceeds a size limit.
    pub async fn manifest(&self, krate: &CrateVersion) -> Result<Manifest, Error> {
        let _permit = self
            .fetches
            .acquire()
            .await
            .map_err(|e| Error::Upstream(e.to_string()))?;
        self.read_manifest(krate).await.inspect_err(|error| {
            if let Error::Upstream(_) = error {
                let (name, version) = (krate.name(), krate.version());
                tracing::warn!(name, version, %error, "upstream failure");
            }
        })
    }

    async fn read_manifest(&self, krate: &CrateVersion) -> Result<Manifest, Error> {
        let (name, version) = (krate.name(), krate.version().replace('+', "%2B"));
        let url = format!("https://static.crates.io/crates/{name}/{name}-{version}.crate");

        // A `.crate` is a gzipped tarfile and cargo seems to pack `Cargo.toml` near the
        // front, so a short prefix of the compressed stream usually decodes far
        // enough to reach it. Fetch growing prefixes until it does.
        let mut prefix = Prefix::open(&self.client, url).await?;
        loop {
            let (scanned, scan) = tokio::task::spawn_blocking(move || {
                let scan = archive::scan(prefix.bytes());
                (prefix, scan)
            })
            .await
            .map_err(|e| Error::Upstream(format!("decoding task failed: {e}")))?;

            prefix = match scan {
                Scan::Manifest(toml) => return Ok(Manifest::Found(toml.into())),
                Scan::TooLarge(what) => {
                    return Err(scanned.error(format_args!("{what} is too large")));
                }
                // Since we cache the absence of a crate's manifest, we need to make
                // sure we've actually fetched the whole crate _and_ checked the checksum
                // before makign that conclusion.
                Scan::Absent if scanned.is_complete() => return Ok(Manifest::Absent),
                Scan::Undecodable(e) if scanned.is_complete() => {
                    return Err(scanned.error(format_args!("corrupt: {e}")));
                }
                Scan::Absent | Scan::Undecodable(_) => scanned.extend().await?,
            };
        }
    }
}
