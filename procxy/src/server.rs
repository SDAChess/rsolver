//! The HTTP API: routes, JSON shapes and how errors are presented.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeStruct};

use crate::{Error, index, manifest::Metadata, service::Service};

const MAX_BATCH: usize = 4096;
const BATCH_CONCURRENCY: usize = 64;

/// The HTTP API: index pass-through plus the metadata routes.
///
/// - `GET /index/{*path}`: the sparse index, see [`index::proxy`].
/// - `GET /v1/crates/{name}/{version}`: one item, with the status of its
///   error if it failed.
/// - `POST /v1/crates` with `{"crates": [{"name", "version"}, ...]}`:
///   `{"crates": [item, ...]}` for up to 4096 versions, in request order.
///   The status is 200 whether or not items failed; a longer list is refused
///   as a whole with 400.
///
/// A successful item is `{"name", "version", "license", "license_file",
/// "proc_macro"}`; a failed one has `"error": {"kind", "message"}` in place
/// of the last three. A request refused as a whole is answered with only
/// `{"error": {"kind", "message"}}`.
///
/// Each error `kind` has one status:
///
/// - `bad_request`, 400: a malformed name, version or path, or a batch that
///   is too long.
/// - `not_found`, 404: crates.io has no such crate version.
/// - `no_manifest` and `invalid_manifest`, 422: the crate has no usable
///   `Cargo.toml`.
/// - `upstream`, 502: crates.io failed; the next request tries again.
/// - `store`, 500: the database failed.
pub fn router(service: Arc<Service>, client: reqwest::Client) -> Router {
    let index = Router::new()
        .route("/index/{*path}", get(index::proxy))
        .with_state(client);
    let crates = Router::new()
        .route("/v1/crates/{name}/{version}", get(one))
        .route("/v1/crates", post(batch))
        .with_state(service);
    index.merge(crates)
}

async fn one(
    State(service): State<Arc<Service>>,
    Path((name, version)): Path<(String, String)>,
) -> Response {
    let item = Item::lookup(&service, name, version).await;
    let status = match &item.outcome {
        Outcome::Metadata(_) => StatusCode::OK,
        Outcome::Error { error } => describe(error).0,
    };
    (status, Json(item)).into_response()
}

#[derive(Deserialize)]
struct BatchRequest {
    crates: Vec<CrateRef>,
}

#[derive(Deserialize)]
struct CrateRef {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct BatchResponse {
    crates: Vec<Item>,
}

async fn batch(
    State(service): State<Arc<Service>>,
    Json(request): Json<BatchRequest>,
) -> Result<Json<BatchResponse>, Error> {
    if request.crates.len() > MAX_BATCH {
        return Err(Error::BadRequest(format!(
            "more than {MAX_BATCH} crates in one request"
        )));
    }
    let crates = stream::iter(request.crates)
        .map(|CrateRef { name, version }| Item::lookup(&service, name, version))
        .buffered(BATCH_CONCURRENCY)
        .collect()
        .await;
    Ok(Json(BatchResponse { crates }))
}

#[derive(Serialize)]
struct Item {
    name: String,
    version: String,
    #[serde(flatten)]
    outcome: Outcome,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Outcome {
    Metadata(Metadata),
    Error { error: Error },
}

impl Item {
    async fn lookup(service: &Arc<Service>, name: String, version: String) -> Self {
        let outcome = match service.metadata(&name, &version).await {
            Ok(metadata) => Outcome::Metadata(metadata),
            Err(error) => Outcome::Error { error },
        };
        Self {
            name,
            version,
            outcome,
        }
    }
}

/// The HTTP status of an error and its stable machine-readable name.
fn describe(error: &Error) -> (StatusCode, &'static str) {
    match error {
        Error::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
        Error::NotFound => (StatusCode::NOT_FOUND, "not_found"),
        Error::NoManifest => (StatusCode::UNPROCESSABLE_ENTITY, "no_manifest"),
        Error::InvalidManifest(_) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_manifest"),
        Error::Upstream(_) => (StatusCode::BAD_GATEWAY, "upstream"),
        Error::Store(_) => (StatusCode::INTERNAL_SERVER_ERROR, "store"),
    }
}

/// In JSON an error is `{"kind", "message"}`.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut json = serializer.serialize_struct("Error", 2)?;
        json.serialize_field("kind", describe(self).1)?;
        json.serialize_field("message", &self.to_string())?;
        json.end()
    }
}

/// A request refused as a whole, with no crate version to report the error
/// under: `{"error": {...}}`.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (describe(&self).0, Json(Outcome::Error { error: self })).into_response()
    }
}
