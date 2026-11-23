//! Downloading the start of a `.crate` file with HTTP Range requests.

use std::fmt::Display;

use reqwest::StatusCode;
use reqwest::header::{ACCEPT_ENCODING, CONTENT_RANGE, ETAG, HeaderValue, RANGE};

use crate::Error;

const FIRST_CHUNK: u64 = 16 * 1024;
/// Crates are uploaded by anyone, so no more than this is downloaded of one.
/// Well above what crates.io accepts.
const MAX_CRATE_SIZE: u64 = 64 * 1024 * 1024;
const GZIP_MAGIC: [u8; 2] = [0x1f, 0x8b];

/// The first bytes of one `.crate` file, fetched with Range requests.
///
/// Fetching consumes the prefix and gives it back only if the response fits
/// what is already held, so a prefix that exists is always the start of one
/// unchanged object.
pub(super) struct Prefix {
    client: reqwest::Client,
    url: String,
    bytes: Vec<u8>,
    /// The size of the whole object, once the server has given it: in a
    /// `Content-Range`, or by sending the whole object.
    total: Option<u64>,
    etag: Option<HeaderValue>,
}

impl Prefix {
    /// Fetches the first chunk of the gzip stream at `url`.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`] if the server has no such object.
    /// - [`Error::Upstream`] if the request fails, or the response is not the
    ///   start of a gzip stream.
    pub async fn open(client: &reqwest::Client, url: String) -> Result<Self, Error> {
        let empty = Self {
            client: client.clone(),
            url: url.clone(),
            bytes: Vec::new(),
            total: None,
            etag: None,
        };
        let opened = match empty.fetch(FIRST_CHUNK).await {
            Ok(prefix) if !prefix.bytes.starts_with(&GZIP_MAGIC) => {
                Err(prefix.error("not a gzip stream"))
            }
            opened => opened,
        };
        // static.crates.io answers a Range request for a missing object with
        // 206 and the S3 error document as body, under headers that do not
        // describe that body. Neither a failed first read nor a body that is
        // not gzip says whether the object exists, so ask without a range.
        match opened {
            Err(error @ Error::Upstream(_)) => match client.head(&url).send().await {
                Ok(response) if is_missing(response.status()) => Err(Error::NotFound),
                _ => Err(error),
            },
            opened => opened,
        }
    }

    /// Roughly doubles an incomplete prefix.
    ///
    /// # Errors
    ///
    /// - [`Error::NotFound`] if the object has disappeared.
    /// - [`Error::Upstream`] if the request fails, the response does not
    ///   continue what is held (another size or `ETag`), or the download limit
    ///   is reached.
    pub async fn extend(self) -> Result<Self, Error> {
        let held = self.bytes.len() as u64;
        if held >= MAX_CRATE_SIZE {
            return Err(self.error(format_args!("gave up after {held} bytes")));
        }
        let end = (held * 2)
            .min(self.total.unwrap_or(u64::MAX))
            .min(MAX_CRATE_SIZE);
        self.fetch(end).await
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Whether this is the whole object. Only a size the server gave makes it
    /// so, never a response that came back short: what is concluded from a
    /// complete object is stored forever.
    pub fn is_complete(&self) -> bool {
        self.total == Some(self.bytes.len() as u64)
    }

    /// An [`Error::Upstream`] whose message starts with this object's URL.
    pub fn error(&self, what: impl Display) -> Error {
        Error::Upstream(format!("{}: {what}", self.url))
    }

    /// Asks for the bytes from the end of the prefix up to `end` (exclusive)
    /// and takes in the answer: that range (206), the whole object if the
    /// server ignored the range (200), or that the object ends where the
    /// prefix does (416). The size and `ETag` are learned on the way.
    async fn fetch(mut self, end: u64) -> Result<Self, Error> {
        let start = self.bytes.len() as u64;
        let mut response = self
            .client
            .get(&self.url)
            .header(RANGE, format!("bytes={start}-{}", end - 1))
            .header(ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(|e| self.error(e))?;
        let etag = response.headers().get(ETAG).cloned();
        let content_range = response
            .headers()
            .get(CONTENT_RANGE)
            .and_then(|v| v.to_str().ok());
        let total = match response.status() {
            StatusCode::PARTIAL_CONTENT => {
                // Ranges of two different objects must never be joined. Sending
                // `If-Range` would be the standard way, but static.crates.io
                // answers it with the whole object even when it matches.
                if !agree(&mut self.etag, etag) {
                    return Err(self.error("object changed between requests"));
                }
                let (first, last, total) = content_range
                    .and_then(parse_content_range)
                    .ok_or_else(|| self.error("bad Content-Range"))?;
                // static.crates.io echoes the requested last byte even when
                // the object ends before it.
                let last = total.map_or(last, |total| last.min(total.saturating_sub(1)));
                if first != start || last < first || last >= end {
                    return Err(self.error("unexpected Content-Range"));
                }
                self.append(&mut response, last - first + 1).await?;
                if self.bytes.len() as u64 != last + 1 {
                    return Err(self.error("body shorter than its Content-Range"));
                }
                total
            }
            // The server ignored the range and sent the whole object.
            StatusCode::OK => {
                self.bytes.clear();
                self.append(&mut response, MAX_CRATE_SIZE).await?;
                Some(self.bytes.len() as u64)
            }
            // The prefix ends exactly at the end of an object whose size the
            // server had not stated yet.
            StatusCode::RANGE_NOT_SATISFIABLE => {
                let total = content_range
                    .and_then(|v| v.strip_prefix("bytes */"))
                    .and_then(|v| v.parse().ok());
                if total != Some(start) {
                    return Err(self.error("range not satisfiable"));
                }
                total
            }
            status if is_missing(status) => return Err(Error::NotFound),
            status => return Err(self.error(format_args!("status {status}"))),
        };
        if !agree(&mut self.total, total) {
            return Err(self.error("object changed size between requests"));
        }
        Ok(self)
    }

    /// Appends the body, refusing to read more than `limit` bytes of it.
    async fn append(&mut self, response: &mut reqwest::Response, limit: u64) -> Result<(), Error> {
        let mut read = 0;
        while let Some(chunk) = response.chunk().await.map_err(|e| self.error(e))? {
            read += chunk.len() as u64;
            if read > limit {
                return Err(self.error(format_args!("body exceeds {limit} bytes")));
            }
            self.bytes.extend_from_slice(&chunk);
        }
        Ok(())
    }
}

/// Records the first value the server states. Afterwards, false if it states
/// a different one; stating none is not a contradiction.
fn agree<T: PartialEq>(known: &mut Option<T>, stated: Option<T>) -> bool {
    match known {
        Some(known) => stated.is_none_or(|stated| *known == stated),
        None => {
            *known = stated;
            true
        }
    }
}

/// Whether a status means the object does not exist. An S3 bucket that does
/// not allow listing answers 403, not 404, for a missing object.
fn is_missing(status: StatusCode) -> bool {
    matches!(status, StatusCode::NOT_FOUND | StatusCode::FORBIDDEN)
}

/// Parses `bytes <first>-<last>/<total|*>`.
fn parse_content_range(value: &str) -> Option<(u64, u64, Option<u64>)> {
    let (range, total) = value.strip_prefix("bytes ")?.split_once('/')?;
    let (first, last) = range.split_once('-')?;
    let total = match total {
        "*" => None,
        total => Some(total.parse().ok()?),
    };
    Some((first.parse().ok()?, last.parse().ok()?, total))
}
