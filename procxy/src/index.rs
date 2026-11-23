//! Pass-through of the crates.io sparse index.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};

use crate::Error;

/// Relays `GET /index/{*path}` (e.g. `se/rd/serde` or `config.json`) to
/// index.crates.io.
///
/// The request's `If-None-Match` and `If-Modified-Since` headers are
/// forwarded. The response is relayed whatever its status (200, 304, 404,
/// ...), with its `Content-Type`, `ETag`, `Last-Modified` and `Cache-Control`
/// headers and its body streamed. Nothing is stored.
///
/// # Errors
///
/// - [`Error::BadRequest`] if `path` has an empty, `.` or `..` segment, or a
///   character other than ASCII letters, digits, `-`, `_`, `.` and `/`.
/// - [`Error::Upstream`] if no response was obtained; this is logged here.
pub async fn proxy(
    State(client): State<reqwest::Client>,
    Path(path): Path<String>,
    request_headers: HeaderMap,
) -> Result<Response, Error> {
    if !is_valid(&path) {
        return Err(Error::BadRequest(format!("invalid index path {path:?}")));
    }
    let mut request = client.get(format!("https://index.crates.io/{path}"));
    // Upstream only looks at the first `If-None-Match` line, so several
    // lines are folded into the equivalent comma-separated list.
    let validators: Vec<_> = request_headers
        .get_all(header::IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .collect();
    if !validators.is_empty() {
        request = request.header(header::IF_NONE_MATCH, validators.join(", "));
    }
    if let Some(since) = request_headers.get(header::IF_MODIFIED_SINCE) {
        request = request.header(header::IF_MODIFIED_SINCE, since);
    }
    let upstream = request.send().await.map_err(|e| {
        let error = Error::Upstream(e.to_string());
        tracing::warn!(path, %error, "upstream failure");
        error
    })?;

    let mut headers = HeaderMap::new();
    for name in [
        header::CONTENT_TYPE,
        header::ETAG,
        header::LAST_MODIFIED,
        header::CACHE_CONTROL,
    ] {
        for value in upstream.headers().get_all(&name) {
            headers.append(&name, value.clone());
        }
    }
    let status = upstream.status();
    Ok((status, headers, Body::from_stream(upstream.bytes_stream())).into_response())
}

fn is_valid(path: &str) -> bool {
    !path.is_empty()
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./".contains(&b))
}
