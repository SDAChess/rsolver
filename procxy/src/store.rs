//! Persistent storage of fetched manifests.

use std::{path::Path, sync::Arc, time::Duration};

use rusqlite::{Connection, OptionalExtension, params};
use tokio::sync::Mutex;

use crate::{CrateVersion, Error, manifest::Manifest};

/// SQLite-backed store of manifests. Rows are written once and never invalidated.
///
/// All access goes through one connection, one query at a time. A failed
/// [`get`](Self::get) or [`put`](Self::put) is logged here, so callers need
/// not report it again.
pub struct Store {
    conn: Arc<Mutex<Connection>>,
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS manifest (
    name       TEXT NOT NULL,
    version    TEXT NOT NULL,
    toml       BLOB,              -- NULL: the tarball has no Cargo.toml
    fetched_at INTEGER NOT NULL,  -- unix seconds
    PRIMARY KEY (name, version)
) STRICT;
";

impl Store {
    /// Opens the database at `path`, creating it and its schema if needed.
    ///
    /// # Errors
    ///
    /// [`Error::Store`] if the file cannot be opened or created, or the
    /// schema cannot be applied.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// The stored manifest of a crate version, or `None` if none is stored.
    ///
    /// # Errors
    ///
    /// [`Error::Store`] if the query fails.
    pub async fn get(&self, krate: &CrateVersion) -> Result<Option<Manifest>, Error> {
        let krate = krate.clone();
        self.with_conn(move |conn| {
            conn.prepare_cached("SELECT toml FROM manifest WHERE name = ?1 AND version = ?2")?
                .query_row(params![krate.name(), krate.version()], |row| {
                    row.get::<_, Option<Arc<[u8]>>>(0)
                })
                .optional()
                .map(|row| row.map(|toml| toml.map_or(Manifest::Absent, Manifest::Found)))
        })
        .await
    }

    /// Stores the manifest of a crate version. A version that is already
    /// stored keeps what it has.
    ///
    /// # Errors
    ///
    /// [`Error::Store`] if the insert fails.
    pub async fn put(&self, krate: &CrateVersion, manifest: &Manifest) -> Result<(), Error> {
        let (krate, manifest) = (krate.clone(), manifest.clone());
        self.with_conn(move |conn| {
            let toml = match &manifest {
                Manifest::Found(toml) => Some(&**toml),
                Manifest::Absent => None,
            };
            conn.prepare_cached(
                "INSERT INTO manifest (name, version, toml, fetched_at)
                 VALUES (?1, ?2, ?3, unixepoch()) ON CONFLICT DO NOTHING",
            )?
            .execute(params![krate.name(), krate.version(), toml])
            .map(drop)
        })
        .await
    }

    /// Runs `f` on a blocking thread and logs its failure.
    async fn with_conn<T, F>(&self, f: F) -> Result<T, Error>
    where
        T: Send + 'static,
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        // Queue for the connection on the executor: a blocking thread is only
        // taken once the connection is free, so one is in use at a time.
        let conn = Arc::clone(&self.conn).lock_owned().await;
        tokio::task::spawn_blocking(move || f(&conn))
            .await
            .map_err(|e| Error::Store(e.to_string()))
            .and_then(|result| result.map_err(Error::from))
            .inspect_err(|error| tracing::error!(%error, "store failure"))
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Error::Store(e.to_string())
    }
}
