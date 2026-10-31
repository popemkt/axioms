//! Shared server state, the blueprint every mutating call answers with, and
//! the API error type.

use axioms_agent::Learner;
use axioms_core::{ConceptId, Library, Roadmap, View, roadmap, view};
use axioms_live::{Identity, Mirror};
use axioms_store::Store;
use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::sync::Semaphore;

pub struct AppState {
    pub lib: Mutex<Library>,
    pub store: Arc<dyn Store>,
    pub learner: Learner,
    /// Bounds concurrent model calls: the host is kept gentle.
    pub model_calls: Semaphore,
    /// Set when the library lives in SpacetimeDB: reads come from the
    /// mirror, writes go from the browser straight to reducers.
    pub live: Option<LiveState>,
}

pub struct LiveState {
    pub mirror: Arc<Mirror>,
    /// The SpacetimeDB address browsers connect to; empty means this
    /// server's own origin, which relays to `upstream` (see `relay`).
    pub public_uri: String,
    /// The SpacetimeDB address this server reaches, and the database.
    pub upstream: String,
    pub db: String,
}

/// A goal's view and roadmap: what the UI draws.
#[derive(Serialize)]
pub struct Blueprint {
    pub view: View,
    pub roadmap: Roadmap,
}

impl AppState {
    pub fn new(lib: Library, store: Arc<dyn Store>, learner: Learner) -> Self {
        AppState {
            lib: Mutex::new(lib),
            store,
            learner,
            model_calls: Semaphore::new(2),
            live: None,
        }
    }

    pub fn with_live(mut self, live: LiveState) -> Self {
        self.live = Some(live);
        self
    }

    /// The library as `who` sees it: in live mode the shared graph with
    /// their knowledge, else this server's own library.
    pub fn read(&self, who: Option<Identity>) -> Result<Library, ApiError> {
        match &self.live {
            None => Ok(self.lib().clone()),
            Some(l) => {
                let live = l.mirror.get().ok_or_else(ApiError::unavailable)?;
                Ok(axioms_live::rows::library(&live, who))
            }
        }
    }

    /// Refuse a write in live mode, where writes are reducer calls.
    pub fn local(&self) -> Result<(), ApiError> {
        match self.live {
            None => Ok(()),
            Some(_) => Err(ApiError(
                StatusCode::CONFLICT,
                "this server is live: write through SpacetimeDB reducers".into(),
            )),
        }
    }

    pub fn lib(&self) -> MutexGuard<'_, Library> {
        self.lib.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn blueprint(&self, goal: &ConceptId) -> Result<Blueprint, ApiError> {
        blueprint(&self.lib(), goal)
    }

    pub fn save(&self) -> Result<(), ApiError> {
        let lib = self.lib();
        self.store.save(&lib).map_err(ApiError::from)
    }
}

/// A goal's blueprint in a library: the core view and roadmap.
pub fn blueprint(lib: &Library, goal: &ConceptId) -> Result<Blueprint, ApiError> {
    let view = view(&lib.graph, goal).ok_or_else(|| ApiError::not_found(goal))?;
    let roadmap = roadmap(&view);
    Ok(Blueprint { view, roadmap })
}

pub struct ApiError(StatusCode, String);

impl ApiError {
    pub fn not_found(id: &ConceptId) -> Self {
        ApiError(StatusCode::NOT_FOUND, format!("no concept `{id}`"))
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        ApiError(StatusCode::BAD_REQUEST, msg.into())
    }

    pub fn unavailable() -> Self {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "connecting to SpacetimeDB…".into(),
        )
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!("{e:#}");
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}
