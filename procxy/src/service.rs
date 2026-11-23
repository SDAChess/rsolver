//! The proxy's logic, independent of HTTP.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use futures_util::{
    FutureExt,
    future::{BoxFuture, Shared},
};

use crate::{
    CrateVersion, Error,
    manifest::{Manifest, Metadata},
    store::Store,
    upstream::Upstream,
};

type Fetch = Shared<BoxFuture<'static, Result<Manifest, Error>>>;

/// Answers metadata requests: store lookup, shared upstream fetch, parse.
///
/// Concurrent requests for a version that is not stored yet share one upstream
/// fetch. Once a fetch has found the manifest, or established that there is
/// none, and that has been stored, the version is never fetched again.
pub struct Service {
    store: Store,
    upstream: Upstream,
    in_flight: Mutex<HashMap<CrateVersion, Fetch>>,
}

impl Service {
    /// Creates the service over a store and a source of manifests.
    ///
    /// It is returned in an [`Arc`] because the fetches it starts outlive the
    /// requests that start them.
    pub fn new(store: Store, upstream: Upstream) -> Arc<Self> {
        Arc::new(Self {
            store,
            upstream,
            in_flight: Mutex::default(),
        })
    }

    /// Metadata for one crate version, fetched upstream on first use.
    ///
    /// A fetch that this call starts or joins keeps running if the returned
    /// future is dropped, so its result is still stored. If storing a fetched
    /// manifest fails, this call still answers from it and a later call
    /// fetches again.
    ///
    /// # Errors
    ///
    /// - [`Error::BadRequest`] if `name` or `version` is malformed, see
    ///   [`CrateVersion::parse`].
    /// - [`Error::NotFound`] if crates.io has no such crate version.
    /// - [`Error::NoManifest`] or [`Error::InvalidManifest`] if the crate has
    ///   no usable `Cargo.toml`, see [`Manifest::metadata`].
    /// - [`Error::Upstream`] if the fetch failed; the next call tries again.
    /// - [`Error::Store`] if the store could not be read.
    pub async fn metadata(self: &Arc<Self>, name: &str, version: &str) -> Result<Metadata, Error> {
        let krate = CrateVersion::parse(name, version)?;
        let manifest = match self.store.get(&krate).await? {
            Some(manifest) => manifest,
            None => self.fetch(&krate).await?,
        };
        manifest.metadata()
    }

    /// Joins the in-flight fetch for this version, starting one if there is none.
    ///
    /// The fetch runs in a detached task, so a dropped caller cannot cancel it
    /// for the others; callers only hold a shared handle to its result. The
    /// task removes its own map entry once the store has been written (or the
    /// fetch failed), so the map only holds fetches that are running and a
    /// failed one is retried on the next request.
    async fn fetch(self: &Arc<Self>, krate: &CrateVersion) -> Result<Manifest, Error> {
        let fetch = self
            .in_flight()
            .entry(krate.clone())
            .or_insert_with(|| {
                // Spawned under the lock: the task's removal blocks until
                // its entry has been inserted.
                let task = tokio::spawn(Arc::clone(self).fetch_and_store(krate.clone()));
                task.map(|joined| {
                    joined
                        .unwrap_or_else(|e| Err(Error::Upstream(format!("fetch task failed: {e}"))))
                })
                .boxed()
                .shared()
            })
            .clone();
        fetch.await
    }

    async fn fetch_and_store(self: Arc<Self>, krate: CrateVersion) -> Result<Manifest, Error> {
        // Removes the entry even if this task panics.
        let _entry = Entry {
            service: &self,
            krate: &krate,
        };
        // A previous fetch may have stored the result between the caller's
        // store lookup and the creation of this task.
        if let Some(manifest) = self.store.get(&krate).await? {
            return Ok(manifest);
        }
        let manifest = self.upstream.manifest(&krate).await?;
        // The manifest is right whether or not it could be written; if not,
        // the store has logged why and it is fetched again next time.
        let _ = self.store.put(&krate, &manifest).await;
        Ok(manifest)
    }

    fn in_flight(&self) -> MutexGuard<'_, HashMap<CrateVersion, Fetch>> {
        // The map is only inserted into and removed from, so it is consistent
        // even if a holder of the lock panicked.
        self.in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

struct Entry<'a> {
    service: &'a Service,
    krate: &'a CrateVersion,
}

impl Drop for Entry<'_> {
    fn drop(&mut self) {
        self.service.in_flight().remove(self.krate);
    }
}
