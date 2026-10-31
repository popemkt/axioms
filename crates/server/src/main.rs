//! `axioms-server`: the JSON API and the static web UI on one port.
//!
//! - `AXIOMS_ADDR` (default `127.0.0.1:4401`)
//! - `AXIOMS_DATA`: the library file (see `axioms_store::default_path`)
//! - `AXIOMS_STORE=jsonl|kb`, `AXIOMS_KB_ROOT`: keep it in a kb store instead
//!   (see `axioms_store::open`)
//! - `AXIOMS_WEB`: the web UI directory (default: this repo's `web/`)
//! - provider settings: see `axioms_agent::provider::from_env`

use axioms_agent::{Learner, LlmSource, provider};
use axioms_server::{api, state};
use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();
    let addr = std::env::var("AXIOMS_ADDR").unwrap_or_else(|_| "127.0.0.1:4401".into());
    let web = std::env::var("AXIOMS_WEB")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/../../web").into());
    let store: Arc<dyn axioms_store::Store> =
        axioms_store::open(axioms_store::default_path())?.into();
    let lib = store.load()?;
    let learner = Learner::new(Arc::new(LlmSource::new(provider::from_env()?)));
    tracing::info!(store = %store.describe(), provider = %learner.label(), concepts = lib.graph.len(), "starting");
    let state = Arc::new(state::AppState::new(lib, store, learner));
    // Revalidate static files on every load, so a redeploy is seen at once.
    let web = tower::ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache"),
        ))
        .service(ServeDir::new(web));
    let app = api::router(state)
        .fallback_service(web)
        .layer(TraceLayer::new_for_http());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
